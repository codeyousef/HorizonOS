//! Explicit disposable-guest qualification. Native app/desktop observations
//! with a synthetic document. The managed bridge scenario includes explicitly
//! labeled test-only assistive input to the actual production permission UI.
#[path="support/native_confirmation.rs"] mod native_confirmation;
use aios_session::{accessibility::WindowBinding,display::DisplayBinding};
use aios_protocol::contracts::ErrorCode;
use std::{fs,io::Read,os::{fd::{OwnedFd,FromRawFd,AsRawFd},unix::fs::{DirBuilderExt,PermissionsExt,OpenOptionsExt}},path::PathBuf,process::{Child,Command,Stdio},sync::atomic::AtomicU8,time::{Duration,Instant}};
use serde_json::{Value,json};
use aios_policy::{consent::{NativeDesktop,ReadPresentation,SelectedWindow}, CurrentResources,Scope,Policy};

struct NativeResources<'a>{ display:&'a DisplayBinding,window:&'a WindowBinding }
impl CurrentResources for NativeResources<'_>{
    fn resolve(&self,field:&str,kind:&str,handle:&str)->aios_policy::Result<String>{
        if field=="selected_session" && kind=="graphical-session" && handle==self.display.session.id {
            self.display.verify()?;aios_policy::digest(self.display)
        } else if field=="window_handle" && kind=="scope-owner-expiry" && handle==self.window.handle {
            self.window.verify()?;self.window.identity_sha256()
        } else {Err(ErrorCode::PermissionDenied)}
    }
    fn dynamic_arguments(&self,_:&str,_:&Value,_:&Scope)->aios_policy::Result<()>{Err(ErrorCode::UnsupportedCapability)}
}

struct Kate { child:Child,directory:PathBuf, native:Option<(u32,OwnedFd)> }
impl Kate {
    fn track_native(&mut self,document:&std::path::Path){
        if self.native.is_some(){return;}
        let expected=vec![env!("AIOS_KATE_LAUNCH").as_bytes().to_vec(),b"-n".to_vec(),document.as_os_str().as_encoded_bytes().to_vec()];
        for entry in fs::read_dir("/proc").unwrap(){
            let entry=entry.unwrap();let Some(pid)=entry.file_name().to_str().and_then(|p|p.parse::<u32>().ok()) else {continue;};
            let raw=unsafe{nix::libc::syscall(nix::libc::SYS_pidfd_open,pid,0)};
            if raw<0{continue;}let fd=unsafe{OwnedFd::from_raw_fd(raw as i32)};
            let Ok(bytes)=fs::read(entry.path().join("cmdline")) else {continue;};
            let args=bytes.split(|b|*b==0).filter(|v|!v.is_empty()).map(|v|v.to_vec()).collect::<Vec<_>>();
            if args!=expected || fs::read_link(entry.path().join("exe")).ok().as_deref()!=Some(std::path::Path::new(env!("AIOS_KATE_NATIVE"))){continue;}
            use std::os::unix::fs::MetadataExt;
            if fs::metadata(entry.path()).unwrap().uid()!=1001{continue;}
            self.native=Some((pid,fd));break;
        }
    }
    fn stop_native(&mut self){
        if let Some((_,fd))=self.native.take(){unsafe{nix::libc::syscall(nix::libc::SYS_pidfd_send_signal,fd.as_raw_fd(),nix::libc::SIGTERM,std::ptr::null::<nix::libc::siginfo_t>(),0);}
            std::thread::sleep(Duration::from_millis(200));
            unsafe{nix::libc::syscall(nix::libc::SYS_pidfd_send_signal,fd.as_raw_fd(),nix::libc::SIGKILL,std::ptr::null::<nix::libc::siginfo_t>(),0);}
        }
    }
}
impl Drop for Kate {
    fn drop(&mut self){
        self.stop_native();
        if self.child.try_wait().ok().flatten().is_none(){let _=self.child.kill();}
        let _=self.child.wait();let _=fs::remove_dir_all(&self.directory);
    }
}
#[test]
#[ignore = "requires the registered disposable real tester Wayland scenario"]
fn native_selected_kate_snapshot_and_stale_owner(){
    assert_eq!(fs::read_to_string("/etc/aios/desktop-test-profile").unwrap().trim(),"synthetic-disposable-plasma-wayland-v1");
    let uid=nix::unistd::geteuid().as_raw();assert_eq!(uid,1001);
    let probe=Command::new("/run/current-system/sw/bin/aios-desktop-test-probe").output().unwrap();assert!(probe.status.success());
    let observed:Value=serde_json::from_slice(&probe.stdout).unwrap();
    let display=DisplayBinding::observe(observed["session_id"].as_str().unwrap(),uid).expect("native selected display");
    let control=AtomicU8::new(0);
    assert!(WindowBinding::discover(&display,&control).expect("initial accessibility discovery").is_empty(),"must not inspect another existing Kate instance");
    let directory=PathBuf::from(format!("/tmp/aios-native-kate-{}",uuid::Uuid::new_v4()));
    fs::DirBuilder::new().mode(0o700).create(&directory).unwrap();
    let document=directory.join("horizon-native-fixture.txt");
    fs::write(&document,"Horizon OS synthetic selected document. No credentials or external effects.\n").unwrap();
    fs::set_permissions(&document,fs::Permissions::from_mode(0o600)).unwrap();
    let home=directory.join("home");fs::DirBuilder::new().mode(0o700).create(&home).unwrap();
    let diagnostic=directory.join("native-kate.log");
    let log=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&diagnostic).unwrap();
    let child=Command::new(env!("AIOS_KATE_LAUNCH")).args(["-n",document.to_str().unwrap()])
        .env_clear().env("PATH","/run/current-system/sw/bin").env("LANG","C.UTF-8").env("HOME",&home)
        .env("XDG_RUNTIME_DIR",format!("/run/user/{uid}"))
        .env("DBUS_SESSION_BUS_ADDRESS",format!("unix:path=/run/user/{uid}/bus"))
        .env("WAYLAND_DISPLAY",&display.socket_name).env("QT_QPA_PLATFORM","wayland")
        .env("QT_LINUX_ACCESSIBILITY_ALWAYS_ON","1")
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(log).spawn().unwrap();
    let mut kate=Kate{child,directory,native:None};
    let until=Instant::now()+Duration::from_secs(10);
    let mut discovery_timeouts=0;
    let window=loop {
        if kate.child.try_wait().unwrap().is_some_and(|s|s.success()){kate.track_native(&document);}
        if let Some(exit)=kate.child.try_wait().unwrap().filter(|s|!s.success()){
            let mut output=String::new();fs::File::open(&diagnostic).unwrap().take(4096).read_to_string(&mut output).unwrap();
            panic!("owned synthetic-document Kate exited before registration: {exit}; {output}");
        }
        let mut windows=match WindowBinding::discover(&display,&control) {
            Ok(windows)=>windows,
            Err(ErrorCode::DeadlineExceeded) if Instant::now()<until=>{
                // Startup registration can be busy. This is a fresh bounded
                // metadata observation, never a retry of semantic input.
                discovery_timeouts+=1;kate.track_native(&document);
                std::thread::sleep(Duration::from_millis(50));continue;
            },
            Err(error)=>panic!("native reviewed-app discovery: {error:?}"),
        };
        if !windows.is_empty(){
            assert_eq!(windows.len(),1,"ambiguous owned window");
            let w=windows.remove(0);assert!(w.title.contains("horizon-native-fixture.txt"));kate.track_native(&document);break w;
        }
        assert!(Instant::now()<until,"Kate accessibility registration timed out");
        std::thread::sleep(Duration::from_millis(50));
    };
    let native=serde_json::to_value(&window).unwrap();
    assert_eq!(native["app"]["pid"],kate.native.as_ref().expect("owned native Kate process").0);
    assert_eq!(native["display"]["session"]["uid"],uid);
    if std::env::var("AIOS_NATIVE_BRIDGE_SCENARIO").ok().as_deref()==Some("disposable-provider-v1"){
        qualify_broker_bridge(&display,&window);
    }
    // Registration exposes the window before Kate finishes populating its
    // accessibility tree. Let this owned fixture finish startup before the
    // first query; a failed snapshot is never retried and its limits stay fixed.
    std::thread::sleep(Duration::from_millis(500));
    let paging=window.snapshot(&control).expect("native paging source snapshot");
    let containers=paging.container_handles().unwrap();
    // Page the owned document's text container, not transient toolbar/menu trees.
    // Selection happens once from native metadata; a failed page is not retried.
    let editors=paging.nodes.iter().filter(|n|n.role=="atspi:61" && containers.contains(&n.node_handle)).collect::<Vec<_>>();
    assert_eq!(editors.len(),1,"one native selected-document text container required");
    let container=editors[0].node_handle.clone();
    let page=window.snapshot_container(&paging,&container,&control).expect("real selected native container page");
    assert_eq!(page.window_handle,window.handle);assert_ne!(page.snapshot_id,paging.snapshot_id);
    assert!(!page.nodes.is_empty() && page.nodes.len()<=300);
    assert!(page.nodes.iter().map(|n|n.name.len()+n.actions.iter().map(String::len).sum::<usize>()).sum::<usize>()<=16384);
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").unwrap(),&serde_json::to_value(&page).unwrap()).unwrap();
    window.verify_snapshot_node(&page,&page.nodes[0].node_handle,&control).expect("paged root retains selected-window ancestors");
    let selector=json!({"role":page.nodes[0].role,"name":page.nodes[0].name});
    let found=window.find_snapshot_nodes(&page,&selector,&control).expect("actual page-local native selector");
    assert!(found["matches"].as_array().unwrap().iter().any(|h|h==&page.nodes[0].node_handle));
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.find","data").unwrap(),&found).unwrap();
    assert_eq!(window.find_snapshot_nodes(&page,&selector,&AtomicU8::new(1)),Err(ErrorCode::Cancelled));
    assert!(matches!(window.snapshot_container(&page,&uuid::Uuid::new_v4().to_string(),&control),Err(ErrorCode::TargetNotFound)));
    assert!(matches!(window.snapshot_container(&page,&page.nodes[0].node_handle,&AtomicU8::new(1)),Err(ErrorCode::Cancelled)));
    let mut snapshot=window.snapshot(&control).expect("real selected-window snapshot");
    assert_eq!(snapshot.window_handle,window.handle);assert!(!snapshot.nodes.is_empty());assert!(snapshot.nodes.len()<=300);
    assert!(snapshot.nodes.iter().map(|n|n.name.len()+n.actions.iter().map(String::len).sum::<usize>()).sum::<usize>()<=16384);
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").unwrap(),&serde_json::to_value(&snapshot).unwrap()).unwrap();
    let selected_node=snapshot.nodes[0].node_handle.clone();
    window.verify_snapshot_node(&snapshot,&selected_node,&control).expect("actual native owner/object/window/generation re-resolution");
    let original_generation=snapshot.snapshot_id.clone();snapshot.snapshot_id=uuid::Uuid::new_v4().to_string();
    assert_eq!(window.verify_snapshot_node(&snapshot,&selected_node,&control),Err(ErrorCode::TargetChanged));
    assert_eq!(window.find_snapshot_nodes(&snapshot,&selector,&control),Err(ErrorCode::TargetChanged));
    snapshot.snapshot_id=original_generation;
    assert_eq!(window.verify_snapshot_node(&snapshot,&uuid::Uuid::new_v4().to_string(),&control),Err(ErrorCode::TargetNotFound));
    let cancelled_node=AtomicU8::new(1);
    assert_eq!(window.verify_snapshot_node(&snapshot,&selected_node,&cancelled_node),Err(ErrorCode::Cancelled));
    std::thread::sleep(Duration::from_millis(2100));
    assert_eq!(window.verify_snapshot_node(&snapshot,&selected_node,&control),Err(ErrorCode::DeadlineExceeded));
    assert_eq!(window.find_snapshot_nodes(&snapshot,&selector,&control),Err(ErrorCode::DeadlineExceeded));
    assert_eq!(snapshot.container_handles(),Err(ErrorCode::DeadlineExceeded));
    assert!(matches!(window.snapshot_container(&snapshot,&selected_node,&control),Err(ErrorCode::DeadlineExceeded)));
    let cancelled=AtomicU8::new(1);
    assert!(matches!(window.snapshot(&cancelled),Err(ErrorCode::Cancelled)));
    // The real originating kernel peer and actual selected Kate/display scope
    // reach the production native transport. These scenarios never press Allow.
    let (proof,_peer)=std::os::unix::net::UnixStream::pair().unwrap();
    let peer=aios_session::identity::authenticate(&proof).unwrap();
    let subject=aios_policy::Subject{uid:peer.uid,pid:peer.pid,start_ticks:peer.start_ticks,boot_id:peer.boot_id.clone(),
        session:peer.logind_session.as_ref().map(|id|aios_policy::Session{id:id.clone(),remote:peer.remote,kind:peer.session_type.clone().unwrap()}),
        client:aios_policy::Client::Unix{connection_id:uuid::Uuid::new_v4().to_string()}};
    let policy=Policy::new(peer.boot_id.clone(),aios_policy::registry_revision()).unwrap();
    let resources=NativeResources{display:&display,window:&window};
    let propose=|expiry|policy.propose_graphical_read(policy.authenticated_user_intent(subject.clone(),uuid::Uuid::new_v4().to_string(),
        "Explain the selected synthetic document",aios_policy::Mode::Ask).unwrap(),
        NativeDesktop{uid,boot_id:display.boot_id.clone(),session_id:display.session.id.clone(),identity_sha256:aios_policy::digest(&display).unwrap(),socket_name:display.socket_name.clone()},
        ReadPresentation{target:"This disposable NixOS VM".into(),profile:"Local CPU, no model invoked in this scenario".into(),goal:"Explain the selected synthetic document".into(),
            windows:vec![SelectedWindow{handle:window.handle.clone(),identity_sha256:window.identity_sha256().unwrap(),name:window.name.clone(),window:window.title.clone()}],evidence:vec![]},expiry).unwrap();
    let mut expiring=propose(1200).launch(&policy,&subject,&resources).expect("pinned native consent launch");
    let native_expiry=loop {
        aios_session::identity::verify(&proof,&peer).unwrap();
        match expiring.poll(&policy,&subject,&resources){
            Ok(None)=>std::thread::sleep(Duration::from_millis(10)),
            Ok(Some(_))=>panic!("expiry scenario created authority without native human Allow"),
            Err(error)=>break error,
        }
    };
    assert!(matches!(native_expiry,ErrorCode::ApprovalExpired|ErrorCode::PermissionDenied));
    let mut withdrawn=propose(90_000).launch(&policy,&subject,&resources).unwrap();
    std::thread::sleep(Duration::from_millis(350));
    assert!(matches!(withdrawn.poll(&policy,&subject,&resources),Ok(None)));
    let start=Instant::now();withdrawn.withdraw();let withdrawal_ms=start.elapsed().as_millis();
    assert!(withdrawal_ms<1000);
    assert!(matches!(withdrawn.poll(&policy,&subject,&resources),Err(ErrorCode::Cancelled)));
    let (proof,peer_socket)=std::os::unix::net::UnixStream::pair().unwrap();
    let origin=aios_session::ui_read::OriginatingClient::authenticate(proof).unwrap();
    let mut task=aios_session::ui_read::NativeReadTask::begin(origin,window.clone(),"Explain the selected synthetic document",aios_policy::Mode::Ask,
        "This disposable NixOS VM".into(),"Local CPU, no model invoked in this scenario".into(),std::sync::Arc::new(AtomicU8::new(0))).unwrap();
    assert!(matches!(task.snapshot(),Err(ErrorCode::AuthRequired)),"unconfirmed task exposed accessibility content");
    drop(task);drop(peer_socket);
    let (proof,peer_socket)=std::os::unix::net::UnixStream::pair().unwrap();
    let origin=aios_session::ui_read::OriginatingClient::authenticate(proof).unwrap();
    let mut task=aios_session::ui_read::NativeReadTask::begin(origin,window.clone(),"Explain the selected synthetic document",aios_policy::Mode::Ask,
        "This disposable NixOS VM".into(),"Local CPU, no model invoked in this scenario".into(),std::sync::Arc::new(AtomicU8::new(0))).unwrap();
    assert_eq!(task.find_snapshot_nodes(&page,&selector),Err(ErrorCode::AuthRequired),"unconfirmed selector reached native page resolution");
    drop(task);drop(peer_socket);
    let (proof,peer_socket)=std::os::unix::net::UnixStream::pair().unwrap();
    let origin=aios_session::ui_read::OriginatingClient::authenticate(proof).unwrap();
    let mut task=aios_session::ui_read::NativeReadTask::begin(origin,window.clone(),"Explain the selected synthetic document",aios_policy::Mode::Ask,
        "This disposable NixOS VM".into(),"Local CPU, no model invoked in this scenario".into(),std::sync::Arc::new(AtomicU8::new(0))).unwrap();
    drop(peer_socket);
    assert!(matches!(task.poll_confirmation(),Err(ErrorCode::Cancelled)),"disconnected original peer retained pending consent");
    assert!(matches!(task.snapshot(),Err(ErrorCode::Cancelled)));
    assert_eq!(task.verify_snapshot_node(&snapshot,&selected_node),Err(ErrorCode::Cancelled));
    assert!(matches!(task.snapshot_container(&snapshot,&selected_node),Err(ErrorCode::Cancelled)));
    assert_eq!(task.find_snapshot_nodes(&page,&selector),Err(ErrorCode::Cancelled));
    let (proof,peer_socket)=std::os::unix::net::UnixStream::pair().unwrap();
    let origin=aios_session::ui_read::OriginatingClient::authenticate(proof).unwrap();
    let mut task=aios_session::ui_read::NativeReadTask::begin(origin,window.clone(),"Explain the selected synthetic document",aios_policy::Mode::Ask,
        "This disposable NixOS VM".into(),"Local CPU, no model invoked in this scenario".into(),std::sync::Arc::new(AtomicU8::new(0))).unwrap();
    let stop=task.stop_handle().unwrap();std::thread::sleep(Duration::from_millis(350));
    let start=Instant::now();stop.stop();let independent_stop_ms=start.elapsed().as_millis();
    assert!(independent_stop_ms<100);
    assert!(matches!(task.poll_confirmation(),Err(ErrorCode::Cancelled)));
    assert_eq!(task.find_snapshot_nodes(&page,&selector),Err(ErrorCode::Cancelled));
    assert!(matches!(task.snapshot(),Err(ErrorCode::Cancelled)));drop(peer_socket);
    kate.stop_native();
    if kate.child.try_wait().unwrap().is_none(){kate.child.kill().unwrap();}kate.child.wait().unwrap();
    let stale=window.verify();assert!(stale.is_err(),"closed native app owner was accepted");
    println!("NATIVE_KATE_SNAPSHOT={}",json!({"evidence_kind":"real-native-app-and-synthetic-document-no-policy-grant","display":display,"window":native,
        "window_identity_sha256":window.identity_sha256().unwrap(),"node_count":snapshot.nodes.len(),"truncated":snapshot.truncated,"snapshot_id":snapshot.snapshot_id,
        "cancelled_before_query":true,"closed_owner_denial":format!("{:?}",stale.unwrap_err()),
        "native_node_lineage_rechecked":true,"unknown_node_denied":true,"node_cancel_denied":true,"actual_node_expiry_wait_ms":2100,
        "changed_snapshot_generation_denied":true,"disconnected_grant_node_denied":true,"startup_metadata_timeouts":discovery_timeouts,
        "native_non_window_container_page_nodes":page.nodes.len(),"container_page_snapshot_id":page.snapshot_id,
        "container_page_ancestors_revalidated":true,"expired_container_denied":true,"container_cancel_denied":true,
        "native_page_selector":found,"selector_cancel_generation_expiry_denied":true,"unconfirmed_selector_denied":true,"disconnected_selector_denied":true,
        "semantic_effect_performed":false}));
    println!("NATIVE_POLICY_CONSENT={}",json!({"evidence_kind":"real-native-client-display-window-and-production-consent-transport-no-allow-or-grant",
        "uid":uid,"origin_pid":subject.pid,"origin_start_ticks":subject.start_ticks,"session_id":display.session.id,
        "native_window_identity_sha256":window.identity_sha256().unwrap(),"expiry_denial":format!("{native_expiry:?}"),"withdrawal_ms":withdrawal_ms,
        "withdrawal_reuse_denied":true,"unconfirmed_snapshot_denied":true,"origin_disconnect_revoked_pending_read":true,
        "independent_stop_ms":independent_stop_ms,"independent_stop_revoked_pending_read":true,"policy_revision":aios_policy::registry_revision()}));
}

fn qualify_broker_bridge(display:&DisplayBinding,window:&WindowBinding){
    let bus=aios_session::bus::Client::connect_user_bus().unwrap();
    let session=bus.select_ui_session(&display.session.id).unwrap();
    let handle=session["candidate_handle"].as_str().unwrap();
    let bus_windows=bus.list_ui_windows(handle).expect("managed native D-Bus origin handoff");
    assert_eq!(bus_windows["session_id"],display.session.id);assert_eq!(bus_windows["ui_authorized"],false);
    let list=bus_windows["windows"].as_array().unwrap();assert_eq!(list.len(),1);assert_eq!(list[0]["title"],window.title);
    let reconnect=aios_session::bus::Client::connect_user_bus().unwrap();
    assert_eq!(reconnect.list_ui_windows(handle),Err(ErrorCode::PermissionDenied));
    let fresh=reconnect.select_ui_session(&display.session.id).unwrap();
    let fresh_windows=reconnect.list_ui_windows(fresh["candidate_handle"].as_str().unwrap()).unwrap();
    assert_ne!(list[0]["window_handle"],fresh_windows["windows"][0]["window_handle"]);
    qualify_public_task(&bus,&reconnect,handle,&list[0],display);
    println!("NATIVE_BUS_WINDOW_DISCOVERY={}",json!({"evidence_kind":"actual-hardened-public-ui1-native-original-sender-managed-provider-metadata",
        "session_id":display.session.id,"uid":display.session.uid,"windows":bus_windows,"reconnect_candidate_denied":true,
        "fresh_selection_distinct_window_handles":true,"ui_authorized":false,"no_content_or_grant_returned":true}));
    let path=PathBuf::from(format!("/run/user/{}/aios/session.sock",display.session.uid));
    let mut client=aios_session::Client::connect(&path).unwrap();
    let selected=client.call(json!({"kind":"select_ui_session","session_id":display.session.id})).unwrap();
    assert!(selected.error.is_none(),"{selected:?}");let handle=selected.data.unwrap()["candidate_handle"].as_str().unwrap().to_owned();
    let discovered=client.call(json!({"kind":"list_ui_windows","session_handle":handle})).unwrap();
    assert!(discovered.error.is_none(),"{discovered:?}");let discovered=discovered.data.unwrap();
    assert_eq!(discovered["ui_authorized"],false);let windows=discovered["windows"].as_array().unwrap();assert_eq!(windows.len(),1);
    assert_eq!(windows[0]["title"],window.title);let selected_window=windows[0]["window_handle"].as_str().unwrap().to_owned();
    let start=client.call(json!({"kind":"start_ui_read","window_handle":selected_window,"goal":"Explain the selected synthetic document","mode":"ask"})).unwrap();
    assert!(start.error.is_none(),"{start:?}");let task=start.data.unwrap()["task_id"].as_str().unwrap().to_owned();
    let until=Instant::now()+Duration::from_secs(5);
    loop{
        let status=client.call(json!({"kind":"get_ui_read_status","task_id":task})).unwrap();assert!(status.error.is_none(),"{status:?}");
        let status=status.data.unwrap();if status["state"]=="needs_permission"{break;}
        assert!(!matches!(status["state"].as_str(),Some("failed"|"cancelled"|"completed")),"unexpected native read state {status}");
        assert!(Instant::now()<until,"native provider permission state timed out");std::thread::sleep(Duration::from_millis(20));
    }
    let denied=client.call(json!({"kind":"take_ui_snapshot","task_id":task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::AuthRequired);
    let forged=client.call(json!({"kind":"cancel_ui_read","task_id":task,"decision":"allow"})).unwrap();assert_eq!(forged.error.unwrap().code,ErrorCode::InvalidArgument);
    let mut reconnect=aios_session::Client::connect(&path).unwrap();
    let denied=reconnect.call(json!({"kind":"get_ui_read_status","task_id":task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::AuthRequired);
    let start=Instant::now();let cancel=client.call(json!({"kind":"cancel_ui_read","task_id":task})).unwrap();
    let cancellation_ms=start.elapsed().as_millis();assert!(cancel.error.is_none(),"{cancel:?}");assert!(cancellation_ms<1000);
    let status=client.call(json!({"kind":"get_ui_read_status","task_id":task})).unwrap().data.unwrap();assert_eq!(status["state"],"cancelled");
    let denied=client.call(json!({"kind":"take_ui_snapshot","task_id":task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::Cancelled);
    assert!(client.call(json!({"kind":"forget_ui_read","task_id":task})).unwrap().error.is_none());
    let denied=client.call(json!({"kind":"get_ui_read_status","task_id":task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::TargetNotFound);
    let goal=format!("Native permission fixture {}",uuid::Uuid::new_v4());
    let start=client.call(json!({"kind":"start_ui_read","window_handle":selected_window,"goal":goal,"mode":"ask"})).unwrap();
    assert!(start.error.is_none(),"{start:?}");let allowed_task=start.data.unwrap()["task_id"].as_str().unwrap().to_owned();
    let until=Instant::now()+Duration::from_secs(5);
    loop{
        let status=client.call(json!({"kind":"get_ui_read_status","task_id":allowed_task})).unwrap();assert!(status.error.is_none(),"{status:?}");
        let status=status.data.unwrap();if status["state"]=="needs_permission"{break;}
        assert_eq!(status["state"],"queued","unexpected positive read state {status}");
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));
    }
    let candidates=WindowBinding::discover(display,&AtomicU8::new(0)).unwrap();
    assert_eq!(candidates.len(),1,"production provider must exclude its permission UI");assert_eq!(candidates[0].title,window.title);
    let input=native_confirmation::allow_owned_read(display,&goal,&selected_window,&window.title,windows[0]["identity_sha256"].as_str().unwrap());
    let until=Instant::now()+Duration::from_secs(8);
    let completed=loop{
        let status=client.call(json!({"kind":"get_ui_read_status","task_id":allowed_task})).unwrap();assert!(status.error.is_none(),"{status:?}");
        let status=status.data.unwrap();if status["state"]=="completed"{break status;}
        assert!(matches!(status["state"].as_str(),Some("needs_permission"|"inspecting")),"native Allow did not complete scoped read {status}");
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(completed["snapshot_ready"],true);
    let denied=reconnect.call(json!({"kind":"take_ui_snapshot","task_id":allowed_task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::AuthRequired);
    let scoped=client.call(json!({"kind":"take_ui_snapshot","task_id":allowed_task})).unwrap();assert!(scoped.error.is_none(),"{scoped:?}");let scoped=scoped.data.unwrap();
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").unwrap(),&scoped).unwrap();
    assert_eq!(scoped["window_handle"],selected_window);let nodes=scoped["nodes"].as_array().unwrap();assert!(!nodes.is_empty() && nodes.len()<=300);
    let denied=client.call(json!({"kind":"take_ui_snapshot","task_id":allowed_task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::TargetNotFound);
    let snapshot_id=scoped["snapshot_id"].as_str().unwrap();
    let denied=reconnect.call(json!({"kind":"get_ui_snapshot_containers","task_id":allowed_task,"snapshot_id":snapshot_id})).unwrap();
    assert_eq!(denied.error.unwrap().code,ErrorCode::AuthRequired);
    let containers=client.call(json!({"kind":"get_ui_snapshot_containers","task_id":allowed_task,"snapshot_id":snapshot_id})).unwrap();
    assert!(containers.error.is_none(),"{containers:?}");let containers=containers.data.unwrap();
    let handles=containers["container_handles"].as_array().unwrap();
    let editors=nodes.iter().filter(|n|n["role"]=="atspi:61" && handles.contains(&n["node_handle"])).collect::<Vec<_>>();
    assert_eq!(editors.len(),1,"one real selected-document text container required");
    let container=&editors[0]["node_handle"];
    let page=client.call(json!({"kind":"page_ui_snapshot","task_id":allowed_task,"snapshot_id":snapshot_id,"container_handle":container})).unwrap();
    assert!(page.error.is_none(),"{page:?}");let page_id=page.data.unwrap()["snapshot_id"].as_str().unwrap().to_owned();
    assert_ne!(page_id,snapshot_id);
    let page=client.call(json!({"kind":"take_ui_snapshot","task_id":allowed_task})).unwrap();assert!(page.error.is_none(),"{page:?}");
    let page=page.data.unwrap();assert_eq!(page["snapshot_id"],page_id);assert_eq!(page["window_handle"],selected_window);
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").unwrap(),&page).unwrap();
    assert!(client.call(json!({"kind":"cancel_ui_read","task_id":allowed_task})).unwrap().error.is_none());
    let denied=client.call(json!({"kind":"take_ui_snapshot","task_id":allowed_task})).unwrap();assert_eq!(denied.error.unwrap().code,ErrorCode::Cancelled);
    assert!(client.call(json!({"kind":"forget_ui_read","task_id":allowed_task})).unwrap().error.is_none());
    println!("NATIVE_PROVIDER_BRIDGE={}",json!({"evidence_kind":"real-exact-managed-broker-provider-original-fd-scoped-snapshot-with-owned-native-input-fixture",
        "session_id":display.session.id,"uid":display.session.uid,"metadata":discovered,"original_fd_bound":true,"unconfirmed_snapshot_denied":true,
        "forged_decision_denied":true,"reconnect_denied":true,"cancellation_ms":cancellation_ms,"cancelled_snapshot_denied":true,"forget_verified":true,
        "permission_ui_excluded_from_production_discovery":true,"native_input_fixture":input,"completed_status":completed,
        "scoped_snapshot_id":scoped["snapshot_id"],"scoped_node_count":nodes.len(),"one_shot_snapshot_verified":true,
        "reconnected_snapshot_denied":true,"post_completion_stop_revokes_snapshot":true}));
}

fn qualify_public_task(bus:&aios_session::bus::Client,reconnect:&aios_session::bus::Client,session_handle:&str,window:&Value,display:&DisplayBinding){
    let mut request=aios_session::Submit{mode:aios_session::Mode::Ask,text:format!("Native permission fixture {}",uuid::Uuid::new_v4()),
        client_nonce:uuid::Uuid::new_v4().to_string(),context_handles:vec![],retain_for_history:false,history_handles:vec![],selected_session_handle:Some(session_handle.into()),
        selected_app_handle:Some(window["window_handle"].as_str().unwrap().into())};
    let task=bus.submit(&request).expect("selected public task admission");assert_eq!(bus.submit(&request).unwrap(),task);
    request.text.push_str(" changed");assert_eq!(bus.submit(&request),Err(ErrorCode::Conflict));request.text.truncate(request.text.len()-8);
    for result in [reconnect.status(&task),reconnect.events(&task,0,100),reconnect.cancel(&task),reconnect.forget(&task)]{
        assert_eq!(result,Err(ErrorCode::PermissionDenied));
    }
    let until=Instant::now()+Duration::from_secs(8);
    loop {let status=bus.status(&task).unwrap();if status["state"]=="needs_permission"{break;}
        assert!(matches!(status["state"].as_str(),Some("queued"|"inspecting")),"public permission failed {status}");
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));}
    let pending=bus.events(&task,0,100).unwrap();assert!(pending["events"].as_array().unwrap().iter().any(|e|e["kind"]=="needs_permission"));
    let start=Instant::now();let cancellation=bus.cancel(&task).unwrap();let cancellation_ms=start.elapsed().as_millis();assert!(cancellation_ms<1000);
    assert_eq!(cancellation["mutation_performed"],false);assert_eq!(cancellation["boundary"],"no_side_effects");
    let until=Instant::now()+Duration::from_secs(5);
    loop {let status=bus.status(&task).unwrap();if status["state"]=="cancelled"{assert_eq!(status["error"],"CANCELLED");break;}
        assert_eq!(status["state"],"cancelling");assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));}
    let stopped=bus.events(&task,0,100).unwrap();assert_eq!(stopped["complete"],true);
    assert_eq!(bus.cancel(&task).unwrap()["already_terminal"],true);assert_eq!(bus.forget(&task).unwrap()["deleted"],true);
    assert_eq!(bus.status(&task),Err(ErrorCode::TargetNotFound));
    // Mode selection never silently expands read authority or enables rules.
    let mut mode_results=Vec::new();
    for mode in [aios_session::Mode::Act,aios_session::Mode::Automate]{
        request.mode=mode;request.client_nonce=uuid::Uuid::new_v4().to_string();
        let denied=bus.submit(&request).unwrap();let status=bus.status(&denied).unwrap();
        assert_eq!(status["state"],"failed");assert_eq!(status["error"],"UNSUPPORTED_CAPABILITY");assert_eq!(status["mutation_performed"],false);
        let events=bus.events(&denied,0,100).unwrap();assert!(!events["events"].as_array().unwrap().iter().any(|e|e["kind"]=="needs_permission"));
        mode_results.push(status);bus.forget(&denied).unwrap();
    }
    request.mode=aios_session::Mode::Diagnose;request.client_nonce=uuid::Uuid::new_v4().to_string();
    let diagnosis=bus.submit(&request).unwrap();wait_public(bus,&diagnosis,"needs_permission");
    request.mode=aios_session::Mode::Ask;request.client_nonce=uuid::Uuid::new_v4().to_string();
    let queued=bus.submit(&request).unwrap();assert_eq!(bus.status(&queued).unwrap()["state"],"queued");
    assert_eq!(bus.cancel(&queued).unwrap()["cancelled"],true);assert_eq!(bus.status(&queued).unwrap()["error"],"CANCELLED");
    bus.forget(&queued).unwrap();assert_eq!(bus.forget(&diagnosis).unwrap()["deleted"],true);
    assert_eq!(bus.status(&diagnosis),Err(ErrorCode::TargetNotFound));

    // One native Allow attempt starts actual AT-SPI traversal. Public Stop
    // must stay responsive while that independent native worker is inspecting.
    request.client_nonce=uuid::Uuid::new_v4().to_string();request.text=format!("Native permission fixture {}",uuid::Uuid::new_v4());
    let inspecting=bus.submit(&request).unwrap();wait_public(bus,&inspecting,"needs_permission");
    let busy_input=native_confirmation::allow_owned_task_read(display,&request.text,window["window_handle"].as_str().unwrap(),
        window["title"].as_str().unwrap(),window["identity_sha256"].as_str().unwrap());
    wait_public(bus,&inspecting,"inspecting");
    let start=Instant::now();let busy_cancel=bus.cancel(&inspecting).unwrap();let busy_stop_ms=start.elapsed().as_millis();
    assert_eq!(busy_cancel["cancelled"],true);assert!(busy_stop_ms<1000);wait_public(bus,&inspecting,"cancelled");
    let busy_events=bus.events(&inspecting,0,100).unwrap();assert_eq!(busy_events["complete"],true);
    assert!(!busy_events["events"].as_array().unwrap().iter().any(|e|e["kind"]=="scoped_observation_ready"),"cancelled capture reached inference");
    assert_eq!(bus.status(&inspecting).unwrap()["mutation_performed"],false);bus.forget(&inspecting).unwrap();
    // This exact installed package has no installed model socket. Native
    // consent and capture still must finish before the honest model error.
    assert!(!std::path::Path::new("/run/aios/model.sock").exists(),"use the installed-model scenario instead");
    request.client_nonce=uuid::Uuid::new_v4().to_string();request.text=format!("Native permission fixture {}",uuid::Uuid::new_v4());
    let positive=bus.submit(&request).unwrap();
    let until=Instant::now()+Duration::from_secs(8);
    loop {let status=bus.status(&positive).unwrap();if status["state"]=="needs_permission"{break;}
        assert!(matches!(status["state"].as_str(),Some("queued"|"inspecting")),"positive public permission failed {status}");
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));}
    let input=native_confirmation::allow_owned_task_read(display,&request.text,window["window_handle"].as_str().unwrap(),
        window["title"].as_str().unwrap(),window["identity_sha256"].as_str().unwrap());
    let until=Instant::now()+Duration::from_secs(15);
    let final_status=loop {let status=bus.status(&positive).unwrap();if status["state"]=="failed"{break status;}
        assert!(matches!(status["state"].as_str(),Some("needs_permission"|"inspecting")),"unexpected public state {status}");
        assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(20));};
    assert_eq!(final_status["error"],"MODEL_UNAVAILABLE");assert!(final_status["output"].is_null());assert_eq!(final_status["mutation_performed"],false);
    let events=bus.events(&positive,0,100).unwrap();assert_eq!(events["complete"],true);
    assert!(events["events"].as_array().unwrap().iter().any(|e|e["kind"]=="scoped_observation_ready"),"no verified observation {events}");
    assert_eq!(bus.forget(&positive).unwrap()["deleted"],true);assert_eq!(bus.events(&positive,0,100),Err(ErrorCode::TargetNotFound));
    println!("NATIVE_PUBLIC_GRAPHICAL_TASK={}",json!({"evidence_kind":"actual-hardened-public-agent1-original-sender-native-consent-read-and-cancellation-no-installed-model",
        "uid":display.session.uid,"session_id":display.session.id,"cancelled_task":task,"cancellation_ms":cancellation_ms,"pending_events":pending,"terminal_events":stopped,
        "native_input_fixture":input,"positive_task":positive,"final_status":final_status,"positive_events":events,
        "reconnect_all_four_operations_denied":true,"nonce_reuse_and_conflict_verified":true,"forget_verified":true,"model_answer_verified":false,
        "mode_results":mode_results,"diagnose_native_permission_and_pending_forget":true,"queued_native_task_cancelled":true,
        "busy_native_input_fixture":busy_input,"busy_native_stop_ms":busy_stop_ms,"busy_native_events":busy_events,"cancelled_capture_excluded_from_inference":true}));
}

fn wait_public(bus:&aios_session::bus::Client,id:&str,expected:&str){
    let until=Instant::now()+Duration::from_secs(8);
    loop {let status=bus.status(id).unwrap();if status["state"]==expected{return;}
        assert!(!matches!(status["state"].as_str(),Some("failed"|"completed"|"cancelled")),"public task did not reach {expected}: {status}");
        assert!(Instant::now()<until,"public task did not reach {expected}: {status}");std::thread::sleep(Duration::from_millis(10));}
}
