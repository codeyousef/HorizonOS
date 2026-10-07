//! Exact R2 power-profile consent. Native brokers bind the requested profile,
//! prior profile, advertised choices, desktop identity and live provider state.
use super::*;

#[derive(Clone,PartialEq,Eq,Serialize)]
struct PowerProfilePreview {
    prior:String, requested:String, available_profiles:Vec<String>, reversible:bool,
}
pub struct PowerProfilePresentation {
    pub target:String, pub profile:String, pub goal:String, pub evidence:Vec<String>,
}
pub struct PowerProfileProposal {
    intent:Intent, scope:Scope, pub(super) desktop:NativeDesktop,
    presentation:PowerProfilePresentation, preview:PowerProfilePreview,
    revision:String, incarnation:Uuid, nonce:Uuid, issued_ms:u64, expires_ms:u64,
    pub(super) digest:String,
}
pub struct NativePowerProfileDecision {proposal:PowerProfileProposal,cancelled:Arc<AtomicBool>}
pub struct PowerProfileConfirmation {native:NativeConfirmation}
pub struct PowerProfileDelivery {proposal:PowerProfileProposal,cancelled:Arc<AtomicBool>,remaining:u8}

impl Policy {
    #[allow(clippy::too_many_arguments)]
    pub fn propose_power_profile(&self,intent:Intent,desktop:NativeDesktop,
        presentation:PowerProfilePresentation,prior:String,requested:String,
        available_profiles:Vec<String>,expiry_ms:u64)->Result<PowerProfileProposal> {
        if intent.mode!=Mode::Act || intent.subject.uid==0 || intent.subject.uid!=desktop.uid
            || intent.subject.boot_id!=desktop.boot_id || desktop.boot_id!=self.boot_id{return Err(ErrorCode::PermissionDenied);}
        if !session_id(&desktop.session_id)||!hash(&desktop.identity_sha256)
            || !desktop.socket_name.starts_with("wayland-")||!bounded(&desktop.socket_name)||desktop.socket_name.contains('/')
            || !text(&presentation.target,256)||!text(&presentation.profile,256)||!text(&presentation.goal,4096)
            || digest(&presentation.goal)?!=intent.goal_sha256||presentation.evidence.len()>16
            || presentation.evidence.iter().any(|value|!text(value,128))||expiry_ms==0||expiry_ms>MAX_EXPIRY_MS
            || available_profiles.is_empty()||available_profiles.len()>3
            || available_profiles.windows(2).any(|pair|pair[0]>=pair[1])
            || available_profiles.iter().any(|value|!matches!(value.as_str(),"power-saver"|"balanced"|"performance"))
            || !available_profiles.contains(&prior)||!available_profiles.contains(&requested){return Err(ErrorCode::InvalidArgument);}
        if requirement("power.profile_set",intent.mode,Risk::R2)?!=Requirement::ExactApproval{return Err(ErrorCode::PolicyChanged);}
        let preview=PowerProfilePreview{prior,requested,available_profiles,reversible:true};
        let scope=Scope{actions:["power.profile_set".into()].into(),resources:[Resource{
            field:"power_profile".into(),kind:"native-setting-state".into(),handle:"current".into(),identity_sha256:digest(&(&preview.prior,&preview.available_profiles))?,
        }].into(),..Default::default()};scope.validate()?;
        let issued_ms=boottime_ms()?;let expires_ms=issued_ms.checked_add(expiry_ms).ok_or(ErrorCode::InvalidArgument)?;
        let mut proposal=PowerProfileProposal{intent,scope,desktop,presentation,preview,revision:self.revision.clone(),
            incarnation:self.incarnation,nonce:Uuid::new_v4(),issued_ms,expires_ms,digest:String::new()};
        proposal.digest=proposal.bound_digest()?;Ok(proposal)
    }
    pub fn consume_power_profile(&self,decision:NativePowerProfileDecision,subject:&Subject,
        current:&impl CurrentResources)->Result<PowerProfileDelivery>{
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        decision.proposal.fresh(self,subject,current)?;
        if decision.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        Ok(PowerProfileDelivery{proposal:decision.proposal,cancelled:decision.cancelled,remaining:1})
    }
}
impl PowerProfileProposal {
    fn bound_digest(&self)->Result<String>{digest(&(&self.intent.subject,&self.intent.request_id,&self.intent.goal_sha256,
        self.intent.mode,&self.scope,&self.revision,self.incarnation,self.nonce,self.issued_ms,self.expires_ms,
        &self.desktop.identity_sha256,&self.desktop.socket_name,self.wire(false)))}
    pub(super) fn fresh(&self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<()>{
        subject.validate()?;if &self.intent.subject!=subject{return Err(ErrorCode::PermissionDenied);}
        if policy.boot_id!=subject.boot_id||policy.incarnation!=self.incarnation{return Err(ErrorCode::ApprovalExpired);}
        if policy.revision!=self.revision{return Err(ErrorCode::PolicyChanged);}
        if self.bound_digest()!=Ok(self.digest.clone()){return Err(ErrorCode::PlanChanged);}
        let now=boottime_ms()?;if now<self.issued_ms||now>=self.expires_ms||self.nonce.is_nil(){return Err(ErrorCode::ApprovalExpired);}
        for resource in &self.scope.resources{if current.resolve(&resource.field,&resource.kind,&resource.handle)?!=resource.identity_sha256{return Err(ErrorCode::TargetChanged);}}
        current.dynamic_arguments("power.profile_set",&serde_json::json!({"profile":self.preview.requested}),&self.scope)?;
        let after=boottime_ms()?;if after<self.issued_ms||after>=self.expires_ms{return Err(ErrorCode::ApprovalExpired);}Ok(())
    }
    pub(super) fn wire(&self,include_digest:bool)->Value{serde_json::json!({"schema_version":1,"kind":"power_profile_change",
        "digest":if include_digest{self.digest.as_str()}else{""},"uid":self.desktop.uid,"session_id":self.desktop.session_id,
        "target":self.presentation.target,"profile":self.presentation.profile,"mode":self.intent.mode,"goal":self.presentation.goal,
        "preview":self.preview,"actions":["power.profile_set"],"issued_ms":self.issued_ms,"expires_ms":self.expires_ms,
        "evidence":self.presentation.evidence})}
    pub fn confirmation_digest(&self)->&str{&self.digest}
    pub fn native_preview_digest(&self)->Result<String>{digest(&self.preview)}
    pub fn launch(self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<PowerProfileConfirmation>{
        Ok(PowerProfileConfirmation{native:PendingProposal::PowerProfile(self).launch(policy,subject,current)?})
    }
}
impl PowerProfileConfirmation {
    pub fn cancellation(&self)->Result<ReadCancellation>{self.native.cancellation()}
    pub fn withdraw(&mut self){self.native.withdraw();}
    pub fn poll(&mut self,policy:&Policy,subject:&Subject,current:&impl CurrentResources)->Result<Option<NativePowerProfileDecision>>{
        match self.native.poll(policy,subject,current)?{
            None=>Ok(None),Some((PendingProposal::PowerProfile(proposal),cancelled))=>Ok(Some(NativePowerProfileDecision{proposal,cancelled})),
            Some(_)=>{self.native.withdraw();Err(ErrorCode::PolicyChanged)},
        }
    }
}
impl PowerProfileDelivery {
    pub fn revoke(&self){self.cancelled.store(true,Ordering::Release);}
    pub fn revalidate(&mut self,policy:&Policy,subject:&Subject,request_id:&str,native_preview_sha256:&str,
        current:&impl CurrentResources)->Result<()>{
        let result=(||{if self.remaining==0{return Err(ErrorCode::ApprovalExpired);}self.remaining-=1;
            if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
            if request_id!=self.proposal.intent.request_id{return Err(ErrorCode::PermissionDenied);}
            if native_preview_sha256!=self.proposal.native_preview_digest()?{return Err(ErrorCode::PlanChanged);}
            self.proposal.fresh(policy,subject,current)?;
            if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}Ok(())})();
        if result.is_err(){self.remaining=0;self.revoke();}result
    }
    pub fn check_deadline(&mut self)->Result<()>{
        let result=(||{if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
            let now=boottime_ms()?;if now<self.proposal.issued_ms||now>=self.proposal.expires_ms{return Err(ErrorCode::ApprovalExpired);}Ok(())})();
        if result.is_err(){self.revoke();}result
    }
}
impl Drop for PowerProfileDelivery{fn drop(&mut self){self.revoke();}}

#[cfg(test)] mod tests;
