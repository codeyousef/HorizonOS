//! Test-only assistive input for the owned production permission dialog.
//! This is never linked into an installed product or exposed as an AIOS tool.
//! It proves transport/grant behavior, not that a human reviewed the request.
use aios_session::display::DisplayBinding;
use std::{collections::{HashSet,VecDeque},fs,os::unix::{net::UnixStream,fs::MetadataExt},path::Path,time::{Duration,Instant}};
use nix::sys::socket::{getsockopt,sockopt::PeerCredentials};
use zbus::{blocking::{Connection,Proxy},zvariant::OwnedObjectPath};
type Object=(String,OwnedObjectPath);
const ROOT:&str="/org/a11y/atspi/accessible/root";
const ACCESSIBLE:&str="org.a11y.atspi.Accessible";
fn proxy<'a>(bus:&Connection,owner:&'a str,path:&'a str,interface:&'a str)->Proxy<'a>{
    // AT-SPI objects need direct property reads, not zbus's GetAll cache.
    // This also ensures every input precondition is a fresh observation.
    zbus::blocking::proxy::Builder::new(bus).destination(owner).unwrap().path(path).unwrap().interface(interface).unwrap()
        .cache_properties(zbus::proxy::CacheProperties::No).build().unwrap()
}
fn owner(bus:&Connection,name:&str)->(String,u32){
    let p=proxy(bus,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus");
    let actual:String=p.call("GetNameOwner",&(name,)).unwrap();assert!(actual.starts_with(':'));
    let uid:u32=p.call("GetConnectionUnixUser",&(actual.as_str(),)).unwrap();assert_eq!(uid,1001);
    let pid:u32=p.call("GetConnectionUnixProcessID",&(actual.as_str(),)).unwrap();
    (actual,pid)
}
fn process_stamp(pid:u32)->(u64,String){
    assert!(pid>1);assert_eq!(fs::metadata(format!("/proc/{pid}")).unwrap().uid(),1001);
    let stat=fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    let fields=stat.rsplit_once(')').unwrap().1.split_whitespace().collect::<Vec<_>>();
    assert_ne!(fields[0],"Z");let ticks=fields[19].parse::<u64>().unwrap();assert!(ticks>0);
    let boot=fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim().to_owned();
    assert_eq!(uuid::Uuid::parse_str(&boot).unwrap().to_string(),boot);(ticks,boot)
}
fn native(pid:u32,parent:u32,stamp:&(u64,String)){
    assert_eq!(process_stamp(pid),*stamp);
    assert_eq!(fs::read_link(format!("/proc/{pid}/exe")).unwrap(),Path::new(env!("AIOS_CONSENT_NATIVE")));
    let stat=fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    assert_eq!(stat.rsplit_once(')').unwrap().1.split_whitespace().nth(1).unwrap().parse::<u32>().unwrap(),parent);
}
fn provider(bus:&Connection)->u32{
    let manager=owner(bus,"org.freedesktop.systemd1");
    let m=proxy(bus,&manager.0,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager");
    let path:OwnedObjectPath=m.call("GetUnit",&("aios-ui-agent.service",)).unwrap();
    let unit=proxy(bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Unit");
    assert_eq!(unit.get_property::<String>("ActiveState").unwrap(),"active");
    let service=proxy(bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Service");
    let pid:u32=service.get_property("MainPID").unwrap();assert!(pid>1);
    type Commands=Vec<(String,Vec<String>,bool,u64,u64,u64,u64,u32,i32,i32)>;
    let commands:Commands=service.get_property("ExecStart").unwrap();assert_eq!(commands.len(),1);
    let exe=fs::read_link(format!("/proc/{pid}/exe")).unwrap();
    assert!(exe.starts_with("/nix/store"));assert_eq!(exe.file_name().unwrap(),"aios-ui-agent");
    assert_eq!(commands[0].0,exe.to_str().unwrap());assert_eq!(commands[0].1,vec![commands[0].0.clone()]);assert!(!commands[0].2);
    pid
}

/// Invoke one native action, only after observing this exact synthetic request
/// in the renderer owned by the canonical managed provider. No retry of input.
pub fn allow_owned_read(display:&DisplayBinding,goal:&str,window:&str,title:&str,window_identity:&str)->serde_json::Value{
    assert_eq!(std::env::var("AIOS_NATIVE_BRIDGE_SCENARIO").unwrap(),"disposable-provider-v1");
    assert_eq!(nix::unistd::geteuid().as_raw(),1001);
    assert_eq!(fs::read_to_string("/etc/aios/desktop-test-profile").unwrap().trim(),"synthetic-disposable-plasma-wayland-v1");
    assert!(goal.starts_with("Native permission fixture "));display.verify().unwrap();
    let session=zbus::blocking::connection::Builder::address("unix:path=/run/user/1001/bus").unwrap().method_timeout(Duration::from_millis(100)).build().unwrap();
    let provider_pid=provider(&session);let provider_stamp=process_stamp(provider_pid);
    let launcher=owner(&session,"org.a11y.Bus");let launcher_stamp=process_stamp(launcher.1);
    assert_eq!(fs::read_link(format!("/proc/{}/exe",launcher.1)).unwrap(),Path::new(env!("AIOS_ATSPI_LAUNCHER")));
    let address:String=proxy(&session,&launcher.0,"/org/a11y/bus","org.a11y.Bus").call("GetAddress",&()).unwrap();
    assert_eq!(address,"unix:path=/run/user/1001/at-spi/bus_0");
    let stream=UnixStream::connect("/run/user/1001/at-spi/bus_0").unwrap();let peer=getsockopt(&stream,PeerCredentials).unwrap();
    assert_eq!(peer.uid(),1001);assert_eq!(peer.pid() as u32,launcher.1);
    let bus=zbus::blocking::connection::Builder::address(address.as_str()).unwrap().method_timeout(Duration::from_millis(100)).build().unwrap();
    let registry=owner(&bus,"org.a11y.atspi.Registry");let root=proxy(&bus,&registry.0,ROOT,ACCESSIBLE);
    let deadline=Instant::now()+Duration::from_secs(8);
    let (app,pid,stamp)=loop{
        assert!(Instant::now()<deadline,"owned production permission renderer did not register");
        let count:i32=root.get_property("ChildCount").unwrap();assert!((0..=64).contains(&count));let mut candidates=Vec::new();
        for i in 0..count{
            let child:Object=root.call("GetChildAtIndex",&(i,)).unwrap();if child.1.as_str()!=ROOT{continue;}
            let (actual,pid)=owner(&bus,&child.0);assert_eq!(actual,child.0);
            if fs::read_link(format!("/proc/{pid}/exe")).ok().as_deref()!=Some(Path::new(env!("AIOS_CONSENT_NATIVE"))){continue;}
            let stamp=process_stamp(pid);native(pid,provider_pid,&stamp);candidates.push((actual,pid,stamp));
        }
        assert!(candidates.len()<=1,"ambiguous native permission renderer");if let Some(candidate)=candidates.pop(){break candidate;}
        std::thread::sleep(Duration::from_millis(20));
    };
    let app_root=proxy(&bus,&app,ROOT,ACCESSIBLE);
    assert_eq!(app_root.get_property::<String>("Name").unwrap(),"Horizon OS confirmation");
    let count:i32=app_root.get_property("ChildCount").unwrap();assert_eq!(count,1);
    let dialog:Object=app_root.call("GetChildAtIndex",&(0i32,)).unwrap();assert_eq!(dialog.0,app);
    let tree_deadline=Instant::now()+Duration::from_secs(2);
    let mut queue=VecDeque::from([(dialog.1,0u8,OwnedObjectPath::try_from(ROOT).unwrap())]);
    let mut visited=HashSet::new();let mut labels=Vec::new();let mut bytes=0;let mut allow=None;
    while let Some((path,depth,parent))=queue.pop_front(){
        assert!(Instant::now()<tree_deadline && visited.len()<300 && depth<=8);assert!(visited.insert(path.clone()));
        let p=proxy(&bus,&app,path.as_str(),ACCESSIBLE);
        assert_eq!(p.get_property::<Object>("Parent").unwrap(),(app.clone(),parent));
        let name:String=p.get_property("Name").unwrap();bytes+=name.len();assert!(bytes<=65536);
        let role:u32=p.call("GetRole",&()).unwrap();
        if depth==0{assert_eq!(name,"Horizon OS application read permission");assert_eq!(role,16);}
        if name=="Allow this read scope"{assert_eq!(role,43);assert!(allow.replace(path.clone()).is_none());}
        labels.push(name);
        let children:i32=p.get_property("ChildCount").unwrap();assert!((0..=300).contains(&children));
        for i in 0..children{let child:Object=p.call("GetChildAtIndex",&(i,)).unwrap();assert_eq!(child.0,app);queue.push_back((child.1,depth+1,path.clone()));}
    }
    assert!(labels.contains(&format!("Request:\n{goal}")),"synthetic request mismatch");
    assert!(labels.iter().any(|v|v.contains(title) && v.contains(&format!("Resource: {window}")) && v.contains(&format!("Identity: {window_identity}"))),"selected native window mismatch");
    let hostname=fs::read_to_string("/proc/sys/kernel/hostname").unwrap();
    assert!(labels.contains(&format!("Target: {}\nDesktop: {} · User: 1001",hostname.trim(),display.session.id)));
    assert!(labels.contains(&"Local / CPU: Local CPU (observation only; no model requested)\nMode: ask".into()));
    assert!(labels.contains(&"Read access: ui.snapshot\nNo input or external effects are authorized by this read scope.".into()));
    let proposal=labels.iter().find_map(|v|v.strip_prefix("Proposal: ")).unwrap();
    assert_eq!(proposal.len(),64);assert!(proposal.bytes().all(|v|v.is_ascii_digit() || (b'a'..=b'f').contains(&v)));
    let path=allow.expect("native Allow button");native(pid,provider_pid,&stamp);display.verify().unwrap();
    assert_eq!(provider(&session),provider_pid);assert_eq!(process_stamp(provider_pid),provider_stamp);
    assert_eq!(owner(&session,"org.a11y.Bus"),launcher);assert_eq!(process_stamp(launcher.1),launcher_stamp);
    assert_eq!(owner(&bus,&app),(app.clone(),pid));
    let p=proxy(&bus,&app,path.as_str(),ACCESSIBLE);assert_eq!(p.get_property::<String>("Name").unwrap(),"Allow this read scope");
    assert_eq!(p.call::<_,_,u32>("GetRole",&()).unwrap(),43);
    let states:Vec<u32>=p.call("GetState",&()).unwrap();assert!(states.len()<=2);
    // AT-SPI StateType: ENABLED=8, SENSITIVE=24, SHOWING=25, VISIBLE=30.
    for bit in [8,24,25,30]{assert!(states.first().is_some_and(|word|word&(1u32<<bit)!=0));}
    let action=proxy(&bus,&app,path.as_str(),"org.a11y.atspi.Action");
    let count:i32=action.get_property("NActions").unwrap();assert!((1..=2).contains(&count));
    // Qt exposes its non-localized Press action, plus optional SetFocus.
    // Select the unique reviewed action by name, never by a guessed index.
    let mut press=None;
    for i in 0..count{let name:String=action.call("GetName",&(i,)).unwrap();
        assert!(matches!(name.as_str(),"Press"|"SetFocus"));if name=="Press"{assert!(press.replace(i).is_none());}}
    let index=press.expect("unique Qt Press action");
    native(pid,provider_pid,&stamp);
    assert_eq!(action.call::<_,_,String>("GetName",&(index,)).unwrap(),"Press");
    let accepted:bool=action.call("DoAction",&(index,)).expect("one native input attempt; timeout or failure is never retried");assert!(accepted);
    serde_json::json!({"evidence_kind":"owned-production-dialog-native-assistive-input-fixture-not-human-approval","renderer_pid":pid,
        "provider_pid":provider_pid,"proposal_digest":proposal,"native_action":"Press","input_attempts":1,"reviewed_nodes":visited.len()})
}
