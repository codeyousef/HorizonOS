//! Real standard user-bus introspection and unique-sender authorization in guest.
use aios_session::{State, bus::{self, NAME, PATH, INTERFACE}};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt}, sync::{Arc, Mutex, mpsc}, time::Duration};
use zbus::blocking::{Connection, MessageIterator, Proxy};

fn connect() -> Connection {
    let address = format!("unix:path=/run/user/{}/bus", nix::unistd::geteuid());
    zbus::blocking::connection::Builder::address(address.as_str()).unwrap()
        .method_timeout(Duration::from_secs(5)).build().expect("real standard user bus required")
}
fn proxy(conn: &Connection) -> Proxy<'_> { Proxy::new(conn, NAME, PATH, INTERFACE).unwrap() }
fn request(text: &str, nonce: &str) -> String {
    json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
        "operation":{"kind":"submit","request":{"mode":"ask","text":text,"client_nonce":nonce}}}).to_string()
}
fn code(error: zbus::Error, expected: &str) {
    let zbus::Error::MethodError(name, _, _) = error else { panic!("unexpected error: {error}") };
    assert_eq!(name.as_str(),format!("org.aios.Error.{expected}"));
}

#[test]
fn public_methods_authenticate_real_bus_senders_and_keep_tasks_private() {
    // Different registered jobs share the real user's well-known bus name.
    let directory = std::path::PathBuf::from(format!("/run/user/{}/aios-qualification",nix::unistd::geteuid()));
    if !directory.exists() { fs::DirBuilder::new().mode(0o700).create(&directory).unwrap(); }
    let info = directory.symlink_metadata().unwrap();
    assert!(info.is_dir() && info.uid() == nix::unistd::geteuid().as_raw() && info.mode() & 0o077 == 0);
    let lock = fs::OpenOptions::new().read(true).write(true).create(true).mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW).open(directory.join("public-session.lock")).unwrap();
    lock.lock().unwrap();
    let server = bus::export_user_bus(Arc::new(Mutex::new(State::default()))).unwrap();
    // A normal subscriber (not an eavesdropping monitor) listens to ALL signals
    // from this unique server owner. Test-only public fences delimit the real
    // lifecycle calls; any intervening broadcast fails the privacy check.
    let listener = connect();
    let rule = zbus::MatchRule::builder().msg_type(zbus::message::Type::Signal)
        .sender(server.unique_name().unwrap().clone()).unwrap().build();
    let mut signals = MessageIterator::for_match_rule(rule, &listener, Some(64)).unwrap();
    let fence = uuid::Uuid::new_v4().to_string(); let expected_fence = fence.clone();
    let (completed, observed) = mpsc::sync_channel(1);
    let listener_thread = std::thread::spawn(move || {
        let mut started = false; let mut broadcasts = 0;
        for _ in 0..64 {
            let message = signals.next().expect("live signal listener").unwrap();
            let header = message.header();
            if header.interface().is_some_and(|name| name.as_str() == "org.aios.TestQualification1")
                && header.member().is_some_and(|name| name.as_str() == "Fence") {
                let (phase, token): (String, String) = message.body().deserialize().unwrap();
                assert_eq!(token, expected_fence);
                if phase == "begin" { assert!(!started); started = true; }
                else { assert_eq!(phase, "end"); assert!(started); completed.send(broadcasts).unwrap(); return; }
            } else if started { broadcasts += 1; }
        }
        panic!("bounded signal listener exceeded its message limit");
    });
    server.emit_signal(None::<&str>, PATH, "org.aios.TestQualification1", "Fence", &("begin", fence.as_str())).unwrap();
    let conn = connect(); let api = proxy(&conn);
    let introspection = Proxy::new(&conn, NAME, PATH, "org.freedesktop.DBus.Introspectable").unwrap();
    let xml: String = introspection.call("Introspect", &()).unwrap();
    for method in ["GetCapabilities", "Submit", "GetStatus", "GetEvents", "Cancel", "Forget"] {
        assert!(xml.contains(&format!("name=\"{method}\"")));
    }
    let agent_xml = xml.split("<interface name=\"org.aios.Agent1\">").nth(1).unwrap().split("</interface>").next().unwrap();
    // zbus also advertises standard PropertiesChanged; only the AIOS interface
    // is prohibited from exposing task/evidence signals.
    assert!(!agent_xml.contains("<signal"));
    let ui=Proxy::new(&conn,NAME,"/org/aios/UI1","org.aios.UI1").unwrap();
    let ui_introspection=Proxy::new(&conn,NAME,"/org/aios/UI1","org.freedesktop.DBus.Introspectable").unwrap();
    let ui_xml:String=ui_introspection.call("Introspect",&()).unwrap();
    let own_ui=ui_xml.split("<interface name=\"org.aios.UI1\">").nth(1).unwrap().split("</interface>").next().unwrap();
    assert!(own_ui.contains("name=\"SelectSession\""));assert!(!own_ui.contains("<signal"));
    code(ui.call::<_,_,String>("SelectSession",&("aios-no-such-session",)).unwrap_err(),"TARGET_NOT_FOUND");
    let caps: String = api.call("GetCapabilities", &()).unwrap();
    let caps: Value = serde_json::from_str(&caps).unwrap();
    assert_eq!(caps["transport"], "session-dbus"); assert_eq!(caps["ui_enabled"],false);
    println!("AIOS_DBUS_CAPABILITIES={caps}");
    let submitted = request("Private prompt must not be broadcast", "dbus-real-nonce");
    let task: String = api.call("Submit", &(submitted.as_str(),)).unwrap();
    let same: String = api.call("Submit", &(submitted.as_str(),)).unwrap(); assert_eq!(task,same);
    let changed = request("changed", "dbus-real-nonce");
    code(api.call::<_,_,String>("Submit", &(changed.as_str(),)).unwrap_err(),"CONFLICT");
    let status: String = api.call("GetStatus", &(task.as_str(),)).unwrap();
    let status: Value = serde_json::from_str(&status).unwrap();
    assert_eq!(status["error"],"MODEL_UNAVAILABLE"); assert_eq!(status["mutation_performed"], false);
    let other = connect(); let other_api = proxy(&other);
    assert_ne!(conn.unique_name(), other.unique_name());
    for method in ["GetStatus", "Cancel", "Forget"] {
        code(other_api.call::<_,_,String>(method, &(task.as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    }
    code(other_api.call::<_,_,String>("GetEvents", &(task.as_str(),0_u64,10_u32)).unwrap_err(),"PERMISSION_DENIED");
    let events: String = api.call("GetEvents", &(task.as_str(),0_u64,1_u32)).unwrap();
    let events: Value = serde_json::from_str(&events).unwrap();
    assert_eq!(events["events"].as_array().unwrap().len(),1); assert_eq!(events["complete"], false);
    let cancelled: String = api.call("Cancel", &(task.as_str(),)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&cancelled).unwrap()["already_terminal"], true);
    for forged in [submitted.replace("\"mode\":\"ask\"", "\"mode\":\"ask\",\"approved\":true"),
                   submitted.replace("\"schema_version\":1", "\"schema_version\":1,\"uid\":0")] {
        code(api.call::<_,_,String>("Submit", &(forged.as_str(),)).unwrap_err(), "INVALID_ARGUMENT");
    }
    let oversized = " ".repeat(aios_protocol::MAX_TASK_BYTES + 1);
    code(api.call::<_,_,String>("Submit", &(oversized.as_str(),)).unwrap_err(), "RESOURCE_EXHAUSTED");
    let deleted: String = api.call("Forget", &(task.as_str(),)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&deleted).unwrap()["deleted"],true);
    code(api.call::<_,_,String>("GetStatus", &(task.as_str(),)).unwrap_err(), "TARGET_NOT_FOUND");
    server.emit_signal(None::<&str>, PATH, "org.aios.TestQualification1", "Fence", &("end", fence.as_str())).unwrap();
    let private_broadcasts = observed.recv_timeout(Duration::from_secs(5)).expect("signal fences must be delivered");
    listener_thread.join().unwrap(); assert_eq!(private_broadcasts, 0, "task lifecycle broadcast a signal");
    println!("AIOS_PRIVATE_SIGNAL_LISTENER=passed; actual unique-owner subscription; no lifecycle broadcasts between public test fences");
    println!("AIOS_DBUS_VERIFIED={}",json!({"server_unique_name":server.unique_name().unwrap().as_str(),
        "client_unique_name":conn.unique_name().unwrap().as_str(),"other_client_unique_name":other.unique_name().unwrap().as_str(),
        "uid":nix::unistd::geteuid().as_raw(),"private_events":true,"ui_enabled":false,"actual_user_bus":true,
        "actual_signal_listener":true,"lifecycle_broadcast_signal_count":private_broadcasts}));
}
