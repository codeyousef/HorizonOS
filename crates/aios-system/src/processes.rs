//! Own-user process observations. Handles retain a native pidfd and proc
//! directory; no command line, environment, process name or signal API.
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{fs::{File,OpenOptions},io::Read,os::{fd::{AsRawFd,FromRawFd,OwnedFd},unix::fs::{MetadataExt,OpenOptionsExt}},path::Path};
type Result<T> = std::result::Result<T,ErrorCode>;
const MAX_BYTES:u64=65536;
const MAX_SAFE:u64=9_007_199_254_740_991;

fn error(e:std::io::Error)->ErrorCode {
    match e.raw_os_error() {
        Some(libc::ENOENT|libc::ESRCH)=>ErrorCode::TargetNotFound,
        Some(libc::EACCES|libc::EPERM)=>ErrorCode::PermissionDenied,
        Some(libc::EMFILE|libc::ENFILE|libc::ENOMEM)=>ErrorCode::ResourceExhausted,
        Some(libc::ENOSYS)=>ErrorCode::UnsupportedCapability,
        _=>ErrorCode::PartialResult,
    }
}
fn boot()->Result<String> {
    let value=crate::bounded(Path::new("/proc/sys/kernel/random/boot_id"),64)?.trim().to_owned();
    crate::boot_id(&value)
}
fn open_field(directory:&File,name:&std::ffi::CStr,flags:i32)->Result<File> {
    let fd=unsafe{libc::openat(directory.as_raw_fd(),name.as_ptr(),flags|libc::O_CLOEXEC)};
    if fd<0{return Err(error(std::io::Error::last_os_error()));}
    Ok(unsafe{File::from_raw_fd(fd)})
}
fn text(directory:&File,name:&std::ffi::CStr)->Result<String> {
    let file=open_field(directory,name,libc::O_RDONLY|libc::O_NOFOLLOW)?;
    let mut value=String::new();file.take(MAX_BYTES+1).read_to_string(&mut value).map_err(error)?;
    if value.len() as u64>MAX_BYTES{return Err(ErrorCode::ResourceExhausted);}Ok(value)
}
fn owner(status:&str,uid:u32)->Result<()> {
    let lines:Vec<_>=status.lines().filter_map(|l|l.strip_prefix("Uid:")).collect();
    if lines.len()!=1 {return Err(ErrorCode::PartialResult);}
    let ids:Vec<u32>=lines[0].split_whitespace().map(|x|x.parse().map_err(|_|ErrorCode::PartialResult)).collect::<Result<_>>()?;
    if ids.len()!=4 {return Err(ErrorCode::PartialResult);}
    if ids.iter().any(|&id|id!=uid){return Err(ErrorCode::PermissionDenied);}Ok(())
}
#[derive(Debug,Clone,PartialEq,Eq,Serialize)]
pub struct Identity { pub pid:u32,pub uid:u32,pub start_time_ticks:u64,pub boot_id:String,pub executable_identity:String }
#[derive(Debug,Serialize)]
pub struct Metrics { pub cpu_time_ms:u64,pub rss_bytes:u64,pub threads:u32 }
#[derive(Debug,Serialize)]
pub struct Observation { pub identity:Identity,pub metrics:Metrics }
struct Stat { start:u64,user:u64,system:u64,rss:u64,threads:u32 }
fn stat(value:&str,pid:u32)->Result<Stat> {
    let (prefix,tail)=value.rsplit_once(')').ok_or(ErrorCode::PartialResult)?;
    if prefix.split_once(" (").and_then(|(s,_)|s.parse::<u32>().ok())!=Some(pid){return Err(ErrorCode::TargetChanged);}
    let fields:Vec<_>=tail.split_whitespace().collect();
    if fields.len()<22 || fields[0].len()!=1{return Err(ErrorCode::PartialResult);}
    if matches!(fields[0],"Z"|"X"|"x"){return Err(ErrorCode::TargetNotFound);}
    let number=|i:usize|fields[i].parse::<u64>().map_err(|_|ErrorCode::PartialResult);
    let start=number(19)?;
    if start==0 || start>MAX_SAFE{return Err(ErrorCode::PartialResult);}
    Ok(Stat{start,user:number(11)?,system:number(12)?,rss:number(21)?,threads:u32::try_from(number(17)?).map_err(|_|ErrorCode::PartialResult)?})
}
fn executable(directory:&File)->Result<String> {
    // Only the kernel-owned /proc/PID/exe magic link is followed, through the
    // retained proc directory. Never a caller-selected path or symlink.
    let exe=open_field(directory,c"exe",libc::O_PATH)?;
    let m=exe.metadata().map_err(error)?;
    if !m.is_file(){return Err(ErrorCode::TargetChanged);}
    Ok(format!("dev={};ino={};size={};mtime={}:{};ctime={}:{}",m.dev(),m.ino(),m.len(),m.mtime(),m.mtime_nsec(),m.ctime(),m.ctime_nsec()))
}
fn observe(directory:&File,pid:u32,uid:u32)->Result<Observation> {
    if directory.metadata().map_err(error)?.uid()!=uid{return Err(ErrorCode::PermissionDenied);}
    owner(&text(directory,c"status")?,uid)?;
    let s=stat(&text(directory,c"stat")?,pid)?;
    let executable_identity=executable(directory)?;
    let ticks=unsafe{libc::sysconf(libc::_SC_CLK_TCK)};let page=unsafe{libc::sysconf(libc::_SC_PAGESIZE)};
    if ticks<=0 || page<=0{return Err(ErrorCode::UnsupportedCapability);}
    let cpu_time_ms=s.user.checked_add(s.system).and_then(|v|v.checked_mul(1000)).ok_or(ErrorCode::ResourceExhausted)?/(ticks as u64);
    let rss_bytes=s.rss.checked_mul(page as u64).ok_or(ErrorCode::ResourceExhausted)?;
    if cpu_time_ms>MAX_SAFE || rss_bytes>MAX_SAFE{return Err(ErrorCode::ResourceExhausted);}
    Ok(Observation{identity:Identity{pid,uid,start_time_ticks:s.start,boot_id:boot()?,executable_identity},metrics:Metrics{cpu_time_ms,rss_bytes,threads:s.threads}})
}

/// Private native identity, never deserialized or constructed from a claimed
/// UID, boot or start time. The session broker separately owns grants/expiry.
pub struct OwnProcess { directory:File,pidfd:OwnedFd,identity:Identity }
/// A bounded native snapshot. Never return a truncated inventory as complete.
/// Processes that disappear during enumeration are absent from the snapshot;
/// unreadable own-user entries make the inventory explicitly incomplete.
pub struct Inventory { pub processes: Vec<OwnProcess>, pub access_denied: bool }
pub fn inventory() -> Result<Inventory> {
    let uid = unsafe { libc::geteuid() };
    if uid == 0 { return Err(ErrorCode::PermissionDenied); }
    let started = std::time::Instant::now();
    let mut result = Vec::new();
    let mut access_denied = false;
    for (index, entry) in std::fs::read_dir("/proc").map_err(error)?.enumerate() {
        if index >= 32768 { return Err(ErrorCode::ResourceExhausted); }
        if started.elapsed() >= std::time::Duration::from_secs(2) { return Err(ErrorCode::DeadlineExceeded); }
        let entry = entry.map_err(error)?;
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue; };
        let metadata = match std::fs::symlink_metadata(entry.path()) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(error(e)),
        };
        if pid <= 1 || metadata.uid() != uid { continue; }
        match OwnProcess::open(pid) {
            Ok(value) => result.push(value),
            Err(ErrorCode::TargetNotFound) => continue,
            Err(ErrorCode::PermissionDenied) => access_denied = true,
            Err(code) => return Err(code),
        }
        if result.len() > 256 { return Err(ErrorCode::ResourceExhausted); }
    }
    result.sort_by_key(|p| p.identity.pid);
    Ok(Inventory { processes: result, access_denied })
}
impl OwnProcess {
    pub fn open(pid:u32)->Result<Self> {
        let uid=unsafe{libc::geteuid()};
        if uid==0 || pid<=1 || pid>i32::MAX as u32{return Err(ErrorCode::PermissionDenied);}
        let directory=OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY|libc::O_NOFOLLOW|libc::O_CLOEXEC)
            .open(format!("/proc/{pid}")).map_err(error)?;
        let mut filesystem=std::mem::MaybeUninit::<libc::statfs>::uninit();
        if unsafe{libc::fstatfs(directory.as_raw_fd(),filesystem.as_mut_ptr())}!=0{return Err(error(std::io::Error::last_os_error()));}
        if unsafe{filesystem.assume_init()}.f_type!=libc::PROC_SUPER_MAGIC{return Err(ErrorCode::PermissionDenied);}
        let before=observe(&directory,pid,uid)?;
        let fd=unsafe{libc::syscall(libc::SYS_pidfd_open,pid as libc::pid_t,0u32)};
        if fd<0{return Err(error(std::io::Error::last_os_error()));}
        let pidfd=unsafe{OwnedFd::from_raw_fd(i32::try_from(fd).map_err(|_|ErrorCode::ResourceExhausted)?)};
        let value=Self{directory,pidfd,identity:before.identity};value.inspect()?;Ok(value)
    }
    pub fn exited(&self)->Result<bool> {
        if unsafe{libc::geteuid()}!=self.identity.uid || boot()!=Ok(self.identity.boot_id.clone()){return Err(ErrorCode::TargetChanged);}
        let mut p=libc::pollfd{fd:self.pidfd.as_raw_fd(),events:libc::POLLIN,revents:0};
        if unsafe{libc::poll(&mut p,1,0)}<0{return Err(error(std::io::Error::last_os_error()));}
        if p.revents&(libc::POLLERR|libc::POLLNVAL)!=0{return Err(ErrorCode::TargetChanged);}
        Ok(p.revents&(libc::POLLIN|libc::POLLHUP)!=0)
    }
    pub fn inspect(&self)->Result<Observation> {
        if self.exited()?{return Err(ErrorCode::TargetNotFound);}
        let first=observe(&self.directory,self.identity.pid,self.identity.uid)?;
        if first.identity!=self.identity{return Err(ErrorCode::TargetChanged);}
        let second=observe(&self.directory,self.identity.pid,self.identity.uid)?;
        if second.identity!=self.identity{return Err(ErrorCode::TargetChanged);}
        if self.exited()?{return Err(ErrorCode::TargetNotFound);}Ok(second)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixed_privileged_credentials_are_refused() {
        assert_eq!(owner("Uid:\t1000 0 0 1000\n",1000),Err(ErrorCode::PermissionDenied));
        assert_eq!(owner("Uid: 1000 1000 1000 1000\nUid: 1000 1000 1000 1000",1000),Err(ErrorCode::PartialResult));
        assert_eq!(owner("Uid: 1000 1000 1000",1000),Err(ErrorCode::PartialResult));
    }
    #[test]
    fn native_identity_survives_only_the_original_process() {
        // Actual guest process and pidfd, not fixture identity. No signal sent.
        assert!(unsafe{libc::geteuid()}!=0);
        let current=OwnProcess::open(std::process::id()).unwrap();let observed=current.inspect().unwrap();
        assert_eq!(observed.identity.pid,std::process::id());assert!(observed.identity.start_time_ticks>0);
        assert!(observed.metrics.threads>0);assert!(observed.identity.executable_identity.starts_with("dev="));
        assert!(matches!(OwnProcess::open(1),Err(ErrorCode::PermissionDenied)));
        let mut child=std::process::Command::new("/run/current-system/sw/bin/sleep").arg("1").spawn().unwrap();
        let retained=OwnProcess::open(child.id()).unwrap();assert!(!retained.exited().unwrap());
        child.wait().unwrap();assert!(retained.exited().unwrap());assert!(matches!(retained.inspect(),Err(ErrorCode::TargetNotFound)));
        let mut changed=OwnProcess::open(std::process::id()).unwrap();changed.identity.start_time_ticks+=1;
        assert!(matches!(changed.inspect(),Err(ErrorCode::TargetChanged)));
    }
}
