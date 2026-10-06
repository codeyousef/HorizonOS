//! Own-user unit identity from the root-managed native user manager.
//! No UID, endpoint, bus owner or method is accepted from a request.
use super::NativeUser;
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{fs, os::{fd::AsRawFd, unix::{fs::{MetadataExt, FileTypeExt}, net::UnixStream}}, path::PathBuf,
    sync::{atomic::{AtomicUsize, Ordering}, mpsc}, time::{Duration, Instant}};
use zbus::{blocking::{Connection, Proxy}, zvariant::OwnedObjectPath};
type Result<T> = std::result::Result<T, ErrorCode>;
mod direct;
// Fixed diagnostic labels and error codes only: never log a unit name, UID,
// endpoint, bus message, property value, environment or authentication payload.
fn diagnostic(stage: &'static str, error: ErrorCode) -> ErrorCode {
    eprintln!("AIOS_JOURNAL_USER_UNIT_FAILURE stage={stage} code={error:?}");
    error
}
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Admission(&'static AtomicUsize);
impl Admission {
    fn acquire(counter: &'static AtomicUsize) -> Result<Self> {
        counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| if value < 4 { Some(value + 1) } else { None })
            .map_err(|_| ErrorCode::ResourceExhausted)?; Ok(Self(counter))
    }
}
impl Drop for Admission { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::AcqRel); } }
fn bounded(user: NativeUser, name: String) -> Result<Identity> {
    let admission = Admission::acquire(&ACTIVE)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new().name("journal-user-unit".into()).spawn(move || {
        let _admission = admission; let _ = sender.send(observe(&user, &name));
    }).map_err(|_| ErrorCode::ResourceExhausted)?;
    // A daemon can stall even during D-Bus authentication, before method
    // timeouts apply. Abandoned work retains its slot until it actually ends.
    receiver.recv_timeout(Duration::from_secs(5)).map_err(|error| match error {
        mpsc::RecvTimeoutError::Timeout => ErrorCode::DeadlineExceeded,
        mpsc::RecvTimeoutError::Disconnected => ErrorCode::PartialResult,
    })?
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Manager { root_owner: String, pid: u32, started_usec: u64, invocation: Vec<u8> }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Endpoint { runtime_inode: u64, manager_directory_inode: u64, device: u64, inode: u64, mode: u32 }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Identity { uid: u32, boot_id: String, name: String, manager: Manager, endpoint: Endpoint, path: String, invocation: Vec<u8> }
/// Opaque native binding. Wire data cannot reconstruct it.
#[derive(Clone, Debug)]
pub struct UserUnit { user: NativeUser, identity: Identity }
fn proxy<'a>(bus: &Connection, owner: &'a str, path: &'a str, interface: &'a str) -> Result<Proxy<'a>> {
    zbus::blocking::proxy::Builder::new(bus).destination(owner).map_err(crate::services::dbus_error)?
        .path(path).map_err(crate::services::dbus_error)?.interface(interface).map_err(crate::services::dbus_error)?
        .cache_properties(zbus::proxy::CacheProperties::No).build().map_err(crate::services::dbus_error)
}
fn manager(uid: u32) -> Result<Manager> {
    let bus = crate::services::system_connection()?;
    let owner = crate::services::root_owner(&bus, "org.freedesktop.systemd1")?;
    let native = proxy(&bus, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")?;
    if native.call::<_, _, u32>("GetConnectionUnixProcessID", &(owner.as_str(),)).map_err(crate::services::dbus_error)? != 1 { return Err(ErrorCode::PermissionDenied); }
    let m = proxy(&bus, &owner, "/org/freedesktop/systemd1", "org.freedesktop.systemd1.Manager")?;
    let name = format!("user@{uid}.service");
    let path: OwnedObjectPath = m.call("GetUnit", &(name.as_str(),)).map_err(crate::services::dbus_error)?;
    let unit = proxy(&bus, &owner, path.as_str(), "org.freedesktop.systemd1.Unit")?;
    let service = proxy(&bus, &owner, path.as_str(), "org.freedesktop.systemd1.Service")?;
    let id: String = unit.get_property("Id").map_err(crate::services::dbus_error)?;
    let active: String = unit.get_property("ActiveState").map_err(crate::services::dbus_error)?;
    let group: String = service.get_property("ControlGroup").map_err(crate::services::dbus_error)?;
    let pid: u32 = service.get_property("MainPID").map_err(crate::services::dbus_error)?;
    let started_usec: u64 = service.get_property("ExecMainStartTimestampMonotonic").map_err(crate::services::dbus_error)?;
    let invocation: Vec<u8> = unit.get_property("InvocationID").map_err(crate::services::dbus_error)?;
    type Commands = Vec<(String, Vec<String>, bool, u64, u64, u64, u64, u32, i32, i32)>;
    let commands: Commands = service.get_property("ExecStart").map_err(crate::services::dbus_error)?;
    let expected = option_env!("AIOS_USER_MANAGER").ok_or(ErrorCode::UnsupportedCapability)?;
    if id != name || active != "active" || pid <= 1 || started_usec == 0 || invocation.len() != 16 || invocation.iter().all(|v| *v == 0)
        || group != format!("/user.slice/user-{uid}.slice/user@{uid}.service")
        || commands.len() != 1 || commands[0].0 != expected || !commands[0].1.iter().any(|v| v == "--user") || commands[0].2 {
        return Err(ErrorCode::PermissionDenied);
    }
    if crate::services::root_owner(&bus, "org.freedesktop.systemd1")? != owner { return Err(ErrorCode::TargetChanged); }
    Ok(Manager { root_owner: owner.clone(), pid, started_usec, invocation })
}
fn endpoint(uid: u32) -> Result<Endpoint> {
    let directory = PathBuf::from(format!("/run/user/{uid}"));
    let runtime = fs::symlink_metadata(&directory).map_err(|_| ErrorCode::TargetNotFound)?;
    if !runtime.is_dir() || runtime.uid() != uid || runtime.mode() & 0o077 != 0 || directory.canonicalize().map_err(|_| ErrorCode::TargetChanged)? != directory {
        return Err(ErrorCode::PermissionDenied);
    }
    let manager_directory = directory.join("systemd");
    let native = fs::symlink_metadata(&manager_directory).map_err(|_| ErrorCode::TargetNotFound)?;
    if !native.is_dir() || native.uid() != uid || native.mode() & 0o022 != 0
        || manager_directory.canonicalize().map_err(|_| ErrorCode::TargetChanged)? != manager_directory {
        return Err(ErrorCode::PermissionDenied);
    }
    let bus = fs::symlink_metadata(manager_directory.join("private")).map_err(|_| ErrorCode::TargetNotFound)?;
    if !bus.file_type().is_socket() || bus.uid() != uid { return Err(ErrorCode::PermissionDenied); }
    Ok(Endpoint { runtime_inode: runtime.ino(), manager_directory_inode: native.ino(), device: bus.dev(), inode: bus.ino(), mode: bus.mode() })
}
fn peer(stream: &UnixStream, uid: u32, pid: u32) -> Result<()> {
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe { libc::getsockopt(stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, credentials.as_mut_ptr().cast(), &mut size) } != 0
        || size as usize != std::mem::size_of::<libc::ucred>() { return Err(ErrorCode::PermissionDenied); }
    let credentials = unsafe { credentials.assume_init() };
    if credentials.uid != uid || u32::try_from(credentials.pid).ok() != Some(pid) { return Err(ErrorCode::PermissionDenied); }
    let mut poll = libc::pollfd { fd: stream.as_raw_fd(), events: libc::POLLRDHUP, revents: 0 };
    if unsafe { libc::poll(&mut poll, 1, 0) } < 0 || poll.revents & (libc::POLLRDHUP | libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 { return Err(ErrorCode::TargetChanged); }
    Ok(())
}
fn connect(uid: u32, manager_pid: u32) -> Result<(UnixStream, direct::Bus)> {
    // This is systemd's native point-to-point manager endpoint, not a session
    // application bus. Its actual kernel peer must be the root-managed PID.
    // No alternate endpoint, claimed auth UID, fallback or mutation RPC exists.
    let proof = UnixStream::connect(format!("/run/user/{uid}/systemd/private")).map_err(|error| {
        let code = match error.kind() {
            std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
            std::io::ErrorKind::NotFound => ErrorCode::TargetNotFound,
            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionReset => ErrorCode::TargetChanged,
            _ => ErrorCode::PartialResult,
        };
        diagnostic("runtime-socket-connect", code)
    })?;
    peer(&proof, uid, manager_pid)?;
    proof.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_| ErrorCode::TargetChanged)?;
    proof.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_| ErrorCode::TargetChanged)?;
    let bus = direct::Bus::connect(proof.try_clone().map_err(|_| ErrorCode::TargetChanged)?)
        .map_err(|error| diagnostic("direct-manager-connection", error))?;
    peer(&proof, uid, manager_pid)?;
    Ok((proof, bus))
}
fn observe(user: &NativeUser, name: &str) -> Result<Identity> {
    crate::services::validate_service_name(name)?; user.verify()?;
    let started = Instant::now(); let uid = user.uid();
    let before = manager(uid).map_err(|error| diagnostic("root-managed-identity", error))?;
    let socket = endpoint(uid).map_err(|error| diagnostic("runtime-endpoint", error))?;
    let (proof, bus) = connect(uid, before.pid)?;
    let observed = bus.unit(name).map_err(|error| diagnostic("direct-unit-identity", error))?;
    if observed.id != name || observed.load != "loaded" || observed.invocation.len() != 16 { return Err(ErrorCode::TargetChanged); }
    if manager(uid)? != before || endpoint(uid)? != socket { return Err(ErrorCode::TargetChanged); }
    peer(&proof, uid, before.pid)?; user.verify()?;
    if bus.unit(name)? != observed {
        return Err(ErrorCode::TargetChanged);
    }
    if started.elapsed() >= Duration::from_secs(5) { return Err(ErrorCode::DeadlineExceeded); }
    Ok(Identity { uid, boot_id: user.boot.clone(), name: name.into(), manager: before, endpoint: socket, path: observed.path, invocation: observed.invocation })
}
impl NativeUser {
    pub fn resolve_unit(&self, name: &str) -> Result<UserUnit> {
        crate::services::validate_service_name(name)?;
        Ok(UserUnit { user: self.clone(), identity: bounded(self.clone(), name.into())? })
    }
}
impl UserUnit {
    pub fn name(&self) -> &str { &self.identity.name }
    pub fn identity(&self) -> &Identity { &self.identity }
    pub fn verify(&self) -> Result<()> {
        if bounded(self.user.clone(), self.name().into())? != self.identity { return Err(ErrorCode::TargetChanged); } Ok(())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a verified NixOS guest with the current normal user's native user manager"]
    fn native_own_user_manager_transport() {
        assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|line| line == "ID=nixos"));
        let uid = unsafe { libc::geteuid() }; assert!(uid >= 1000);
        let manager = manager(uid).expect("root-managed own-user identity");
        let endpoint_before = endpoint(uid).expect("own-user runtime endpoint");
        let (socket, bus) = connect(uid, manager.pid).expect("native direct-manager authentication");
        let unit = bus.unit("aios-sessiond.service").unwrap();
        assert_eq!(unit.id, "aios-sessiond.service");
        assert_eq!(unit.load, "loaded"); assert_eq!(unit.invocation.len(), 16);
        assert!(matches!(bus.unit("../sshd.service"), Err(ErrorCode::InvalidArgument)));
        assert!(matches!(bus.unit("horizon-missing-unit-11111111111111111111111111111111.service"), Err(ErrorCode::TargetNotFound)));
        assert_eq!(bus.unit("aios-sessiond.service").unwrap(), unit);
        assert_eq!(endpoint(uid).unwrap(), endpoint_before);
        peer(&socket, uid, manager.pid).unwrap();
        println!("AIOS_NATIVE_USER_MANAGER_TRANSPORT={{\"uid\":{uid},\"manager_pid\":{},\"kernel_peer_and_direct_manager_verified\":true,\"root_caller_authentication_verified\":false}}", manager.pid);
    }
    #[test] fn abandoned_observations_keep_bounded_admission_until_native_work_ends() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let workers = (0..4).map(|_| Admission::acquire(&COUNTER).unwrap()).collect::<Vec<_>>();
        assert!(matches!(Admission::acquire(&COUNTER), Err(ErrorCode::ResourceExhausted)));
        let (sender, receiver) = mpsc::sync_channel::<()>(1);
        assert!(matches!(receiver.recv_timeout(Duration::from_millis(1)), Err(mpsc::RecvTimeoutError::Timeout)));
        assert_eq!(COUNTER.load(Ordering::Acquire), 4);
        drop(receiver); assert!(sender.send(()).is_err());
        assert_eq!(COUNTER.load(Ordering::Acquire), 4);
        drop(workers); assert_eq!(COUNTER.load(Ordering::Acquire), 0);
        assert!(Admission::acquire(&COUNTER).is_ok());
    }
}
