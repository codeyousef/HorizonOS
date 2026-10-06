//! Authenticated human process task control. No model action calls this route.
use crate::{identity::{self,Peer},process_selection,ui_bridge,Operation,Mode,SharedState};
use aios_protocol::contracts::{ErrorCode,parse_tool_call};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::{collections::{HashMap,HashSet},sync::{Arc,Mutex}};
type Result<T>=std::result::Result<T,ErrorCode>;

#[derive(Deserialize,Serialize)]#[serde(deny_unknown_fields)]
struct Identity{pid:u32,uid:u32,start_time_ticks:u64,boot_id:String,executable_identity:String}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Preview{identity:Identity,signal:String,verification_timeout_ms:u32,reversible:bool,automatic_escalation:bool}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Receipt{preview:Preview,signal_requested:bool,verified_exit:bool,complete:bool,error:Option<ErrorCode>}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Status{schema_version:u32,task_id:String,state:String,error:Option<ErrorCode>,receipt:Option<Receipt>}
pub(crate) fn validate_status(value:Value,id:&str)->Result<Value>{
    if value.as_object().map_or(true,|fields|fields.len()!=5){return Err(ErrorCode::InvalidArgument);}
    let status:Status=serde_json::from_value(value.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
    if !crate::uuid(id) || status.schema_version!=1 || status.task_id!=id{return Err(ErrorCode::TargetChanged);}
    match (&*status.state,&status.receipt){
        ("queued"|"needs_permission",None) if status.error.is_none()=>{},
        ("failed",None) if status.error.is_some() && status.error!=Some(ErrorCode::Cancelled)=>{},
        ("cancelled",None) if status.error==Some(ErrorCode::Cancelled)=>{},
        ("completed"|"partial",Some(receipt))=>{
            let p=&receipt.preview;let i=&p.identity;
            if !receipt.signal_requested || p.signal!="SIGTERM" || p.verification_timeout_ms!=30_000 || p.reversible || p.automatic_escalation
                || i.pid<=1 || i.uid==0 || i.start_time_ticks==0 || i.start_time_ticks>9_007_199_254_740_991 || !crate::uuid(&i.boot_id)
                || i.executable_identity.is_empty() || i.executable_identity.len()>256 || i.executable_identity.chars().any(char::is_control)
                || status.error!=receipt.error || receipt.complete!=(receipt.verified_exit && receipt.error.is_none())
                || (status.state=="completed")!=receipt.complete || (status.state=="partial" && receipt.error.is_none())
                || value["receipt"].as_object().map_or(true,|fields|fields.len()!=5){return Err(ErrorCode::InvalidArgument);}
        },
        _=>return Err(ErrorCode::InvalidArgument),
    }
    Ok(value)
}
pub(crate) fn validate_deleted(value:Value,id:&str)->Result<Value>{
    #[derive(Deserialize)]#[serde(deny_unknown_fields)]struct Deleted{schema_version:u32,task_id:String,deleted:bool}
    let response:Deleted=serde_json::from_value(value.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
    if response.schema_version!=1 || response.task_id!=id || !response.deleted{return Err(ErrorCode::TargetChanged);}Ok(value)
}
pub(crate) fn is_operation(operation:&Operation)->bool{matches!(operation,Operation::StartProcessTermination{..}|Operation::GetProcessTermination{..}|Operation::CancelProcessTermination{..}|Operation::ForgetProcessTermination{..})}
#[derive(Clone,Copy)]enum Phase{Preparing,Native,Uncertain,Failed(ErrorCode)}
struct Entry{cancel:Arc<ui_bridge::Cancellation>,identity_sha256:Option<String>,phase:Phase}
#[derive(Default)]struct Records{tasks:HashMap<String,Entry>,seen:HashSet<String>}
impl Records{
    fn insert(&mut self,id:&str,cancel:Arc<ui_bridge::Cancellation>)->Result<()>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        if self.seen.contains(id){return Err(ErrorCode::Conflict);}
        if self.tasks.len()>=8 || self.seen.len()>=4096{return Err(ErrorCode::ResourceExhausted);}
        self.seen.insert(id.into());self.tasks.insert(id.into(),Entry{cancel,identity_sha256:None,phase:Phase::Preparing});Ok(())
    }
    fn entry(&self,id:&str)->Result<&Entry>{if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}self.tasks.get(id).ok_or(ErrorCode::TargetNotFound)}
    fn cancel(&self,id:&str)->Result<Value>{self.entry(id)?.cancel.cancel();Ok(json!({"schema_version":1,"task_id":id,"cancel_requested":true}))}
}
impl Drop for Records{fn drop(&mut self){for entry in self.tasks.values(){entry.cancel.cancel();}}}
pub(crate) struct Connection{peer:Peer,client:process_selection::Connection,records:Mutex<Records>}
impl Connection{
    pub(crate) fn new(peer:Peer,client:process_selection::Connection)->Self{Self{peer,client,records:Mutex::new(Records::default())}}
    pub(crate) fn execute(&self,state:&SharedState,peer:&Peer,operation:Operation)->Result<Value>{
        if self.peer!=*peer{return Err(ErrorCode::PermissionDenied);}identity::verify_peer(peer)?;
        match operation{
            Operation::StartProcessTermination{task_id,process_id,session_handle,goal,mode}=>{
                if mode!=Mode::Act{return Err(ErrorCode::PermissionDenied);}
                if !crate::uuid(&task_id) || !crate::uuid(&process_id) || !crate::uuid(&session_handle) || goal.trim().is_empty() || goal.len()>4096{return Err(ErrorCode::InvalidArgument);}
                let session=crate::selected_ui_session(state,peer,&session_handle)?;
                let (cancel,receiver)=ui_bridge::Cancellation::pair()?;
                self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?.insert(&task_id,cancel.clone())?;
                let result=(||->Result<Value>{
                    let mut client=self.client.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                    let action=parse_tool_call(json!({"kind":"tool_call","action_id":"process.inspect","arguments":{"process_id":process_id}}).to_string().as_bytes())?;
                    let value=client.call(&action)?;
                    let resource=process_selection::resource(peer,&process_id,&value)?;
                    identity::verify_peer(peer)?;
                    if cancel.cancelled(){return Err(ErrorCode::Cancelled);}
                    // Revalidate the exact chosen native session after the
                    // process observation, without renewing its selection.
                    if crate::selected_ui_session(state,peer,&session_handle)?!=session{return Err(ErrorCode::TargetChanged);}
                    {let mut records=self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                        let entry=records.tasks.get_mut(&task_id).ok_or(ErrorCode::Cancelled)?;
                        entry.identity_sha256=Some(resource.identity_sha256);entry.phase=Phase::Uncertain;}
                    let value=client.start_termination(&task_id,&process_id,&session.id,&goal,&receiver)?;
                    {let mut records=self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                        records.tasks.get_mut(&task_id).ok_or(ErrorCode::Cancelled)?.phase=Phase::Native;}
                    self.check_receipt(&task_id,value)
                })();
                if let Err(code)=result{cancel.cancel();if let Ok(mut records)=self.records.lock(){
                    if let Some(entry)=records.tasks.get_mut(&task_id){if matches!(entry.phase,Phase::Preparing){entry.phase=Phase::Failed(code);}}
                }}
                result
            },
            Operation::CancelProcessTermination{task_id}=>self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?.cancel(&task_id),
            Operation::GetProcessTermination{task_id}=>{
                {let records=self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?;let entry=records.entry(&task_id)?;
                    match entry.phase{
                        Phase::Preparing=>return Ok(json!({"schema_version":1,"task_id":task_id,"state":"queued","error":null,"receipt":null})),
                        Phase::Failed(code)=>return Ok(json!({"schema_version":1,"task_id":task_id,"state":if code==ErrorCode::Cancelled{"cancelled"}else{"failed"},"error":code,"receipt":null})),
                        _=>{},
                    }}
                let value=self.client.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?.termination_status(&task_id)?;
                self.check_receipt(&task_id,value)
            },
            Operation::ForgetProcessTermination{task_id}=>{
                let native={let records=self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?;let entry=records.entry(&task_id)?;entry.cancel.cancel();!matches!(entry.phase,Phase::Failed(_))};
                // A busy native channel gives a bounded error after Stop;
                // keep ownership so deletion can be retried safely.
                if native{let mut client=self.client.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                    match client.forget_termination(&task_id){Ok(_)=>{},Err(ErrorCode::TargetNotFound)=>{},Err(code)=>return Err(code)}}
                self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?.tasks.remove(&task_id);
                Ok(json!({"schema_version":1,"task_id":task_id,"deleted":true}))
            },
            _=>Err(ErrorCode::UnsupportedCapability),
        }
    }
    fn check_receipt(&self,id:&str,value:Value)->Result<Value>{
        let value=validate_status(value,id)?;
        let records=self.records.lock().map_err(|_|ErrorCode::ResourceExhausted)?;let entry=records.entry(id)?;
        if !value["receipt"].is_null(){
            let identity:Identity=serde_json::from_value(value["receipt"]["preview"]["identity"].clone()).map_err(|_|ErrorCode::InvalidArgument)?;
            if identity.uid!=self.peer.uid || identity.boot_id!=self.peer.boot_id
                || Some(aios_policy::digest(&identity)?)!=entry.identity_sha256{return Err(ErrorCode::TargetChanged);}
        }
        Ok(value)
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn terminal_status_cannot_hide_signal_or_forge_complete_exit(){
        let id=uuid::Uuid::new_v4().to_string();
        let queued=json!({"schema_version":1,"task_id":id,"state":"queued","error":null,"receipt":null});
        assert!(validate_status(queued.clone(),&id).is_ok());
        let mut missing=queued.clone();missing.as_object_mut().unwrap().remove("error");assert!(validate_status(missing,&id).is_err());
        let partial=json!({"schema_version":1,"task_id":id,"state":"partial","error":"CANCELLED","receipt":{
            "preview":{"identity":{"pid":42,"uid":1000,"start_time_ticks":100,"boot_id":uuid::Uuid::new_v4().to_string(),"executable_identity":"dev=1:ino=2"},
                "signal":"SIGTERM","verification_timeout_ms":30000,"reversible":false,"automatic_escalation":false},
            "signal_requested":true,"verified_exit":false,"complete":false,"error":"CANCELLED"}});
        assert!(validate_status(partial.clone(),&id).is_ok());
        for (path,replacement) in [("/receipt/preview/signal",json!("SIGKILL")),("/receipt/signal_requested",json!(false)),
            ("/receipt/complete",json!(true)),("/receipt/preview/reversible",json!(true)),("/receipt/preview/automatic_escalation",json!(true)),
            ("/receipt/preview/identity/uid",json!(0)),("/receipt/preview/verification_timeout_ms",json!(60000))]{
            let mut altered=partial.clone();*altered.pointer_mut(path).unwrap()=replacement;assert!(validate_status(altered,&id).is_err());
        }
        let mut extra=partial.clone();extra["receipt"]["approved"]=json!(true);assert!(validate_status(extra,&id).is_err());
        let mut missing=partial.clone();missing["receipt"].as_object_mut().unwrap().remove("error");assert!(validate_status(missing,&id).is_err());
        assert!(validate_status(partial,&uuid::Uuid::new_v4().to_string()).is_err());
    }
    #[test]fn broker_records_stop_native_channel_without_observation_lock_and_remember_replay(){
        use std::io::Read;
        let channel_lock=Mutex::new(());let _observing=channel_lock.lock().unwrap();
        let id=uuid::Uuid::new_v4().to_string();let (cancel,mut receiver)=ui_bridge::Cancellation::pair().unwrap();
        let mut records=Records::default();records.insert(&id,cancel.clone()).unwrap();
        assert!(matches!(Records::default().entry(&id),Err(ErrorCode::TargetNotFound)));
        records.cancel(&id).unwrap();assert!(cancel.cancelled());
        receiver.set_read_timeout(Some(std::time::Duration::from_secs(1))).unwrap();assert_eq!(receiver.read(&mut [0u8]).unwrap(),0);
        records.tasks.remove(&id);let (replacement,_)=ui_bridge::Cancellation::pair().unwrap();
        assert!(matches!(records.insert(&id,replacement),Err(ErrorCode::Conflict)));
        let second=uuid::Uuid::new_v4().to_string();let (cancel,mut receiver)=ui_bridge::Cancellation::pair().unwrap();
        records.insert(&second,cancel.clone()).unwrap();drop(records);assert!(cancel.cancelled());assert_eq!(receiver.read(&mut [0u8]).unwrap(),0);
    }
    #[test]fn public_termination_shapes_and_plain_state_cannot_create_native_authority(){
        let id=uuid::Uuid::new_v4().to_string();let start=json!({"kind":"start_process_termination","task_id":id,"process_id":id,"session_handle":id,"goal":"Stop this worker","mode":"act"});
        assert!(crate::parse_operation(&start.to_string()).is_ok());
        for field in ["uid","pid","signal","approved","token","session_id","closure"]{
            let mut value=start.clone();value[field]=json!(true);assert!(crate::parse_operation(&value.to_string()).is_err());
        }
        let peer=identity::authenticate_process(nix::unistd::geteuid().as_raw(),std::process::id()).unwrap();
        let mut state=crate::State::default();
        assert_eq!(state.dispatch(&peer,crate::parse_operation(&start.to_string()).unwrap()),Err(ErrorCode::AuthRequired));
        for kind in ["get_process_termination","cancel_process_termination","forget_process_termination"]{
            let operation=crate::parse_operation(&json!({"kind":kind,"task_id":id}).to_string()).unwrap();
            assert_eq!(state.dispatch(&peer,operation),Err(ErrorCode::AuthRequired));
        }
    }
}
