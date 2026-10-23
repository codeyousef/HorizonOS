//! Transfer scoped read authority only to the compile-pinned managed sandbox.
//! A socket activation listener belongs to systemd: its SO_PEERCRED alone does
//! not identify the worker. The ready byte's kernel SCM_CREDENTIALS does.
use crate::display::{connection,owner,proxy,user_manager_program,user_manager_stamp};
use aios_files::{Manager,Owner,ExtractedDocument,extract::{Request,MAX_WIRE,transport::{Response,MARKER,READY_MARKER}}};
use aios_protocol::contracts::ErrorCode;
use nix::sys::socket::{self,AddressFamily,SockType,SockFlag,UnixAddr,MsgFlags,ControlMessage,ControlMessageOwned,sockopt};
use std::{fs,io::{Read,Write,IoSlice,IoSliceMut},net::Shutdown,os::{fd::{AsRawFd,OwnedFd},unix::{net::UnixStream,fs::{MetadataExt,FileTypeExt}}},sync::{Arc,Mutex},time::Duration};
use zbus::{blocking::Connection,zvariant::OwnedObjectPath};
type Result<T> = std::result::Result<T,ErrorCode>;
fn changed<T>(_:T)->ErrorCode {ErrorCode::TargetChanged}
fn exhausted<T>(_:T)->ErrorCode {ErrorCode::ResourceExhausted}
fn dbus(error:zbus::Error)->ErrorCode {
    match error {
        zbus::Error::MethodError(name,_,_)=>eprintln!("aios-sessiond: extractor native lookup failed ({name})"),
        zbus::Error::InputOutput(error)=>eprintln!("aios-sessiond: extractor native transport failed ({:?})",error.kind()),
        _=>eprintln!("aios-sessiond: extractor native property type failed"),
    }
    ErrorCode::TargetChanged
}

fn connect(uid:u32)->Result<UnixStream> {
    let base=format!("/run/user/{uid}");
    for (path,is_socket) in [(base.clone(),false),(format!("{base}/aios-extract"),false),(format!("{base}/aios-extract/worker.sock"),true)] {
        let m=fs::symlink_metadata(&path).map_err(changed)?;
        if m.uid()!=uid || m.mode()&0o077!=0 || m.file_type().is_socket()!=is_socket || (!is_socket && !m.is_dir())
            || fs::canonicalize(&path).map_err(changed)?!=std::path::PathBuf::from(&path) {return Err(ErrorCode::PermissionDenied);}
    }
    let fd:OwnedFd=socket::socket(AddressFamily::Unix,SockType::Stream,SockFlag::SOCK_CLOEXEC|SockFlag::SOCK_NONBLOCK,None).map_err(exhausted)?;
    // Set before connect: the first received byte must carry credentials.
    socket::setsockopt(&fd,sockopt::PassCred,&true).map_err(changed)?;
    let addr=UnixAddr::new(format!("{base}/aios-extract/worker.sock").as_str()).map_err(changed)?;
    match socket::connect(fd.as_raw_fd(),&addr) {
        Ok(())=>{},
        Err(nix::errno::Errno::EINPROGRESS)=>{
            let mut poll=nix::libc::pollfd {fd:fd.as_raw_fd(),events:nix::libc::POLLOUT,revents:0};
            if unsafe {nix::libc::poll(&mut poll,1,500)}!=1 || socket::getsockopt(&fd,sockopt::SocketError).map_err(changed)?!=0 {return Err(ErrorCode::ResourceExhausted);}
        },
        Err(nix::errno::Errno::EAGAIN)=>return Err(ErrorCode::ResourceExhausted),
        Err(_)=>return Err(ErrorCode::TargetChanged),
    }
    let stream=UnixStream::from(fd);stream.set_nonblocking(false).map_err(changed)?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).map_err(changed)?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(changed)?;
    Ok(stream)
}
fn ready(stream:&UnixStream,uid:u32)->Result<u32> {
    let mut byte=[0u8];let mut iov=[IoSliceMut::new(&mut byte)];
    let mut ancillary=nix::cmsg_space!(nix::sys::socket::UnixCredentials, [i32;253]);
    let msg=socket::recvmsg::<()>(stream.as_raw_fd(),&mut iov,Some(&mut ancillary),MsgFlags::MSG_CMSG_CLOEXEC).map_err(|error|{eprintln!("aios-sessiond: extractor ready receive failed ({error})");ErrorCode::TargetChanged})?;
    let mut credentials=None;let mut invalid=false;
    for c in msg.cmsgs().map_err(changed)? {match c {
        ControlMessageOwned::ScmCredentials(c) if credentials.is_none()=>credentials=Some(c),
        ControlMessageOwned::ScmRights(fds)=>{use std::os::fd::FromRawFd;for fd in fds {drop(unsafe {OwnedFd::from_raw_fd(fd)});}invalid=true;},
        _=>invalid=true,
    }}
    if invalid || msg.bytes!=1 || msg.flags.intersects(MsgFlags::MSG_CTRUNC|MsgFlags::MSG_TRUNC) || byte[0]!=READY_MARKER {return Err(ErrorCode::PermissionDenied);}
    let c=credentials.ok_or(ErrorCode::PermissionDenied)?;
    if c.uid()!=uid || c.pid()<=0 {return Err(ErrorCode::PermissionDenied);}Ok(c.pid() as u32)
}
fn manager_service(system:&Connection,root:&str,uid:u32)->Result<u32> {
    let m=proxy(system,root,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let path:OwnedObjectPath=m.call("GetUnit",&(format!("user@{uid}.service"),)).map_err(dbus)?;
    let unit=proxy(system,root,path.as_str(),"org.freedesktop.systemd1.Unit")?;
    let service=proxy(system,root,path.as_str(),"org.freedesktop.systemd1.Service")?;
    if unit.get_property::<String>("ActiveState").map_err(dbus)?!="active"
        || service.get_property::<String>("ControlGroup").map_err(dbus)?!=format!("/user.slice/user-{uid}.slice/user@{uid}.service") {return Err(ErrorCode::PermissionDenied);}
    let pid:u32=service.get_property("MainPID").map_err(dbus)?;if pid==0 {return Err(ErrorCode::TargetChanged);}Ok(pid)
}
fn authenticate(stream:&UnixStream,uid:u32,pid:u32)->Result<()> {
    let expected=option_env!("AIOS_EXTRACT").ok_or(ErrorCode::UnsupportedCapability)?;
    if !expected.starts_with("/nix/store/") {return Err(ErrorCode::PermissionDenied);}
    let system=connection("unix:path=/run/dbus/system_bus_socket")?;
    let root=owner(&system,"org.freedesktop.systemd1",0)?;if root.1!=1 {return Err(ErrorCode::PermissionDenied);}
    let name=format!("user@{uid}.service");let manager_pid=manager_service(&system,&root.0,uid)?;
    let stamp=user_manager_stamp(&system,&root.0,&name)?;user_manager_program(&system,&root.0,&name)?;
    let listener=socket::getsockopt(stream,sockopt::PeerCredentials).map_err(changed)?;
    if listener.uid()!=uid || listener.pid()<=0 || listener.pid() as u32!=manager_pid {return Err(ErrorCode::PermissionDenied);}
    let bus=connection(&format!("unix:path=/run/user/{uid}/bus"))?;
    let manager=owner(&bus,"org.freedesktop.systemd1",uid)?;if manager.1!=manager_pid {return Err(ErrorCode::PermissionDenied);}
    let m=proxy(&bus,&manager.0,"/org/freedesktop/systemd1","org.freedesktop.systemd1.Manager")?;
    let path:OwnedObjectPath=m.call("GetUnitByPID",&(pid,)).map_err(dbus)?;
    let unit=proxy(&bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Unit")?;
    let service=proxy(&bus,&manager.0,path.as_str(),"org.freedesktop.systemd1.Service")?;
    let id:String=unit.get_property("Id").map_err(dbus)?;
    if !id.starts_with("aios-extract@") || !id.ends_with(".service") || unit.get_property::<String>("ActiveState").map_err(dbus)?!="active"
        || service.get_property::<u32>("MainPID").map_err(dbus)?!=pid || service.get_property::<String>("Type").map_err(dbus)?!="exec" {return Err(ErrorCode::PermissionDenied);}
    type ExecStart=Vec<(String,Vec<String>,bool,u64,u64,u64,u64,u32,i32,i32)>;
    let exec:ExecStart=service.get_property("ExecStart").map_err(dbus)?;
    if exec.len()!=1 || exec[0].0!=expected || exec[0].1!=[expected,"--socket-worker"] {return Err(ErrorCode::PermissionDenied);}
    // Loader injection would make the nominal ExecStart an insufficient
    // identity. These values must also be removed from manager inheritance.
    let unset:Vec<String>=service.get_property("UnsetEnvironment").map_err(dbus)?;
    for key in ["LD_PRELOAD","LD_LIBRARY_PATH","LD_AUDIT","GLIBC_TUNABLES"] {
        if !unset.iter().any(|v|v==key) {return Err(ErrorCode::PermissionDenied);}
    }
    let invocation:Vec<u8>=unit.get_property("InvocationID").map_err(dbus)?;
    if invocation.len()!=16 || invocation.iter().all(|b|*b==0) || service.get_property::<u64>("ExecMainStartTimestampMonotonic").map_err(dbus)?==0 {return Err(ErrorCode::TargetChanged);}
    for (key,value) in [("MemoryMax",512*1024*1024),("MemorySwapMax",0),("TasksMax",8),("RuntimeMaxUSec",10_000_000),("CPUQuotaPerSecUSec",900_000)] {
        if service.get_property::<u64>(key).map_err(dbus)?!=value {return Err(ErrorCode::PermissionDenied);}
    }
    for key in ["NoNewPrivileges","PrivateNetwork"] {if !service.get_property::<bool>(key).map_err(dbus)? {return Err(ErrorCode::PermissionDenied);}}
    // PrivatePIDs is a string enum on systemd's native bus, even though the
    // unit file accepts a boolean. Unknown modes must fail closed.
    if service.get_property::<String>("PrivatePIDs").map_err(dbus)?!="yes" {return Err(ErrorCode::PermissionDenied);}
    if service.get_property::<String>("KillMode").map_err(dbus)?!="control-group" {return Err(ErrorCode::PermissionDenied);}
    if owner(&system,"org.freedesktop.systemd1",0)?!=root || owner(&bus,"org.freedesktop.systemd1",uid)?!=manager
        || manager_service(&system,&root.0,uid)?!=manager_pid || user_manager_stamp(&system,&root.0,&name)?!=stamp
        || service.get_property::<u32>("MainPID").map_err(dbus)?!=pid || unit.get_property::<Vec<u8>>("InvocationID").map_err(dbus)?!=invocation {return Err(ErrorCode::TargetChanged);}
    Ok(())
}
pub(crate) fn extract(files:Arc<Mutex<Manager>>,owner:Owner,handle:String,request:Request,maximum:usize)->Result<ExtractedDocument> {
    if owner.uid!=nix::unistd::geteuid().as_raw() || owner.uid==0 || maximum==0 || maximum>131072 {return Err(ErrorCode::PermissionDenied);}
    let ticket=files.lock().map_err(exhausted)?.begin_extraction(&owner,&handle,request,aios_policy::boottime_ms()?)?;
    // Never hold the grant lock while waiting for a parser. Revocation has its
    // own admission budget and can invalidate this one-use ticket immediately.
    let mut stream=connect(owner.uid).inspect_err(|_|eprintln!("aios-sessiond: extractor connection gate failed"))?;
    let pid=ready(&stream,owner.uid).inspect_err(|_|eprintln!("aios-sessiond: extractor ready gate failed"))?;
    authenticate(&stream,owner.uid,pid).inspect_err(|_|eprintln!("aios-sessiond: extractor managed identity gate failed"))?;
    let body=serde_json::to_vec(ticket.request()).map_err(changed)?;
    let descriptors=[ticket.input().as_raw_fd()];
    {
        let manager=files.lock().map_err(exhausted)?;
        manager.revalidate_extraction(&owner,&ticket,aios_policy::boottime_ms()?)?;
        if socket::sendmsg::<()>(stream.as_raw_fd(),&[IoSlice::new(&[MARKER])],&[ControlMessage::ScmRights(&descriptors)],MsgFlags::MSG_NOSIGNAL,None).map_err(changed)?!=1 {return Err(ErrorCode::TargetChanged);}
    }
    stream.write_all(&(body.len() as u32).to_be_bytes()).map_err(changed)?;stream.write_all(&body).map_err(changed)?;stream.shutdown(Shutdown::Write).map_err(changed)?;
    stream.set_read_timeout(Some(Duration::from_secs(12))).map_err(changed)?;
    let mut bytes=Vec::new();stream.take(MAX_WIRE+1).read_to_end(&mut bytes).map_err(exhausted)?;
    if bytes.len() as u64>MAX_WIRE {return Err(ErrorCode::ResourceExhausted);}
    let response:Response=serde_json::from_slice(&bytes).map_err(changed)?;
    if response.schema_version!=1 {return Err(ErrorCode::TargetChanged);}
    let data=match (response.data,response.error) {(Some(data),None)=>data,(None,Some(error))=>return Err(error),_=>return Err(ErrorCode::TargetChanged)};
    let mut document=files.lock().map_err(exhausted)?.finish_extraction(&owner,ticket,data,aios_policy::boottime_ms()?)?;
    document.extraction.limit_response(maximum)?;Ok(document)
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]fn ready_byte_requires_kernel_credentials_and_no_injected_descriptors() {
        for scenario in ["valid","wrong-marker","no-credentials","rights"] {
            let (client,mut server)=UnixStream::pair().unwrap();
            if scenario!="no-credentials" {socket::setsockopt(&client,sockopt::PassCred,&true).unwrap();}
            let marker=if scenario=="wrong-marker" {0} else {READY_MARKER};
            if scenario=="rights" {
                let file=std::fs::File::open("/dev/null").unwrap();
                socket::sendmsg::<()>(server.as_raw_fd(),&[IoSlice::new(&[marker])],&[ControlMessage::ScmRights(&[file.as_raw_fd()])],MsgFlags::MSG_NOSIGNAL,None).unwrap();
            } else {server.write_all(&[marker]).unwrap();}
            let result=ready(&client,nix::unistd::geteuid().as_raw());
            if scenario=="valid" {assert_eq!(result.unwrap(),std::process::id());}else{assert_eq!(result,Err(ErrorCode::PermissionDenied));}
        }
    }
}
