//! Registered disposable desktop only. A controlled child and test-only
//! assistive input exercise the real managed broker, native consent and pidfd.
//! Mechanical input is explicitly not evidence of human review.
#[path="support/native_confirmation.rs"] mod native_confirmation;
use aios_session::{bus::Client,display::DisplayBinding};
use aios_system::processes::OwnProcess;
use serde_json::{json,Value};
use std::{collections::HashSet,fs,process::{Child,Command,Stdio},time::{Duration,Instant}};
struct OwnedChild(Child);
impl Drop for OwnedChild{
    fn drop(&mut self){if self.0.try_wait().ok().flatten().is_none(){let _=self.0.kill();}let _=self.0.wait();}
}
fn until(client:&Client,id:&str,state:&str)->Value{
    let deadline=Instant::now()+Duration::from_secs(8);
    loop{
        let status=client.process_termination_status(id).unwrap();
        if status["state"]==state{return status;}
        assert!(matches!(status["state"].as_str(),Some("queued"|"needs_permission")),"unexpected termination state: {status}");
        assert!(Instant::now()<deadline,"native termination status timed out: {status}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
#[ignore="requires registered disposable native managed process provider"]
fn managed_native_confirmation_cancel_and_verified_sigterm(){
    assert_eq!(std::env::var("AIOS_NATIVE_BRIDGE_SCENARIO").unwrap(),"disposable-provider-v1");
    assert_eq!(nix::unistd::geteuid().as_raw(),1001);
    assert_eq!(fs::read_to_string("/etc/aios/desktop-test-profile").unwrap().trim(),"synthetic-disposable-plasma-wayland-v1");
    let probe=Command::new("/run/current-system/sw/bin/aios-desktop-test-probe").output().unwrap();assert!(probe.status.success());
    let observed:Value=serde_json::from_slice(&probe.stdout).unwrap();
    let display=DisplayBinding::observe(observed["session_id"].as_str().unwrap(),1001).unwrap();
    let mut child=OwnedChild(Command::new("/run/current-system/sw/bin/sleep").arg("300").env_clear()
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    let native=OwnProcess::open(child.0.id()).unwrap();let identity=native.inspect().unwrap().identity;
    let client=Client::connect_user_bus().unwrap();
    let selected=client.select_ui_session(&display.session.id).unwrap();
    let session=selected["candidate_handle"].as_str().unwrap();
    let mut cursor:Option<String>=None;let mut seen=HashSet::new();let mut handle=None;
    for _ in 0..16{
        let inventory=client.list_processes(cursor.as_deref()).unwrap();
        for row in inventory["data"]["processes"].as_array().unwrap(){
            if row["pid"]==identity.pid{
                assert_eq!(row["start_time_ticks"],identity.start_time_ticks);assert!(handle.is_none());
                handle=Some(row["process_id"].as_str().unwrap().to_owned());
            }
        }
        if handle.is_some(){break;}
        cursor=inventory["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none(){break;}assert!(seen.insert(cursor.clone()));
    }
    let handle=handle.expect("actual owned child omitted from native inventory");
    let inspected=client.inspect_process(&handle).unwrap();assert_eq!(inspected["complete"],true);
    assert_eq!(inspected["data"]["executable_identity"],identity.executable_identity);
    let foreign=Client::connect_user_bus().unwrap();
    assert!(foreign.inspect_process(&handle).is_err(),"reconnected caller inherited a native process handle");
    let cancelled=uuid::Uuid::new_v4().to_string();
    let cancel_goal=format!("Native termination fixture cancel {cancelled}");
    client.start_process_termination(&cancelled,&handle,session,&cancel_goal).unwrap();
    let pending=until(&client,&cancelled,"needs_permission");assert!(pending["receipt"].is_null());
    assert!(child.0.try_wait().unwrap().is_none());
    assert!(foreign.process_termination_status(&cancelled).is_err());
    assert!(foreign.cancel_process_termination(&cancelled).is_err());
    let started=Instant::now();client.cancel_process_termination(&cancelled).unwrap();let stop_ms=started.elapsed().as_millis();
    assert!(stop_ms<1000);let stopped=until(&client,&cancelled,"cancelled");assert!(stopped["receipt"].is_null());
    assert_eq!(native.inspect().unwrap().identity,identity);assert!(child.0.try_wait().unwrap().is_none());
    client.forget_process_termination(&cancelled).unwrap();assert!(client.process_termination_status(&cancelled).is_err());
    assert!(client.start_process_termination(&cancelled,&handle,session,&cancel_goal).is_err(),"forgotten task UUID replayed");
    let task=uuid::Uuid::new_v4().to_string();let goal=format!("Native termination fixture exit {task}");
    client.start_process_termination(&task,&handle,session,&goal).unwrap();until(&client,&task,"needs_permission");
    assert!(child.0.try_wait().unwrap().is_none());
    let consent=native_confirmation::allow_owned_termination(&display,&goal,&native,&handle);
    let completed=until(&client,&task,"completed");let receipt=&completed["receipt"];
    assert_eq!(receipt["preview"]["identity"],serde_json::to_value(&identity).unwrap());
    assert_eq!(receipt["preview"]["signal"],"SIGTERM");assert_eq!(receipt["preview"]["verification_timeout_ms"],30000);
    assert_eq!(receipt["preview"]["automatic_escalation"],false);assert_eq!(receipt["preview"]["reversible"],false);
    assert_eq!(receipt["signal_requested"],true);assert_eq!(receipt["verified_exit"],true);assert_eq!(receipt["complete"],true);
    assert!(receipt["error"].is_null());
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(child.0.wait().unwrap().signal(),Some(nix::libc::SIGTERM),"owned child exit was not SIGTERM");
    assert_eq!(client.process_termination_status(&task).unwrap(),completed,"terminal receipt changed");
    client.forget_process_termination(&task).unwrap();assert!(client.process_termination_status(&task).is_err());
    display.verify().unwrap();
    println!("NATIVE_MANAGED_PROCESS_TERMINATION={}",json!({"evidence_kind":"actual-managed-package-native-consent-and-pidfd-with-owned-child-assistive-input-fixture-not-human-review-or-installed-image",
        "identity":identity,"display":display,"consent":consent,"cancelled":stopped,"completed":completed,"stop_ms":stop_ms,
        "pending_no_signal":true,"cancel_no_signal":true,"foreign_handle_denied":true,"foreign_status_and_stop_denied":true,
        "forgotten_replay_denied":true,"actual_child_exit_signal":"SIGTERM","stable_receipt":true}));
}
