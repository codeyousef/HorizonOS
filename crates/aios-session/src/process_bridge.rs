//! Fixed native process component. Only the managed broker may transfer an
//! originating kernel socket or native bus reference. Observations and a fixed
//! native-confirmed termination lifecycle accept no PID/UID claims, approval,
//! signal overrides, shell or arbitrary file operations.
use crate::{identity::Peer,managed_service::{ManagedService,Role},Operation,Request,SharedState};
use aios_protocol::{read_frame_with_limit,write_frame,MAX_TASK_BYTES,MAX_FRAME_BYTES,contracts::{Action,ErrorCode,parse_tool_call}};
use serde_json::{json,Value};
use std::{os::unix::net::UnixStream,time::Duration};
type Result<T>=std::result::Result<T,ErrorCode>;

pub(crate) fn action(operation:&Operation)->Result<Option<Action>>{
    if let Operation::Invoke{tool_call}=operation{
        let action=parse_tool_call(tool_call.get().as_bytes())?;
        if matches!(action.action_id(),"process.list"|"process.inspect"){return Ok(Some(action));}
    }
    Ok(None)
}
fn required_action(operation:Operation)->Result<Action>{
    action(&operation)?.ok_or(ErrorCode::UnsupportedCapability)
}

/// A bridge connection retains one original caller proof for its entire
/// lifetime. The shared state preserves owner-aware denial across connections.
pub fn serve(mut stream:UnixStream,state:SharedState)->Result<()>{
    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
    let broker=ManagedService::authenticate(&stream,Role::Broker)?;
    let origin=crate::ui_bridge::receive_origin(&mut stream)?;
    let peer=origin.peer()?;broker.verify(&stream)?;
    struct Owner{state:SharedState,peer:Peer,tasks:crate::process_tasks::Context}
    impl Drop for Owner{fn drop(&mut self){
        // Latch cancellation before waiting for shared inventory cleanup.
        self.tasks.cancel_all();
        if let Ok(mut state)=self.state.lock(){state.disconnect(&self.peer);}
    }}
    let mut owner=Owner{state:state.clone(),peer:peer.clone(),tasks:Default::default()};
    write_frame(&mut stream,&json!({"schema_version":2,"request_id":uuid::Uuid::new_v4().to_string(),
        "operation":"process_bound","origin_sha256":origin.identity_sha256()?}).to_string()).map_err(|_|ErrorCode::TargetChanged)?;
    // Native handles still expire at 30s without renewal. A retained task has
    // its own finite 90s bound; transport idleness must not silently shorten it.
    stream.set_read_timeout(Some(Duration::from_secs(95))).map_err(|_|ErrorCode::TargetChanged)?;
    for _ in 0..4096{
        let Some(raw)=read_frame_with_limit(&mut stream,MAX_TASK_BYTES).map_err(|_|ErrorCode::InvalidArgument)? else{return Ok(());};
        broker.verify(&stream)?;origin.verify()?;
        let request:Request=serde_json::from_str(&raw).map_err(|_|ErrorCode::InvalidArgument)?;
        if !crate::uuid(&request.request_id){return Err(ErrorCode::InvalidArgument);}
        #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)]
        struct SelectedRead{kind:String,task_id:String,process_id:String}
        let selected=serde_json::from_str::<SelectedRead>(request.operation.get()).ok();
        let managed=crate::process_tasks::parse(request.operation.get());
        let result=if request.schema_version!=1{Err(ErrorCode::UnsupportedSchema)}else if let Err(code)=managed{Err(code)}else if let Some(operation)=managed?{
            let cancellation=if matches!(operation,crate::process_tasks::Operation::Start{..}){
                Some(crate::ui_bridge::receive_cancellation(&stream,&broker)?)
            }else{None};
            owner.tasks.execute(operation,&request.request_id,&origin,&broker,&stream,&state,cancellation)
        }else if let Some(selected)=selected{
            if selected.kind!="task_process_inspect" || !crate::uuid(&selected.task_id) || !crate::uuid(&selected.process_id){Err(ErrorCode::InvalidArgument)}else{
                // Only the fixed managed broker can enter this branch. It owns
                // the opaque original task grant and checks it on both sides
                // of this native read. No user intent/grant is minted here.
                state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.process_observe_selected(&peer,&selected.process_id)
            }
        }else{
            crate::parse_operation(request.operation.get()).and_then(required_action).and_then(|action|{
                let mut state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                state.prune();state.process_read(&peer,&action)
            })
        };
        broker.verify(&stream)?;origin.verify()?;
        let (data,error)=match result{Ok(value)=>(Some(value),None),Err(code)=>(None,Some(code))};
        let response=json!({"schema_version":1,"request_id":request.request_id,"operation":"response","data":data,"error":error}).to_string();
        if response.len()>MAX_FRAME_BYTES{return Err(ErrorCode::ResourceExhausted);}
        write_frame(&mut stream,&response).map_err(|_|ErrorCode::TargetChanged)?;
    }
    Ok(())
}
pub(crate) struct Client{inner:crate::ui_bridge::Client}
impl Client{
    pub(crate) fn start_termination(&mut self,id:&str,process:&str,session:&str,goal:&str,receiver:&UnixStream)->Result<Value>{
        let value=self.inner.start_task_with_id(json!({"kind":"start_process_termination","task_id":id,"process_id":process,
            "session_id":session,"goal":goal,"mode":"act"}),receiver,id)?;
        crate::process_control::validate_status(value,id)
    }
    pub(crate) fn termination_status(&mut self,id:&str)->Result<Value>{
        let value=self.inner.call(json!({"kind":"get_process_termination","task_id":id}))?;
        crate::process_control::validate_status(value,id)
    }
    pub(crate) fn forget_termination(&mut self,id:&str)->Result<Value>{
        let value=self.inner.call(json!({"kind":"forget_process_termination","task_id":id}))?;
        crate::process_control::validate_deleted(value,id)
    }
    pub(crate) fn observe_selected(&mut self,task:&str,id:&str)->Result<Value>{
        if !crate::uuid(task) || !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        let value=self.inner.call(json!({"kind":"task_process_inspect","task_id":task,"process_id":id}))?;
        aios_protocol::validation::validate_result("process.inspect",value.to_string().as_bytes())
    }
    pub(crate) fn connect(origin:Option<&UnixStream>,peer:&Peer)->Result<Self>{
        Ok(Self{inner:crate::ui_bridge::Client::connect_process(origin,peer)?})
    }
    pub(crate) fn call(&mut self,action:&Action)->Result<Value>{
        if !matches!(action.action_id(),"process.list"|"process.inspect"){return Err(ErrorCode::UnsupportedCapability);}
        let result=self.inner.call(json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":action.action_id(),"arguments":action.arguments_value()}}))?;
        aios_protocol::validation::validate_result(action.action_id(),result.to_string().as_bytes())
    }
}

#[cfg(test)]mod tests{
    use super::*;
    fn parse(value:Value)->Operation{crate::parse_operation(&value.to_string()).unwrap()}
    #[test]fn fixed_process_bridge_refuses_effects_and_other_domains(){
        for id in ["system.info","process.terminate"]{
            let args=if id=="process.terminate"{json!({"process_id":uuid::Uuid::new_v4().to_string(),"graceful":true})}else{json!({})};
            assert!(matches!(required_action(parse(json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":id,"arguments":args}}))),Err(ErrorCode::UnsupportedCapability)));
        }
        assert!(matches!(required_action(Operation::GetSystemInfo),Err(ErrorCode::UnsupportedCapability)));
        let operation=parse(json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"process.list","arguments":{"uid":0}}}));
        assert!(matches!(required_action(operation),Err(ErrorCode::InvalidArgument)));
    }
}
