use super::*;
use crate::tests::{policy,subject};
struct Resources{resource:Resource,requested:String}
impl CurrentResources for Resources{
    fn resolve(&self,field:&str,kind:&str,handle:&str)->Result<String>{
        if (field,kind,handle)==(self.resource.field.as_str(),self.resource.kind.as_str(),self.resource.handle.as_str()){
            Ok(self.resource.identity_sha256.clone())
        }else{Err(ErrorCode::PermissionDenied)}
    }
    fn dynamic_arguments(&self,id:&str,args:&Value,scope:&Scope)->Result<()>{
        if id=="power.profile_set"&&args==&serde_json::json!({"profile":self.requested})&&scope.actions.contains(id){Ok(())}else{Err(ErrorCode::PermissionDenied)}
    }
}
fn proposal(policy:&Policy,subject:&Subject)->PowerProfileProposal{
    policy.propose_power_profile(policy.authenticated_user_intent(subject.clone(),Uuid::new_v4().to_string(),
        "Use the power-saving profile",Mode::Act).unwrap(),NativeDesktop{uid:subject.uid,boot_id:subject.boot_id.clone(),
        session_id:"1".into(),identity_sha256:"d".repeat(64),socket_name:"wayland-0".into()},
        PowerProfilePresentation{target:"This VM".into(),profile:"Local CPU".into(),goal:"Use the power-saving profile".into(),evidence:vec![]},
        "balanced".into(),"power-saver".into(),vec!["balanced".into(),"performance".into(),"power-saver".into()],90_000).unwrap()
}
fn resources(proposal:&PowerProfileProposal)->Resources{Resources{resource:proposal.scope.resources.iter().next().unwrap().clone(),requested:proposal.preview.requested.clone()}}
fn decision(proposal:PowerProfileProposal)->NativePowerProfileDecision{NativePowerProfileDecision{proposal,cancelled:Arc::new(AtomicBool::new(false))}}
#[test]fn proposal_binds_exact_reversible_profile_and_rejects_expansion(){
    let subject=subject();let policy=policy(&subject);let proposal=proposal(&policy,&subject);let wire=proposal.wire(true);
    assert_eq!(wire["kind"],"power_profile_change");assert_eq!(wire["actions"],serde_json::json!(["power.profile_set"]));
    assert_eq!(wire["preview"]["prior"],"balanced");assert_eq!(wire["preview"]["requested"],"power-saver");assert_eq!(wire["preview"]["reversible"],true);
    for choices in [vec![],vec!["balanced".into(),"balanced".into()],vec!["balanced".into(),"turbo".into()]]{
        let intent=policy.authenticated_user_intent(subject.clone(),Uuid::new_v4().to_string(),"Use the power-saving profile",Mode::Act).unwrap();
        assert!(policy.propose_power_profile(intent,NativeDesktop{uid:subject.uid,boot_id:subject.boot_id.clone(),session_id:"1".into(),identity_sha256:"d".repeat(64),socket_name:"wayland-0".into()},
            PowerProfilePresentation{target:"This VM".into(),profile:"Local CPU".into(),goal:"Use the power-saving profile".into(),evidence:vec![]},
            "balanced".into(),"power-saver".into(),choices,90_000).is_err());
    }
}
#[test]fn decision_is_single_use_and_live_state_bound(){
    let subject=subject();let policy=policy(&subject);let first=proposal(&policy,&subject);let current=resources(&first);
    let request=first.intent.request_id.clone();let preview=first.native_preview_digest().unwrap();
    let mut delivery=policy.consume_power_profile(decision(first),&subject,&current).unwrap();
    assert_eq!(delivery.revalidate(&policy,&subject,&request,&preview,&current),Ok(()));
    assert_eq!(delivery.revalidate(&policy,&subject,&request,&preview,&current),Err(ErrorCode::ApprovalExpired));
    let second=proposal(&policy,&subject);let mut changed=resources(&second);changed.resource.identity_sha256="a".repeat(64);
    assert!(policy.consume_power_profile(decision(second),&subject,&changed).is_err());
}
