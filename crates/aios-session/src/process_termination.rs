//! Native worker joining original caller, owner-bound process, exact consent
//! and retained-pidfd effect. No public IPC or model decision constructor.
use crate::{display::DisplayBinding,ui_read::OriginatingClient,processes::SelectedProcess};
use aios_policy::{self as policy,consent::{NativeDesktop,ReadCancellation,termination::{ProcessIdentity,
    TerminationPresentation,TerminationConfirmation,TerminationDelivery}},CurrentResources};
use aios_protocol::contracts::ErrorCode;
use aios_system::processes::termination::{Prepared,Attempt,Receipt,Cancellation};
use serde_json::Value;
use std::sync::{Arc,atomic::{AtomicU8,Ordering}};
type Result<T>=std::result::Result<T,ErrorCode>;
fn closure()->Result<String>{
    let path=std::fs::canonicalize("/run/current-system").map_err(|_|ErrorCode::TargetChanged)?;
    if path.parent()!=Some(std::path::Path::new("/nix/store")){return Err(ErrorCode::TargetChanged);}
    path.to_str().map(str::to_owned).ok_or(ErrorCode::TargetChanged)
}
struct Resources<'a>{selection:&'a SelectedProcess,origin:&'a OriginatingClient,display:&'a DisplayBinding,closure:&'a str}
impl CurrentResources for Resources<'_>{
    fn resolve(&self,field:&str,kind:&str,handle:&str)->Result<String>{
        self.origin.verify()?;
        match (field,kind){
            ("process_id","scope-owner-expiry") if handle==self.selection.id=>{
                self.selection.verify(&self.origin.peer()?)?;
                policy::digest(&self.selection.native.inspect()?.identity)
            },
            ("selected_session","graphical-session") if handle==self.display.session.id=>{
                self.display.verify()?;policy::digest(self.display)
            },
            ("current_closure","system-closure") if handle==self.closure=>{
                let actual=closure()?;
                if actual!=self.closure{return Err(ErrorCode::TargetChanged);}policy::digest(&actual)
            },
            _=>Err(ErrorCode::PermissionDenied),
        }
    }
    fn dynamic_arguments(&self,_:&str,_:&Value,_:&policy::Scope)->Result<()>{Err(ErrorCode::UnsupportedCapability)}
}
pub(crate) struct NativeTerminationTask {
    broker:Option<(crate::managed_service::ManagedService,std::os::unix::net::UnixStream)>,
    origin:OriginatingClient,selection:SelectedProcess,display:DisplayBinding,closure:String,
    policy:policy::Policy,request_id:String,control:Arc<AtomicU8>,
    pending:Option<TerminationConfirmation>,delivery:Option<TerminationDelivery>,prepared:Option<Prepared>,
    attempt:Option<Attempt>,signal_cancel:Cancellation,terminal:Option<Receipt>,failure:Option<ErrorCode>,
}
pub(crate) struct NativeTerminationStop {control:Arc<AtomicU8>,prompt:ReadCancellation,signal:Cancellation}
impl NativeTerminationStop {
    pub(crate) fn stop(&self){self.signal.cancel();self.control.store(1,Ordering::Release);self.prompt.cancel();}
}
impl NativeTerminationTask {
    /// Only a trusted authenticated human task route may call this after
    /// native selection/admission. No claimed PID or approval argument.
    pub(crate) fn begin(origin:OriginatingClient,selection:SelectedProcess,display:DisplayBinding,
        request_id:String,goal:&str,mode:policy::Mode,target:String,profile:String,control:Arc<AtomicU8>)->Result<Self>{
        if mode!=policy::Mode::Act{return Err(ErrorCode::PermissionDenied);}
        if !crate::uuid(&request_id){return Err(ErrorCode::InvalidArgument);}
        if control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
        origin.verify()?;selection.verify(&origin.peer()?)?;display.verify()?;
        let subject=origin.peer()?.policy_subject()?;
        if display.session.uid!=subject.uid || display.boot_id!=subject.boot_id{return Err(ErrorCode::PermissionDenied);}
        let native=selection.native.inspect()?.identity;
        let prepared=selection.native.prepare_termination(30_000)?;
        let signal_cancel=prepared.cancellation();let current_closure=closure()?;
        let policy=policy::Policy::new(subject.boot_id.clone(),policy::registry_revision())?;
        let proposal=policy.propose_termination(policy.authenticated_user_intent(subject.clone(),request_id.clone(),goal,mode)?,
            NativeDesktop{uid:display.session.uid,boot_id:display.boot_id.clone(),session_id:display.session.id.clone(),
                identity_sha256:policy::digest(&display)?,socket_name:display.socket_name.clone()},
            TerminationPresentation{target,profile,goal:goal.into(),evidence:vec![]},selection.id.clone(),
            ProcessIdentity{pid:native.pid,uid:native.uid,start_time_ticks:native.start_time_ticks,
                boot_id:native.boot_id,executable_identity:native.executable_identity},current_closure.clone(),30_000,90_000)?;
        if proposal.native_preview_digest()?!=policy::digest(prepared.preview())?{return Err(ErrorCode::PlanChanged);}
        let pending=proposal.launch(&policy,&subject,&Resources{selection:&selection,origin:&origin,display:&display,closure:&current_closure})?;
        let mut task=Self{broker:None,origin,selection,display,closure:current_closure,policy,request_id,control,
            pending:Some(pending),delivery:None,prepared:Some(prepared),attempt:None,signal_cancel,terminal:None,failure:None};
        task.check()?;Ok(task)
    }
    /// The managed transport is retained only as native lifetime evidence.
    /// It cannot be reconstructed from a serialized PID or service name.
    pub(crate) fn bind_broker(&mut self,broker:crate::managed_service::ManagedService,stream:std::os::unix::net::UnixStream)->Result<()>{
        if self.broker.is_some(){return Err(ErrorCode::Conflict);}
        broker.verify(&stream)?;self.broker=Some((broker,stream));self.check()
    }
    fn resources(&self)->Resources<'_>{Resources{selection:&self.selection,origin:&self.origin,display:&self.display,closure:&self.closure}}
    fn check(&mut self)->Result<()>{
        let result=(||{
            if self.control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
            if let Some((broker,stream))=&self.broker{broker.verify(stream)?;}
            self.origin.verify()?;self.selection.verify(&self.origin.peer()?)?;self.display.verify()?;
            if closure()?!=self.closure{return Err(ErrorCode::TargetChanged);}Ok(())
        })();
        if let Err(code)=result{self.fail(code);}result
    }
    fn fail(&mut self,code:ErrorCode){self.pending.take();self.delivery.take();self.prepared.take();self.signal_cancel.cancel();self.failure=Some(code);}
    pub(crate) fn stop_handle(&self)->Result<NativeTerminationStop>{
        Ok(NativeTerminationStop{control:self.control.clone(),prompt:self.pending.as_ref().ok_or(ErrorCode::ApprovalExpired)?.cancellation()?,signal:self.signal_cancel.clone()})
    }
    /// Poll without holding global task state. Once sent, a signal is never
    /// replayed. Disconnect/Stop after delivery preserve the partial receipt.
    pub(crate) fn poll(&mut self)->Result<Option<Receipt>>{
        if let Some(value)=&self.terminal{return Ok(Some(value.clone()));}
        if let Some(code)=self.failure{return Err(code);}
        if let Some(attempt)=&mut self.attempt{
            if self.control.load(Ordering::Acquire)!=0 || self.origin.verify().is_err()
                || self.broker.as_ref().map_or(true,|(broker,stream)|broker.verify(stream).is_err()){self.signal_cancel.cancel();}
            let receipt=attempt.poll();if let Some(value)=&receipt{self.terminal=Some(value.clone());}return Ok(receipt);
        }
        let result=self.poll_before_effect();
        if let Err(code)=result{self.fail(code);}result
    }
    fn poll_before_effect(&mut self)->Result<Option<Receipt>>{
        if self.broker.is_none(){return Err(ErrorCode::PermissionDenied);}
        self.check()?;
        let subject=self.origin.peer()?.policy_subject()?;
        let resources=Resources{selection:&self.selection,origin:&self.origin,display:&self.display,closure:&self.closure};
        let Some(decision)=self.pending.as_mut().ok_or(ErrorCode::ApprovalExpired)?.poll(&self.policy,&subject,&resources)? else{return Ok(None);};
        self.check()?;
        let delivery=self.policy.consume_termination(decision,&subject,&self.resources())?;
        self.pending.take();self.delivery=Some(delivery);
        let prepared=self.prepared.take().ok_or(ErrorCode::ApprovalExpired)?;
        let mut delivery=self.delivery.take().ok_or(ErrorCode::ApprovalExpired)?;
        let attempt=prepared.execute(|preview|{
            self.check()?;
            let subject=self.origin.peer()?.policy_subject()?;
            delivery.revalidate(&self.policy,&subject,&self.request_id,&policy::digest(preview)?,&self.resources())?;
            if self.control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
            self.origin.verify()?;self.selection.verify_lifetime(&self.origin.peer()?)?;
            let (broker,stream)=self.broker.as_ref().ok_or(ErrorCode::PermissionDenied)?;broker.verify(stream)?;
            if self.control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
            self.selection.verify_lifetime(&self.origin.peer()?)?;
            delivery.check_deadline()
        })?;
        self.attempt=Some(attempt);self.poll()
    }
}
impl Drop for NativeTerminationTask {fn drop(&mut self){
    self.control.store(1,Ordering::Release);self.signal_cancel.cancel();self.pending.take();self.delivery.take();
}}
