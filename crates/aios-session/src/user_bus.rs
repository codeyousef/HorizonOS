//! Fixed native user bus, bound to the root-managed user manager and kernel
//! socket peer. Bus replies alone cannot authenticate a substituted endpoint.
use crate::{display,identity::{self,Peer}};
use aios_protocol::contracts::ErrorCode;
use nix::sys::socket::{getsockopt,sockopt::PeerCredentials};
use std::{fs,os::{fd::AsRawFd,unix::{fs::{MetadataExt,FileTypeExt},net::UnixStream}},path::PathBuf,time::Duration};
use zbus::{blocking::Connection,zvariant::OwnedObjectPath};
type Result<T> = std::result::Result<T,ErrorCode>;

#[derive(Clone,PartialEq,Eq)]
struct Manager { owner:String,pid:u32,start:u64,invocation:Vec<u8> }
fn manager(uid:u32)->Result<Manager>{
    let root=display::connection("unix:path=/run/dbus/system_bus_socket")?;
    let owner=display::owner(&root,"org.freedesktop.systemd1",0)?;
    if owner.1!=1{return Err(ErrorCode::PermissionDenied);}
    let name=format!("user@{uid}.service");
    let m=display::proxy(&root,&owner.0,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let path:OwnedObjectPath=m.call("GetUnit",&(name.as_str(),)).map_err(|_|ErrorCode::TargetChanged)?;
    let unit=display::proxy(&root,&owner.0,path.as_str(),"org.freedesktop.systemd1.Unit")?;
    let service=display::proxy(&root,&owner.0,path.as_str(),"org.freedesktop.systemd1.Service")?;
    let active:String=unit.get_property("ActiveState").map_err(|_|ErrorCode::TargetChanged)?;
    let group:String=service.get_property("ControlGroup").map_err(|_|ErrorCode::TargetChanged)?;
    let pid:u32=service.get_property("MainPID").map_err(|_|ErrorCode::TargetChanged)?;
    if active!="active" || pid<=1 || group!=format!("/user.slice/user-{uid}.slice/user@{uid}.service"){
        return Err(ErrorCode::PermissionDenied);
    }
    display::user_manager_program(&root,&owner.0,&name)?;
    let (start,invocation)=display::user_manager_stamp(&root,&owner.0,&name)?;
    if display::owner(&root,"org.freedesktop.systemd1",0)?!=owner{return Err(ErrorCode::TargetChanged);}
    Ok(Manager{owner:owner.0.clone(),pid,start,invocation})
}
#[derive(Clone,PartialEq,Eq)]
struct Endpoint { directory_inode:u64,device:u64,inode:u64,mode:u32 }
fn endpoint(uid:u32)->Result<Endpoint>{
    let directory=PathBuf::from(format!("/run/user/{uid}"));
    let info=fs::symlink_metadata(&directory).map_err(|_|ErrorCode::TargetChanged)?;
    if !info.is_dir() || info.uid()!=uid || info.mode()&0o077!=0 || directory.canonicalize().map_err(|_|ErrorCode::TargetChanged)?!=directory{
        return Err(ErrorCode::PermissionDenied);
    }
    let info_bus=fs::symlink_metadata(directory.join("bus")).map_err(|_|ErrorCode::TargetChanged)?;
    if !info_bus.file_type().is_socket() || info_bus.uid()!=uid{return Err(ErrorCode::PermissionDenied);}
    Ok(Endpoint{directory_inode:info.ino(),device:info_bus.dev(),inode:info_bus.ino(),mode:info_bus.mode()})
}
pub(crate) struct NativeUserBus { connection:Connection,proof:UnixStream,uid:u32,manager:Manager,endpoint:Endpoint }
impl NativeUserBus {
    pub(crate) fn connect()->Result<Self>{
        let uid=nix::unistd::geteuid().as_raw();if uid==0{return Err(ErrorCode::PermissionDenied);}
        let manager=manager(uid)?;let endpoint=endpoint(uid)?;
        let proof=UnixStream::connect(format!("/run/user/{uid}/bus")).map_err(|_|ErrorCode::UnsupportedCapability)?;
        let credentials=getsockopt(&proof,PeerCredentials).map_err(|_|ErrorCode::PermissionDenied)?;
        if credentials.uid()!=uid || u32::try_from(credentials.pid()).ok()!=Some(manager.pid){return Err(ErrorCode::PermissionDenied);}
        proof.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
        proof.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_|ErrorCode::TargetChanged)?;
        // zbus uses this very kernel connection, not a second address lookup.
        let connection=zbus::blocking::connection::Builder::async_io_unix_stream(proof.try_clone().map_err(|_|ErrorCode::TargetChanged)?)
            .method_timeout(Duration::from_millis(250)).build().map_err(|_|ErrorCode::TargetChanged)?;
        let value=Self{connection,proof,uid,manager,endpoint};value.verify()?;Ok(value)
    }
    fn verify(&self)->Result<()>{
        let mut poll=nix::libc::pollfd{fd:self.proof.as_raw_fd(),events:nix::libc::POLLRDHUP,revents:0};
        if unsafe{nix::libc::poll(&mut poll,1,0)}<0 || poll.revents&(nix::libc::POLLRDHUP|nix::libc::POLLHUP|nix::libc::POLLERR|nix::libc::POLLNVAL)!=0{
            return Err(ErrorCode::TargetChanged);
        }
        let credentials=getsockopt(&self.proof,PeerCredentials).map_err(|_|ErrorCode::PermissionDenied)?;
        if credentials.uid()!=self.uid || u32::try_from(credentials.pid()).ok()!=Some(self.manager.pid)
            || manager(self.uid)?!=self.manager || endpoint(self.uid)?!=self.endpoint{return Err(ErrorCode::TargetChanged);}
        Ok(())
    }
    pub(crate) fn caller(&self,sender:&str,bus_id:&str)->Result<Peer>{
        validate_reference(sender,bus_id)?;self.verify()?;
        let bus=display::proxy(&self.connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus")?;
        let actual_id:String=bus.call("GetId",&()).map_err(|_|ErrorCode::TargetChanged)?;
        if actual_id!=bus_id{return Err(ErrorCode::TargetChanged);}
        let credentials:zbus::fdo::ConnectionCredentials=bus.call("GetConnectionCredentials",&(sender,)).map_err(caller_error)?;
        let uid=credentials.unix_user_id().ok_or(ErrorCode::PermissionDenied)?;
        let pid=credentials.process_id().ok_or(ErrorCode::PermissionDenied)?;
        let mut peer=identity::authenticate_process(uid,pid)?;
        let after:zbus::fdo::ConnectionCredentials=bus.call("GetConnectionCredentials",&(sender,)).map_err(caller_error)?;
        if after.unix_user_id()!=Some(uid) || after.process_id()!=Some(pid){return Err(ErrorCode::TargetChanged);}
        self.verify()?;
        peer.bus_sender=Some(sender.into());peer.bus_id=Some(bus_id.into());Ok(peer)
    }
    pub(crate) fn try_clone(&self)->Result<Self>{
        self.verify()?;Ok(Self{connection:self.connection.clone(),proof:self.proof.try_clone().map_err(|_|ErrorCode::TargetChanged)?,
            uid:self.uid,manager:self.manager.clone(),endpoint:self.endpoint.clone()})
    }
}
fn caller_error(error:zbus::Error)->ErrorCode{
    match error {
        zbus::Error::MethodError(name,_,_) if name.as_str()=="org.freedesktop.DBus.Error.NameHasNoOwner"=>ErrorCode::Cancelled,
        _=>ErrorCode::TargetChanged,
    }
}
pub(crate) fn validate_reference(sender:&str,bus_id:&str)->Result<()>{
    if sender.len()>255 || !sender.starts_with(':') || zbus::names::UniqueName::try_from(sender).is_err()
        || bus_id.len()!=32 || !bus_id.bytes().all(|b|b.is_ascii_hexdigit()){return Err(ErrorCode::InvalidArgument);}Ok(())
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]
    #[ignore="requires the registered disposable managed native-provider scenario"]
    fn native_bus_origin_checks_kernel_manager_unique_sender_reconnect_and_disconnect(){
        assert_eq!(fs::read_to_string("/etc/aios/desktop-test-profile").unwrap().trim(),"synthetic-disposable-plasma-wayland-v1");
        assert_eq!(std::env::var("AIOS_NATIVE_BRIDGE_SCENARIO").unwrap(),"disposable-provider-v1");
        assert_eq!(nix::unistd::geteuid().as_raw(),1001);
        let socket=UnixStream::connect("/run/user/1001/bus").unwrap();
        let caller=zbus::blocking::connection::Builder::async_io_unix_stream(socket.try_clone().unwrap()).build().unwrap();
        let reconnect=zbus::blocking::connection::Builder::address("unix:path=/run/user/1001/bus").unwrap().build().unwrap();
        let sender=caller.unique_name().unwrap().to_string();let second=reconnect.unique_name().unwrap().to_string();assert_ne!(sender,second);
        let b=display::proxy(&caller,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus").unwrap();
        let id:String=b.call("GetId",&()).unwrap();
        let bus=NativeUserBus::connect().expect("kernel bus peer/root user-manager association");
        let peer=bus.caller(&sender,&id).unwrap();let other=bus.caller(&second,&id).unwrap();
        assert_eq!(peer.uid,1001);assert_eq!(peer.pid,std::process::id());assert_eq!(other.pid,peer.pid);assert_ne!(peer,other);
        let origin=crate::ui_read::OriginatingClient::authenticate_bus(&sender,&id).unwrap();
        let retained=origin.try_clone().unwrap();origin.verify().unwrap();retained.verify().unwrap();
        let mut forged_id=id.clone();forged_id.replace_range(..1,if id.starts_with('0'){"1"}else{"0"});
        assert_eq!(bus.caller(&sender,&forged_id),Err(ErrorCode::TargetChanged));
        assert_eq!(bus.caller("org.aios.Session1",&id),Err(ErrorCode::InvalidArgument));
        assert_eq!(bus.caller(":4294967295.4294967295",&id),Err(ErrorCode::Cancelled));
        socket.shutdown(std::net::Shutdown::Both).unwrap();
        let deadline=std::time::Instant::now()+Duration::from_secs(2);
        loop {
            match origin.verify(){Err(ErrorCode::Cancelled)=>break,Ok(())=>{},Err(error)=>panic!("unexpected disconnected-origin error {error:?}")}
            assert!(std::time::Instant::now()<deadline);std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(retained.verify(),Err(ErrorCode::Cancelled));
        assert_eq!(bus.caller(&sender,&id),Err(ErrorCode::Cancelled));bus.caller(&second,&id).unwrap();
        println!("NATIVE_BUS_ORIGIN={}",serde_json::json!({"evidence_kind":"actual-kernel-bus-peer-root-user-manager-and-native-distinct-connections",
            "uid":peer.uid,"pid":peer.pid,"start_ticks":peer.start_ticks,"boot_id":peer.boot_id,"original_sender":sender,"reconnected_sender":second,
            "bus_id":id,"native_manager_pid":bus.manager.pid,"manager_start_usec":bus.manager.start,"manager_invocation":bus.manager.invocation,
            "same_pid_reconnect_isolated":true,"forged_bus_id_denied":true,"well_known_name_denied":true,"unknown_sender_denied":true,
            "disconnect_revokes_original_and_cloned_proof":true,"independent_sender_survives":true}));
    }
}
