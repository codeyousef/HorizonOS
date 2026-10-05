//! Installed broker qualification, never a replacement server or claimed UID.
use aios_session::bus::{NAME, PATH, INTERFACE};
use aios_system::processes::OwnProcess;
use serde_json::{json, Value};
use std::{fs, process::Command, thread, time::{Duration, Instant}};
use zbus::blocking::{Connection, Proxy};

fn connect(uid: u32) -> Connection {
    zbus::blocking::connection::Builder::address(format!("unix:path=/run/user/{uid}/bus").as_str()).unwrap()
        .method_timeout(Duration::from_secs(12)).build().unwrap()
}
fn request(action: &str, args: Value) -> String {
    json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":{"kind":"invoke",
        "tool_call":{"kind":"tool_call","action_id":action,"arguments":args}}}).to_string()
}
fn call(api: &Proxy<'_>, method: &str, action: &str, args: Value) -> Value {
    let text: String = api.call(method, &(request(action, args),)).unwrap();
    aios_protocol::validation::validate_result(action, text.as_bytes()).unwrap()
}
fn denied(api: &Proxy<'_>, method: &str, action: &str, args: Value, code: &str) {
    let error = api.call::<_,_,String>(method, &(request(action,args),)).unwrap_err();
    let zbus::Error::MethodError(name, _, _) = error else { panic!("installed process refusal must be explicit") };
    assert_eq!(name.as_str(),format!("org.aios.Error.{code}"));
}
fn bus(connection: &Connection) -> Proxy<'_> {
    Proxy::new(connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus").unwrap()
}
fn owner(connection: &Connection, name: &str, uid: u32) -> (String,u32) {
    let b = bus(connection);
    let sender: String = b.call("GetNameOwner",&(name,)).unwrap();
    let credentials: zbus::fdo::ConnectionCredentials = b.call("GetConnectionCredentials",&(&sender,)).unwrap();
    assert_eq!(credentials.unix_user_id(),Some(uid));
    let pid = credentials.process_id().unwrap(); assert!(pid > 0);
    (sender,pid)
}
fn unit_pid(connection: &Connection, sender: &str, unit: &str) -> u32 {
    let manager = Proxy::new(connection,sender,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager").unwrap();
    let path: zbus::zvariant::OwnedObjectPath = manager.call("GetUnit",&(unit,)).unwrap();
    Proxy::new(connection,sender,path.as_str(),"org.freedesktop.systemd1.Service").unwrap().get_property("MainPID").unwrap()
}

#[test]
#[ignore="requires the installed process broker in a verified enrolled NixOS guest"]
fn installed_process_handles_pages_native_identity_and_refusals() {
    assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|line|line=="ID=nixos"));
    assert_eq!(fs::read_to_string("/etc/aios/guest-role").unwrap().trim(),"development");
    let uid = nix::unistd::geteuid().as_raw(); assert!(uid >= 1000);
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim().to_owned();
    let connection = connect(uid);
    let broker = owner(&connection,NAME,uid);
    let manager = owner(&connection,"org.freedesktop.systemd1",uid);
    let root = zbus::blocking::connection::Builder::address("unix:path=/run/dbus/system_bus_socket").unwrap()
        .method_timeout(Duration::from_secs(2)).build().unwrap();
    let root_manager = owner(&root,"org.freedesktop.systemd1",0); assert_eq!(root_manager.1,1);
    assert_eq!(unit_pid(&root,&root_manager.0,&format!("user@{uid}.service")),manager.1);
    assert_eq!(unit_pid(&connection,&manager.0,"aios-sessiond.service"),broker.1);
    let m = Proxy::new(&connection,manager.0.as_str(),"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager").unwrap();
    let path: zbus::zvariant::OwnedObjectPath = m.call("GetUnit",&("aios-sessiond.service",)).unwrap();
    let service = Proxy::new(&connection,manager.0.as_str(),path.as_str(),"org.freedesktop.systemd1.Service").unwrap();
    type Commands = Vec<(String,Vec<String>,bool,u64,u64,u64,u64,u32,i32,i32)>;
    let commands: Commands = service.get_property("ExecStart").unwrap();
    let installed = fs::canonicalize("/run/current-system/sw/bin/aios-sessiond").unwrap(); assert!(installed.starts_with("/nix/store"));
    let installed = installed.to_str().unwrap();
    assert_eq!(commands.len(),1); assert_eq!(commands[0].0,installed); assert_eq!(commands[0].1,vec![installed]); assert!(!commands[0].2);
    let api = Proxy::new(&connection,NAME,PATH,INTERFACE).unwrap();
    let reconnect = connect(uid); assert_ne!(connection.unique_name(),reconnect.unique_name());
    let other = Proxy::new(&reconnect,NAME,PATH,INTERFACE).unwrap();
    let mut child = Command::new("/run/current-system/sw/bin/sleep").arg("10").spawn().unwrap();
    let native = OwnProcess::open(child.id()).unwrap(); let original = native.inspect().unwrap();
    let all = call(&api,"ListProcesses","process.list",json!({"limit":100}));
    let rows = all["data"]["processes"].as_array().unwrap(); assert!(!rows.is_empty());
    assert!(rows.len() <= 100);
    for row in rows {
        let process = OwnProcess::open(row["pid"].as_u64().unwrap().try_into().unwrap()).unwrap().inspect().unwrap();
        assert_eq!(process.identity.uid,uid); assert_eq!(process.identity.boot_id,boot);
        assert_eq!(row["start_time_ticks"],process.identity.start_time_ticks);
    }
    let selected = rows.iter().find(|row|row["pid"]==child.id()).expect("actual controlled own-user child must be listed");
    let id = selected["process_id"].as_str().unwrap();
    let inspected = call(&api,"InspectProcess","process.inspect",json!({"process_id":id}));
    assert_eq!(inspected["data"]["pid"],child.id());
    assert_eq!(inspected["data"]["start_time_ticks"],original.identity.start_time_ticks);
    assert_eq!(inspected["data"]["executable_identity"],original.identity.executable_identity);
    assert_eq!(inspected["complete"],true);
    denied(&other,"InspectProcess","process.inspect",json!({"process_id":id}),"PERMISSION_DENIED");
    denied(&api,"InspectProcess","process.inspect",json!({"process_id":id,"uid":0}),"INVALID_ARGUMENT");
    denied(&api,"ListProcesses","process.terminate",json!({"process_id":id}),"INVALID_ARGUMENT");
    assert!(!native.exited().unwrap()); // The rejected action did not signal it.
    child.wait().unwrap(); assert!(native.exited().unwrap());
    denied(&api,"InspectProcess","process.inspect",json!({"process_id":id}),"TARGET_NOT_FOUND");
    let first = call(&api,"ListProcesses","process.list",json!({"limit":1}));
    let start = Instant::now();
    assert_eq!(first["complete"],false); assert_eq!(first["status"],"partial");
    let cursor = first["next_cursor"].as_str().expect("required real inventory pagination");
    let handle = first["data"]["processes"][0]["process_id"].as_str().unwrap();
    let second = call(&api,"ListProcesses","process.list",json!({"limit":1,"cursor":cursor}));
    assert_ne!(first["data"]["processes"][0]["process_id"],second["data"]["processes"][0]["process_id"]);
    denied(&other,"ListProcesses","process.list",json!({"limit":1,"cursor":cursor}),"PERMISSION_DENIED");
    denied(&api,"ListProcesses","process.list",json!({"limit":2,"cursor":cursor}),"INVALID_ARGUMENT");
    denied(&api,"ListProcesses","process.list",json!({"app_id":"unqualified"}),"UNSUPPORTED_CAPABILITY");
    denied(&api,"InspectProcess","process.inspect",json!({"process_id":uuid::Uuid::new_v4().to_string()}),"TARGET_NOT_FOUND");
    while start.elapsed() < Duration::from_millis(30_100) { thread::sleep(Duration::from_millis(20)); }
    denied(&api,"ListProcesses","process.list",json!({"limit":1,"cursor":cursor}),"TARGET_NOT_FOUND");
    denied(&api,"InspectProcess","process.inspect",json!({"process_id":handle}),"TARGET_NOT_FOUND");
    assert_eq!(owner(&connection,NAME,uid),broker); assert_eq!(owner(&connection,"org.freedesktop.systemd1",uid),manager);
    assert_eq!(unit_pid(&connection,&manager.0,"aios-sessiond.service"),broker.1);
    assert_eq!(fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim(),boot);
    println!("AIOS_INSTALLED_PROCESS={}",json!({"evidence_kind":"real-installed-native-process-broker","uid":uid,"broker_uid":uid,
        "broker_pid":broker.1,"boot_id":boot,"installed_executable":installed,"managed_service_verified":true,
        "own_uid_filter":true,"native_child_identity_verified":true,"metrics_schema_verified":true,
        "natural_exit_refused":true,"cursor_continuation":true,"cross_connection_refused":true,"query_drift_refused":true,
        "claimed_uid_refused":true,"app_filter_refused":true,"expiry_refused":true,"termination_performed":false}));
}
