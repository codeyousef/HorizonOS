//! Native managed peer authentication across the broker's private user
//! namespace. Root-authenticated user-manager association and exact immutable
//! service ExecStart/MainPID/invocation are the authority, never request fields.
use crate::{display,identity::{self,Peer}};
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{fs,os::unix::{fs::MetadataExt,net::UnixStream},path::PathBuf};
use std::os::fd::AsRawFd;
use zbus::zvariant::OwnedObjectPath;
type Result<T> = std::result::Result<T,ErrorCode>;
fn live(stream:&UnixStream)->Result<()>{
    let mut status=nix::libc::pollfd{fd:stream.as_raw_fd(),events:nix::libc::POLLRDHUP,revents:0};
    if unsafe{nix::libc::poll(&mut status,1,0)}<0 || status.revents&(nix::libc::POLLRDHUP|nix::libc::POLLHUP|nix::libc::POLLERR|nix::libc::POLLNVAL)!=0{return Err(ErrorCode::TargetChanged);}Ok(())
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize)]
pub enum Role { Broker, UiProvider }
impl Role {
    fn unit(self)->&'static str{match self {Self::Broker=>"aios-sessiond.service",Self::UiProvider=>"aios-ui-agent.service"}}
    fn program(self)->&'static str{match self {Self::Broker=>"aios-sessiond",Self::UiProvider=>"aios-ui-agent"}}
    fn service_type(self)->&'static str{match self {Self::Broker=>"dbus",Self::UiProvider=>"exec"}}
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct ManagedService {
    role:Role,peer:Peer,program:String,manager_owner:String,manager_pid:u32,
    manager_start:u64,manager_invocation:Vec<u8>,service_start:u64,service_invocation:Vec<u8>,
}
impl ManagedService {
    pub fn authenticate(stream:&UnixStream,role:Role)->Result<Self>{
        live(stream)?;
        let peer=identity::authenticate(stream)?;
        if peer.uid==0{return Err(ErrorCode::PermissionDenied);}
        let runtime=PathBuf::from(format!("/run/user/{}",peer.uid));
        let meta=fs::symlink_metadata(&runtime).map_err(|_|ErrorCode::TargetChanged)?;
        if !meta.is_dir() || meta.uid()!=peer.uid || meta.mode()&0o077!=0
            || fs::canonicalize(&runtime).map_err(|_|ErrorCode::TargetChanged)?!=runtime{return Err(ErrorCode::PermissionDenied);}
        let own=fs::read_link("/proc/self/exe").map_err(|_|ErrorCode::TargetChanged)?;
        if !own.starts_with("/nix/store") || !matches!(own.file_name().and_then(|p|p.to_str()),Some("aios-sessiond"|"aios-ui-agent")) {return Err(ErrorCode::PermissionDenied);}
        let expected=own.with_file_name(role.program());
        if fs::canonicalize(&expected).map_err(|_|ErrorCode::TargetChanged)?!=expected{return Err(ErrorCode::PermissionDenied);}
        let program=expected.to_str().ok_or(ErrorCode::TargetChanged)?.to_owned();
        let root=display::connection("unix:path=/run/dbus/system_bus_socket")?;
        let root_manager=display::owner(&root,"org.freedesktop.systemd1",0)?;
        if root_manager.1!=1{return Err(ErrorCode::PermissionDenied);}
        let user_unit=format!("user@{}.service",peer.uid);
        let m=display::proxy(&root,&root_manager.0,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
        let path:OwnedObjectPath=m.call("GetUnit",&(user_unit.as_str(),)).map_err(|_|ErrorCode::TargetChanged)?;
        let unit=display::proxy(&root,&root_manager.0,path.as_str(),"org.freedesktop.systemd1.Unit")?;
        let service=display::proxy(&root,&root_manager.0,path.as_str(),"org.freedesktop.systemd1.Service")?;
        let active:String=unit.get_property("ActiveState").map_err(|_|ErrorCode::TargetChanged)?;
        let manager_pid:u32=service.get_property("MainPID").map_err(|_|ErrorCode::TargetChanged)?;
        let group:String=service.get_property("ControlGroup").map_err(|_|ErrorCode::TargetChanged)?;
        let manager_group=format!("/user.slice/user-{}.slice/user@{}.service",peer.uid,peer.uid);
        if active!="active" || group!=manager_group{return Err(ErrorCode::TargetChanged);}
        display::user_manager_program(&root,&root_manager.0,&user_unit)?;
        let (manager_start,manager_invocation)=display::user_manager_stamp(&root,&root_manager.0,&user_unit)?;
        let bus=display::connection(&format!("unix:path={}/bus",runtime.display()))?;
        let manager=display::owner(&bus,"org.freedesktop.systemd1",peer.uid)?;
        if manager.1!=manager_pid{return Err(ErrorCode::PermissionDenied);}
        let m=display::proxy(&bus,&manager.0,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
        let path:OwnedObjectPath=m.call("GetUnit",&(role.unit(),)).map_err(|_|ErrorCode::TargetChanged)?;
        let u=display::proxy(&bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Unit")?;
        let s=display::proxy(&bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Service")?;
        let active:String=u.get_property("ActiveState").map_err(|_|ErrorCode::TargetChanged)?;
        let kind:String=s.get_property("Type").map_err(|_|ErrorCode::TargetChanged)?;
        let pid:u32=s.get_property("MainPID").map_err(|_|ErrorCode::TargetChanged)?;
        let service_start:u64=s.get_property("ExecMainStartTimestampMonotonic").map_err(|_|ErrorCode::TargetChanged)?;
        let service_invocation:Vec<u8>=u.get_property("InvocationID").map_err(|_|ErrorCode::TargetChanged)?;
        type Commands=Vec<(String,Vec<String>,bool,u64,u64,u64,u64,u32,i32,i32)>;
        let commands:Commands=s.get_property("ExecStart").map_err(|_|ErrorCode::TargetChanged)?;
        if active!="active" || kind!=role.service_type() || pid!=peer.pid || service_start==0
            || service_invocation.len()!=16 || service_invocation.iter().all(|v|*v==0)
            || commands.len()!=1 || commands[0].0!=program || commands[0].1!=vec![program.clone()] || commands[0].2 {
            return Err(ErrorCode::PermissionDenied);
        }
        if role==Role::Broker {
            let owner=display::owner(&bus,"org.aios.Session1",peer.uid)?;
            let name:String=s.get_property("BusName").map_err(|_|ErrorCode::TargetChanged)?;
            if owner.1!=peer.pid || name!="org.aios.Session1"{return Err(ErrorCode::PermissionDenied);}
        }
        if display::user_manager_stamp(&root,&root_manager.0,&user_unit)?!=(manager_start,manager_invocation.clone())
            || display::owner(&bus,"org.freedesktop.systemd1",peer.uid)?!=manager {return Err(ErrorCode::TargetChanged);}
        identity::verify(stream,&peer)?;
        live(stream)?;
        drop(s);drop(u);drop(m);
        Ok(Self{role,peer,program,manager_owner:manager.0,manager_pid,manager_start,manager_invocation,service_start,service_invocation})
    }
    pub fn verify(&self,stream:&UnixStream)->Result<()>{
        if Self::authenticate(stream,self.role)?!=*self{return Err(ErrorCode::TargetChanged);}Ok(())
    }
    pub(crate) fn verify_cancellation_peer(&self,stream:&UnixStream)->Result<()>{
        if self.role!=Role::Broker || identity::authenticate(stream)?!=self.peer{return Err(ErrorCode::PermissionDenied);}Ok(())
    }
}
