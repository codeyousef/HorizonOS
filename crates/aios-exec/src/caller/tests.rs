//! Parser/identity drift fixtures plus read-only actual guest bus observations.
use super::*;
fn fixture() -> Snapshot {
    Snapshot {
        caller: CallerIdentity {
            uid: 1000,
            pid: 123,
            start_ticks: 42,
            boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
            bus_id: "a".repeat(32),
            sender: ":1.7".into(),
            session: Some(SessionIdentity {
                id: "c1".into(),
                remote: false,
                kind: "wayland".into(),
                class: "user".into(),
                state: "active".into(),
                active: true,
            }),
        },
        logind: ServiceIdentity {
            sender: ":1.2".into(),
            uid: 0,
            pid: 99,
            start_ticks: 20,
            boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
        },
    }
}
fn stat(pid: u32, name: &str, ticks: &str) -> String {
    format!("{pid} ({name}) S {} {ticks} 1 2", vec!["0"; 18].join(" "))
}
#[test]
fn only_unique_connection_names_are_identity() {
    for name in [":1.2", ":bus.name_3", ":a-b.c"] {
        assert!(unique(name));
    }
    for name in [
        "org.aios.Session1",
        "org.freedesktop.DBus",
        "",
        ":",
        ":1",
        ":1.",
        ":.1",
        ":1.2/forged",
        ":1.2\n",
    ] {
        assert!(!unique(name));
    }
    assert!(!unique(&format!(":1.{}", "a".repeat(128))));
}
#[test]
fn proc_stat_uses_kernel_pid_and_last_parenthesis() {
    for name in ["worker", "worker (with) spaces)", "worker\nname"] {
        assert_eq!(start_ticks(&stat(123, name, "42"), 123), Ok(42));
    }
}
#[test]
fn malformed_reused_or_zero_process_identity_is_denied() {
    for value in [
        stat(124, "worker", "42"),
        stat(123, "worker", "0"),
        stat(123, "worker", "-1"),
        "123 (broken) S 0".into(),
        "123 worker S 0".into(),
    ] {
        assert!(start_ticks(&value, 123).is_err());
    }
}
#[test]
fn all_process_uids_must_match_bus_credentials() {
    assert_eq!(
        check_uids("Name:\tworker\nUid:\t1000 1000 1000 1000\n", 1000),
        Ok(())
    );
    for line in [
        "Uid: 1000 0 1000 1000",
        "Uid: 1000 1000 1000",
        "Uid: 1000 1000 1000 1000 0",
        "Uid: 1000 1000 1000 1000\nUid: 1000 1000 1000 1000",
        "Name: worker",
    ] {
        assert!(check_uids(line, 1000).is_err());
    }
}
#[test]
fn every_connection_process_boot_session_and_logind_change_invalidates_identity() {
    let original = fixture();
    assert_eq!(compare(&original, &original), Ok(()));
    for field in 0..18 {
        let mut c = original.clone();
        match field {
            0 => c.caller.uid += 1,
            1 => c.caller.pid += 1,
            2 => c.caller.start_ticks += 1,
            3 => c.caller.boot_id = uuid::Uuid::new_v4().to_string(),
            4 => c.caller.bus_id = "b".repeat(32),
            5 => c.caller.sender = ":1.8".into(),
            6 => c.caller.session = None,
            7 => c.caller.session.as_mut().unwrap().id = "c2".into(),
            8 => c.caller.session.as_mut().unwrap().remote = true,
            9 => c.caller.session.as_mut().unwrap().kind = "tty".into(),
            10 => c.caller.session.as_mut().unwrap().class = "manager".into(),
            11 => c.caller.session.as_mut().unwrap().state = "online".into(),
            12 => c.caller.session.as_mut().unwrap().active = false,
            13 => c.logind.sender = ":1.9".into(),
            14 => c.logind.uid = 1000,
            15 => c.logind.pid += 1,
            16 => c.logind.start_ticks += 1,
            17 => c.logind.boot_id = uuid::Uuid::new_v4().to_string(),
            _ => unreachable!(),
        }
        assert_eq!(
            compare(&original, &c),
            Err(Error::TargetChanged),
            "field {field}"
        );
    }
}
#[test]
fn desktop_availability_never_guesses_a_session() {
    let original = fixture();
    assert!(original.caller.local_desktop_available());
    for field in 0..6 {
        let mut caller = original.caller.clone();
        match field {
            0 => caller.session = None,
            1 => caller.session.as_mut().unwrap().remote = true,
            2 => caller.session.as_mut().unwrap().active = false,
            3 => caller.session.as_mut().unwrap().class = "manager".into(),
            4 => caller.session.as_mut().unwrap().state = "online".into(),
            5 => caller.session.as_mut().unwrap().kind = "tty".into(),
            _ => unreachable!(),
        }
        assert!(!caller.local_desktop_available());
    }
}
#[test]
fn root_or_closing_observations_do_not_confer_request_authority() {
    let mut caller = fixture().caller;
    assert_eq!(requesting_user(&caller), Ok(()));
    caller.uid = 0;
    assert_eq!(requesting_user(&caller), Err(Error::Authority));
    caller.uid = 1000;
    caller.session.as_mut().unwrap().state = "closing".into();
    assert_eq!(requesting_user(&caller), Err(Error::TargetChanged));
}
#[test]
fn unprivileged_process_cannot_mint_system_broker_authority() {
    assert_ne!(
        unsafe { libc::getuid() },
        0,
        "guest dev verification must run unprivileged"
    );
    assert_eq!(SystemBus::connect().err(), Some(Error::Authority));
    let pid = std::process::id();
    assert_eq!(process(pid, 0).err(), Some(Error::Ownership));
}
#[test]
fn actual_system_bus_process_and_logind_owner_are_observed_read_only() {
    // No VerifiedCaller is minted: dev UID cannot enroll a production broker.
    let connection = connect_native().expect("actual NixOS system bus required");
    let sender = connection.unique_name().unwrap().as_str();
    let bus = bus(&connection).unwrap();
    let (uid, pid) = credentials(&bus, sender).expect("actual caller bus credentials");
    process(pid, uid).expect("actual caller kernel process");
    service(&bus).expect("actual root logind credentials/process");
    let snapshot =
        capture(&connection, sender).expect("live native logind/system-bus observations required");
    assert_eq!(snapshot.caller.uid, unsafe { libc::getuid() });
    assert_eq!(snapshot.caller.pid, std::process::id());
    assert_eq!(snapshot.logind.uid, 0);
    assert_eq!(
        compare(&snapshot, &capture(&connection, sender).unwrap()),
        Ok(())
    );
    println!(
        "AIOS_BROKER_CALLER_OBSERVATIONS {}",
        serde_json::json!({"evidence_kind":"real-guest-read-only-system-bus-process-and-logind",
        "uid":snapshot.caller.uid,"pid":snapshot.caller.pid,"start_ticks":snapshot.caller.start_ticks,"boot_id":snapshot.caller.boot_id,
        "bus_id":snapshot.caller.bus_id,"sender":snapshot.caller.sender,"logind_uid":snapshot.logind.uid,"logind_pid":snapshot.logind.pid,
        "logind_owner":snapshot.logind.sender,"logind_session":snapshot.caller.session.as_ref().map(|s|s.id.as_str()),
        "session_state":snapshot.caller.session.as_ref().map(|s|s.state.as_str()),"session_type":snapshot.caller.session.as_ref().map(|s|s.kind.as_str()),
        "session_class":snapshot.caller.session.as_ref().map(|s|s.class.as_str()),"session_remote":snapshot.caller.session.as_ref().map(|s|s.remote),
        "local_desktop_available":snapshot.caller.local_desktop_available(),"production_verified_caller_minted":false,"polkit_authorization_verified":false})
    );
}
#[test]
fn actual_bus_reconnect_disconnect_and_forged_owner_are_denied() {
    let observer = connect_native().unwrap();
    let first = connect_native().unwrap();
    let original = capture(&observer, first.unique_name().unwrap().as_str()).unwrap();
    let second = connect_native().unwrap();
    let replacement = capture(&observer, second.unique_name().unwrap().as_str()).unwrap();
    assert_eq!(original.caller.uid, replacement.caller.uid);
    assert_eq!(original.caller.pid, replacement.caller.pid);
    assert_eq!(compare(&original, &replacement), Err(Error::TargetChanged));
    first.close().unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match capture(&observer, &original.caller.sender) {
            Err(Error::TargetChanged) => break,
            other => {
                assert!(
                    deadline > std::time::Instant::now(),
                    "disconnect not observed: {other:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    for name in [LOGIN, "org.aios.Session1", ":999999.999999", ":1.2/forged"] {
        assert!(capture(&observer, name).is_err());
    }
}
