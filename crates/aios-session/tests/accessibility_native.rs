//! Explicit disposable-guest qualification. Native app/desktop observations
//! with a synthetic document; no model context or policy grant is issued.
use aios_session::{accessibility::WindowBinding,display::DisplayBinding};
use aios_protocol::contracts::ErrorCode;
use std::{fs,io::Read,os::{fd::{OwnedFd,FromRawFd,AsRawFd},unix::fs::{DirBuilderExt,PermissionsExt,OpenOptionsExt}},path::PathBuf,process::{Child,Command,Stdio},sync::atomic::AtomicU8,time::{Duration,Instant}};
use serde_json::{Value,json};

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
    let window=loop {
        if kate.child.try_wait().unwrap().is_some_and(|s|s.success()){kate.track_native(&document);}
        if let Some(exit)=kate.child.try_wait().unwrap().filter(|s|!s.success()){
            let mut output=String::new();fs::File::open(&diagnostic).unwrap().take(4096).read_to_string(&mut output).unwrap();
            panic!("owned synthetic-document Kate exited before registration: {exit}; {output}");
        }
        let mut windows=WindowBinding::discover(&display,&control).expect("native reviewed-app discovery");
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
    let snapshot=window.snapshot(&control).expect("real selected-window snapshot");
    assert_eq!(snapshot.window_handle,window.handle);assert!(!snapshot.nodes.is_empty());assert!(snapshot.nodes.len()<=300);
    assert!(snapshot.nodes.iter().map(|n|n.name.len()+n.actions.iter().map(String::len).sum::<usize>()).sum::<usize>()<=16384);
    aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").unwrap(),&serde_json::to_value(&snapshot).unwrap()).unwrap();
    let cancelled=AtomicU8::new(1);
    assert!(matches!(window.snapshot(&cancelled),Err(ErrorCode::Cancelled)));
    kate.stop_native();
    if kate.child.try_wait().unwrap().is_none(){kate.child.kill().unwrap();}kate.child.wait().unwrap();
    let stale=window.verify();assert!(stale.is_err(),"closed native app owner was accepted");
    println!("NATIVE_KATE_SNAPSHOT={}",json!({"evidence_kind":"real-native-app-and-synthetic-document-no-policy-grant","display":display,"window":native,
        "window_identity_sha256":window.identity_sha256().unwrap(),"node_count":snapshot.nodes.len(),"truncated":snapshot.truncated,"snapshot_id":snapshot.snapshot_id,
        "cancelled_before_query":true,"closed_owner_denial":format!("{:?}",stale.unwrap_err())}));
}
