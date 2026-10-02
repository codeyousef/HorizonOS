//! Real kernel credentials/private socket and systemd tests in a verified guest.
use aios_session::{Client, State};
use serde_json::json;
use std::{fs, os::unix::{fs::PermissionsExt, net::UnixStream}, path::PathBuf,
    process::{Child, Command, Stdio}, thread, time::{Duration, Instant}};

struct Server { child: Child, directory: PathBuf, socket: PathBuf }
impl Server {
    fn start() -> Self {
        // The supervisor TMPDIR includes a UUID job path and can exceed Linux
        // Unix socket limits. This private directory is on the guest's /tmp.
        let directory = PathBuf::from("/tmp").join(format!("aios-ipc-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap(); fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = directory.join("session.sock");
        let child = Command::new(env!("CARGO_BIN_EXE_aios-sessiond")).args(["--socket",socket.to_str().unwrap()])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        let mut server = Self { child, directory, socket };
        let end = Instant::now() + Duration::from_secs(5);
        while !server.socket.exists() {
            assert!(server.child.try_wait().unwrap().is_none(), "daemon exited before binding");
            assert!(Instant::now() < end,"daemon bind timeout"); thread::sleep(Duration::from_millis(10));
        }
        server
    }
    fn client(&self) -> Client { Client::connect(&self.socket).unwrap() }
}
impl Drop for Server { fn drop(&mut self) { let _=self.child.kill(); let _=self.child.wait(); let _=fs::remove_dir_all(&self.directory); } }

#[test]
fn real_service_handle_and_task_lifecycle() {
    let server = Server::start(); let mut client = server.client();
    let cap = client.call(json!({"kind":"get_capabilities"})).unwrap().data.unwrap();
    assert_eq!(cap["ui_enabled"],false); assert_eq!(cap["inference_available"],false);
    let resolved = client.call(json!({"kind":"resolve_service","unit_name":"sshd.service"})).unwrap();
    assert!(resolved.error.is_none(),"service resolve: {:?}",resolved.error);
    let id = resolved.data.unwrap()["service_id"].as_str().unwrap().to_owned();
    let result = client.call(json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":id}}})).unwrap().data.unwrap();
    assert_eq!(result["status"],"ok"); assert_eq!(result["data"]["active_state"],"active");
    assert_eq!(result["data"]["unit_name"],"sshd.service"); assert_eq!(result["data"]["scope"],"system");
    assert_eq!(result["data"]["boot_id"], fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim());
    assert!(result["data"]["main_pid"].as_u64().unwrap()>1);
    println!("AIOS_SERVICE_STATUS={result}");
    let missing = client.call(json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":"invented"}}})).unwrap();
    assert_eq!(missing.error.unwrap().code, aios_protocol::contracts::ErrorCode::TargetNotFound);
    let request = json!({"kind":"submit","request":{"mode":"ask","text":"Why did the boot fail?","client_nonce":"nonce-real-1"}});
    let task = client.call(request.clone()).unwrap().data.unwrap()["request_id"].as_str().unwrap().to_owned();
    assert_eq!(client.call(request).unwrap().data.unwrap()["request_id"],task);
    let status = client.call(json!({"kind":"get_status","task_id":task})).unwrap().data.unwrap();
    assert_eq!(status["error"],"MODEL_UNAVAILABLE"); assert_eq!(status["mutation_performed"],false);
    let events = client.call(json!({"kind":"get_events","task_id":task,"after_sequence":0,"limit":1})).unwrap().data.unwrap();
    assert_eq!(events["events"].as_array().unwrap().len(),1); assert_eq!(events["complete"],false);
    let cancelled = client.call(json!({"kind":"cancel","task_id":task})).unwrap().data.unwrap();
    assert_eq!(cancelled["already_terminal"],true);
    assert_eq!(client.call(json!({"kind":"forget","task_id":task})).unwrap().data.unwrap()["deleted"],true);
    assert_eq!(client.call(json!({"kind":"get_status","task_id":task})).unwrap().error.unwrap().code,aios_protocol::contracts::ErrorCode::TargetNotFound);
    let unknown = client.call(json!({"kind":"resolve_service","unit_name":"aios-does-not-exist.service"})).unwrap();
    assert!(unknown.error.is_some());
}

#[test]
fn malformed_authority_and_oversized_frames_close_connection() {
    use std::io::{Read, Write};
    let server = Server::start();
    for payload in [vec![0,1,0,1], vec![0,16,0,1]] {
        let mut stream = UnixStream::connect(&server.socket).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap(); stream.write_all(&payload).unwrap();
        let mut one=[0]; assert!(stream.read(&mut one).is_ok_and(|n| n==0));
    }
    let mut client=server.client();
    assert!(client.call(json!({"kind":"submit","request":{"mode":"ask","text":"test","client_nonce":"x","uid":0,"approved":true}})).is_err());
}

#[test]
fn handle_and_task_ownership_do_not_follow_a_supplied_identity() {
    // This state fixture is separate from real peer-credential acceptance above.
    let mut state=State::default();
    let peer=aios_session::identity::Peer{uid:1000,pid:200,start_ticks:3,boot_id:"boot-fixture".into(),logind_session:None,remote:true,session_type:None,ui_enabled:false};
    let task=state.dispatch(&peer,aios_session::Operation::Submit{request:aios_session::Submit{mode:aios_session::Mode::Ask,text:"private".into(),client_nonce:"fixture".into(),context_handles:vec![],selected_app_handle:None,selected_session_handle:None}}).unwrap()["request_id"].as_str().unwrap().to_owned();
    for other in [aios_session::identity::Peer{uid:1001,..peer.clone()},aios_session::identity::Peer{start_ticks:4,..peer.clone()},aios_session::identity::Peer{pid:201,..peer.clone()}] {
        assert_eq!(state.dispatch(&other,aios_session::Operation::GetStatus{task_id:task.clone()}).unwrap_err(),aios_protocol::contracts::ErrorCode::PermissionDenied);
    }
}
