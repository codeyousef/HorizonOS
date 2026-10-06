//! Exact R2 process consent. Native brokers provide freshly resolved identity;
//! no model/client JSON can construct a decision or delivery authority.
use super::*;

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProcessIdentity {
    pub pid:u32, pub uid:u32, pub start_time_ticks:u64,
    pub boot_id:String, pub executable_identity:String,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
struct ProcessPreview {
    identity:ProcessIdentity, signal:&'static str, verification_timeout_ms:u32,
    reversible:bool, automatic_escalation:bool,
}
pub struct TerminationPresentation {
    pub target:String, pub profile:String, pub goal:String, pub evidence:Vec<String>,
}
/// Immutable native snapshot, not permission. The originating broker must
/// resolve this handle's native identity, selected desktop and current closure.
pub struct TerminationProposal {
    intent:Intent, scope:Scope, pub(super) desktop:NativeDesktop,
    presentation:TerminationPresentation, process_id:String, preview:ProcessPreview,
    closure:String, revision:String, incarnation:Uuid, nonce:Uuid,
    issued_ms:u64, expires_ms:u64, pub(super) digest:String,
}
pub struct NativeTerminationDecision { proposal:TerminationProposal, cancelled:Arc<AtomicBool> }
pub struct TerminationConfirmation { native:NativeConfirmation }
/// Single consumed decision, with at most two pre-delivery validations for the
/// retained-pidfd adapter. It is not serializable, cloneable or replayable.
pub struct TerminationDelivery { proposal:TerminationProposal, cancelled:Arc<AtomicBool>, remaining:u8 }

impl Policy {
    #[allow(clippy::too_many_arguments)]
    pub fn propose_termination(&self,intent:Intent,desktop:NativeDesktop,presentation:TerminationPresentation,
        process_id:String,identity:ProcessIdentity,closure:String,verification_timeout_ms:u32,expiry_ms:u64)->Result<TerminationProposal> {
        if intent.mode!=Mode::Act || intent.subject.uid==0 || intent.subject.uid!=desktop.uid
            || intent.subject.boot_id!=desktop.boot_id || desktop.boot_id!=self.boot_id
            || identity.uid!=intent.subject.uid || identity.boot_id!=self.boot_id
            || identity.pid==intent.subject.pid {return Err(ErrorCode::PermissionDenied);}
        if !session_id(&desktop.session_id) || !hash(&desktop.identity_sha256)
            || !desktop.socket_name.starts_with("wayland-") || !bounded(&desktop.socket_name) || desktop.socket_name.contains('/')
            || !text(&presentation.target,256) || !text(&presentation.profile,256) || !text(&presentation.goal,4096)
            || digest(&presentation.goal)?!=intent.goal_sha256 || presentation.evidence.len()>16
            || presentation.evidence.iter().any(|e|!text(e,128)) || !uuid(&process_id)
            || identity.pid<=1 || identity.start_time_ticks==0 || identity.start_time_ticks>9_007_199_254_740_991
            || !text(&identity.executable_identity,256) || !store_closure(&closure) || !bounded(&closure)
            || verification_timeout_ms==0 || verification_timeout_ms>30_000 || expiry_ms==0 || expiry_ms>MAX_EXPIRY_MS {
            return Err(ErrorCode::InvalidArgument);
        }
        if requirement("process.terminate",intent.mode,Risk::R2)?!=Requirement::ExactApproval {return Err(ErrorCode::PolicyChanged);}
        let preview=ProcessPreview{identity,signal:"SIGTERM",verification_timeout_ms,reversible:false,automatic_escalation:false};
        let scope=Scope{actions:["process.terminate".into()].into(),resources:[
            Resource{field:"process_id".into(),kind:"scope-owner-expiry".into(),handle:process_id.clone(),identity_sha256:digest(&preview.identity)?},
            Resource{field:"selected_session".into(),kind:"graphical-session".into(),handle:desktop.session_id.clone(),identity_sha256:desktop.identity_sha256.clone()},
            Resource{field:"current_closure".into(),kind:"system-closure".into(),handle:closure.clone(),identity_sha256:digest(&closure)?},
        ].into(),..Default::default()};scope.validate()?;
        let issued_ms=boottime_ms()?;let expires_ms=issued_ms.checked_add(expiry_ms).ok_or(ErrorCode::InvalidArgument)?;
        let mut proposal=TerminationProposal{intent,scope,desktop,presentation,process_id,preview,closure,
            revision:self.revision.clone(),incarnation:self.incarnation,nonce:Uuid::new_v4(),issued_ms,expires_ms,digest:String::new()};
        proposal.digest=proposal.bound_digest()?;Ok(proposal)
    }
    pub fn consume_termination(&self,decision:NativeTerminationDecision,subject:&Subject,
        current:&impl CurrentResources)->Result<TerminationDelivery> {
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        decision.proposal.fresh(self,subject,current)?;
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        Ok(TerminationDelivery{proposal:decision.proposal,cancelled:decision.cancelled,remaining:2})
    }
}
impl TerminationProposal {
    fn bound_digest(&self)->Result<String> {
        digest(&(&self.intent.subject,&self.intent.request_id,&self.intent.goal_sha256,self.intent.mode,
            &self.scope,&self.revision,self.incarnation,self.nonce,self.issued_ms,self.expires_ms,
            &self.desktop.identity_sha256,&self.desktop.socket_name,self.wire(false)))
    }
    pub(super) fn fresh(&self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<()> {
        subject.validate()?;
        if &self.intent.subject!=subject {return Err(ErrorCode::PermissionDenied);}
        if policy.boot_id!=subject.boot_id || policy.incarnation!=self.incarnation {return Err(ErrorCode::ApprovalExpired);}
        if policy.revision!=self.revision {return Err(ErrorCode::PolicyChanged);}
        if self.bound_digest()?!=self.digest {return Err(ErrorCode::PlanChanged);}
        let time=boottime_ms()?;
        if time<self.issued_ms || time>=self.expires_ms || self.nonce.is_nil() {return Err(ErrorCode::ApprovalExpired);}
        for r in &self.scope.resources {
            if current.resolve(&r.field,&r.kind,&r.handle)?!=r.identity_sha256 {return Err(ErrorCode::TargetChanged);}
        }
        let after=boottime_ms()?;
        if after<self.issued_ms || after>=self.expires_ms {return Err(ErrorCode::ApprovalExpired);}Ok(())
    }
    pub(super) fn wire(&self,include_digest:bool)->Value {
        serde_json::json!({"schema_version":1,"kind":"process_termination","digest":if include_digest{self.digest.as_str()}else{""},
            "uid":self.desktop.uid,"session_id":self.desktop.session_id,"target":self.presentation.target,
            "profile":self.presentation.profile,"mode":self.intent.mode,"goal":self.presentation.goal,
            "process_id":self.process_id,"preview":self.preview,"closure":self.closure,
            "actions":["process.terminate"],"issued_ms":self.issued_ms,"expires_ms":self.expires_ms,"evidence":self.presentation.evidence})
    }
    pub fn confirmation_digest(&self)->&str {&self.digest}
    pub fn native_preview_digest(&self)->Result<String> {digest(&self.preview)}
    pub fn launch(self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<TerminationConfirmation> {
        Ok(TerminationConfirmation{native:PendingProposal::Termination(self).launch(policy,subject,current)?})
    }
}
impl TerminationConfirmation {
    pub fn cancellation(&self)->Result<ReadCancellation>{self.native.cancellation()}
    pub fn withdraw(&mut self){self.native.withdraw();}
    pub fn poll(&mut self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<Option<NativeTerminationDecision>> {
        match self.native.poll(policy,subject,current)? {
            None=>Ok(None),Some((PendingProposal::Termination(proposal),cancelled))=>Ok(Some(NativeTerminationDecision{proposal,cancelled})),
            Some(_)=>{self.native.withdraw();Err(ErrorCode::PolicyChanged)},
        }
    }
}
impl TerminationDelivery {
    pub fn revoke(&self){self.cancelled.store(true,Ordering::Release);}
    /// Trusted broker reauthenticates the original native caller before each
    /// callback, resolves native resources, and supplies the retained adapter's
    /// canonical preview digest. No prompt or model output can renew authority.
    pub fn revalidate(&mut self,policy:&Policy,subject:&Subject,request_id:&str,
        native_preview_sha256:&str,current:&impl CurrentResources)->Result<()> {
        let result=(||{
            if self.remaining==0 {return Err(ErrorCode::ApprovalExpired);}
            self.remaining-=1;
            if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
            if request_id!=self.proposal.intent.request_id {return Err(ErrorCode::PermissionDenied);}
            if native_preview_sha256!=self.proposal.native_preview_digest()? {return Err(ErrorCode::PlanChanged);}
            self.proposal.fresh(policy,subject,current)?;
            if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}Ok(())
        })();
        if result.is_err(){self.remaining=0;self.revoke();}result
    }
}
impl Drop for TerminationDelivery {fn drop(&mut self){self.revoke();}}

#[cfg(test)] mod tests;
