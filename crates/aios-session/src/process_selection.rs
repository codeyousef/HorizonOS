//! Explicit user-selected process context. Task authority stays opaque in the
//! broker; the native component receives only an already authorized read.
use crate::{identity::{self,Peer},process_bridge,ReadResources,SharedState};
use aios_protocol::contracts::{Action,ErrorCode,parse_tool_call};
use serde_json::{json,Value};
use std::sync::{Arc,Mutex};
type Result<T>=std::result::Result<T,ErrorCode>;
pub(crate) type Connection=Arc<Mutex<process_bridge::Client>>;
pub(crate) struct Selection{owner:Peer,connection:Connection,resources:Vec<aios_policy::Resource>}
fn inspect(id:&str)->Result<Action>{parse_tool_call(json!({"kind":"tool_call","action_id":"process.inspect","arguments":{"process_id":id}}).to_string().as_bytes())}
fn resource(peer:&Peer,id:&str,value:&Value)->Result<aios_policy::Resource>{
    if value["complete"]!=true || value["data"]["process_id"]!=id{return Err(ErrorCode::PartialResult);}
    let data=&value["data"];
    let identity=aios_system::processes::Identity{pid:u32::try_from(data["pid"].as_u64().ok_or(ErrorCode::InvalidArgument)?).map_err(|_|ErrorCode::InvalidArgument)?,
        uid:peer.uid,start_time_ticks:data["start_time_ticks"].as_u64().ok_or(ErrorCode::InvalidArgument)?,boot_id:peer.boot_id.clone(),
        executable_identity:data["executable_identity"].as_str().ok_or(ErrorCode::InvalidArgument)?.into()};
    Ok(aios_policy::Resource{field:"process_id".into(),kind:"scope-owner-expiry".into(),handle:id.into(),identity_sha256:aios_policy::digest(&identity)?})
}
impl Selection{
    /// Called only from authenticated Submit, never from a model proposal.
    pub(crate) fn select(owner:&Peer,connection:Connection,ids:&[String])->Result<Self>{
        if ids.is_empty() || ids.len()>8{return Err(ErrorCode::InvalidArgument);}
        identity::verify_peer(owner)?;
        let mut resources=Vec::new();
        for id in ids{
            if resources.iter().any(|r:&aios_policy::Resource|r.handle==*id){return Err(ErrorCode::InvalidArgument);}
            let value=connection.lock().map_err(|_|ErrorCode::ResourceExhausted)?.call(&inspect(id)?)?;
            resources.push(resource(owner,id,&value)?);
        }
        identity::verify_peer(owner)?;
        Ok(Self{owner:owner.clone(),connection,resources})
    }
    pub(crate) fn ids(&self)->Vec<String>{self.resources.iter().map(|r|r.handle.clone()).collect()}
    pub(crate) fn resources(&self)->Vec<aios_policy::Resource>{self.resources.clone()}
    pub(crate) fn current(&self,peer:&Peer,id:&str)->Result<ReadResources>{
        if peer!=&self.owner{return Err(ErrorCode::PermissionDenied);}
        Ok(ReadResources(vec![self.resources.iter().find(|r|r.handle==id).ok_or(ErrorCode::PermissionDenied)?.clone()]))
    }
    pub(crate) fn observe(&self,state:&SharedState,task:&str,peer:&Peer,action:&Action)->Result<Value>{
        let Action::ProcessInspect(args)=action else{return Err(ErrorCode::PermissionDenied);};
        identity::verify_peer(peer)?;
        state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.task_read_target(task,peer,action)?;
        // No shared task lock during native I/O: Cancel/Forget can revoke the
        // original grant while this read is in flight.
        let value=self.connection.lock().map_err(|_|ErrorCode::ResourceExhausted)?.observe_selected(task,&args.process_id)?;
        let expected=self.current(peer,&args.process_id)?;
        if resource(peer,&args.process_id,&value)?!=expected.0[0]{return Err(ErrorCode::TargetChanged);}
        identity::verify_peer(peer)?;
        state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.task_read_target(task,peer,action)?;
        Ok(value)
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn native_selected_identity_is_bound_to_original_grant_fixture(){
        let mut peer=identity::authenticate_process(nix::unistd::geteuid().as_raw(),std::process::id()).unwrap();
        peer.connection_id=Some(uuid::Uuid::new_v4().to_string());
        let native=aios_system::processes::OwnProcess::open(peer.pid).unwrap().inspect().unwrap();
        let id=uuid::Uuid::new_v4().to_string();
        let value=json!({"complete":true,"data":{"process_id":id,"pid":native.identity.pid,"start_time_ticks":native.identity.start_time_ticks,"executable_identity":native.identity.executable_identity}});
        let enrolled=resource(&peer,&id,&value).unwrap();
        assert_eq!(enrolled.identity_sha256,aios_policy::digest(&native.identity).unwrap());
        let policy=aios_policy::Policy::new(peer.boot_id.clone(),aios_policy::registry_revision()).unwrap();
        let task=uuid::Uuid::new_v4().to_string();let subject=peer.policy_subject().unwrap();
        let intent=policy.authenticated_user_intent(subject.clone(),task.clone(),"Inspect this selected process",aios_policy::Mode::Ask).unwrap();
        let grant=policy.grant_reads(intent,aios_policy::Scope{actions:["system.info".into(),"process.inspect".into()].into(),resources:[enrolled.clone()].into(),..Default::default()},100,1000).unwrap();
        let current=ReadResources(vec![enrolled.clone()]);
        assert_eq!(policy.check_read(&grant,&subject,&task,&inspect(&id).unwrap(),&current,101),Ok(()));
        let other=uuid::Uuid::new_v4().to_string();
        assert_eq!(policy.check_read(&grant,&subject,&task,&inspect(&other).unwrap(),&current,101),Err(ErrorCode::PermissionDenied));
        let mut changed=enrolled;changed.identity_sha256="a".repeat(64);
        assert_eq!(policy.check_read(&grant,&subject,&task,&inspect(&id).unwrap(),&ReadResources(vec![changed]),101),Err(ErrorCode::TargetChanged));
        let mut foreign=subject.clone();foreign.pid+=1;
        assert_eq!(policy.check_read(&grant,&foreign,&task,&inspect(&id).unwrap(),&current,101),Err(ErrorCode::PermissionDenied));
        assert_eq!(policy.check_read(&grant,&subject,&other,&inspect(&id).unwrap(),&current,101),Err(ErrorCode::PermissionDenied));
        assert_eq!(policy.check_read(&grant,&subject,&task,&inspect(&id).unwrap(),&current,1100),Err(ErrorCode::ApprovalExpired));
        assert_eq!(policy.check_read(&grant,&subject,&task,&inspect(&id).unwrap(),&current,101),Err(ErrorCode::ApprovalExpired));
    }
    #[test]fn selected_process_result_cannot_change_handle_or_claim_completeness_fixture(){
        let mut peer=identity::authenticate_process(nix::unistd::geteuid().as_raw(),std::process::id()).unwrap();peer.connection_id=Some(uuid::Uuid::new_v4().to_string());
        let id=uuid::Uuid::new_v4().to_string();
        assert!(matches!(resource(&peer,&id,&json!({"complete":false,"data":{"process_id":id}})),Err(ErrorCode::PartialResult)));
        assert!(matches!(resource(&peer,&id,&json!({"complete":true,"data":{"process_id":"other"}})),Err(ErrorCode::PartialResult)));
    }
}
