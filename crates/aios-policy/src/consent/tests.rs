//! Synthetic policy fixtures. No simulated/native Allow constitutes guest
//! confirmation evidence; private construction here exercises postconditions.
use super::*;
use crate::tests::{subject, policy, Resources};
use aios_protocol::contracts::parse_tool_call;
fn proposal(p: &Policy, s: &Subject) -> ReadProposal {
    p.propose_graphical_read(p.authenticated_user_intent(s.clone(),Uuid::new_v4().to_string(),"Explain this document",Mode::Ask).unwrap(),
        NativeDesktop {uid:s.uid,boot_id:s.boot_id.clone(),session_id:"1".into(),identity_sha256:"d".repeat(64),socket_name:"wayland-0".into()},
        ReadPresentation {target:"This VM".into(),profile:"Local CPU".into(),goal:"Explain this document".into(),
            windows:vec![SelectedWindow{handle:"selected-window".into(),identity_sha256:"e".repeat(64),name:"Kate".into(),window:"Document".into()}],evidence:vec![]},90_000).unwrap()
}
#[test]
fn frozen_presentation_goal_uid_mode_and_window_uniqueness_are_enforced() {
    let s=subject();let p=policy(&s);
    for field in 0..10 {
        let mut r=proposal(&p,&s);
        match field {
            0=>r.presentation.goal="Another task".into(),1=>r.desktop.uid+=1,2=>r.intent.mode=Mode::Act,
            3=>r.presentation.windows.push(r.presentation.windows[0].clone()),4=>r.desktop.session_id="../1".into(),
            5=>r.desktop.socket_name="wayland-../escape".into(),6=>r.presentation.windows[0].identity_sha256="bad".into(),
            7=>r.presentation.windows[0].window="safe\u{202e}unsafe".into(),8=>r.presentation.goal="x".repeat(4097),
            9=>r.desktop.boot_id=Uuid::new_v4().to_string(),_=>unreachable!(),
        }
        assert!(p.propose_graphical_read(r.intent,r.desktop,r.presentation,90_000).is_err());
    }
    let a=proposal(&p,&s);let b=proposal(&p,&s);
    assert_ne!(a.digest,b.digest);assert!(!a.wire(true).to_string().contains("nonce"));
}
#[test]
fn native_response_requires_one_exact_canonical_line_and_matching_digest() {
    let hash="a".repeat(64);
    let allow=format!("{{\"decision\":\"allow\",\"digest\":\"{hash}\"}}\n");
    assert_eq!(allowed(allow.as_bytes(),&hash),Ok(true));
    assert_eq!(allowed(allow.replace("allow","cancel").as_bytes(),&hash),Ok(false));
    for bytes in [allow.clone()+"\n",allow.clone()+"{}",allow.replace("allow","ALLOW"),
        allow.replace("\"decision\":\"allow\"","\"decision\":\"cancel\",\"decision\":\"allow\""),
        allow.replace('a',"b"),allow.replace(":",": "),format!("{{\"approved\":true,\"decision\":\"allow\",\"digest\":\"{hash}\"}}\n")] {
        assert_eq!(allowed(bytes.as_bytes(),&hash),Err(ErrorCode::InvalidArgument));
    }
}
#[test]
fn native_scope_is_owner_bound_single_use_and_rechecks_all_resources_on_every_read() {
    let s=subject();let p=policy(&s);let r=proposal(&p,&s);
    let resources=Resources(r.scope.resources.iter().cloned().collect());
    let request=r.intent.request_id.clone();
    let g=p.consume_graphical_read(NativeReadDecision{proposal:r,cancelled:Arc::new(AtomicBool::new(false))},&s,&resources).unwrap();
    let action=parse_tool_call(br#"{"kind":"tool_call","action_id":"ui.snapshot","arguments":{"window_handle":"selected-window"}}"#).unwrap();
    assert_eq!(p.check_read(&g,&s,&request,&action,&resources,boottime_ms().unwrap()),Ok(()));
    let mut foreign=s.clone();foreign.client=Client::Unix{connection_id:Uuid::new_v4().to_string()};
    assert_eq!(p.check_read(&g,&foreign,&request,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::PermissionDenied));
    let mut drift=resources.0.clone();drift.iter_mut().find(|r|r.kind=="graphical-session").unwrap().identity_sha256="f".repeat(64);
    assert_eq!(p.check_read(&g,&s,&request,&action,&Resources(drift),boottime_ms().unwrap()),Err(ErrorCode::TargetChanged));
    assert_eq!(p.check_read(&g,&s,&request,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::ApprovalExpired));
}
#[test]
fn pending_confirmation_and_consumption_deny_restart_expiry_scope_and_client_drift() {
    let s=subject();let p=policy(&s);
    for field in 0..6 {
        let mut r=proposal(&p,&s);let resources=Resources(r.scope.resources.iter().cloned().collect());
        let mut native=s.clone();let mut current=resources.0.clone();
        match field {0=>r.expires_ms=boottime_ms().unwrap(),1=>r.incarnation=Uuid::new_v4(),2=>r.revision="f".repeat(64),
            3=>native.start_ticks+=1,4=>current[0].identity_sha256="a".repeat(64),5=>current.clear(),_=>unreachable!()}
        assert!(p.consume_graphical_read(NativeReadDecision{proposal:r,cancelled:Arc::new(AtomicBool::new(false))},&native,&Resources(current)).is_err());
    }
}
#[test]
fn independent_stop_revokes_both_a_delivered_decision_and_an_issued_grant() {
    let s=subject();let p=policy(&s);let r=proposal(&p,&s);
    let resources=Resources(r.scope.resources.iter().cloned().collect());
    let cancelled=Arc::new(AtomicBool::new(true));
    assert!(matches!(p.consume_graphical_read(NativeReadDecision{proposal:r,cancelled},&s,&resources),Err(ErrorCode::Cancelled)));
    let r=proposal(&p,&s);let request=r.intent.request_id.clone();
    let cancelled=Arc::new(AtomicBool::new(false));
    let g=p.consume_graphical_read(NativeReadDecision{proposal:r,cancelled:cancelled.clone()},&s,&resources).unwrap();
    let action=parse_tool_call(br#"{"kind":"tool_call","action_id":"ui.snapshot","arguments":{"window_handle":"selected-window"}}"#).unwrap();
    cancelled.store(true,Ordering::Release);
    assert_eq!(p.check_read(&g,&s,&request,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::ApprovalExpired));
}

struct SelectorResources(Resources);
impl CurrentResources for SelectorResources {
    fn resolve(&self,field:&str,kind:&str,handle:&str)->Result<String>{self.0.resolve(field,kind,handle)}
    fn dynamic_arguments(&self,id:&str,args:&Value,scope:&Scope)->Result<()> {
        if id=="ui.find" && scope.actions.contains(id) && scope.resources.iter().any(|r|r.field=="snapshot_id" && args["snapshot_id"]==r.handle) {Ok(())}else{Err(ErrorCode::PermissionDenied)}
    }
}
#[test]
fn selectors_require_explicit_native_consent_and_exact_derived_snapshot() {
    let s=subject();let p=policy(&s);
    let window=parse_tool_call(br#"{"kind":"tool_call","action_id":"ui.snapshot","arguments":{"window_handle":"selected-window"}}"#).unwrap();
    for enabled in [false,true] {
        let r=proposal(&p,&s);
        let r=if enabled {p.propose_graphical_selector_read(r.intent,r.desktop,r.presentation,90_000).unwrap()}else{r};
        assert_eq!(r.wire(true)["actions"].as_array().unwrap().len(),if enabled{2}else{1});
        let request=r.intent.request_id.clone();
        let snapshot=Resource{field:"snapshot_id".into(),kind:"scope-owner-expiry".into(),handle:Uuid::new_v4().to_string(),identity_sha256:"f".repeat(64)};
        let mut resources=SelectorResources(Resources(r.scope.resources.iter().cloned().collect()));resources.0.0.push(snapshot.clone());
        let g=p.consume_graphical_read(NativeReadDecision{proposal:r,cancelled:Arc::new(AtomicBool::new(false))},&s,&resources).unwrap();
        let find=|id:&str|parse_tool_call(&serde_json::to_vec(&serde_json::json!({"kind":"tool_call","action_id":"ui.find","arguments":{"snapshot_id":id,"selector":{"name":"Document"}}})).unwrap()).unwrap();
        let action=find(&snapshot.handle);
        assert_eq!(p.check_graphical_selector(&g,&s,&request,&window,&snapshot,&action,&resources,boottime_ms().unwrap()),if enabled{Ok(())}else{Err(ErrorCode::PermissionDenied)});
        // A resource UUID by itself cannot make a selector usable via generic
        // check_read, and a missing/mismatched handle cannot enter the scope.
        assert_eq!(p.check_read(&g,&s,&request,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::PermissionDenied));
        assert_eq!(p.check_graphical_selector(&g,&s,&request,&window,&snapshot,&find(&Uuid::new_v4().to_string()),&resources,boottime_ms().unwrap()),Err(ErrorCode::PermissionDenied));
        if !enabled {continue;}
        let mut foreign=s.clone();foreign.client=Client::Unix{connection_id:Uuid::new_v4().to_string()};
        assert_eq!(p.check_graphical_selector(&g,&foreign,&request,&window,&snapshot,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::PermissionDenied));
        assert_eq!(p.check_graphical_selector(&g,&s,&Uuid::new_v4().to_string(),&window,&snapshot,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::PermissionDenied));
        assert_eq!(policy(&s).check_graphical_selector(&g,&s,&request,&window,&snapshot,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::ApprovalExpired));
        let mut invalid=snapshot.clone();invalid.field="node_handle".into();
        assert_eq!(p.check_graphical_selector(&g,&s,&request,&window,&invalid,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::InvalidArgument));
        let mut drift=resources.0.0.clone();drift.last_mut().unwrap().identity_sha256="a".repeat(64);
        assert_eq!(p.check_graphical_selector(&g,&s,&request,&window,&snapshot,&action,&SelectorResources(Resources(drift)),boottime_ms().unwrap()),Err(ErrorCode::TargetChanged));
        assert_eq!(p.check_graphical_selector(&g,&s,&request,&window,&snapshot,&action,&resources,boottime_ms().unwrap()),Err(ErrorCode::ApprovalExpired));
    }
}
