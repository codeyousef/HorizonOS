//! Native AT-SPI selected-window observations. Authority stays in the broker;
//! candidates and snapshots cannot deserialize from model/client JSON.
use crate::{display::{self,DisplayBinding},identity};
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{collections::{HashSet,VecDeque},fs,os::unix::{fs::MetadataExt,net::UnixStream},sync::atomic::{AtomicU8,Ordering},time::{Duration,Instant}};
use nix::sys::socket::{getsockopt,sockopt::PeerCredentials};
use zbus::{blocking::{Connection,Proxy},zvariant::OwnedObjectPath};
use uuid::Uuid;
type Result<T> = std::result::Result<T,ErrorCode>;
type Object = (String,OwnedObjectPath);
const ACCESSIBLE:&str="org.a11y.atspi.Accessible";
const ROOT:&str="/org/a11y/atspi/accessible/root";
fn error(e:zbus::Error)->ErrorCode{
    match e {
        zbus::Error::InputOutput(e) if e.kind()==std::io::ErrorKind::TimedOut=>ErrorCode::DeadlineExceeded,
        zbus::Error::MethodError(name,_,_) if matches!(name.as_str(),"org.freedesktop.DBus.Error.NoReply"|"org.freedesktop.DBus.Error.Timeout")=>ErrorCode::DeadlineExceeded,
        _=>ErrorCode::PartialResult,
    }
}
fn check(deadline:Instant,control:&AtomicU8)->Result<()>{
    if control.load(Ordering::Acquire)!=0{return Err(ErrorCode::Cancelled);}
    if Instant::now()>=deadline{return Err(ErrorCode::DeadlineExceeded);}Ok(())
}
fn text(value:String,max:usize)->Result<String>{
    if value.len()>max || value.chars().any(|c|c=='\0' || ('\u{202a}'..='\u{202e}').contains(&c) || ('\u{2066}'..='\u{2069}').contains(&c)){return Err(ErrorCode::PartialResult);}Ok(value)
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
struct BusIdentity { address:String,id:String,server_pid:u32,server_start:u64,launcher_owner:String,launcher_pid:u32,launcher_start:u64 }
struct Bus { connection:Connection,identity:BusIdentity }
impl Bus {
    fn open(display:&DisplayBinding)->Result<Self>{
        let uid=display.session.uid;
        let session=display::connection(&format!("unix:path=/run/user/{uid}/bus"))?;
        let launcher=display::owner(&session,"org.a11y.Bus",uid)?;
        let (launcher_start,boot)=identity::process(launcher.1,uid)?;
        let expected=option_env!("AIOS_ATSPI_LAUNCHER").ok_or(ErrorCode::UnsupportedCapability)?;
        if boot!=display.boot_id || fs::read_link(format!("/proc/{}/exe",launcher.1)).map_err(|_|ErrorCode::TargetChanged)?!=std::path::Path::new(expected){return Err(ErrorCode::PermissionDenied);}
        let source=display::proxy(&session,&launcher.0,"/org/a11y/bus","org.a11y.Bus")?;
        let address:String=source.call("GetAddress",&()).map_err(error)?;
        drop(source);
        let socket=format!("/run/user/{uid}/at-spi/bus_0");
        if address!=format!("unix:path={socket}"){return Err(ErrorCode::UnsupportedCapability);}
        let path=std::path::Path::new(&socket);let parent=path.parent().ok_or(ErrorCode::TargetChanged)?;
        let info=fs::symlink_metadata(parent).map_err(|_|ErrorCode::TargetChanged)?;
        if !info.is_dir() || info.uid()!=uid || info.mode()&0o077!=0 || fs::canonicalize(parent).map_err(|_|ErrorCode::TargetChanged)?!=parent{return Err(ErrorCode::PermissionDenied);}
        let stream=UnixStream::connect(path).map_err(|_|ErrorCode::TargetChanged)?;
        let peer=getsockopt(&stream,PeerCredentials).map_err(|_|ErrorCode::PermissionDenied)?;
        if peer.uid()!=uid{return Err(ErrorCode::PermissionDenied);}
        let server_pid=u32::try_from(peer.pid()).map_err(|_|ErrorCode::PermissionDenied)?;
        let (server_start,server_boot)=identity::process(server_pid,uid)?;
        // The pinned launcher creates this listening socket before passing it
        // to the accessibility bus. An owned runtime path alone is not proof
        // that a same-UID replacement speaks genuine bus credentials.
        if server_pid!=launcher.1 || server_start!=launcher_start || server_boot!=boot{return Err(ErrorCode::TargetChanged);}
        let connection=zbus::blocking::connection::Builder::address(address.as_str()).map_err(error)?
            .method_timeout(Duration::from_millis(100)).build().map_err(error)?;
        let dbus=display::proxy(&connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus")?;
        let id:String=dbus.call("GetId",&()).map_err(error)?;
        if display::owner(&session,"org.a11y.Bus",uid)?!=launcher || identity::process(launcher.1,uid)?!=(launcher_start,boot){return Err(ErrorCode::TargetChanged);}
        Ok(Self{connection,identity:BusIdentity{address,id,server_pid,server_start,launcher_owner:launcher.0,launcher_pid:launcher.1,launcher_start}})
    }
    fn app(&self,name:&str,uid:u32)->Result<AppIdentity>{
        if !name.starts_with(':') || name.len()>128{return Err(ErrorCode::PermissionDenied);}
        let (owner,pid)=display::owner(&self.connection,name,uid)?;
        if owner!=name{return Err(ErrorCode::PermissionDenied);}
        let (start,boot)=identity::process(pid,uid)?;
        let executable=fs::read_link(format!("/proc/{pid}/exe")).map_err(|_|ErrorCode::TargetChanged)?;
        // Only the reviewed native Kate target is registered at this boundary.
        // Login, lock, polkit, password managers, terminals and our own approval
        // UI are denied before any accessibility content query.
        let expected=option_env!("AIOS_KATE_NATIVE").ok_or(ErrorCode::UnsupportedCapability)?;
        if executable!=std::path::Path::new(expected){return Err(ErrorCode::UnsupportedCapability);}
        Ok(AppIdentity{owner,pid,start,boot,executable:expected.into()})
    }
    fn accessible<'a>(&self,name:&'a str,path:&'a str)->Result<Proxy<'a>>{
        display::proxy(&self.connection,name,path,ACCESSIBLE)
    }
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
struct AppIdentity { owner:String,pid:u32,start:u64,boot:String,executable:String }
#[derive(Clone,Debug,Serialize)]
pub struct WindowBinding {
    display:DisplayBinding,bus:BusIdentity,app:AppIdentity,path:OwnedObjectPath,
    pub handle:String,pub name:String,pub title:String,role:u32,
}
impl WindowBinding {
    pub fn discover(display:&DisplayBinding,control:&AtomicU8)->Result<Vec<Self>>{
        display.verify()?;let bus=Bus::open(display)?;
        let deadline=Instant::now()+Duration::from_secs(2);
        let registry=display::owner(&bus.connection,"org.a11y.atspi.Registry",display.session.uid)?;
        let root=bus.accessible(&registry.0,ROOT)?;
        let count:i32=root.get_property("ChildCount").map_err(error)?;
        if !(0..=64).contains(&count){return Err(ErrorCode::ResourceExhausted);}
        let mut out=Vec::new();
        for i in 0..count {
            check(deadline,control)?;
            let child:Object=root.call("GetChildAtIndex",&(i,)).map_err(error)?;
            if child.1.as_str()!=ROOT{continue;}
            let app=match bus.app(&child.0,display.session.uid){Ok(app)=>app,Err(ErrorCode::UnsupportedCapability)=>continue,Err(e)=>return Err(e)};
            let accessible=bus.accessible(&app.owner,ROOT)?;
            let name=text(accessible.get_property("Name").map_err(error)?,256)?;
            let windows:i32=accessible.get_property("ChildCount").map_err(error)?;
            if !(0..=16).contains(&windows){return Err(ErrorCode::ResourceExhausted);}
            for j in 0..windows {
                check(deadline,control)?;
                let window:Object=accessible.call("GetChildAtIndex",&(j,)).map_err(error)?;
                if window.0!=app.owner{return Err(ErrorCode::PermissionDenied);}
                let p=bus.accessible(&app.owner,window.1.as_str())?;
                let role:u32=p.call("GetRole",&()).map_err(error)?;
                if !matches!(role,23|69){continue;}
                let title=text(p.get_property("Name").map_err(error)?,512)?;
                if protected_title(&title){continue;}
                if p.get_property::<Object>("Parent").map_err(error)?!=(app.owner.clone(),child.1.clone()){return Err(ErrorCode::TargetChanged);}
                drop(p);
                out.push(Self{display:display.clone(),bus:bus.identity.clone(),app:app.clone(),path:window.1,handle:Uuid::new_v4().to_string(),name:name.clone(),title,role});
                if out.len()>16{return Err(ErrorCode::ResourceExhausted);}
            }
        }
        check(deadline,control)?;display.verify()?;
        Ok(out)
    }
    pub fn identity_sha256(&self)->Result<String>{aios_policy::digest(self)}
    pub(crate) fn selected_display(&self)->&DisplayBinding{&self.display}
    fn current(&self)->Result<Bus>{
        self.display.verify()?;
        let bus=Bus::open(&self.display)?;
        if bus.identity!=self.bus || bus.app(&self.app.owner,self.display.session.uid)?!=self.app{return Err(ErrorCode::TargetChanged);}
        let p=bus.accessible(&self.app.owner,self.path.as_str())?;
        let role:u32=p.call("GetRole",&()).map_err(error)?;
        let title:String=p.get_property("Name").map_err(error)?;
        if role!=self.role || title!=self.title || protected_title(&title)
            || p.get_property::<Object>("Parent").map_err(error)?!=(self.app.owner.clone(),OwnedObjectPath::try_from(ROOT).map_err(|_|ErrorCode::TargetChanged)?){return Err(ErrorCode::TargetChanged);}
        Ok(bus)
    }
    pub fn verify(&self)->Result<()>{self.current().map(|_|())}
    /// Trusted broker calls this only after native consent and shared-policy
    /// authorization of this exact window handle. There is no direct IPC route.
    pub fn snapshot(&self,control:&AtomicU8)->Result<Snapshot>{
        let deadline=Instant::now()+Duration::from_secs(2);check(deadline,control)?;
        let bus=self.current()?;
        let snapshot_id=Uuid::new_v4().to_string();let mut nodes=Vec::new();
        let mut queue=VecDeque::from([(self.path.clone(),0u8,OwnedObjectPath::try_from(ROOT).map_err(|_|ErrorCode::TargetChanged)?)]);let mut visited=HashSet::new();
        let mut bytes=0;let mut truncated=false;
        while let Some((path,depth,parent))=queue.pop_front(){
            check(deadline,control)?;
            if !visited.insert(path.clone()){return Err(ErrorCode::TargetChanged);}
            if nodes.len()>=300{truncated=true;break;}
            let p=bus.accessible(&self.app.owner,path.as_str())?;
            // A same-process object reference alone does not prove membership
            // in the selected window. Verify each native parent before content.
            if p.get_property::<Object>("Parent").map_err(error)?!=(self.app.owner.clone(),parent){return Err(ErrorCode::TargetChanged);}
            let role:u32=p.call("GetRole",&()).map_err(error)?;
            // Do not even query protected names, text, actions or children.
            if matches!(role,16|40|60){
                nodes.push(Node{node_handle:Uuid::new_v4().to_string(),role:format!("atspi:{role}"),name:"[redacted]".into(),states:vec![],actions:vec![]});continue;
            }
            let mut name=text(p.get_property("Name").map_err(error)?,16384)?;
            if bytes+name.len()>16384{truncated=true;break;}
            bytes+=name.len();name=name.chars().take(256).collect();
            let states:Vec<u32>=p.call("GetState",&()).map_err(error)?;
            if states.len()>2{return Err(ErrorCode::PartialResult);}
            let states=(0u32..64).filter(|i|states.get((i/32)as usize).is_some_and(|word|word&(1u32<<(i%32))!=0)).take(16).map(|i|format!("atspi:{i}")).collect();
            let interfaces:Vec<String>=p.call("GetInterfaces",&()).map_err(error)?;
            if interfaces.len()>16{return Err(ErrorCode::PartialResult);}
            let mut actions=Vec::new();
            if interfaces.iter().any(|i|i=="org.a11y.atspi.Action"){
                let action=display::proxy(&bus.connection,&self.app.owner,path.as_str(),"org.a11y.atspi.Action")?;
                let count:i32=action.get_property("NActions").map_err(error)?;
                if !(0..=16).contains(&count){return Err(ErrorCode::PartialResult);}
                for i in 0..count {
                    check(deadline,control)?;
                    let name=text(action.call("GetName",&(i,)).map_err(error)?,128)?;
                    if bytes+name.len()>16384{truncated=true;break;}
                    bytes+=name.len();actions.push(name);
                }
            }
            nodes.push(Node{node_handle:Uuid::new_v4().to_string(),role:format!("atspi:{role}"),name,states,actions});
            let children:i32=p.get_property("ChildCount").map_err(error)?;
            if children<0 || children>100000{return Err(ErrorCode::PartialResult);}
            if depth>=8 && children>0{truncated=true;continue;}
            let capacity=300usize.saturating_sub(nodes.len()+queue.len());
            let count=(children as usize).min(capacity);if count<children as usize{truncated=true;}
            for i in 0..count{
                check(deadline,control)?;
                let child:Object=p.call("GetChildAtIndex",&(i as i32,)).map_err(error)?;
                if child.0!=self.app.owner{return Err(ErrorCode::PermissionDenied);}
                queue.push_back((child.1,depth+1,path.clone()));
            }
        }
        check(deadline,control)?;self.verify()?;check(deadline,control)?;
        Ok(Snapshot{snapshot_id,window_handle:self.handle.clone(),nodes,truncated})
    }
}
fn protected_title(title:&str)->bool{
    let name=title.to_lowercase();
    ["id_ed25519","id_rsa","private key","keyring","password","authorization","authentication","wallet","horizon os — needs permission"].iter().any(|w|name.contains(w))
}
#[derive(Serialize)]
pub struct Snapshot {pub snapshot_id:String,pub window_handle:String,pub nodes:Vec<Node>,pub truncated:bool}
#[derive(Serialize)]
pub struct Node {pub node_handle:String,pub role:String,pub name:String,pub states:Vec<String>,pub actions:Vec<String>}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn control_and_deadline_deny_before_query(){
        let c=AtomicU8::new(1);assert_eq!(check(Instant::now()+Duration::from_secs(1),&c),Err(ErrorCode::Cancelled));
        c.store(0,Ordering::Release);assert_eq!(check(Instant::now()-Duration::from_millis(1),&c),Err(ErrorCode::DeadlineExceeded));
    }
    #[test]fn protected_titles_and_directional_names_never_enter_content(){
        for v in ["id_ed25519 — Kate","Password — Kate","Authentication","Horizon OS — Needs permission"]{assert!(protected_title(v));}
        assert!(!protected_title("fixture.txt — Kate"));assert!(text("spoof\u{202e}text".into(),128).is_err());
    }
}
