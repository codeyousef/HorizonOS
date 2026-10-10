//! Protected native root enrollment, with independently latched cancellation.
use crate::{display::DisplayBinding,identity::{self,Peer,GraphicalSession}};
use aios_files::{Access,Manager,Owner,RootGrant};
use aios_policy::{self as policy,consent::{NativeDesktop,ReadCancellation,file_roots::{FileRoot,RootAccess}}};
use aios_protocol::contracts::ErrorCode;
use std::{collections::HashMap,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
type Result<T> = std::result::Result<T,ErrorCode>;
struct Pending { peer:Peer,cancelled:Arc<AtomicBool>,native:Option<ReadCancellation> }
#[derive(Clone,Default)]
pub(crate) struct Controls(Arc<Mutex<HashMap<String,Pending>>>);
struct Lease { controls:Controls,id:String,cancelled:Arc<AtomicBool> }
impl Drop for Lease {
    fn drop(&mut self){
        self.cancelled.store(true,Ordering::Release);
        if let Ok(mut map)=self.controls.0.lock(){
            if map.get(&self.id).is_some_and(|p|Arc::ptr_eq(&p.cancelled,&self.cancelled)){
                if let Some(p)=map.remove(&self.id){if let Some(native)=p.native{native.cancel();}}
            }
        }
    }
}
impl Controls {
    pub(crate) fn cancel(&self,peer:&Peer,id:&str)->Result<()>{
        let mut map=self.0.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        let pending=map.get_mut(id).ok_or(ErrorCode::TargetNotFound)?;
        if pending.peer!=*peer{return Err(ErrorCode::PermissionDenied);}
        pending.cancelled.store(true,Ordering::Release);
        if let Some(native)=&pending.native{native.cancel();}Ok(())
    }
    fn reserve(&self,peer:Peer,id:String)->Result<Lease>{
        let mut map=self.0.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        if map.contains_key(&id){return Err(ErrorCode::Conflict);}if map.len()>=32{return Err(ErrorCode::ResourceExhausted);}
        let cancelled=Arc::new(AtomicBool::new(false));
        map.insert(id.clone(),Pending{peer,cancelled:cancelled.clone(),native:None});
        Ok(Lease{controls:self.clone(),id,cancelled})
    }
}
struct Resources<'a>{manager:&'a Arc<Mutex<Manager>>,owner:&'a Owner,proposal_id:&'a str,approved:&'a [String],access:&'a [Access],display:&'a DisplayBinding}
impl policy::CurrentResources for Resources<'_>{
    fn resolve(&self,field:&str,kind:&str,handle:&str)->Result<String>{
        self.display.verify()?;
        if field=="selected_session"&&kind=="graphical-session"&&handle==self.display.session.id{return policy::digest(self.display);}
        if field!="file_root"||kind!="scoped-root"{return Err(ErrorCode::PermissionDenied);}
        let roots=self.manager.lock().map_err(|_|ErrorCode::ResourceExhausted)?
            .review_enrollment(self.owner,self.proposal_id,self.approved,self.access,policy::boottime_ms()?)?;
        roots.into_iter().find(|root|root.root_id==handle).map(|root|root.identity_sha256).ok_or(ErrorCode::PermissionDenied)
    }
    fn dynamic_arguments(&self,_:&str,_:&serde_json::Value,_:&policy::Scope)->Result<()>{Err(ErrorCode::UnsupportedCapability)}
}
pub(crate) fn enroll(peer:Peer,session:GraphicalSession,manager:Arc<Mutex<Manager>>,controls:Controls,
    proposal_id:String,approved:Vec<String>,access:Vec<Access>)->Result<Vec<RootGrant>>{
    identity::verify_peer(&peer)?;
    let owner=crate::bus::file_owner(&peer)?;
    let display=DisplayBinding::observe(&session.id,peer.uid)?;
    if display.session!=session{return Err(ErrorCode::TargetChanged);}
    let resources=Resources{manager:&manager,owner:&owner,proposal_id:&proposal_id,approved:&approved,access:&access,display:&display};
    let roots=manager.lock().map_err(|_|ErrorCode::ResourceExhausted)?
        .review_enrollment(&owner,&proposal_id,&approved,&access,policy::boottime_ms()?)?
        .into_iter().map(|root|FileRoot{root_id:root.root_id,display_path:root.display_path,identity_sha256:root.identity_sha256}).collect();
    let lease=controls.reserve(peer.clone(),proposal_id.clone())?;
    let subject=peer.policy_subject()?;let policy=policy::Policy::new(subject.boot_id.clone(),policy::registry_revision())?;
    let goal="Enroll only these roots for this originating client";
    let intent=policy.authenticated_user_intent(subject.clone(),proposal_id.clone(),goal,policy::Mode::Act)?;
    let proposal=policy.propose_file_roots(intent,NativeDesktop{uid:display.session.uid,boot_id:display.boot_id.clone(),
        session_id:display.session.id.clone(),identity_sha256:policy::digest(&display)?,socket_name:display.socket_name.clone()},
        roots,access.iter().map(|a|match a{Access::Metadata=>RootAccess::Metadata,Access::Content=>RootAccess::Content,Access::Mutation=>RootAccess::Mutation}).collect(),90_000)?;
    let mut pending=proposal.launch(&policy,&subject,&resources)?;
    {
        let mut map=controls.0.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        let entry=map.get_mut(&proposal_id).ok_or(ErrorCode::Cancelled)?;
        if entry.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        entry.native=Some(pending.cancellation()?);
    }
    let deadline=Instant::now()+Duration::from_secs(90);
    loop{
        if lease.cancelled.load(Ordering::Acquire){pending.withdraw();return Err(ErrorCode::Cancelled);}
        if Instant::now()>=deadline{return Err(ErrorCode::DeadlineExceeded);}
        identity::verify_peer(&peer)?;display.verify()?;
        match pending.poll(&policy,&subject,&resources)?{
            None=>std::thread::sleep(Duration::from_millis(20)),
            Some(decision)=>{
                identity::verify_peer(&peer)?;display.verify()?;
                policy.consume_file_roots(decision,&subject,&resources)?;
                // Cancellation and grant creation share this linearization
                // boundary. Never hold it while waiting for native UI.
                let map=controls.0.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                if lease.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
                let result=manager.lock().map_err(|_|ErrorCode::ResourceExhausted)?
                    .enroll(&owner,&proposal_id,&approved,&access,policy::boottime_ms()?);
                drop(map);return result;
            }
        }
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn peer()->Peer{Peer{uid:1000,pid:42,start_ticks:1,boot_id:uuid::Uuid::new_v4().to_string(),logind_session:None,remote:false,
        session_type:None,ui_enabled:false,bus_sender:Some(":1.1".into()),bus_id:Some("a".repeat(32)),connection_id:None}}
    #[test] fn cancellation_is_owner_bound_and_does_not_withdraw_another_request(){
        let controls=Controls::default();let owner=peer();let id=uuid::Uuid::new_v4().to_string();let lease=controls.reserve(owner.clone(),id.clone()).unwrap();
        let mut foreign=owner.clone();foreign.bus_sender=Some(":1.2".into());
        assert_eq!(controls.cancel(&foreign,&id),Err(ErrorCode::PermissionDenied));assert!(!lease.cancelled.load(Ordering::Acquire));
        assert_eq!(controls.cancel(&owner,&id),Ok(()));assert!(lease.cancelled.load(Ordering::Acquire));
        drop(lease);assert_eq!(controls.cancel(&owner,&id),Err(ErrorCode::TargetNotFound));
    }
}
