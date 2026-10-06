//! Synthetic approval fixtures, never installed/native confirmation evidence.
use super::*;
use crate::tests::{subject,policy,Resources};
fn proposal(p:&Policy,s:&Subject)->TerminationProposal {
    p.propose_termination(p.authenticated_user_intent(s.clone(),Uuid::new_v4().to_string(),"Stop this selected process",Mode::Act).unwrap(),
        NativeDesktop{uid:s.uid,boot_id:s.boot_id.clone(),session_id:"1".into(),identity_sha256:"d".repeat(64),socket_name:"wayland-0".into()},
        TerminationPresentation{target:"This VM".into(),profile:"Local CPU".into(),goal:"Stop this selected process".into(),evidence:vec![]},
        Uuid::new_v4().to_string(),ProcessIdentity{pid:201,uid:s.uid,start_time_ticks:50,boot_id:s.boot_id.clone(),executable_identity:"dev=1;ino=2;size=3;mtime=4:5;ctime=6:7".into()},
        format!("/nix/store/{}-nixos-system-aios-dev","0".repeat(32)),1000,90_000).unwrap()
}
fn decision(proposal:TerminationProposal)->NativeTerminationDecision {
    NativeTerminationDecision{proposal,cancelled:Arc::new(AtomicBool::new(false))}
}
#[test]
fn exact_process_preview_has_fixed_effects_and_private_nonce() {
    let s=subject();let p=policy(&s);let a=proposal(&p,&s);let b=proposal(&p,&s);
    let wire=a.wire(true);assert_eq!(wire["actions"],serde_json::json!(["process.terminate"]));
    assert_eq!(wire["preview"]["signal"],"SIGTERM");assert_eq!(wire["preview"]["automatic_escalation"],false);
    assert_eq!(wire["preview"]["reversible"],false);assert_eq!(wire["preview"]["identity"]["uid"],s.uid);
    assert!(!wire.to_string().contains("nonce"));assert_ne!(a.digest,b.digest);
    assert_eq!(a.native_preview_digest().unwrap(),digest(&serde_json::json!({"identity":a.preview.identity,
        "signal":"SIGTERM","verification_timeout_ms":1000,"reversible":false,"automatic_escalation":false})).unwrap());
}
#[test]
fn invalid_modes_foreign_or_self_targets_and_unbounded_presentations_are_refused() {
    let s=subject();let p=policy(&s);
    for field in 0..14 {
        let mut r=proposal(&p,&s);
        match field {
            0=>r.intent.mode=Mode::Ask,1=>r.intent.mode=Mode::Diagnose,2=>r.intent.mode=Mode::Automate,
            3=>r.preview.identity.uid+=1,4=>r.preview.identity.pid=s.pid,5=>r.preview.identity.boot_id=Uuid::new_v4().to_string(),
            6=>r.desktop.uid+=1,7=>r.presentation.goal="Changed request".into(),8=>r.preview.verification_timeout_ms=0,
            9=>r.preview.verification_timeout_ms=30_001,10=>r.desktop.socket_name="wayland-../escape".into(),
            11=>r.preview.identity.start_time_ticks=9_007_199_254_740_992,12=>r.process_id="claimed-process".into(),
            13=>r.closure="/tmp/claimed-closure".into(),_=>unreachable!(),
        }
        assert!(p.propose_termination(r.intent,r.desktop,r.presentation,r.process_id,r.preview.identity,r.closure,
            r.preview.verification_timeout_ms,90_000).is_err());
    }
}
#[test]
fn consumption_checks_owner_resources_policy_incarnation_expiry_and_immutable_plan() {
    let s=subject();let p=policy(&s);
    for field in 0..12 {
        let mut r=proposal(&p,&s);let mut resources=Resources(r.scope.resources.iter().cloned().collect());let mut peer=s.clone();
        match field {
            0=>peer.uid+=1,1=>peer.client=Client::Unix{connection_id:Uuid::new_v4().to_string()},
            2=>peer.start_ticks+=1,3=>r.incarnation=Uuid::new_v4(),4=>r.revision="f".repeat(64),
            5=>r.expires_ms=boottime_ms().unwrap(),6=>r.preview.verification_timeout_ms+=1,7=>r.presentation.goal="Different goal".into(),
            8=>resources.0[0].identity_sha256="a".repeat(64),9=>resources.0[1].identity_sha256="a".repeat(64),
            10=>resources.0[2].identity_sha256="a".repeat(64),11=>resources.0.clear(),_=>unreachable!(),
        }
        assert!(p.consume_termination(decision(r),&peer,&resources).is_err());
    }
}
#[test]
fn delivery_binds_request_and_native_preview_and_exhausts_without_replay() {
    let s=subject();let p=policy(&s);let r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
    let request=r.intent.request_id.clone();let preview=r.native_preview_digest().unwrap();
    let mut delivery=p.consume_termination(decision(r),&s,&resources).unwrap();
    for _ in 0..2 {assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Ok(()));}
    assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Err(ErrorCode::ApprovalExpired));
    for field in 0..5 {
        let r=proposal(&p,&s);let mut resources=Resources(r.scope.resources.iter().cloned().collect());
        let request=r.intent.request_id.clone();let preview=r.native_preview_digest().unwrap();
        let mut delivery=p.consume_termination(decision(r),&s,&resources).unwrap();
        assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Ok(()));
        let mut peer=s.clone();let mut changed_request=request.clone();let mut changed_preview=preview.clone();
        match field {0=>peer.pid+=1,1=>changed_request=Uuid::new_v4().to_string(),2=>changed_preview="a".repeat(64),
            3=>resources.0[0].identity_sha256="f".repeat(64),4=>delivery.revoke(),_=>unreachable!()}
        assert!(delivery.revalidate(&p,&peer,&changed_request,&changed_preview,&resources).is_err());
        assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Err(ErrorCode::ApprovalExpired));
    }
}
#[test]
fn independent_stop_revokes_delivered_decision_and_delivery() {
    let s=subject();let p=policy(&s);let r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
    let d=decision(r);d.cancelled.store(true,Ordering::Release);
    assert!(matches!(p.consume_termination(d,&s,&resources),Err(ErrorCode::Cancelled)));
    let r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
    let request=r.intent.request_id.clone();let preview=r.native_preview_digest().unwrap();
    let d=decision(r);let stop=d.cancelled.clone();let mut delivery=p.consume_termination(d,&s,&resources).unwrap();
    stop.store(true,Ordering::Release);
    assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Err(ErrorCode::Cancelled));
}
#[test]
fn expired_approval_and_policy_replacement_report_the_actual_reason() {
    let s=subject();let p=policy(&s);let mut r=proposal(&p,&s);
    let resources=Resources(r.scope.resources.iter().cloned().collect());
    r.expires_ms=boottime_ms().unwrap();r.digest=r.bound_digest().unwrap();
    assert!(matches!(p.consume_termination(decision(r),&s,&resources),Err(ErrorCode::ApprovalExpired)));
    let r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
    let request=r.intent.request_id.clone();let preview=r.native_preview_digest().unwrap();
    let mut delivery=p.consume_termination(decision(r),&s,&resources).unwrap();
    let changed=Policy{boot_id:p.boot_id.clone(),revision:"a".repeat(64),incarnation:p.incarnation};
    assert_eq!(delivery.revalidate(&changed,&s,&request,&preview,&resources),Err(ErrorCode::PolicyChanged));
    assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Err(ErrorCode::ApprovalExpired));
}
#[test]
fn final_clock_check_cannot_renew_exhausted_or_expired_delivery() {
    let s=subject();let p=policy(&s);let r=proposal(&p,&s);
    let resources=Resources(r.scope.resources.iter().cloned().collect());
    let request=r.intent.request_id.clone();let preview=r.native_preview_digest().unwrap();
    let mut delivery=p.consume_termination(decision(r),&s,&resources).unwrap();
    for _ in 0..2 {delivery.revalidate(&p,&s,&request,&preview,&resources).unwrap();}
    delivery.check_deadline().unwrap();
    assert_eq!(delivery.revalidate(&p,&s,&request,&preview,&resources),Err(ErrorCode::ApprovalExpired));
    let r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
    let mut delivery=p.consume_termination(decision(r),&s,&resources).unwrap();
    delivery.proposal.expires_ms=boottime_ms().unwrap();
    assert_eq!(delivery.check_deadline(),Err(ErrorCode::ApprovalExpired));
    assert!(delivery.cancelled.load(Ordering::Acquire));assert_eq!(delivery.remaining,0);
}
