//! Native originating-client read task, owned by the graphical provider.
//! This module exports no decision or grant IPC. Original kernel socket proof,
//! selected native window, pending confirmation and live grant remain local.
use crate::{identity::{self,Peer},accessibility::{WindowBinding,Snapshot}};
use aios_policy::{self as policy,consent::{NativeDesktop,ReadPresentation,SelectedWindow,ReadConfirmation,ReadCancellation},CurrentResources};
use aios_protocol::contracts::{ErrorCode,parse_tool_call};
use serde_json::{Value,json};
use std::{os::{fd::AsRawFd,unix::net::UnixStream},sync::{Arc,atomic::{AtomicU8,Ordering}}};
type Result<T> = std::result::Result<T,ErrorCode>;

struct Resources<'a>(&'a WindowBinding);
impl CurrentResources for Resources<'_>{
    fn resolve(&self,field:&str,kind:&str,handle:&str)->Result<String>{
        let display=self.0.selected_display();
        if field=="selected_session" && kind=="graphical-session" && handle==display.session.id {
            display.verify()?;policy::digest(display)
        } else if field=="window_handle" && kind=="scope-owner-expiry" && handle==self.0.handle {
            self.0.verify()?;self.0.identity_sha256()
        } else {Err(ErrorCode::PermissionDenied)}
    }
    fn dynamic_arguments(&self,_:&str,_:&Value,_:&policy::Scope)->Result<()>{Err(ErrorCode::UnsupportedCapability)}
}
/// Constructed from the originating server-side connection FD. A provider
/// bridge must authenticate the fixed broker before accepting this proof via
/// SCM_RIGHTS; serialized PID/UID/session claims are never a substitute.
enum OriginProof { Unix{stream:UnixStream,cookie:u64},Bus(crate::user_bus::NativeUserBus) }
pub struct OriginatingClient { proof:OriginProof,peer:Peer }
fn cookie(stream:&UnixStream)->Result<u64>{
    let mut value=0u64;let mut size=std::mem::size_of::<u64>() as nix::libc::socklen_t;
    if unsafe{nix::libc::getsockopt(stream.as_raw_fd(),nix::libc::SOL_SOCKET,nix::libc::SO_COOKIE,
        (&mut value as *mut u64).cast(),&mut size)}!=0 || size as usize!=std::mem::size_of::<u64>() || value==0 {return Err(ErrorCode::PermissionDenied);}
    Ok(value)
}
impl OriginatingClient {
    pub fn authenticate(proof:UnixStream)->Result<Self>{
        let mut peer=identity::authenticate(&proof)?;
        peer.connection_id=Some(uuid::Uuid::new_v4().to_string());
        let value=Self{proof:OriginProof::Unix{cookie:cookie(&proof)?,stream:proof},peer};value.verify()?;Ok(value)
    }
    pub(crate) fn authenticate_bus(sender:&str,bus_id:&str)->Result<Self>{
        let bus=crate::user_bus::NativeUserBus::connect()?;let peer=bus.caller(sender,bus_id)?;
        Ok(Self{proof:OriginProof::Bus(bus),peer})
    }
    pub(crate) fn identity_sha256(&self)->Result<String>{
        self.verify()?;origin_digest(&self.peer)
    }
    pub(crate) fn verify(&self)->Result<()>{
        let (stream,original_cookie)=match &self.proof {
            OriginProof::Bus(bus)=>{
                let current=bus.caller(self.peer.bus_sender.as_deref().ok_or(ErrorCode::PermissionDenied)?,self.peer.bus_id.as_deref().ok_or(ErrorCode::PermissionDenied)?)?;
                return if current==self.peer{Ok(())}else{Err(ErrorCode::TargetChanged)};
            },
            OriginProof::Unix{stream,cookie}=>(stream,*cookie),
        };
        let mut status=nix::libc::pollfd{fd:stream.as_raw_fd(),events:nix::libc::POLLRDHUP,revents:0};
        if unsafe{nix::libc::poll(&mut status,1,0)}<0 || status.revents&(nix::libc::POLLRDHUP|nix::libc::POLLHUP|nix::libc::POLLERR|nix::libc::POLLNVAL)!=0 {
            return Err(ErrorCode::Cancelled);
        }
        if cookie(stream)?!=original_cookie{return Err(ErrorCode::TargetChanged);}
        identity::verify(stream,&self.peer)
    }
    pub(crate) fn uid(&self)->u32{self.peer.uid}
    pub(crate) fn try_clone(&self)->Result<Self>{
        self.verify()?;
        let proof=match &self.proof {
            OriginProof::Unix{stream,cookie}=>OriginProof::Unix{stream:stream.try_clone().map_err(|_|ErrorCode::TargetChanged)?,cookie:*cookie},
            OriginProof::Bus(bus)=>OriginProof::Bus(bus.try_clone()?),
        };
        Ok(Self{proof,peer:self.peer.clone()})
    }
}
pub(crate) fn origin_digest(peer:&Peer)->Result<String>{
    // Each Unix adapter owns its own connection UUID. Compare native peer
    // identity during handoff; the original FD/cookie remains the authority.
    let mut peer=peer.clone();peer.connection_id=None;policy::digest(&peer)
}
/// Provider worker owns this task; the independent control loop owns `control`.
/// A nonzero control value invalidates pending/active reads before another
/// accessibility query. Dropping the task withdraws its native dialog/grant.
pub struct NativeReadTask {
    origin:OriginatingClient,window:WindowBinding,policy:policy::Policy,request_id:String,
    pending:Option<ReadConfirmation>,grant:Option<policy::ReadGrant>,control:Arc<AtomicU8>,
}
pub struct NativeReadStop { control:Arc<AtomicU8>, cancellation:ReadCancellation }
impl NativeReadStop {
    pub fn stop(&self){self.control.store(1,Ordering::Release);self.cancellation.cancel();}
}
impl NativeReadTask {
    /// Call only from authenticated human Submit, after explicit native window
    /// selection. WindowBinding has no request/model deserializer.
    pub fn begin(origin:OriginatingClient,window:WindowBinding,goal:&str,mode:policy::Mode,
        target:String,profile:String,control:Arc<AtomicU8>)->Result<Self>{
        if control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
        origin.verify()?;window.verify()?;
        let subject=origin.peer.policy_subject()?;
        let policy=policy::Policy::new(subject.boot_id.clone(),policy::registry_revision())?;
        let request_id=uuid::Uuid::new_v4().to_string();
        let intent=policy.authenticated_user_intent(subject.clone(),request_id.clone(),goal,mode)?;
        let display=window.selected_display();
        let proposal=policy.propose_graphical_read(intent,NativeDesktop{uid:display.session.uid,boot_id:display.boot_id.clone(),
            session_id:display.session.id.clone(),identity_sha256:policy::digest(display)?,socket_name:display.socket_name.clone()},
            ReadPresentation{target,profile,goal:goal.into(),windows:vec![SelectedWindow{handle:window.handle.clone(),identity_sha256:window.identity_sha256()?,
                name:window.name.clone(),window:window.title.clone()}],evidence:vec![]},90_000)?;
        let pending=proposal.launch(&policy,&subject,&Resources(&window))?;
        let mut task=Self{origin,window,policy,request_id,pending:Some(pending),grant:None,control};
        task.check_origin()?;Ok(task)
    }
    fn check_origin(&mut self)->Result<()>{
        let result=if self.control.load(Ordering::Acquire)!=0 {Err(ErrorCode::Cancelled)} else {self.origin.verify()};
        if result.is_err(){self.revoke();}result
    }
    fn revoke(&mut self){self.pending.take();self.grant.take();}
    pub fn stop_handle(&self)->Result<NativeReadStop>{
        Ok(NativeReadStop{control:self.control.clone(),cancellation:self.pending.as_ref().ok_or(ErrorCode::ApprovalExpired)?.cancellation()?})
    }
    fn check_grant(&self)->Result<()>{
        let action=parse_tool_call(&serde_json::to_vec(&json!({"kind":"tool_call","action_id":"ui.snapshot","arguments":{"window_handle":self.window.handle}}))
            .map_err(|_|ErrorCode::InvalidArgument)?)?;
        self.policy.check_read(self.grant.as_ref().ok_or(ErrorCode::AuthRequired)?,&self.origin.peer.policy_subject()?,&self.request_id,
            &action,&Resources(&self.window),policy::boottime_ms()?)
    }
    pub fn cancel(&mut self){self.control.store(1,Ordering::Release);self.revoke();}
    /// No model/client decision argument. The exact owned production dialog
    /// response is consumed once and stays inside this worker.
    pub fn poll_confirmation(&mut self)->Result<bool>{
        self.check_origin()?;
        if self.grant.is_some(){
            if let Err(error)=self.check_grant(){self.revoke();return Err(error);}return Ok(true);
        }
        let subject=self.origin.peer.policy_subject()?;
        let result=self.pending.as_mut().ok_or(ErrorCode::ApprovalExpired)?.poll(&self.policy,&subject,&Resources(&self.window));
        match result {
            Ok(Some(decision))=>{
                self.check_origin()?;
                let grant=self.policy.consume_graphical_read(decision,&subject,&Resources(&self.window));
                self.pending.take();
                match grant {Ok(grant)=>{self.grant=Some(grant);self.check_origin()?;Ok(true)},Err(error)=>{self.revoke();Err(error)}}
            },
            Ok(None)=>Ok(false),Err(error)=>{self.revoke();Err(error)},
        }
    }
    pub fn snapshot(&mut self)->Result<Snapshot>{
        self.check_origin()?;
        if let Err(error)=self.check_grant(){self.revoke();return Err(error);}
        let snapshot=self.window.snapshot(&self.control);
        if let Err(error)=self.check_grant(){self.revoke();return Err(error);}
        self.check_origin()?;
        match snapshot {Ok(snapshot)=>Ok(snapshot),Err(error)=>{self.revoke();Err(error)}}
    }
}
