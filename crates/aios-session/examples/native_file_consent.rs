//! Fixed synthetic desktop qualification through the installed public service.
//! No internal enrollment or approval transport is available to this client.
use serde_json::{Value, json};
use std::{fs, path::Path, thread, time::{Duration, Instant}};
use zbus::blocking::{Connection, Proxy};

const NAME: &str = "org.aios.Session1";
const PATH: &str = "/org/aios/Files1";
const INTERFACE: &str = "org.aios.Files1";
const WORK: &str = "/home/alice/scope-consent";

fn phase(name: &str) { fs::write(Path::new(WORK).join(name), b"ready\n").unwrap(); }
fn wait(name: &str) {
    let deadline = Instant::now() + Duration::from_secs(150);
    while !Path::new(WORK).join(name).exists() {
        assert!(Instant::now() < deadline, "fixture handshake timed out: {name}");
        thread::sleep(Duration::from_millis(100));
    }
}
fn connect() -> Connection {
    zbus::blocking::connection::Builder::address("unix:path=/run/user/1000/bus")
        .unwrap().method_timeout(Duration::from_secs(150)).build().unwrap()
}
fn files(connection: &Connection) -> Proxy<'_> {
    Proxy::new(connection, NAME, PATH, INTERFACE).unwrap()
}
fn value(proxy: &Proxy<'_>, method: &str, request: Option<&str>) -> Value {
    let result: Result<String, zbus::Error> = match request {
        Some(request) => proxy.call(method, &(request,)), None => proxy.call(method, &()),
    };
    serde_json::from_str(&result.unwrap_or_else(|e| panic!("{method}: {e}"))).unwrap()
}
fn denied(error: zbus::Error, expected: &str) {
    let zbus::Error::MethodError(name, _, _) = error else { panic!("unexpected error {error}") };
    assert_eq!(name.as_str(), format!("org.aios.Error.{expected}"));
}
fn open(root: &str, path: &str, access: &str) -> String {
    json!({"schema_version":1,"root_id":root,"relative_path":path,"access":access}).to_string()
}
fn proposal(connection: &Connection, session: &str) -> (String, String, String) {
    let f = files(connection);
    let proposal = value(&f, "ProposeRoots", None);
    let roots = proposal["data"]["roots"].as_array().unwrap();
    let mut paths = roots.iter().map(|r|r["display_path"].as_str().unwrap()).collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, vec!["/home/alice/Desktop", "/home/alice/Documents", "/home/alice/Downloads"]);
    let root = roots.iter().find(|r| r["display_path"] == "/home/alice/Documents").unwrap()["root_id"].as_str().unwrap().to_owned();
    denied(f.call::<_,_,String>("OpenScoped", &(open(&root,"note.txt","content").as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    let id = proposal["data"]["proposal_id"].as_str().unwrap().to_owned();
    let ui = Proxy::new(connection, NAME, "/org/aios/UI1", "org.aios.UI1").unwrap();
    let selected = value(&ui, "SelectSession", Some(session));
    let request = json!({"schema_version":1,"proposal_id":id,"approved_root_ids":[root],
        "allowed_access":["content"],"session_handle":selected["candidate_handle"]}).to_string();
    (id, root, request)
}
fn pending(connection: &Connection, request: String) -> thread::JoinHandle<Result<String,zbus::Error>> {
    let connection = connection.clone();
    thread::spawn(move ||files(&connection).call("EnrollRoots", &(request.as_str(),)))
}
fn main() {
    assert_eq!(nix::unistd::geteuid().as_raw(), 1000);
    assert_eq!(std::env::args().len(), 1);
    let expected = option_env!("AIOS_SCOPE_QUALIFICATION_UUID").expect("qualification-only package required");
    let identity: Value = serde_json::from_slice(&fs::read("/run/aios-scope-qualification/identity.json").unwrap()).unwrap();
    assert_eq!(identity["dmi_uuid"], expected);
    assert_eq!(identity["boot_id"].as_str().unwrap(), fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim());
    assert_eq!(identity["current_system"].as_str().unwrap(), fs::canonicalize("/run/current-system").unwrap().to_str().unwrap());
    let session: Value = serde_json::from_slice(&fs::read("/run/aios-scope-consent/session.json").unwrap()).unwrap();
    let expected_broker_pid = u32::try_from(session["broker_pid"].as_u64().unwrap()).unwrap();
    let session = session["session_id"].as_str().unwrap();
    let connection = connect();
    let bus = Proxy::new(&connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").unwrap();
    let pid: u32 = bus.call("GetConnectionUnixProcessID", &(NAME,)).unwrap();
    assert_eq!(pid, expected_broker_pid);
    let executable = fs::canonicalize("/run/current-system/sw/bin/aios-sessiond").unwrap();
    assert!(executable.starts_with("/nix/store"));
    assert_eq!(fs::read_link(format!("/proc/{pid}/exe")).unwrap(), executable);
    let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    let security = status.lines().filter_map(|line|line.split_once(':'))
        .filter(|(key,_)|matches!(*key,"NoNewPrivs"|"Seccomp"|"CapEff"|"CapPrm"|"CapInh"|"CapAmb"|"CapBnd"|"Uid"))
        .map(|(key,value)|(key.to_owned(),value.trim().to_owned()))
        .collect::<std::collections::BTreeMap<_,_>>();
    println!("BROKER_SECURITY {}", serde_json::to_string(&security).unwrap());
    assert_eq!(security["NoNewPrivs"], "1");
    assert_eq!(security["Seccomp"], "2");
    for capability in ["CapEff","CapPrm","CapInh","CapAmb"] {
        assert_eq!(u64::from_str_radix(&security[capability],16).unwrap(), 0);
    }
    assert_eq!(security["Uid"].split_whitespace().collect::<Vec<_>>(), vec!["1000";4]);
    for namespace in ["mnt","user"] {
        assert_eq!(fs::read_link(format!("/proc/{pid}/ns/{namespace}")).unwrap(),
                   fs::read_link(format!("/proc/self/ns/{namespace}")).unwrap());
    }

    let (id, root, request) = proposal(&connection, session);
    let mut forged: Value = serde_json::from_str(&request).unwrap();
    forged["confirmed"] = json!(true);
    denied(files(&connection).call::<_,_,String>("EnrollRoots", &(forged.to_string().as_str(),)).unwrap_err(), "INVALID_ARGUMENT");
    let cancelled = pending(&connection, request);
    phase("cancel-pending"); wait("cancel-visible");
    assert_eq!(value(&files(&connection), "CancelRootEnrollment", Some(&id))["cancelled"], true);
    denied(cancelled.join().unwrap().unwrap_err(), "CANCELLED");
    denied(files(&connection).call::<_,_,String>("OpenScoped", &(open(&root,"note.txt","content").as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    phase("cancel-refused");

    let (_, _, request) = proposal(&connection, session);
    let changed = pending(&connection, request);
    phase("changed-pending");
    denied(changed.join().unwrap().unwrap_err(), "TARGET_CHANGED");
    phase("changed-refused"); wait("root-restored");

    let (_, root, request) = proposal(&connection, session);
    let enrolled = pending(&connection, request);
    phase("allow-pending");
    let grant: Value = serde_json::from_str(&enrolled.join().unwrap().unwrap()).unwrap();
    assert_eq!(grant["data"]["roots"].as_array().unwrap().len(), 1);
    assert_eq!(grant["data"]["roots"][0]["root_id"], root);
    assert_eq!(grant["data"]["roots"][0]["allowed_access"], json!(["content"]));
    let f = files(&connection);
    for path in ["escape.txt", ".env", "id_ed25519", "wallet.dat", ".local/state/aios/private.txt"] {
        denied(f.call::<_,_,String>("OpenScoped", &(open(&root,path,"content").as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    }
    denied(f.call::<_,_,String>("OpenScoped", &(open(&root,"note.txt","mutation").as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    let secret = value(&f, "OpenScoped", Some(&open(&root,"renamed.txt","content")));
    let secret_read = json!({"schema_version":1,"file_handle":secret["data"]["file_handle"],"max_bytes":128}).to_string();
    denied(f.call::<_,_,String>("ReadScoped", &(secret_read.as_str(),)).unwrap_err(), "SECRET_SCOPE_DENIED");
    let opened = value(&f, "OpenScoped", Some(&open(&root,"note.txt","content")));
    let read = json!({"schema_version":1,"file_handle":opened["data"]["file_handle"],"max_bytes":128}).to_string();
    let content = value(&f, "ReadScoped", Some(&read));
    assert_eq!(content["data"]["content"], "consented installed content");
    assert_eq!(content["data"]["bytes_read"], 27);
    let foreign = connect();
    denied(files(&foreign).call::<_,_,String>("OpenScoped", &(open(&root,"note.txt","content").as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    phase("cache-populated"); wait("cache-inspected");
    fs::write("/home/alice/Documents/note.txt", b"changed fixture").unwrap();
    denied(f.call::<_,_,String>("ReadScoped", &(read.as_str(),)).unwrap_err(), "TARGET_CHANGED");
    let receipt = value(&f, "RevokeRoot", Some(&json!({"schema_version":1,"root_id":root}).to_string()));
    assert_eq!(receipt["data"]["access_blocked"], true);
    assert_eq!(receipt["data"]["purge_pending"], false);
    assert_eq!(receipt["data"]["cached_records_purged"], 3);
    denied(f.call::<_,_,String>("ReadScoped", &(read.as_str(),)).unwrap_err(), "PERMISSION_DENIED");
    fs::write(Path::new(WORK).join("result.json"), serde_json::to_vec_pretty(&json!({
        "schema_version":1,"evidence_kind":"installed-public-files-native-consent-synthetic-desktop",
        "identity":identity,"uid":1000,"session_id":session,"installed_executable":executable,
        "broker_security":security,"broker_security_verified":true,
        "existing_xdg_only":true,"preconsent_denied":true,"forged_confirmation_denied":true,
        "native_cancel_blocks_enrollment":true,"pending_root_change_denied":true,
        "native_enrollment_content_verified":true,"secret_paths_and_renamed_key_denied":true,
        "ungranted_mutation_denied":true,"foreign_client_denied":true,"changed_source_denied":true,
        "revocation_blocked":true,"persistent_purge":receipt["data"],
        "production_image_qualified":false,"ranked_index_qualified":false
    })).unwrap()).unwrap();
}
