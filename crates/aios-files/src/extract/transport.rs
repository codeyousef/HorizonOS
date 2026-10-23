//! Private worker transport. File descriptors are authority supplied by the
//! caller, never reconstructed from request paths or serialized identities.
use super::{Extraction, Request, PdfTools, MAX_WIRE, extract};
use aios_protocol::contracts::ErrorCode;
use nix::sys::socket::{recvmsg,ControlMessageOwned,MsgFlags,getsockopt,sockopt::PeerCredentials};
use serde::{Deserialize,Serialize};
use std::{fs::File,io::{Read,Write,IoSliceMut},os::{fd::{AsRawFd,FromRawFd,OwnedFd,RawFd},unix::net::UnixStream},time::Duration};
pub const MARKER:u8=0xe1;
/// Sent by the sandboxed process before the caller transfers any authority.
pub const READY_MARKER:u8=0xe2;
const MAX_REQUEST:usize=4096;
#[derive(Debug,Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Response {pub schema_version:u32,pub data:Option<Extraction>,pub error:Option<ErrorCode>}

fn receive(stream:&mut UnixStream)->Result<(File,Request),ErrorCode> {
    let peer=getsockopt(stream,PeerCredentials).map_err(|_|ErrorCode::PermissionDenied)?;
    if peer.uid()!=nix::unistd::geteuid().as_raw() {return Err(ErrorCode::PermissionDenied);}
    stream.set_read_timeout(Some(Duration::from_secs(8))).map_err(|_|ErrorCode::ResourceExhausted)?;
    let mut byte=[0u8];let mut iov=[IoSliceMut::new(&mut byte)];
    // Own and close all delivered descriptors, including invalid multiplicity.
    let mut ancillary=nix::cmsg_space!([RawFd;253]);
    let m=recvmsg::<()>(stream.as_raw_fd(),&mut iov,Some(&mut ancillary),MsgFlags::MSG_CMSG_CLOEXEC).map_err(|_|ErrorCode::InvalidArgument)?;
    let mut fds=Vec::<OwnedFd>::new();let mut other=false;
    for c in m.cmsgs().map_err(|_|ErrorCode::InvalidArgument)? {
        match c {ControlMessageOwned::ScmRights(raw)=>for fd in raw {fds.push(unsafe{OwnedFd::from_raw_fd(fd)});},_=>other=true}
    }
    let valid=m.bytes==1 && !other && !m.flags.intersects(MsgFlags::MSG_CTRUNC|MsgFlags::MSG_TRUNC);
    if !valid || byte[0]!=MARKER || fds.len()!=1 {return Err(ErrorCode::InvalidArgument);}
    let mut length=[0u8;4];stream.read_exact(&mut length).map_err(|_|ErrorCode::InvalidArgument)?;
    let length=u32::from_be_bytes(length) as usize;
    if length==0 || length>MAX_REQUEST {return Err(ErrorCode::InvalidArgument);}
    let mut data=vec![0;length];stream.read_exact(&mut data).map_err(|_|ErrorCode::InvalidArgument)?;
    let mut trailing=[0u8];if stream.read(&mut trailing).map_err(|_|ErrorCode::InvalidArgument)?!=0 {return Err(ErrorCode::InvalidArgument);}
    let request:Request=serde_json::from_slice(&data).map_err(|_|ErrorCode::InvalidArgument)?;request.validate()?;
    Ok((File::from(fds.remove(0)),request))
}

fn require_profile()->std::io::Result<()> {
    if nix::unistd::geteuid().is_root() {return Err(std::io::Error::other("extractor must be unprivileged"));}
    if unsafe {nix::libc::prctl(nix::libc::PR_GET_NO_NEW_PRIVS,0,0,0,0)}!=1 {return Err(std::io::Error::other("extractor privilege restriction absent"));}
    #[repr(C)]struct Header {version:u32,pid:i32}
    #[repr(C)]#[derive(Clone,Copy)]struct Caps {effective:u32,permitted:u32,inheritable:u32}
    let h=Header {version:0x20080522,pid:0};let caps=[Caps {effective:0,permitted:0,inheritable:0};2];
    if unsafe {nix::libc::syscall(nix::libc::SYS_capset,&h as *const Header,caps.as_ptr())}!=0 {return Err(std::io::Error::other("extractor capability drop failed"));}
    // A setting silently ignored by an older manager/kernel is not a sandbox.
    // Check the enforced PID/filesystem/network view before reading any input.
    if std::process::id()!=1 || std::fs::File::open("/home").is_ok() || std::fs::File::open("/run").is_ok() {
        return Err(std::io::Error::other("extractor namespace restriction absent"));
    }
    for family in [nix::libc::AF_INET,nix::libc::AF_INET6] {
        let fd=unsafe {nix::libc::socket(family,nix::libc::SOCK_STREAM|nix::libc::SOCK_CLOEXEC,0)};
        if fd>=0 {unsafe {nix::libc::close(fd)};return Err(std::io::Error::other("extractor network restriction absent"));}
        if !matches!(std::io::Error::last_os_error().raw_os_error(),Some(nix::libc::EAFNOSUPPORT|nix::libc::EPERM|nix::libc::EACCES)) {
            return Err(std::io::Error::other("extractor network restriction unverified"));
        }
    }
    if unsafe {nix::libc::prctl(nix::libc::PR_SET_DUMPABLE,0,0,0,0)}!=0 || unsafe {nix::libc::prctl(nix::libc::PR_GET_DUMPABLE,0,0,0,0)}!=0 {return Err(std::io::Error::other("extractor parent custody restriction absent"));}
    Ok(())
}

pub fn serve_once(mut stream:UnixStream)->std::io::Result<()> {
    require_profile()?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(&[READY_MARKER])?;
    serve_response(stream)
}

fn serve_response(mut stream:UnixStream)->std::io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let result=receive(&mut stream).and_then(|(file,request)|extract(&file,&request,PdfTools::compiled().as_ref()));
    let reply=match result {Ok(data)=>Response {schema_version:1,data:Some(data),error:None},Err(error)=>Response {schema_version:1,data:None,error:Some(error)}};
    let bytes=serde_json::to_vec(&reply)?;
    if bytes.len() as u64>MAX_WIRE {return Err(std::io::Error::other("extractor response bound exceeded"));}
    stream.write_all(&bytes)?;Ok(())
}

#[cfg(test)]mod tests {
    use super::*;
    use nix::sys::socket::{sendmsg,ControlMessage};
    use std::{io::IoSlice,net::Shutdown};
    fn send(stream:&mut UnixStream,fds:&[RawFd],request:&[u8]) {
        sendmsg::<()>(stream.as_raw_fd(),&[IoSlice::new(&[MARKER])],&[ControlMessage::ScmRights(fds)],MsgFlags::MSG_NOSIGNAL,None).unwrap();
        stream.write_all(&(request.len() as u32).to_be_bytes()).unwrap();stream.write_all(request).unwrap();stream.shutdown(Shutdown::Write).unwrap();
    }
    #[test]fn descriptor_and_typed_request_round_trip_without_a_path() {
        let mut input=tempfile::NamedTempFile::new().unwrap();input.write_all(b"example\n").unwrap();let input=File::open(input.path()).unwrap();
        let(mut client,server)=UnixStream::pair().unwrap();let work=std::thread::spawn(||serve_response(server).unwrap());
        send(&mut client,&[input.as_raw_fd()],br#"{"schema_version":1,"format":"text","range":null}"#);
        let mut out=String::new();client.read_to_string(&mut out).unwrap();work.join().unwrap();let reply:Response=serde_json::from_str(&out).unwrap();
        assert!(reply.error.is_none());assert_eq!(reply.data.unwrap().chunks[0].text,"example\n");
    }
    #[test]fn extra_descriptors_and_claimed_path_are_refused() {
        let input=File::open("/dev/null").unwrap();
        for (fds,data) in [(vec![input.as_raw_fd();2],br#"{"schema_version":1,"format":"text","range":null}"#.as_slice()),(vec![input.as_raw_fd()],br#"{"schema_version":1,"format":"text","range":null,"path":"/etc/passwd"}"#.as_slice())] {
            let(mut client,server)=UnixStream::pair().unwrap();let work=std::thread::spawn(||serve_response(server));
            send(&mut client,&fds,data);let mut out=String::new();let _=client.read_to_string(&mut out);let _=work.join().unwrap();
            if !out.is_empty(){let reply:Response=serde_json::from_str(&out).unwrap();assert!(reply.data.is_none());assert_eq!(reply.error,Some(ErrorCode::InvalidArgument));}
        }
    }
}
