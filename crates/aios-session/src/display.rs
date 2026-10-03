//! Native selected KDE Wayland display binding. No inherited display/session
//! environment is identity, and there is no newest-session fallback.
use crate::identity::{self, GraphicalSession};
use aios_protocol::contracts::ErrorCode;
use nix::sys::socket::{getsockopt,sockopt::PeerCredentials};
use serde::Serialize;
use std::{fs,os::unix::{fs::{MetadataExt,FileTypeExt},net::UnixStream},path::PathBuf,time::Duration};
use zbus::{blocking::{Connection,Proxy},zvariant::OwnedObjectPath};
type Result<T> = std::result::Result<T,ErrorCode>;
fn error(e:zbus::Error)->ErrorCode{
    // Fixed native lookup diagnostics contain no error body, proposal or data.
    match e {
        zbus::Error::MethodError(name,_,_)=>eprintln!("aios-sessiond: native display method lookup failed ({name})"),
        zbus::Error::InputOutput(e)=>eprintln!("aios-sessiond: native display transport failed ({:?})",e.kind()),
        _=>eprintln!("aios-sessiond: native display typed lookup failed"),
    }
    ErrorCode::TargetChanged
}
pub(crate) fn proxy<'a>(c:&Connection,owner:&'a str,path:&'a str,interface:&'a str)->Result<Proxy<'a>>{
    zbus::blocking::proxy::Builder::new(c).destination(owner).map_err(error)?.path(path).map_err(error)?
        .interface(interface).map_err(error)?.cache_properties(zbus::proxy::CacheProperties::No).build().map_err(error)
}
pub(crate) fn connection(address:&str)->Result<Connection>{
    zbus::blocking::connection::Builder::address(address).map_err(error)?.method_timeout(Duration::from_millis(250)).build().map_err(error)
}
pub(crate) fn owner(c:&Connection,name:&str,uid:u32)->Result<(String,u32)>{
    let b=proxy(c,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus")?;
    let n:String=b.call("GetNameOwner",&(name,)).map_err(error)?;
    let creds:zbus::fdo::ConnectionCredentials=b.call("GetConnectionCredentials",&(n.as_str(),)).map_err(error)?;
    if creds.unix_user_id()!=Some(uid) || !n.starts_with(':'){return Err(denied("native bus owner credentials"));}
    Ok((n,creds.process_id().filter(|p|*p>0).ok_or(ErrorCode::PermissionDenied)?))
}
fn service(c:&Connection,owner:&str,name:&str)->Result<(u32,String)>{
    let m=proxy(c,owner,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let p:OwnedObjectPath=m.call("GetUnit",&(name,)).map_err(error)?;
    let unit=proxy(c,owner,p.as_str(),"org.freedesktop.systemd1.Unit")?;
    let s=proxy(c,owner,p.as_str(),"org.freedesktop.systemd1.Service")?;
    let state:String=unit.get_property("ActiveState").map_err(error)?;
    if state!="active"{return Err(ErrorCode::TargetChanged);}
    Ok((s.get_property("MainPID").map_err(error)?,s.get_property("ControlGroup").map_err(error)?))
}
fn denied(stage:&str)->ErrorCode { eprintln!("aios-sessiond: native display rejected ({stage})"); ErrorCode::PermissionDenied }
fn compiled_executable(actual:PathBuf,expected:Option<&str>)->Result<String>{
    let expected=expected.ok_or(ErrorCode::UnsupportedCapability)?;
    let canonical=fs::canonicalize(expected).map_err(|_|ErrorCode::UnsupportedCapability)?;
    if actual!=canonical || !canonical.starts_with("/nix/store") {return Err(denied("compiled compositor executable"));}
    canonical.to_str().map(str::to_owned).ok_or(ErrorCode::TargetChanged)
}
fn user_manager_program(c:&Connection,owner:&str,name:&str)->Result<()>{
    let manager=proxy(c,owner,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let path:OwnedObjectPath=manager.call("GetUnit",&(name,)).map_err(error)?;
    let service=proxy(c,owner,path.as_str(),"org.freedesktop.systemd1.Service")?;
    // The user manager is nondumpable. Authenticate its root-owned unit/PID
    // and fixed native ExecStart without weakening ProtectProc or ptrace rules.
    // Native systemd ExecStart includes realtime and monotonic start/exit
    // timestamps (four u64 fields), followed by PID, code and status.
    type ExecStart = Vec<(String,Vec<String>,bool,u64,u64,u64,u64,u32,i32,i32)>;
    let commands:ExecStart=service.get_property("ExecStart").map_err(error)?;
    if commands.len()!=1{return Err(ErrorCode::PermissionDenied);}
    let expected=option_env!("AIOS_USER_MANAGER").ok_or(ErrorCode::UnsupportedCapability)?;
    if commands[0].0!=expected || !commands[0].1.iter().any(|a|a=="--user"){return Err(denied("compiled user manager program"));}
    Ok(())
}
fn user_manager_stamp(c:&Connection,owner:&str,name:&str)->Result<(u64,Vec<u8>)>{
    let m=proxy(c,owner,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let path:OwnedObjectPath=m.call("GetUnit",&(name,)).map_err(error)?;
    let s=proxy(c,owner,path.as_str(),"org.freedesktop.systemd1.Service")?;
    let unit=proxy(c,owner,path.as_str(),"org.freedesktop.systemd1.Unit")?;
    let stamp:u64=s.get_property("ExecMainStartTimestampMonotonic").map_err(error)?;
    let invocation:Vec<u8>=unit.get_property("InvocationID").map_err(error)?;
    if stamp==0 || invocation.len()!=16 || invocation.iter().all(|v|*v==0){return Err(ErrorCode::TargetChanged);}
    Ok((stamp,invocation))
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct DisplayBinding {
    pub session:GraphicalSession,
    pub runtime_inode:u64, pub socket_name:String, pub socket_inode:u64,
    pub manager_pid:u32,pub manager_start_usec:u64,pub manager_invocation:Vec<u8>,pub manager_owner:String,
    pub compositor_pid:u32,pub compositor_start:u64,pub compositor_executable:String,
    pub boot_id:String,
}
impl DisplayBinding {
    pub fn observe(session_id:&str,uid:u32)->Result<Self>{
        if uid==0 || uid!=nix::unistd::geteuid().as_raw(){return Err(ErrorCode::PermissionDenied);}
        let session=identity::observe_graphical_session(session_id,uid)?;
        if session.kind!="wayland" {return Err(ErrorCode::UnsupportedCapability);}
        let system=connection("unix:path=/run/dbus/system_bus_socket")?;
        let login=owner(&system,"org.freedesktop.login1",0)?;
        let login_manager=proxy(&system,&login.0,"/org/freedesktop/login1","org.freedesktop.login1.Manager")?;
        let user_path:OwnedObjectPath=login_manager.call("GetUser",&(uid,)).map_err(error)?;
        let user=proxy(&system,&login.0,user_path.as_str(),"org.freedesktop.login1.User")?;
        let selected_path:OwnedObjectPath=login_manager.call("GetSession",&(session_id,)).map_err(error)?;
        let display:(String,OwnedObjectPath)=user.get_property("Display").map_err(error)?;
        let user_service:String=user.get_property("Service").map_err(error)?;
        if display!=(session_id.into(),selected_path) || user_service!=format!("user@{uid}.service") {return Err(denied("selected primary display"));}
        let root_manager=owner(&system,"org.freedesktop.systemd1",0)?;
        if root_manager.1!=1{return Err(denied("root manager PID"));}
        let root_service=service(&system,&root_manager.0,&user_service)?;
        let expected_group=format!("/user.slice/user-{uid}.slice/user@{uid}.service");
        if root_service.1!=expected_group {return Err(denied("user manager control group"));}
        let runtime=PathBuf::from(format!("/run/user/{uid}"));
        let info=fs::symlink_metadata(&runtime).map_err(|_|ErrorCode::TargetChanged)?;
        if !info.is_dir() || info.uid()!=uid || info.mode()&0o077!=0 || fs::canonicalize(&runtime).map_err(|_|ErrorCode::TargetChanged)?!=runtime{return Err(ErrorCode::PermissionDenied);}
        let runtime_property:String=user.get_property("RuntimePath").map_err(error)?;
        if PathBuf::from(runtime_property)!=runtime{return Err(ErrorCode::PermissionDenied);}
        let address=format!("unix:path={}/bus",runtime.display());
        let bus=connection(&address)?;
        let manager=owner(&bus,"org.freedesktop.systemd1",uid)?;
        if manager.1!=root_service.0{return Err(denied("user manager native PID"));}
        let (manager_start_usec,manager_invocation)=user_manager_stamp(&system,&root_manager.0,&user_service)?;
        let boot_id=fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|_|ErrorCode::TargetChanged)?.trim().to_owned();
        user_manager_program(&system,&root_manager.0,&user_service)?;
        let compositor=service(&bus,&manager.0,"plasma-kwin_wayland.service")?;
        if compositor.1!=format!("{expected_group}/session.slice/plasma-kwin_wayland.service"){return Err(denied("compositor control group"));}
        let (compositor_start,compositor_boot)=identity::process(compositor.0,uid).inspect_err(|_|eprintln!("aios-sessiond: compositor kernel process identity unavailable"))?;
        if compositor_boot!=boot_id{return Err(ErrorCode::TargetChanged);}
        let compositor_executable=compiled_executable(fs::read_link(format!("/proc/{}/exe",compositor.0)).map_err(|_|{eprintln!("aios-sessiond: compositor executable identity unavailable");ErrorCode::TargetChanged})?,option_env!("AIOS_KWIN_WRAPPER"))?;
        let mut sockets=Vec::new();let mut count=0;
        for entry in fs::read_dir(&runtime).map_err(|_|ErrorCode::TargetChanged)?{
            count+=1;if count>256{return Err(ErrorCode::ResourceExhausted);}
            let entry=entry.map_err(|_|ErrorCode::TargetChanged)?;
            let name=entry.file_name().into_string().map_err(|_|ErrorCode::TargetChanged)?;
            if !name.starts_with("wayland-") || name.len()>32 || !name.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'){continue;}
            let meta=fs::symlink_metadata(entry.path()).map_err(|_|ErrorCode::TargetChanged)?;
            if !meta.file_type().is_socket() || meta.uid()!=uid{continue;}
            let socket=UnixStream::connect(entry.path()).map_err(|_|ErrorCode::TargetChanged)?;
            let credentials=getsockopt(&socket,PeerCredentials).map_err(|_|ErrorCode::PermissionDenied)?;
            if credentials.uid()==uid && u32::try_from(credentials.pid()).ok()==Some(compositor.0){sockets.push((name,meta.ino()));}
        }
        if sockets.len()!=1{return Err(ErrorCode::TargetChanged);}
        // No cached properties: primary Display, manager/unit and native process
        // identities must still agree after acquiring the display endpoint.
        if user.get_property::<(String,OwnedObjectPath)>("Display").map_err(error)?!=display
            || service(&system,&root_manager.0,&user_service)?!=root_service
            || service(&bus,&manager.0,"plasma-kwin_wayland.service")?!=compositor
            || owner(&system,"org.freedesktop.login1",0)?!=login || owner(&system,"org.freedesktop.systemd1",0)?!=root_manager
            || owner(&bus,"org.freedesktop.systemd1",uid)?!=manager || identity::observe_graphical_session(session_id,uid)?!=session
            || user_manager_stamp(&system,&root_manager.0,&user_service)?!=(manager_start_usec,manager_invocation.clone())
            || identity::process(compositor.0,uid)!=(Ok((compositor_start,boot_id.clone()))){return Err(ErrorCode::TargetChanged);}
        user_manager_program(&system,&root_manager.0,&user_service)?;
        let (socket_name,socket_inode)=sockets.remove(0);
        Ok(Self{session,runtime_inode:info.ino(),socket_name,socket_inode,manager_pid:manager.1,manager_start_usec,manager_invocation,manager_owner:manager.0,compositor_pid:compositor.0,compositor_start,compositor_executable,boot_id})
    }
    pub fn verify(&self)->Result<()>{
        if Self::observe(&self.session.id,self.session.uid)?!=*self {return Err(ErrorCode::TargetChanged);}Ok(())
    }
}
