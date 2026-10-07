//! Exact native-confirmed power-profile mutation. The public request contains
//! no decision, token, prior value, profile choices or provider coordinates.
use crate::{display::DisplayBinding,identity::{self,GraphicalSession,Peer},native_settings::PreparedMutation};
use aios_policy::{self as policy,consent::{NativeDesktop,power_profile::{PowerProfileConfirmation,PowerProfileDelivery,PowerProfilePresentation}}};
use aios_protocol::contracts::{Action,ErrorCode};
use serde_json::{Value,json};
use std::{sync::atomic::{AtomicBool,Ordering},time::{Duration,Instant}};
type Result<T,E=ErrorCode>=std::result::Result<T,E>;

struct Task{
    peer:Peer,display:DisplayBinding,policy:policy::Policy,request_id:String,
    prepared:Option<PreparedMutation>,pending:Option<PowerProfileConfirmation>,
    delivery:Option<PowerProfileDelivery>,preview_sha256:String,cancelled:AtomicBool,
}
impl Task{
    fn begin(peer:Peer,session:GraphicalSession,request_id:String,goal:&str,action:&Action)->Result<Self>{
        if !crate::uuid(&request_id)||goal.trim().is_empty()||goal.len()>4096||goal.contains('\0'){return Err(ErrorCode::InvalidArgument);}
        identity::verify_peer(&peer)?;let display=DisplayBinding::observe(&session.id,peer.uid)?;
        if display.session!=session{return Err(ErrorCode::TargetChanged);}
        let subject=peer.policy_subject()?;if subject.uid!=display.session.uid||subject.boot_id!=display.boot_id{return Err(ErrorCode::PermissionDenied);}
        let prepared=PreparedMutation::prepare(action)?;let (prior,requested,mut choices)=prepared.power_profile_preview()?;choices.sort();
        let policy=policy::Policy::new(subject.boot_id.clone(),policy::registry_revision())?;
        let intent=policy.authenticated_user_intent(subject.clone(),request_id.clone(),goal,policy::Mode::Act)?;
        let proposal=policy.propose_power_profile(intent,NativeDesktop{uid:display.session.uid,boot_id:display.boot_id.clone(),
            session_id:display.session.id.clone(),identity_sha256:policy::digest(&display)?,socket_name:display.socket_name.clone()},
            PowerProfilePresentation{target:"This desktop".into(),profile:"Local power profile".into(),goal:goal.into(),evidence:vec![]},
            prior,requested,choices,90_000)?;
        let preview_sha256=proposal.native_preview_digest()?;
        let pending=proposal.launch(&policy,&subject,&prepared)?;
        Ok(Self{peer,display,policy,request_id,prepared:Some(prepared),pending:Some(pending),delivery:None,preview_sha256,cancelled:AtomicBool::new(false)})
    }
    fn verify(&self)->Result<policy::Subject>{
        if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        identity::verify_peer(&self.peer)?;self.display.verify()?;let subject=self.peer.policy_subject()?;
        if subject.uid!=self.display.session.uid||subject.boot_id!=self.display.boot_id{return Err(ErrorCode::PermissionDenied);}Ok(subject)
    }
    fn run(mut self)->Result<Value>{
        let deadline=Instant::now()+Duration::from_secs(90);
        loop{
            if Instant::now()>=deadline{return Err(ErrorCode::DeadlineExceeded);}
            let subject=self.verify()?;let prepared=self.prepared.as_ref().ok_or(ErrorCode::ApprovalExpired)?;
            match self.pending.as_mut().ok_or(ErrorCode::ApprovalExpired)?.poll(&self.policy,&subject,prepared)?{
                None=>std::thread::sleep(Duration::from_millis(20)),
                Some(decision)=>{
                    self.verify()?;let delivery=self.policy.consume_power_profile(decision,&self.peer.policy_subject()?,prepared)?;
                    self.pending.take();self.delivery=Some(delivery);break;
                }
            }
        }
        let prepared=self.prepared.take().ok_or(ErrorCode::ApprovalExpired)?;
        let mut delivery=self.delivery.take().ok_or(ErrorCode::ApprovalExpired)?;let mut checks=0u8;
        let receipt=prepared.execute(|current|{
            checks=checks.checked_add(1).ok_or(ErrorCode::ApprovalExpired)?;
            if checks==1{let subject=self.verify()?;delivery.revalidate(&self.policy,&subject,&self.request_id,&self.preview_sha256,current)}
            else if checks==2{delivery.check_deadline()}else{Err(ErrorCode::ApprovalExpired)}
        })?;
        Ok(json!({"schema_version":1,"request_id":self.request_id.clone(),"operation":"power_profile_change","state":"completed",
            "mutation_performed":receipt.changed,"output":receipt.output,"recovery":receipt.recovery}))
    }
}
impl Drop for Task{
    fn drop(&mut self){self.cancelled.store(true,Ordering::Release);if let Some(pending)=&mut self.pending{pending.withdraw();}if let Some(delivery)=&self.delivery{delivery.revoke();}}
}
pub(crate) fn execute(peer:Peer,session:GraphicalSession,request_id:String,goal:String,action:Action)->Result<Value>{
    Task::begin(peer,session,request_id,&goal,&action)?.run()
}
