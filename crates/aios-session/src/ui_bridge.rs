//! Private fixed-broker/native-provider bridge. The first kernel message
//! carries the originating client FD or a native unique-sender reference.
//! References are resolved on the authenticated native bus, never trusted as
//! serialized caller credentials, and only the fixed broker can hand them off.
use crate::{managed_service::{ManagedService,Role},ui_read::{OriginatingClient,NativeReadTask,NativeReadStop},
    display::DisplayBinding,accessibility::WindowBinding};
use aios_protocol::{read_frame_with_limit,write_frame,MAX_TASK_BYTES,MAX_FRAME_BYTES,contracts::ErrorCode};
use nix::sys::socket::{sendmsg,recvmsg,ControlMessage,ControlMessageOwned,MsgFlags};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json,value::RawValue};
use std::{collections::HashMap,fs,io::{IoSlice,IoSliceMut,Read},net::Shutdown,os::{fd::{AsRawFd,OwnedFd,FromRawFd,RawFd},unix::{fs::{MetadataExt,FileTypeExt},net::UnixStream}},
    path::PathBuf,sync::{Arc,Mutex,atomic::{AtomicU8,AtomicUsize,Ordering}},time::Duration};
type Result<T> = std::result::Result<T,ErrorCode>;
const MARKER:u8=0xa7;
const BUS_MARKER:u8=0xa8;
enum TransferredProof { Unix(UnixStream),Bus }
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct BusReference { schema_version:u32,sender:String,bus_id:String }
static ACTIVE_READS:AtomicUsize=AtomicUsize::new(0);
struct ReadAdmission;
impl Drop for ReadAdmission{fn drop(&mut self){ACTIVE_READS.fetch_sub(1,Ordering::AcqRel);}}
fn send_proof(bridge:&UnixStream,origin:&UnixStream)->Result<()>{
    let fds=[origin.as_raw_fd()];
    let sent=sendmsg::<()>(bridge.as_raw_fd(),&[IoSlice::new(&[MARKER])],&[ControlMessage::ScmRights(&fds)],MsgFlags::MSG_NOSIGNAL,None)
        .map_err(|_|ErrorCode::TargetChanged)?;
    if sent!=1{return Err(ErrorCode::TargetChanged);}Ok(())
}
fn receive_proof(bridge:&UnixStream)->Result<TransferredProof>{
    let mut byte=[0u8];let mut io=[IoSliceMut::new(&mut byte)];
    // Linux permits at most 253 rights per message. Receive enough ancillary
    // space to own/close *all* unexpected FDs before rejecting multiplicity.
    let mut ancillary=nix::cmsg_space!([RawFd;253]);
    let message=recvmsg::<()>(bridge.as_raw_fd(),&mut io,Some(&mut ancillary),MsgFlags::MSG_CMSG_CLOEXEC)
        .map_err(|_|ErrorCode::PermissionDenied)?;
    let mut descriptors=Vec::<OwnedFd>::new();let mut other=false;
    for control in message.cmsgs().map_err(|_|ErrorCode::PermissionDenied)? {
        match control {ControlMessageOwned::ScmRights(fds)=>for fd in fds{descriptors.push(unsafe{OwnedFd::from_raw_fd(fd)});},_=>other=true}
    }
    let valid=message.bytes==1 && !message.flags.intersects(MsgFlags::MSG_CTRUNC|MsgFlags::MSG_TRUNC) && !other;
    if !valid{return Err(ErrorCode::PermissionDenied);}
    match (byte[0],descriptors.len()) {
        (MARKER,1)=>Ok(TransferredProof::Unix(UnixStream::from(descriptors.remove(0)))),
        (BUS_MARKER,0)=>Ok(TransferredProof::Bus),_=>Err(ErrorCode::PermissionDenied),
    }
}
enum Operation {
    Discover{session_id:String},StartRead{window_handle:String,goal:String,mode:String},
    StartTaskRead{window_handle:String,goal:String,mode:String,task_id:String},
    GetReadStatus{task_id:String},TakeSnapshot{task_id:String},Cancel{task_id:String},Forget{task_id:String},
}
fn parse(raw:&str)->Result<Operation>{
    #[derive(Deserialize)]struct Kind{kind:String}
    let kind:Kind=serde_json::from_str(raw).map_err(|_|ErrorCode::InvalidArgument)?;
    macro_rules! fields {($variant:ident {$($field:ident:$type:ty),*})=>{{
        #[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Fields{kind:String,$($field:$type),*}
        let value:Fields=serde_json::from_str(raw).map_err(|_|ErrorCode::InvalidArgument)?;
        if value.kind!=kind.kind{return Err(ErrorCode::InvalidArgument);}Ok(Operation::$variant{$($field:value.$field),*})
    }};}
    match kind.kind.as_str(){
        "discover"=>fields!(Discover{session_id:String}),"start_read"=>fields!(StartRead{window_handle:String,goal:String,mode:String}),
        "start_task_read"=>fields!(StartTaskRead{window_handle:String,goal:String,mode:String,task_id:String}),
        "get_read_status"=>fields!(GetReadStatus{task_id:String}),"take_snapshot"=>fields!(TakeSnapshot{task_id:String}),
        "cancel"=>fields!(Cancel{task_id:String}),"forget"=>fields!(Forget{task_id:String}),_=>Err(ErrorCode::InvalidArgument),
    }
}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Request{schema_version:u32,request_id:String,operation:Box<RawValue>}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct Bound{schema_version:u32,request_id:String,operation:String,origin_sha256:String}
#[derive(Serialize,Deserialize)]#[serde(deny_unknown_fields)]
struct Response{schema_version:u32,request_id:String,operation:String,data:Option<Value>,error:Option<ErrorCode>}
#[derive(Clone,Serialize)]
struct ReadStatus{schema_version:u32,task_id:String,state:String,error:Option<ErrorCode>,snapshot_ready:bool}
struct ReadWork{status:ReadStatus,control:Arc<AtomicU8>,stop:Option<NativeReadStop>,snapshot:Option<Value>,window:WindowBinding,deadline:u64}
impl ReadWork {
    fn cancel(&mut self){self.control.store(1,Ordering::Release);if let Some(stop)=&self.stop{stop.stop();}
        self.snapshot.take();self.status.snapshot_ready=false;self.status.state="cancelled".into();self.status.error=Some(ErrorCode::Cancelled);}
}
struct Context{origin:OriginatingClient,windows:HashMap<String,(WindowBinding,u64)>,tasks:HashMap<String,Arc<Mutex<ReadWork>>>}
impl Drop for Context{fn drop(&mut self){for task in self.tasks.values(){if let Ok(mut task)=task.lock(){task.cancel();}}}}
impl Context{
    fn execute(&mut self,operation:Operation,cancellation:Option<UnixStream>)->Result<Value>{
        self.origin.verify()?;
        let now=aios_policy::boottime_ms()?;
        self.windows.retain(|_,(_,expires)|*expires>now);
        self.tasks.retain(|_,task|{if let Ok(mut task)=task.lock(){if task.deadline<=now{task.cancel();return false;}}true});
        match operation {
            Operation::Discover{session_id}=>{
                let display=DisplayBinding::observe(&session_id,self.origin.uid())?;
                let windows=WindowBinding::discover(&display,&AtomicU8::new(0))?;
                self.origin.verify()?;
                let metadata=windows.iter().map(|w|Ok(json!({"window_handle":w.handle,"name":w.name,"title":w.title,"identity_sha256":w.identity_sha256()?})))
                    .collect::<Result<Vec<_>>>()?;
                let expires=aios_policy::boottime_ms()?.checked_add(30_000).ok_or(ErrorCode::TargetChanged)?;
                self.windows=windows.into_iter().map(|w|(w.handle.clone(),(w,expires))).collect();
                Ok(json!({"schema_version":1,"operation":"ui_window_candidates","session_id":session_id,"windows":metadata,
                    "expires_after_ms":30000,"confirmation_required":true,"ui_authorized":false}))
            },
            operation @ (Operation::StartRead{..}|Operation::StartTaskRead{..})=>{
                let (window_handle,goal,mode,id,public)=match operation {
                    Operation::StartRead{window_handle,goal,mode}=>(window_handle,goal,mode,uuid::Uuid::new_v4().to_string(),false),
                    Operation::StartTaskRead{window_handle,goal,mode,task_id}=>{
                        if !crate::uuid(&task_id) || cancellation.is_none(){return Err(ErrorCode::InvalidArgument);}
                        (window_handle,goal,mode,task_id,true)
                    },_=>unreachable!(),
                };
                if self.tasks.contains_key(&id){return Err(ErrorCode::Conflict);}
                if self.tasks.len()>=8{return Err(ErrorCode::ResourceExhausted);}
                let mode=match mode.as_str(){"ask"=>aios_policy::Mode::Ask,"diagnose"=>aios_policy::Mode::Diagnose,_=>return Err(ErrorCode::PermissionDenied)};
                if goal.trim().is_empty() || goal.len()>4096{return Err(ErrorCode::InvalidArgument);}
                let window=self.windows.get(&window_handle).ok_or(ErrorCode::TargetNotFound)?.0.clone();
                let origin=self.origin.try_clone()?;
                if ACTIVE_READS.fetch_add(1,Ordering::AcqRel)>=4{ACTIVE_READS.fetch_sub(1,Ordering::AcqRel);return Err(ErrorCode::ResourceExhausted);}
                let admission=ReadAdmission;
                let control=Arc::new(AtomicU8::new(0));
                let status=ReadStatus{schema_version:1,task_id:id.clone(),state:"queued".into(),error:None,snapshot_ready:false};
                let work=Arc::new(Mutex::new(ReadWork{status:status.clone(),control:control.clone(),stop:None,snapshot:None,
                    window:window.clone(),deadline:aios_policy::boottime_ms()?.checked_add(90_000).ok_or(ErrorCode::TargetChanged)?}));
                self.tasks.insert(id.clone(),work.clone());
                if let Some(receiver)=cancellation{let work=work.clone();std::thread::spawn(move||watch_cancellation(receiver,work));}
                std::thread::spawn(move||{let _admission=admission;run_read(work,origin,window,goal,mode,control,id,public);});
                serde_json::to_value(status).map_err(|_|ErrorCode::InvalidArgument)
            },
            Operation::GetReadStatus{task_id}=>{
                let task=self.task(&task_id)?.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                serde_json::to_value(&task.status).map_err(|_|ErrorCode::InvalidArgument)
            },
            Operation::TakeSnapshot{task_id}=>{
                let task=self.task(&task_id)?.clone();
                let window=task.lock().map_err(|_|ErrorCode::ResourceExhausted)?.window.clone();
                window.verify()?;self.origin.verify()?;
                let mut task=task.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                if task.control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
                if task.status.state!="completed"{return Err(task.status.error.unwrap_or(ErrorCode::AuthRequired));}
                let snapshot=task.snapshot.take().ok_or(ErrorCode::TargetNotFound)?;task.status.snapshot_ready=false;
                Ok(snapshot)
            },
            Operation::Cancel{task_id}=>{self.task(&task_id)?.lock().map_err(|_|ErrorCode::ResourceExhausted)?.cancel();Ok(json!({"task_id":task_id,"cancelled":true}))},
            Operation::Forget{task_id}=>{self.task(&task_id)?.lock().map_err(|_|ErrorCode::ResourceExhausted)?.cancel();self.tasks.remove(&task_id);Ok(json!({"task_id":task_id,"deleted":true}))},
        }
    }
    fn task(&self,id:&str)->Result<&Arc<Mutex<ReadWork>>>{if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}self.tasks.get(id).ok_or(ErrorCode::TargetNotFound)}
}
fn watch_cancellation(mut stream:UnixStream,work:Arc<Mutex<ReadWork>>){
    if stream.set_read_timeout(Some(Duration::from_millis(100))).is_err(){if let Ok(mut work)=work.lock(){work.cancel();}return;}
    loop {
        if let Ok(mut work)=work.lock(){
            if work.control.load(Ordering::Acquire)!=0 || (matches!(work.status.state.as_str(),"failed"|"completed") && work.snapshot.is_none()){return;}
            if aios_policy::boottime_ms().map_or(true,|now|now>=work.deadline){work.cancel();return;}
        }else{return;}
        let mut byte=[0u8];match stream.read(&mut byte){
            Err(error) if matches!(error.kind(),std::io::ErrorKind::WouldBlock|std::io::ErrorKind::TimedOut)=>continue,
            // Any byte, EOF or failure only revokes this owned read. There is
            // no approval, input or caller-selected action on this channel.
            _=>{if let Ok(mut work)=work.lock(){work.cancel();}return;},
        }
    }
}
fn run_read(work:Arc<Mutex<ReadWork>>,origin:OriginatingClient,window:WindowBinding,goal:String,mode:aios_policy::Mode,control:Arc<AtomicU8>,id:String,public:bool){
    let result=(||->Result<Value>{
        let hostname=fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_|ErrorCode::TargetChanged)?.trim().to_owned();
        if hostname.is_empty() || hostname.len()>128 || hostname.chars().any(char::is_control){return Err(ErrorCode::TargetChanged);}
        let profile=if public{"Local CPU (normal; read-only task inference)"}else{"Local CPU (observation only; no model requested)"};
        let mut task=NativeReadTask::begin_owned(origin,window,&goal,mode,hostname,profile.into(),control.clone(),id)?;
        let stop=task.stop_handle()?;
        {let mut work=work.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            if control.load(Ordering::Acquire)!=0{stop.stop();return Err(ErrorCode::Cancelled);}
            work.stop=Some(stop);work.status.state="needs_permission".into();}
        loop {
            if control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
            if task.poll_confirmation()?{break;}
            std::thread::sleep(Duration::from_millis(20));
        }
        {let mut work=work.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            if control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}work.status.state="inspecting".into();}
        serde_json::to_value(task.snapshot()?).map_err(|_|ErrorCode::InvalidArgument)
    })();
    if let Ok(mut work)=work.lock(){
        work.stop.take();
        if control.load(Ordering::Acquire)!=0{work.cancel();return;}
        match result{Ok(snapshot)=>{work.snapshot=Some(snapshot);work.status.snapshot_ready=true;work.status.state="completed".into();},
            Err(error)=>{work.status.state=if error==ErrorCode::Cancelled{"cancelled"}else{"failed"}.into();work.status.error=Some(error);}}
    }
}
pub fn serve(mut stream:UnixStream)->Result<()>{
    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
    let broker=ManagedService::authenticate(&stream,Role::Broker)?;
    let origin=match receive_proof(&stream)? {
        TransferredProof::Unix(proof)=>OriginatingClient::authenticate(proof)?,
        TransferredProof::Bus=>{
            let raw=read_frame_with_limit(&mut stream,MAX_TASK_BYTES).map_err(|_|ErrorCode::InvalidArgument)?.ok_or(ErrorCode::InvalidArgument)?;
            let reference:BusReference=serde_json::from_str(&raw).map_err(|_|ErrorCode::InvalidArgument)?;
            if reference.schema_version!=1{return Err(ErrorCode::UnsupportedSchema);}
            OriginatingClient::authenticate_bus(&reference.sender,&reference.bus_id)?
        },
    };
    broker.verify(&stream)?;
    let mut context=Context{origin,windows:HashMap::new(),tasks:HashMap::new()};
    let bound=Bound{schema_version:2,request_id:uuid::Uuid::new_v4().to_string(),operation:"bound".into(),origin_sha256:context.origin.identity_sha256()?};
    write_frame(&mut stream,&serde_json::to_string(&bound).map_err(|_|ErrorCode::InvalidArgument)?).map_err(|_|ErrorCode::TargetChanged)?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).map_err(|_|ErrorCode::TargetChanged)?;
    for _ in 0..4096{
        let Some(raw)=read_frame_with_limit(&mut stream,MAX_TASK_BYTES).map_err(|_|ErrorCode::InvalidArgument)? else{return Ok(());};
        broker.verify(&stream)?;context.origin.verify()?;
        let request:Request=serde_json::from_str(&raw).map_err(|_|ErrorCode::InvalidArgument)?;
        if !crate::uuid(&request.request_id){return Err(ErrorCode::InvalidArgument);}
        let outcome=if request.schema_version!=1{Err(ErrorCode::UnsupportedSchema)}else{parse(request.operation.get()).and_then(|op|{
            let cancellation=if matches!(op,Operation::StartTaskRead{..}){
                let TransferredProof::Unix(receiver)=receive_proof(&stream)? else{return Err(ErrorCode::PermissionDenied);};
                broker.verify_cancellation_peer(&receiver)?;Some(receiver)
            }else{None};context.execute(op,cancellation)
        })};
        broker.verify(&stream)?;context.origin.verify()?;
        let (data,error)=match outcome{Ok(value)=>(Some(value),None),Err(error)=>(None,Some(error))};
        let response=serde_json::to_string(&Response{schema_version:1,request_id:request.request_id,operation:"response".into(),data,error}).map_err(|_|ErrorCode::InvalidArgument)?;
        if response.len()>MAX_FRAME_BYTES{return Err(ErrorCode::ResourceExhausted);}
        write_frame(&mut stream,&response).map_err(|_|ErrorCode::TargetChanged)?;
    }
    Ok(())
}
pub(crate) struct Client{stream:UnixStream,provider:ManagedService}
pub(crate) struct Cancellation{stream:UnixStream}
impl Cancellation {
    pub(crate) fn pair()->Result<(Arc<Self>,UnixStream)>{
        let (stream,receiver)=UnixStream::pair().map_err(|_|ErrorCode::ResourceExhausted)?;Ok((Arc::new(Self{stream}),receiver))
    }
    pub(crate) fn cancel(&self){let _=self.stream.shutdown(Shutdown::Both);}
}
impl Drop for Cancellation{fn drop(&mut self){self.cancel();}}
impl Client {
    pub(crate) fn connect(origin:&UnixStream)->Result<Self>{
        let peer=crate::identity::authenticate(origin)?;
        Self::connect_with(Some(origin),&peer)
    }
    pub(crate) fn connect_bus(peer:&crate::identity::Peer)->Result<Self>{
        crate::identity::verify_peer(peer)?;Self::connect_with(None,peer)
    }
    fn connect_with(origin:Option<&UnixStream>,peer:&crate::identity::Peer)->Result<Self>{
        let uid=nix::unistd::geteuid().as_raw();let directory=PathBuf::from(format!("/run/user/{uid}/aios-ui"));
        let meta=fs::symlink_metadata(&directory).map_err(|_|ErrorCode::UnsupportedCapability)?;
        if !meta.is_dir() || meta.uid()!=uid || meta.mode()&0o077!=0 || directory.canonicalize().map_err(|_|ErrorCode::TargetChanged)?!=directory{return Err(ErrorCode::PermissionDenied);}
        let path=directory.join("provider.sock");let before=fs::symlink_metadata(&path).map_err(|_|ErrorCode::UnsupportedCapability)?;
        if !before.file_type().is_socket() || before.uid()!=uid || before.mode()&0o777!=0o600{return Err(ErrorCode::PermissionDenied);}
        let mut stream=UnixStream::connect(&path).map_err(|_|ErrorCode::UnsupportedCapability)?;
        stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(|_|ErrorCode::TargetChanged)?;
        stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
        let provider=ManagedService::authenticate(&stream,Role::UiProvider)?;
        let after=fs::symlink_metadata(&path).map_err(|_|ErrorCode::TargetChanged)?;
        if (before.dev(),before.ino(),before.uid(),before.mode())!=(after.dev(),after.ino(),after.uid(),after.mode()){return Err(ErrorCode::TargetChanged);}
        if let Some(origin)=origin {send_proof(&stream,origin)?;} else {
            let sender=peer.bus_sender.as_deref().ok_or(ErrorCode::PermissionDenied)?;
            let bus_id=peer.bus_id.as_deref().ok_or(ErrorCode::PermissionDenied)?;
            if peer.connection_id.is_some(){return Err(ErrorCode::PermissionDenied);}
            crate::user_bus::validate_reference(sender,bus_id)?;
            let sent=sendmsg::<()>(stream.as_raw_fd(),&[IoSlice::new(&[BUS_MARKER])],&[],MsgFlags::MSG_NOSIGNAL,None).map_err(|_|ErrorCode::TargetChanged)?;
            if sent!=1{return Err(ErrorCode::TargetChanged);}
            let reference=BusReference{schema_version:1,sender:sender.into(),bus_id:bus_id.into()};
            write_frame(&mut stream,&serde_json::to_string(&reference).map_err(|_|ErrorCode::InvalidArgument)?).map_err(|_|ErrorCode::TargetChanged)?;
        }
        let bound=read_frame_with_limit(&mut stream,MAX_TASK_BYTES).map_err(|_|ErrorCode::TargetChanged)?.ok_or(ErrorCode::TargetChanged)?;
        let bound:Bound=serde_json::from_str(&bound).map_err(|_|ErrorCode::InvalidArgument)?;
        if bound.schema_version!=2 || bound.operation!="bound" || !crate::uuid(&bound.request_id)
            || bound.origin_sha256!=crate::ui_read::origin_digest(peer)?{return Err(ErrorCode::PermissionDenied);}
        if let Some(origin)=origin {crate::identity::verify(origin,peer)?;}else{crate::identity::verify_peer(peer)?;}
        provider.verify(&stream)?;Ok(Self{stream,provider})
    }
    pub(crate) fn call(&mut self,operation:Value)->Result<Value>{
        self.call_with_cancellation(operation,None)
    }
    pub(crate) fn start_task(&mut self,operation:Value,receiver:&UnixStream)->Result<Value>{
        self.call_with_cancellation(operation,Some(receiver))
    }
    fn call_with_cancellation(&mut self,operation:Value,receiver:Option<&UnixStream>)->Result<Value>{
        self.provider.verify(&self.stream)?;
        let id=uuid::Uuid::new_v4().to_string();let frame=json!({"schema_version":1,"request_id":id,"operation":operation}).to_string();
        if frame.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted);}
        write_frame(&mut self.stream,&frame).map_err(|_|ErrorCode::TargetChanged)?;
        if let Some(receiver)=receiver{send_proof(&self.stream,receiver)?;}
        let raw=read_frame_with_limit(&mut self.stream,MAX_FRAME_BYTES).map_err(|_|ErrorCode::TargetChanged)?.ok_or(ErrorCode::TargetChanged)?;
        let response:Response=serde_json::from_str(&raw).map_err(|_|ErrorCode::InvalidArgument)?;
        self.provider.verify(&self.stream)?;
        if response.schema_version!=1 || response.request_id!=id || response.operation!="response" || response.data.is_some()==response.error.is_some(){return Err(ErrorCode::InvalidArgument);}
        match response.error{Some(error)=>Err(error),None=>response.data.ok_or(ErrorCode::InvalidArgument)}
    }
}
impl Drop for Client{fn drop(&mut self){let _=self.stream.shutdown(Shutdown::Both);}}

#[cfg(test)]mod tests;
