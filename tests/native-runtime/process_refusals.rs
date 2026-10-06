//! Installed-service negatives with owned children and native consent.
//! Mechanical assistive input proves transport/effect behavior, not human review.
#[path = "../../crates/aios-session/tests/support/native_confirmation.rs"]
#[allow(dead_code)]
mod native_confirmation;
use aios_session::{bus::Client, display::DisplayBinding};
use aios_system::processes::{Identity, OwnProcess};
use serde_json::{Value, json};
use std::{fs, io::Read, os::fd::AsRawFd, process::{Child, Command, Stdio}, time::{Duration, Instant}};

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        // Fixture cleanup only: the product never receives SIGKILL authority.
        if self.0.try_wait().ok().flatten().is_none() { let _ = self.0.kill(); }
        let _ = self.0.wait();
    }
}
impl OwnedChild {
    fn new() -> Self {
        let mut child = Self(Command::new("/run/current-system/sw/bin/python3")
            .args(["-c", "import os,signal,time; signal.signal(signal.SIGTERM,lambda *_:os.write(1,b'T')); os.write(1,b'R'); time.sleep(300)"])
            .env_clear().stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap());
        child.witness(b'R'); child
    }
    fn witness(&mut self, expected: u8) {
        let stdout = self.0.stdout.as_mut().unwrap();
        let mut poll = nix::libc::pollfd { fd: stdout.as_raw_fd(), events: nix::libc::POLLIN, revents: 0 };
        assert_eq!(unsafe { nix::libc::poll(&mut poll, 1, 5000) }, 1, "owned child signal witness timed out");
        let mut byte = [0]; stdout.read_exact(&mut byte).unwrap(); assert_eq!(byte, [expected]);
    }
    fn live(&mut self, native: &OwnProcess, identity: &Identity) {
        assert!(self.0.try_wait().unwrap().is_none(), "product escalated or child unexpectedly exited");
        assert_eq!(native.inspect().unwrap().identity, *identity);
    }
    fn no_repeat(&self) {
        let mut poll = nix::libc::pollfd { fd: self.0.stdout.as_ref().unwrap().as_raw_fd(), events: nix::libc::POLLIN, revents: 0 };
        assert_eq!(unsafe { nix::libc::poll(&mut poll, 1, 0) }, 0, "product retried SIGTERM or closed child witness");
    }
}
fn status(client: &Client, id: &str, expected: &str, seconds: u64) -> Value {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    loop {
        let value = client.process_termination_status(id).unwrap();
        if value["state"] == expected { return value; }
        assert!(matches!(value["state"].as_str(), Some("queued" | "needs_permission")), "unexpected state: {value}");
        assert!(Instant::now() < deadline, "status deadline: {value}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn select(client: &Client, identity: &Identity) -> String {
    let mut cursor = None; let mut visited = std::collections::HashSet::new();
    for _ in 0..16 {
        let value = client.list_processes(cursor.as_deref()).unwrap();
        for row in value["data"]["processes"].as_array().unwrap() {
            if row["pid"] == identity.pid {
                assert_eq!(row["start_time_ticks"], identity.start_time_ticks);
                let handle = row["process_id"].as_str().unwrap();
                assert_eq!(client.inspect_process(handle).unwrap()["data"]["executable_identity"], identity.executable_identity);
                return handle.into();
            }
        }
        cursor = value["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none() { break; } assert!(visited.insert(cursor.clone()));
    }
    panic!("owned child missing from native inventory");
}

#[test]
#[ignore = "requires registered disposable installed native process provider"]
fn installed_ignored_sigterm_and_post_delivery_stop() {
    assert_eq!(std::env::var("AIOS_NATIVE_BRIDGE_SCENARIO").unwrap(), "disposable-provider-v1");
    assert_eq!(nix::unistd::geteuid().as_raw(), 1001);
    assert_eq!(fs::read_to_string("/etc/aios/desktop-test-profile").unwrap().trim(), "synthetic-disposable-plasma-wayland-v1");
    let probe = Command::new("/run/current-system/sw/bin/aios-desktop-test-probe").output().unwrap();
    assert!(probe.status.success()); let observed: Value = serde_json::from_slice(&probe.stdout).unwrap();
    let display = DisplayBinding::observe(observed["session_id"].as_str().unwrap(), 1001).unwrap();
    let mut results = Vec::new();
    // These are sequential tasks from one original caller, not a client-quota
    // stress test. Keep its authenticated native connection throughout.
    let client = Client::connect_user_bus().unwrap();
    for mode in ["deadline", "cancel", "forget"] {
        let mut child = OwnedChild::new();
        let native = OwnProcess::open(child.0.id()).unwrap(); let identity = native.inspect().unwrap().identity;
        let selected = client.select_ui_session(&display.session.id).unwrap();
        let handle = select(&client, &identity); let id = uuid::Uuid::new_v4().to_string();
        let goal = format!("Native termination fixture ignored {mode} {id}");
        client.start_process_termination(&id, &handle, selected["candidate_handle"].as_str().unwrap(), &goal).unwrap();
        assert!(status(&client, &id, "needs_permission", 8)["receipt"].is_null());
        child.live(&native, &identity);
        let consent = native_confirmation::allow_owned_termination(&display, &goal, &native, &handle);
        // Native child handler acknowledges actual SIGTERM before any Stop/Forget.
        child.witness(b'T'); child.live(&native, &identity); let delivered = Instant::now();
        let mut stop_ms = None;
        let terminal = if mode == "forget" {
            client.forget_process_termination(&id).unwrap();
            assert!(client.process_termination_status(&id).is_err());
            assert!(client.start_process_termination(&id, &handle, selected["candidate_handle"].as_str().unwrap(), &goal).is_err());
            Value::Null
        } else {
            if mode == "cancel" {
                let started = Instant::now(); client.cancel_process_termination(&id).unwrap();
                stop_ms = Some(started.elapsed().as_millis()); assert!(stop_ms.unwrap() < 1000);
            }
            let value = status(&client, &id, "partial", 35);
            if mode == "deadline" { assert!(delivered.elapsed() >= Duration::from_secs(29), "premature deadline receipt"); }
            assert_eq!(value["error"], if mode == "deadline" { "DEADLINE_EXCEEDED" } else { "CANCELLED" });
            let receipt = &value["receipt"];
            assert_eq!(receipt["preview"]["identity"], serde_json::to_value(&identity).unwrap());
            assert_eq!(receipt["preview"]["signal"], "SIGTERM");
            assert_eq!(receipt["preview"]["verification_timeout_ms"], 30000);
            assert_eq!(receipt["preview"]["automatic_escalation"], false); assert_eq!(receipt["preview"]["reversible"], false);
            assert_eq!(receipt["signal_requested"], true); assert_eq!(receipt["verified_exit"], false); assert_eq!(receipt["complete"], false);
            assert_eq!(receipt["error"], value["error"]);
            assert_eq!(client.process_termination_status(&id).unwrap(), value);
            client.forget_process_termination(&id).unwrap(); value
        };
        // Observe past terminal status/forget, before separate fixture cleanup.
        std::thread::sleep(Duration::from_secs(1)); child.live(&native, &identity); child.no_repeat();
        let result = json!({"case":mode,"identity":identity,"consent":consent,"terminal":terminal,
            "signal_handler_witness":true,"alive_after_terminal_or_forget":true,"no_repeat_signal":true,"stop_ms":stop_ms,
            "elapsed_after_signal_ms":delivered.elapsed().as_millis(),"fixture_cleanup":"owned child only; SIGKILL after assertions"});
        println!("NATIVE_MANAGED_PROCESS_REFUSALS_CASE={result}"); results.push(result);
    }
    display.verify().unwrap();
    println!("NATIVE_MANAGED_PROCESS_REFUSALS={}", json!({"evidence_kind":"native-installed-provider-negative-fixtures-not-human-review",
        "display":display,"cases":results,"no_product_escalation":true,"no_false_verified_exit":true}));
}
