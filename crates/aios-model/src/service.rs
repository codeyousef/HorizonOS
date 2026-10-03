//! One inference worker; authenticated connection-owned requests. No execution API.
use crate::{ArtifactTrust, Cancellation, Model, protocol::{Generation, Operation, Request, parse_operation}};
use aios_protocol::{MAX_TASK_BYTES, contracts::ErrorCode, write_frame};
use serde_json::{Value, json};
use std::{collections::{HashMap,VecDeque},fs::File,io,
    os::{fd::{AsRawFd,FromRawFd},unix::net::{UnixListener,UnixStream}},path::PathBuf,
    sync::{Arc,Condvar,Mutex,atomic::{AtomicU8,Ordering}},thread,time::{Duration,Instant}};
use uuid::Uuid;

const QUEUE_LIMIT: usize = 8;
const UID_REQUEST_LIMIT: usize = 2;
const RECORD_LIMIT: usize = 64;
const IDLE: Duration = Duration::from_secs(600);
const RETENTION: Duration = Duration::from_secs(30);

pub(crate) fn wipe(bytes: &mut [u8]) {
    for byte in bytes { unsafe { std::ptr::write_volatile(byte,0); } }
    std::sync::atomic::compiler_fence(Ordering::SeqCst);
}
fn wipe_text(text: &mut String) { unsafe { wipe(text.as_bytes_mut()); } }
struct Secret(String);
impl Drop for Secret { fn drop(&mut self) { wipe_text(&mut self.0); } }


#[derive(Debug,Clone,Copy,PartialEq,Eq)]
struct Owner { uid:u32,gid:u32,pid:i32,connection:Uuid }
struct Peer { owner:Owner,pidfd:File }
impl Peer {
    fn authenticate(stream:&UnixStream)->io::Result<Self> {
        let mut credentials:nix::libc::ucred=unsafe {std::mem::zeroed()};
        let mut size=std::mem::size_of_val(&credentials) as nix::libc::socklen_t;
        let status=unsafe {nix::libc::getsockopt(stream.as_raw_fd(),nix::libc::SOL_SOCKET,nix::libc::SO_PEERCRED,
            (&mut credentials as *mut nix::libc::ucred).cast(),&mut size)};
        if status!=0 || size as usize!=std::mem::size_of_val(&credentials) || credentials.pid<=1 {return Err(io::Error::new(io::ErrorKind::PermissionDenied,"peer credentials unavailable"));}
        let mut descriptor=-1;let mut size=std::mem::size_of_val(&descriptor) as nix::libc::socklen_t;
        // This kernel-provided handle binds the actual connection origin even
        // under ProtectProc=invisible and after PID reuse. No /proc peer reads.
        let status=unsafe {nix::libc::getsockopt(stream.as_raw_fd(),nix::libc::SOL_SOCKET,nix::libc::SO_PEERPIDFD,
            (&mut descriptor as *mut i32).cast(),&mut size)};
        if status!=0 || descriptor<0 || size as usize!=std::mem::size_of_val(&descriptor) {return Err(io::Error::new(io::ErrorKind::PermissionDenied,"peer process handle unavailable"));}
        let pidfd=unsafe {File::from_raw_fd(descriptor)};
        let peer=Self {owner:Owner {uid:credentials.uid,gid:credentials.gid,pid:credentials.pid,connection:Uuid::new_v4()},pidfd};
        peer.verify()?;Ok(peer)
    }
    fn verify(&self)->io::Result<()> {
        let mut descriptor=nix::libc::pollfd {fd:self.pidfd.as_raw_fd(),events:nix::libc::POLLIN,revents:0};
        let result=unsafe {nix::libc::poll(&mut descriptor,1,0)};
        if result!=0 {return Err(io::Error::new(io::ErrorKind::PermissionDenied,"origin process ended"));}Ok(())
    }
}

fn pass_credentials(fd:i32)->io::Result<()> {
    let value:i32=1;
    if unsafe {nix::libc::setsockopt(fd,nix::libc::SOL_SOCKET,nix::libc::SO_PASSCRED,
        (&value as *const i32).cast(),std::mem::size_of_val(&value) as nix::libc::socklen_t)}!=0 {return Err(io::Error::last_os_error());}Ok(())
}
fn authenticated_read(stream:&UnixStream,peer:&Peer,mut bytes:&mut [u8])->io::Result<bool> {
    let mut total=0;
    while !bytes.is_empty() {
        peer.verify()?;
        let mut control=[0_usize;32];
        let mut vector=nix::libc::iovec {iov_base:bytes.as_mut_ptr().cast(),iov_len:bytes.len()};
        let mut message:nix::libc::msghdr=unsafe {std::mem::zeroed()};
        message.msg_iov=&mut vector;message.msg_iovlen=1;
        message.msg_control=control.as_mut_ptr().cast();message.msg_controllen=std::mem::size_of_val(&control);
        let count=unsafe {nix::libc::recvmsg(stream.as_raw_fd(),&mut message,nix::libc::MSG_CMSG_CLOEXEC)};
        if count<0 {let error=io::Error::last_os_error();if error.kind()==io::ErrorKind::Interrupted {continue;}return Err(error);}
        if count==0 {if total==0 {return Ok(false);}return Err(io::Error::new(io::ErrorKind::UnexpectedEof,"truncated frame"));}
        let mut valid=false;let mut unexpected=message.msg_flags & nix::libc::MSG_CTRUNC != 0;
        unsafe {
            let mut item=nix::libc::CMSG_FIRSTHDR(&message);
            while !item.is_null() {
                let header=&*item;let base=nix::libc::CMSG_LEN(0) as usize;
                if header.cmsg_level==nix::libc::SOL_SOCKET && header.cmsg_type==nix::libc::SCM_CREDENTIALS &&
                    header.cmsg_len==nix::libc::CMSG_LEN(std::mem::size_of::<nix::libc::ucred>() as u32) as usize {
                    let credentials=std::ptr::read_unaligned(nix::libc::CMSG_DATA(item).cast::<nix::libc::ucred>());
                    valid=!valid && credentials.uid==peer.owner.uid && credentials.gid==peer.owner.gid && credentials.pid==peer.owner.pid;
                    if !valid {unexpected=true;}
                } else {
                    unexpected=true;
                    if header.cmsg_level==nix::libc::SOL_SOCKET && header.cmsg_type==nix::libc::SCM_RIGHTS && header.cmsg_len>=base {
                        let descriptors=(header.cmsg_len-base)/std::mem::size_of::<i32>();
                        for index in 0..descriptors {
                            nix::libc::close(std::ptr::read_unaligned(nix::libc::CMSG_DATA(item).cast::<i32>().add(index)));
                        }
                    }
                }
                item=nix::libc::CMSG_NXTHDR(&message,item);
            }
        }
        if !valid || unexpected {return Err(io::Error::new(io::ErrorKind::PermissionDenied,"frame sender changed or ancillary data forbidden"));}
        let count=count as usize;total+=count;bytes=&mut bytes[count..];peer.verify()?;
    }
    Ok(true)
}
fn frame(stream:&UnixStream,peer:&Peer)->io::Result<Option<Secret>> {
    let mut header=[0_u8;4];if !authenticated_read(stream,peer,&mut header)? {return Ok(None);}
    let count=u32::from_be_bytes(header) as usize;
    if count==0 || count>MAX_TASK_BYTES {return Err(io::Error::new(io::ErrorKind::InvalidData,"invalid frame length"));}
    let mut bytes=vec![0;count];
    if let Err(error)=authenticated_read(stream,peer,&mut bytes) {wipe(&mut bytes);return Err(error);}
    match String::from_utf8(bytes) {
        Ok(value)=>Ok(Some(Secret(value))),
        Err(error)=>{let mut bytes=error.into_bytes();wipe(&mut bytes);Err(io::Error::new(io::ErrorKind::InvalidData,"invalid UTF-8"))},
    }
}

#[derive(Clone)]
struct Control { token:Cancellation,cause:Arc<AtomicU8> }
impl Control {
    fn new()->Result<Self,ErrorCode> {Ok(Self {token:Cancellation::new()?,cause:Arc::new(AtomicU8::new(0))})}
    fn cancel(&self,cause:u8) {let _=self.cause.compare_exchange(0,cause,Ordering::SeqCst,Ordering::SeqCst);self.token.cancel();}
    fn error(&self)->Option<ErrorCode> {match self.cause.load(Ordering::SeqCst) {0=>None,2=>Some(ErrorCode::DeadlineExceeded),_=>Some(ErrorCode::Cancelled)}}
}
struct Pending { id:String,owner:Owner,generation:Generation,deadline:Instant,control:Control }
struct Record { owner:Owner,state:&'static str,error:Option<ErrorCode>,output:Option<Secret>,expires:Option<Instant>,input_tokens:u32,output_tokens:u32 }
struct State {
    queue:VecDeque<Pending>,records:HashMap<String,Record>,active:Option<(String,Control)>,
    connections:HashMap<u32,usize>,loaded:bool,unload:bool,
}
impl State {
    fn new()->Self {Self {queue:VecDeque::new(),records:HashMap::new(),active:None,connections:HashMap::new(),loaded:false,unload:false}}
    fn prune(&mut self) {let now=Instant::now();self.records.retain(|_,record|record.expires.is_none_or(|expires|expires>now));}
    fn expire_queued(&mut self) {
        let now=Instant::now();let mut keep=VecDeque::new();
        while let Some(pending)=self.queue.pop_front() {
            if pending.deadline<=now {
                pending.control.cancel(2);self.finish(&pending.id,Err(ErrorCode::DeadlineExceeded),0,0);
            } else {keep.push_back(pending);}
        }
        self.queue=keep;
    }
    fn submit(&mut self,owner:Owner,generation:Generation)->Result<String,ErrorCode> {
        generation.validate()?;self.prune();
        let outstanding=self.records.values().filter(|r|r.owner.uid==owner.uid && r.expires.is_none()).count();
        let retained=self.records.values().filter(|r|r.owner.uid==owner.uid).count();
        if self.queue.len()>=QUEUE_LIMIT || outstanding>=UID_REQUEST_LIMIT || retained>=8 || self.records.len()>=RECORD_LIMIT {return Err(ErrorCode::ResourceExhausted);}
        let id=Uuid::new_v4().to_string();let control=Control::new()?;
        let deadline=Instant::now()+Duration::from_millis(generation.deadline_ms as u64);
        self.records.insert(id.clone(),Record {owner,state:"queued",error:None,output:None,expires:None,input_tokens:0,output_tokens:0});
        self.queue.push_back(Pending {id:id.clone(),owner,generation,deadline,control});Ok(id)
    }
    fn result(&self,owner:Owner,id:&str)->Result<Value,ErrorCode> {
        let record=self.records.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if record.owner!=owner {return Err(ErrorCode::PermissionDenied);}
        let output=record.output.as_ref().map(|value|serde_json::from_str::<Value>(&value.0)).transpose().map_err(|_|ErrorCode::ModelOutputInvalid)?;
        Ok(json!({"generation_id":id,"state":record.state,"error":record.error,"output":output,
            "input_tokens":record.input_tokens,"output_tokens":record.output_tokens,"mutation_performed":false}))
    }
    fn cancel(&mut self,owner:Owner,id:&str)->Result<Value,ErrorCode> {
        let record=self.records.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if record.owner!=owner {return Err(ErrorCode::PermissionDenied);}
        if record.expires.is_some() {return Ok(json!({"cancelled":false,"already_terminal":true}));}
        if let Some(index)=self.queue.iter().position(|p|p.id==id) {
            let pending=self.queue.remove(index).unwrap();pending.control.cancel(1);
            self.finish(id,Err(ErrorCode::Cancelled),0,0);
        } else if let Some((active,control))=&self.active {if active==id {control.cancel(1);}}
        Ok(json!({"cancelled":true,"already_terminal":false}))
    }
    fn finish(&mut self,id:&str,result:Result<Secret,ErrorCode>,input:u32,output:u32) {
        if let Some(record)=self.records.get_mut(id) {
            match result {Ok(value)=>{record.state="completed";record.output=Some(value);},Err(error)=>{record.state=if error==ErrorCode::Cancelled {"cancelled"} else {"failed"};record.error=Some(error);}}
            record.expires=Some(Instant::now()+RETENTION);record.input_tokens=input;record.output_tokens=output;
        }
    }
    fn disconnect(&mut self,owner:Owner) {
        self.queue.retain(|p|p.owner!=owner);
        if let Some((id,control))=&self.active {if self.records.get(id).is_some_and(|r|r.owner==owner) {control.cancel(1);}}
        self.records.retain(|_,record|record.owner!=owner);
        if let Some(count)=self.connections.get_mut(&owner.uid) {*count-=1;if *count==0 {self.connections.remove(&owner.uid);}}
    }
}
type Shared=Arc<(Mutex<State>,Condvar)>;
pub struct Config {pub model_directory:PathBuf,pub qualification:bool}

fn restrict_execution()->io::Result<()> {
    // Install before creating threads: they inherit the filter. Inference has
    // no reason to execute a program, including through the x32 syscall ABI.
    let statement=|code,k|nix::libc::sock_filter {code,jt:0,jf:0,k};
    let jump=|k,jt,jf|nix::libc::sock_filter {code:0x15,jt,jf,k};
    let deny=0x00050000 | nix::libc::EPERM as u32;
    let filter=[statement(0x20,4),jump(0xc000003e,1,0),statement(0x06,0x80000000),statement(0x20,0),
        nix::libc::sock_filter {code:0x45,jt:0,jf:1,k:0x40000000},statement(0x06,deny),
        jump(nix::libc::SYS_execve as u32,0,1),statement(0x06,deny),
        jump(nix::libc::SYS_execveat as u32,0,1),statement(0x06,deny),
        statement(0x06,0x7fff0000)];
    let program=nix::libc::sock_fprog {len:filter.len() as u16,filter:filter.as_ptr().cast_mut()};
    if unsafe {nix::libc::prctl(nix::libc::PR_SET_NO_NEW_PRIVS,1,0,0,0)}!=0 ||
        unsafe {nix::libc::prctl(nix::libc::PR_SET_SECCOMP,2,&program,0,0)}!=0 {return Err(io::Error::last_os_error());}
    Ok(())
}

struct Deadline {signal:Arc<(Mutex<bool>,Condvar)>,worker:Option<thread::JoinHandle<()>>}
impl Deadline {
    fn new(deadline:Instant,control:Control)->Self {
        let signal=Arc::new((Mutex::new(false),Condvar::new()));let wake=signal.clone();
        let worker=thread::spawn(move||{
            let (lock,changed)=&*wake;let mut done=lock.lock().unwrap();
            while !*done {
                let remaining=deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {control.cancel(2);break;}
                done=changed.wait_timeout(done,remaining).unwrap().0;
            }
        });Self {signal,worker:Some(worker)}
    }
}
impl Drop for Deadline {
    fn drop(&mut self) {let (lock,changed)=&*self.signal;*lock.lock().unwrap()=true;changed.notify_one();if let Some(worker)=self.worker.take() {let _=worker.join();}}
}
fn worker(shared:Shared,config:Config) {
    let mut model=None;let mut last=Instant::now();
    loop {
        let pending={
            let (lock,wake)=&*shared;let mut state=lock.lock().unwrap();
            loop {
                state.prune();state.expire_queued();
                if state.unload || (model.is_some() && last.elapsed()>=IDLE && state.queue.is_empty()) {
                    drop(model.take());state.loaded=false;state.unload=false;
                }
                if let Some(pending)=state.queue.pop_front() {
                    state.active=Some((pending.id.clone(),pending.control.clone()));
                    if let Some(record)=state.records.get_mut(&pending.id) {record.state="running";}
                    break pending;
                }
                let remaining=if model.is_some() {IDLE.saturating_sub(last.elapsed()).min(RETENTION)} else {RETENTION};
                state=wake.wait_timeout(state,remaining).unwrap().0;
            }
        };
        let mut input_tokens=0;let mut output_tokens=0;
        let deadline=Deadline::new(pending.deadline,pending.control.clone());
        let result=(||{
            if Instant::now()>=pending.deadline {pending.control.cancel(2);}
            if let Some(error)=pending.control.error() {return Err(error);}
            if model.is_none() {
                model=Some(Model::load_cancelable(&config.model_directory,
                    if config.qualification {ArtifactTrust::Qualification} else {ArtifactTrust::Production},Some(&pending.control.token))?);
                shared.0.lock().unwrap().loaded=true;
            }
            let model=model.as_ref().unwrap();let mut context=model.context(pending.control.token.clone())?;
            let grammar=pending.generation.grammar()?;
            let prompt=model.prompt(&pending.generation.system_prompt,&pending.generation.user_prompt)?;
            input_tokens=context.evaluate(prompt,&grammar)?;
            let output=Secret(context.generate(pending.generation.output_budget(),pending.deadline,|_|output_tokens+=1)?);
            pending.generation.parse_output(&output.0)?;
            Ok(output)
        })();
        drop(deadline);
        let result=match pending.control.error() {Some(error)=>Err(error),None=>result};
        let (lock,wake)=&*shared;let mut state=lock.lock().unwrap();
        state.finish(&pending.id,result,input_tokens,output_tokens);state.active=None;last=Instant::now();wake.notify_all();
    }
}
struct Connection {shared:Shared,owner:Owner}
impl Drop for Connection {fn drop(&mut self) {let (lock,wake)=&*self.shared;if let Ok(mut state)=lock.lock() {state.disconnect(self.owner);wake.notify_all();}}}
fn connection(mut stream:UnixStream,peer:Peer,shared:Shared)->io::Result<()> {
    let _guard=Connection {shared:shared.clone(),owner:peer.owner};
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    for _ in 0..4096 {
        let Some(frame)=frame(&stream,&peer)? else {return Ok(());};
        let request:Request=serde_json::from_str(&frame.0).map_err(|_|io::Error::new(io::ErrorKind::InvalidData,"invalid request"))?;
        if request.schema_version!=1 || !Uuid::parse_str(&request.request_id).is_ok_and(|id|id.to_string()==request.request_id) {return Err(io::Error::new(io::ErrorKind::InvalidData,"invalid request version or correlation"));}
        let raw:Box<str>=request.operation.into();
        let raw=Secret(raw.into_string());
        let operation=parse_operation(&raw.0);
        let (lock,wake)=&*shared;let result={
            let mut state=lock.lock().map_err(|_|io::Error::other("model worker unavailable"))?;state.prune();
            match operation {
                Ok(Operation::Generate(generation))=>state.submit(peer.owner,generation).map(|id|json!({"generation_id":id,"state":"queued"})),
                Ok(Operation::GetStatus)=>Ok(json!({"loaded":state.loaded,"busy":state.active.is_some(),
                    "own_queued":state.queue.iter().filter(|p|p.owner.uid==peer.owner.uid).count(),"queue_limit":QUEUE_LIMIT,
                    "idle_unload_seconds":600,"context_tokens":8192,"maximum_input_tokens":6144,
                    "profiles":{"normal":{"configured":true,"loaded":state.loaded,"quality_qualified":false,"performance_qualified":false},"low":{"configured":false,"available":false},"high":{"configured":false,"available":false}}})),
                Ok(Operation::GetResult(id))=>state.result(peer.owner,&id),
                Ok(Operation::Cancel(id))=>state.cancel(peer.owner,&id),
                Ok(Operation::Unload) if state.active.is_none() && state.queue.is_empty()=>{state.unload=true;Ok(json!({"unload_requested":true}))},
                Ok(Operation::Unload)=>Err(ErrorCode::Conflict),Err(error)=>Err(error),
            }
        };wake.notify_all();peer.verify()?;
        let (data,error)=match result {Ok(data)=>(Some(data),None),Err(error)=>(None,Some(error))};
        let response=Secret(json!({"schema_version":1,"request_id":request.request_id,"operation":"response","data":data,"error":error}).to_string());
        write_frame(&mut stream,&response.0)?;
    }Ok(())
}
pub fn run(listener:UnixListener,config:Config)->io::Result<()> {
    restrict_execution()?;
    pass_credentials(listener.as_raw_fd())?;
    let shared=Arc::new((Mutex::new(State::new()),Condvar::new()));let work=shared.clone();
    let deadlines=shared.clone();
    thread::spawn(move||{
        let (lock,wake)=&*deadlines;let mut state=lock.lock().unwrap();
        loop {
            state.prune();state.expire_queued();
            let remaining=state.queue.iter().map(|pending|pending.deadline.saturating_duration_since(Instant::now())).min().unwrap_or(RETENTION).min(RETENTION);
            state=wake.wait_timeout(state,remaining).unwrap().0;
        }
    });
    thread::spawn(move||{
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(||worker(work,config))).is_err() {
            eprintln!("aios-modeld: model worker crashed");std::process::exit(1);
        }
    });
    println!("AIOS_MODELD_READY");
    for stream in listener.incoming() {
        let stream=stream?;pass_credentials(stream.as_raw_fd())?;
        let Ok(peer)=Peer::authenticate(&stream) else {continue;};
        let mut state=shared.0.lock().map_err(|_|io::Error::other("model worker unavailable"))?;
        let count=state.connections.get(&peer.owner.uid).copied().unwrap_or(0);
        if count>=4 || state.connections.values().sum::<usize>()>=32 {continue;}
        state.connections.insert(peer.owner.uid,count+1);drop(state);
        let shared=shared.clone();thread::spawn(move||{let _=connection(stream,peer,shared);});
    }Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Profile,ResponseMode};
    fn owner(uid:u32)->Owner {Owner {uid,gid:uid,pid:100+uid as i32,connection:Uuid::new_v4()}}
    fn generation()->Generation {Generation {profile:Profile::Normal,system_prompt:"system".into(),user_prompt:"private fixture".into(),
        response_mode:ResponseMode::FinalAnswer,allowed_tools:vec![],evidence_ids:vec![],deadline_ms:90000}}
    #[test] fn inference_threads_cannot_execute_programs() {
        thread::spawn(||{
            restrict_execution().unwrap();
            let file=c"/inference-must-never-execute";let arguments=[file.as_ptr(),std::ptr::null()];let environment=[std::ptr::null()];
            assert_eq!(unsafe {nix::libc::execve(file.as_ptr(),arguments.as_ptr(),environment.as_ptr())},-1);
            assert_eq!(io::Error::last_os_error().raw_os_error(),Some(nix::libc::EPERM));
            assert_eq!(unsafe {nix::libc::syscall(0x40000000_i64+520,std::ptr::null::<u8>(),std::ptr::null::<u8>(),std::ptr::null::<u8>())},-1);
            assert_eq!(io::Error::last_os_error().raw_os_error(),Some(nix::libc::EPERM));
        }).join().unwrap();
    }
    #[test] fn queue_and_uid_quotas_and_cross_owner_cancellation_are_bounded() {
        let mut state=State::new();let owner_one=owner(1000);let other=owner(1001);
        let first=state.submit(owner_one,generation()).unwrap();state.submit(owner_one,generation()).unwrap();
        assert_eq!(state.submit(owner_one,generation()),Err(ErrorCode::ResourceExhausted));
        assert_eq!(state.result(other,&first),Err(ErrorCode::PermissionDenied));assert_eq!(state.cancel(other,&first),Err(ErrorCode::PermissionDenied));
        for uid in 1002..1008 {state.submit(owner(uid),generation()).unwrap();}
        assert_eq!(state.queue.len(),8);assert_eq!(state.submit(other,generation()),Err(ErrorCode::ResourceExhausted));
        state.cancel(owner_one,&first).unwrap();assert_eq!(state.queue.len(),7);
        assert_eq!(state.result(owner_one,&first).unwrap()["error"],"CANCELLED");
        state.disconnect(owner_one);assert!(!state.records.values().any(|r|r.owner==owner_one));assert_eq!(state.queue.len(),6);
    }
    #[test] fn retention_and_deadline_use_original_request_lifecycle() {
        assert_eq!(IDLE.as_secs(),600);let mut state=State::new();let owner=owner(1000);
        let id=state.submit(owner,generation()).unwrap();let queued=state.queue.pop_front().unwrap();
        assert!(queued.deadline.saturating_duration_since(Instant::now())<=Duration::from_secs(90));
        let deadline=Deadline::new(Instant::now(),queued.control.clone());
        thread::sleep(Duration::from_millis(10));drop(deadline);
        assert_eq!(queued.control.error(),Some(ErrorCode::DeadlineExceeded));assert!(queued.control.token.is_cancelled());
        state.finish(&id,Err(ErrorCode::DeadlineExceeded),0,0);state.records.get_mut(&id).unwrap().expires=Some(Instant::now());state.prune();assert!(state.records.is_empty());
        let id=state.submit(owner,generation()).unwrap();state.queue.front_mut().unwrap().deadline=Instant::now();
        state.expire_queued();assert!(state.queue.is_empty());assert_eq!(state.result(owner,&id).unwrap()["error"],"DEADLINE_EXCEEDED");
    }
    #[test] fn every_received_chunk_is_bound_to_kernel_origin() {
        use std::io::Write;
        let (mut client,server)=UnixStream::pair().unwrap();pass_credentials(server.as_raw_fd()).unwrap();
        let peer=Peer::authenticate(&server).unwrap();
        client.write_all(&[1,2,3,4]).unwrap();let mut bytes=[0;4];authenticated_read(&server,&peer,&mut bytes).unwrap();assert_eq!(bytes,[1,2,3,4]);
        let mut other=Peer::authenticate(&server).unwrap();other.owner.pid+=1;
        client.write_all(&[1]).unwrap();assert!(authenticated_read(&server,&other,&mut [0]).is_err());
    }
}
