//! Connection-owned managed termination lifecycle. Only the authenticated
//! broker bridge enters here; this is not a model action or approval endpoint.
use crate::{display::DisplayBinding,managed_service::ManagedService,process_termination::{NativeTerminationTask,NativeTerminationStop},ui_read::OriginatingClient,SharedState};
use aios_protocol::contracts::ErrorCode;
use aios_system::processes::termination::Receipt;
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::{collections::{HashMap,HashSet},io::Read,os::unix::net::UnixStream,sync::{Arc,Mutex,atomic::{AtomicU8,AtomicUsize,Ordering}},time::Duration};
type Result<T>=std::result::Result<T,ErrorCode>;
static ACTIVE:AtomicUsize=AtomicUsize::new(0);
const MAX_ACTIVE:usize=4;
const MAX_RETAINED:usize=8;
const MAX_SEEN:usize=4096;
struct Admission<'a>(&'a AtomicUsize);
impl<'a> Admission<'a>{fn acquire(counter:&'a AtomicUsize)->Result<Self>{
    counter.fetch_update(Ordering::AcqRel,Ordering::Acquire,|count|if count<MAX_ACTIVE{Some(count+1)}else{None})
        .map_err(|_|ErrorCode::ResourceExhausted)?;Ok(Self(counter))
}}
impl Drop for Admission<'_>{fn drop(&mut self){self.0.fetch_sub(1,Ordering::AcqRel);}}

pub(crate) enum Operation{
    Start{task_id:String,process_id:String,session_id:String,goal:String,mode:String},
    Status{task_id:String},Cancel{task_id:String},Forget{task_id:String},
}
/// Closed wire shapes: no claimed credentials, numeric PID, signal choice,
/// consent result, token or generic tool call can enter this route.
pub(crate) fn parse(raw:&str)->Result<Option<Operation>>{
    #[derive(Deserialize)]struct Kind{kind:String}
    let kind:Kind=serde_json::from_str(raw).map_err(|_|ErrorCode::InvalidArgument)?;
    macro_rules! fields{($variant:ident{$($field:ident:$ty:ty),*})=>{{
        #[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Fields{kind:String,$($field:$ty),*}
        let value:Fields=serde_json::from_str(raw).map_err(|_|ErrorCode::InvalidArgument)?;
        if value.kind!=kind.kind{return Err(ErrorCode::InvalidArgument);}
        Ok(Some(Operation::$variant{$($field:value.$field),*}))
    }};}
    match kind.kind.as_str(){
        "start_process_termination"=>fields!(Start{task_id:String,process_id:String,session_id:String,goal:String,mode:String}),
        "get_process_termination"=>fields!(Status{task_id:String}),
        "cancel_process_termination"=>fields!(Cancel{task_id:String}),
        "forget_process_termination"=>fields!(Forget{task_id:String}),
        _=>Ok(None),
    }
}
#[derive(Clone,Serialize)]#[serde(deny_unknown_fields)]
struct Status{schema_version:u32,task_id:String,state:String,error:Option<ErrorCode>,receipt:Option<Receipt>}
struct Work{
    control:Arc<AtomicU8>,stop:Mutex<Option<NativeTerminationStop>>,status:Mutex<Status>,deadline:u64,
}
impl Work{
    fn new(id:String,deadline:u64)->Self{Self{control:Arc::new(AtomicU8::new(0)),stop:Mutex::new(None),
        status:Mutex::new(Status{schema_version:1,task_id:id,state:"queued".into(),error:None,receipt:None}),deadline}}
    // Never wait for process state or native I/O to latch Stop. The stop lock
    // is only used to exchange an already-created independent cancellation.
    fn cancel(&self){self.control.store(1,Ordering::Release);if let Ok(stop)=self.stop.lock(){if let Some(stop)=stop.as_ref(){stop.stop();}}}
    fn publish_stop(&self,stop:NativeTerminationStop)->Result<()>{
        let mut slot=self.stop.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        if self.control.load(Ordering::Acquire)!=0{stop.stop();}
        *slot=Some(stop);Ok(())
    }
    fn finish(&self,result:Result<Receipt>){
        if let Ok(mut status)=self.status.lock(){match result{
            Ok(receipt)=>{status.state=if receipt.complete{"completed"}else{"partial"}.into();status.error=receipt.error;status.receipt=Some(receipt);},
            Err(code)=>{status.state=if code==ErrorCode::Cancelled{"cancelled"}else{"failed"}.into();status.error=Some(code);},
        }}
        if let Ok(mut stop)=self.stop.lock(){stop.take();}
    }
}
#[derive(Default)]
pub(crate) struct Context{tasks:HashMap<String,Arc<Work>>,seen:HashSet<String>}
impl Drop for Context{fn drop(&mut self){self.cancel_all();}}
impl Context{
    pub(crate) fn cancel_all(&self){for work in self.tasks.values(){work.cancel();}}
    fn reserve(&mut self,id:&str)->Result<()>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        if self.seen.contains(id){return Err(ErrorCode::Conflict);}
        if self.tasks.len()>=MAX_RETAINED || self.seen.len()>=MAX_SEEN{return Err(ErrorCode::ResourceExhausted);}
        // Even a failed/forgotten task cannot be replayed on this connection.
        self.seen.insert(id.to_owned());Ok(())
    }
    fn task(&self,id:&str)->Result<&Arc<Work>>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        self.tasks.get(id).ok_or(ErrorCode::TargetNotFound)
    }
    pub(crate) fn execute(&mut self,operation:Operation,request_id:&str,origin:&OriginatingClient,
        broker:&ManagedService,stream:&UnixStream,state:&SharedState,cancellation:Option<UnixStream>)->Result<Value>{
        origin.verify()?;broker.verify(stream)?;
        let now=aios_policy::boottime_ms()?;
        for work in self.tasks.values(){if now>=work.deadline{work.cancel();}}
        match operation{
            Operation::Start{task_id,process_id,session_id,goal,mode}=>{
                if request_id!=task_id{return Err(ErrorCode::InvalidArgument);}
                let receiver=cancellation.ok_or(ErrorCode::PermissionDenied)?;
                if !crate::uuid(request_id) || !crate::uuid(&process_id) || goal.trim().is_empty() || goal.len()>4096
                    || session_id.is_empty() || session_id.len()>128 || session_id.chars().any(char::is_control){return Err(ErrorCode::InvalidArgument);}
                if mode!="act"{return Err(ErrorCode::PermissionDenied);}
                self.reserve(&task_id)?;
                let admission=Admission::acquire(&ACTIVE)?;
                let peer=origin.peer()?;
                let selection=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.select_process_termination(&peer,&process_id)?;
                let origin=origin.try_clone()?;let broker=broker.clone();
                let transport=stream.try_clone().map_err(|_|ErrorCode::TargetChanged)?;
                let deadline=now.checked_add(90_000).ok_or(ErrorCode::TargetChanged)?;
                let work=Arc::new(Work::new(task_id.clone(),deadline));
                let native_cancel=receiver.try_clone().map_err(|_|ErrorCode::TargetChanged)?;
                let status=serde_json::to_value(work.status.lock().map_err(|_|ErrorCode::ResourceExhausted)?.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
                let request_id=request_id.to_owned();let worker=work.clone();
                self.tasks.insert(task_id.clone(),work);
                let watcher=worker.clone();
                if std::thread::Builder::new().name("process-stop".into()).spawn(move||watch_cancellation(receiver,watcher)).is_err(){
                    if let Some(work)=self.tasks.remove(&task_id){work.cancel();}return Err(ErrorCode::ResourceExhausted);
                }
                if std::thread::Builder::new().name("process-termination".into()).spawn(move||{
                    let _admission=admission;
                    let result=(||->Result<Receipt>{
                        if worker.control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
                        broker.verify(&transport)?;
                        let display=DisplayBinding::observe(&session_id,origin.uid())?;
                        let target=std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_|ErrorCode::TargetChanged)?.trim().to_owned();
                        if target.is_empty() || target.len()>128 || target.chars().any(char::is_control){return Err(ErrorCode::TargetChanged);}
                        if aios_policy::boottime_ms()?>=worker.deadline{worker.cancel();return Err(ErrorCode::DeadlineExceeded);}
                        let mut task=NativeTerminationTask::begin(origin,selection,display,request_id,&goal,aios_policy::Mode::Act,
                            target,"Human-confirmed own-user process termination".into(),worker.control.clone())?;
                        task.bind_broker(broker,transport,native_cancel)?;
                        worker.publish_stop(task.stop_handle()?)?;
                        {let mut status=worker.status.lock().map_err(|_|ErrorCode::ResourceExhausted)?;status.state="needs_permission".into();}
                        loop{
                            // A failed clock observation after delivery must
                            // not discard the native irreversible receipt.
                            if aios_policy::boottime_ms().map_or(true,|now|now>=worker.deadline){worker.cancel();}
                            // Poll even after Stop: an already-delivered signal
                            // must retain its irreversible partial receipt.
                            if let Some(receipt)=task.poll()?{return Ok(receipt);}
                            std::thread::sleep(Duration::from_millis(20));
                        }
                    })();
                    worker.finish(result);
                }).is_err(){if let Some(work)=self.tasks.remove(&task_id){work.cancel();}return Err(ErrorCode::ResourceExhausted);}
                Ok(status)
            },
            Operation::Status{task_id}=>serde_json::to_value(self.task(&task_id)?.status.lock().map_err(|_|ErrorCode::ResourceExhausted)?.clone()).map_err(|_|ErrorCode::InvalidArgument),
            Operation::Cancel{task_id}=>{self.task(&task_id)?.cancel();Ok(json!({"schema_version":1,"task_id":task_id,"cancel_requested":true}))},
            Operation::Forget{task_id}=>{self.task(&task_id)?.cancel();self.tasks.remove(&task_id);Ok(json!({"schema_version":1,"task_id":task_id,"deleted":true}))},
        }
    }
}
fn watch_cancellation(mut receiver:UnixStream,work:Arc<Work>){
    if receiver.set_read_timeout(Some(Duration::from_millis(100))).is_err(){work.cancel();return;}
    loop{
        if work.control.load(Ordering::Acquire)!=0{return;}
        match work.status.try_lock(){
            Ok(status)=>if matches!(status.state.as_str(),"completed"|"partial"|"failed"|"cancelled"){return;},
            Err(std::sync::TryLockError::WouldBlock)=>{},
            Err(std::sync::TryLockError::Poisoned(_))=>{work.cancel();return;},
        }
        if aios_policy::boottime_ms().map_or(true,|now|now>=work.deadline){work.cancel();return;}
        let mut byte=[0u8];match receiver.read(&mut byte){
            Err(error) if matches!(error.kind(),std::io::ErrorKind::TimedOut|std::io::ErrorKind::WouldBlock)=>{},
            // Any byte, EOF or error only revokes this original task.
            _=>{work.cancel();return;},
        }
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn native_cancel_socket_withdraws_worker_while_status_is_busy(){
        let id=uuid::Uuid::new_v4().to_string();let work=Arc::new(Work::new(id,aios_policy::boottime_ms().unwrap()+5000));
        let (sender,receiver)=UnixStream::pair().unwrap();
        let locked=work.status.lock().unwrap();let other=work.clone();let watcher=std::thread::spawn(move||watch_cancellation(receiver,other));
        sender.shutdown(std::net::Shutdown::Both).unwrap();
        let deadline=std::time::Instant::now()+Duration::from_secs(1);
        while work.control.load(Ordering::Acquire)==0{assert!(std::time::Instant::now()<deadline);std::thread::sleep(Duration::from_millis(1));}
        drop(locked);watcher.join().unwrap();
    }
    #[test]fn managed_shapes_reject_authority_and_effect_overrides(){
        let id=uuid::Uuid::new_v4().to_string();
        let start=json!({"kind":"start_process_termination","task_id":id,"process_id":id,"session_id":"2","goal":"Stop my worker","mode":"act"});
        assert!(matches!(parse(&start.to_string()),Ok(Some(Operation::Start{..}))));
        for field in ["pid","uid","signal","approved","token","closure","timeout_ms"]{
            let mut altered=start.clone();altered[field]=json!(true);
            assert!(matches!(parse(&altered.to_string()),Err(ErrorCode::InvalidArgument)));
        }
        let duplicate=format!("{{\"kind\":\"get_process_termination\",\"task_id\":\"{id}\",\"task_id\":\"{id}\"}}");
        assert!(matches!(parse(&duplicate),Err(ErrorCode::InvalidArgument)));
        for kind in ["get_process_termination","cancel_process_termination","forget_process_termination"]{
            assert!(parse(&json!({"kind":kind,"task_id":id}).to_string()).unwrap().is_some());
            assert!(parse(&json!({"kind":kind,"task_id":id,"approved":true}).to_string()).is_err());
        }
        assert!(parse(&json!({"kind":"invoke"}).to_string()).unwrap().is_none());
    }
    #[test]fn retained_tasks_are_connection_private_and_forgotten_ids_never_replay(){
        let id=uuid::Uuid::new_v4().to_string();let mut owner=Context::default();let stranger=Context::default();
        owner.reserve(&id).unwrap();let work=Arc::new(Work::new(id.clone(),u64::MAX));owner.tasks.insert(id.clone(),work.clone());
        assert!(matches!(stranger.task(&id),Err(ErrorCode::TargetNotFound)));
        assert!(matches!(owner.reserve(&id),Err(ErrorCode::Conflict)));
        owner.tasks.remove(&id).unwrap().cancel();assert_eq!(work.control.load(Ordering::Acquire),1);
        assert!(matches!(owner.task(&id),Err(ErrorCode::TargetNotFound)));
        assert!(matches!(owner.reserve(&id),Err(ErrorCode::Conflict)));
        for _ in 0..MAX_RETAINED{let id=uuid::Uuid::new_v4().to_string();owner.reserve(&id).unwrap();owner.tasks.insert(id.clone(),Arc::new(Work::new(id,u64::MAX)));}
        assert!(matches!(owner.reserve(&uuid::Uuid::new_v4().to_string()),Err(ErrorCode::ResourceExhausted)));
    }
    #[test]fn admission_is_bounded_and_released_on_all_drops(){
        let counter=AtomicUsize::new(0);let mut slots=Vec::new();
        for _ in 0..MAX_ACTIVE{slots.push(Admission::acquire(&counter).unwrap());}
        assert!(matches!(Admission::acquire(&counter),Err(ErrorCode::ResourceExhausted)));
        slots.pop();let replacement=Admission::acquire(&counter).unwrap();drop(replacement);drop(slots);
        assert_eq!(counter.load(Ordering::Acquire),0);
    }
    #[test]fn teardown_latches_stop_without_process_or_status_mutex_and_keeps_terminal_facts(){
        let id=uuid::Uuid::new_v4().to_string();let work=Arc::new(Work::new(id.clone(),u64::MAX));
        let mut context=Context::default();context.tasks.insert(id,work.clone());
        let locked=work.status.lock().unwrap();drop(context);
        assert_eq!(work.control.load(Ordering::Acquire),1);drop(locked);
        work.finish(Err(ErrorCode::Cancelled));assert_eq!(work.status.lock().unwrap().state,"cancelled");
        work.cancel();assert_eq!(work.status.lock().unwrap().error,Some(ErrorCode::Cancelled));
    }
    #[test]fn cancellation_after_delivery_preserves_irreversible_partial_receipt(){
        // Synthetic receipt tests lifecycle retention, not native delivery.
        let work=Work::new(uuid::Uuid::new_v4().to_string(),u64::MAX);
        let receipt=Receipt{preview:aios_system::processes::termination::Preview{
            identity:aios_system::processes::Identity{pid:123,uid:1000,start_time_ticks:42,
                boot_id:uuid::Uuid::new_v4().to_string(),executable_identity:"fixture".into()},
            signal:"SIGTERM",verification_timeout_ms:30_000,reversible:false,automatic_escalation:false},
            signal_requested:true,verified_exit:false,complete:false,error:Some(ErrorCode::Cancelled)};
        work.cancel();work.finish(Ok(receipt.clone()));work.cancel();
        let status=work.status.lock().unwrap();assert_eq!(status.state,"partial");
        assert_eq!(status.receipt,Some(receipt));assert_eq!(status.error,Some(ErrorCode::Cancelled));
    }
}
