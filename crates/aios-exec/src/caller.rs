//! Native system-bus callers. Identity observations never confer intent/approval.
use crate::{Error, Result, native::VerifiedTarget};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    time::Duration,
};
use zbus::{
    blocking::{Connection, Proxy},
    message::Header,
    zvariant::OwnedObjectPath,
};

const ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const LOGIN: &str = "org.freedesktop.login1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionIdentity {
    pub id: String,
    pub remote: bool,
    pub kind: String,
    pub class: String,
    pub state: String,
    pub active: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallerIdentity {
    pub uid: u32,
    pub pid: u32,
    pub start_ticks: u64,
    pub boot_id: String,
    pub bus_id: String,
    pub sender: String,
    pub session: Option<SessionIdentity>,
}
impl CallerIdentity {
    /// This is availability only. It neither selects a desktop nor grants UI access.
    pub fn local_desktop_available(&self) -> bool {
        self.session.as_ref().is_some_and(|s| {
            !s.remote
                && s.active
                && s.class == "user"
                && s.state == "active"
                && matches!(s.kind.as_str(), "x11" | "wayland")
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceIdentity {
    pub(crate) sender: String,
    pub(crate) uid: u32,
    pub(crate) pid: u32,
    pub(crate) start_ticks: u64,
    pub(crate) boot_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    caller: CallerIdentity,
    logind: ServiceIdentity,
}

/// No serde, public constructor, root override, or authority derived from JSON.
/// This capability is bound to the broker connection which authenticated it.
#[derive(Clone)]
pub struct VerifiedCaller {
    snapshot: Snapshot,
    broker_epoch: uuid::Uuid,
}
impl VerifiedCaller {
    pub fn identity(&self) -> &CallerIdentity {
        &self.snapshot.caller
    }
}

/// Fixed native endpoint, created only after full installed target verification.
/// The zbus method adapter passes its injected header, never request data.
pub struct SystemBus {
    connection: Connection,
    target: VerifiedTarget,
    epoch: uuid::Uuid,
}
impl SystemBus {
    pub(crate) fn connection(&self) -> Connection {
        self.connection.clone()
    }
    pub(crate) fn target(&self) -> &VerifiedTarget {
        &self.target
    }
    pub(crate) fn polkit_connection(&self) -> Result<Connection> {
        self.target.recheck()?;
        let connection = connect_native_timeout(Duration::from_secs(120))?;
        if bus_id(&bus(&connection)?)? != bus_id(&bus(&self.connection)?)? {
            return Err(Error::TargetChanged);
        }
        self.target.recheck()?;
        Ok(connection)
    }
    pub(crate) fn polkit_owner(
        &self,
        authority: &crate::approval::policy::Authority,
    ) -> Result<ServiceIdentity> {
        self.target.recheck()?;
        start_polkit(&self.connection)?;
        let observed = observe_service(
            &self.connection,
            "org.freedesktop.PolicyKit1",
            authority.polkit_uid,
        )?;
        let actual =
            fs::read_link(format!("/proc/{}/exe", observed.pid)).map_err(|_| Error::Authority)?;
        if actual.to_str() != Some(authority.daemon().as_str())
            || observed.boot_id != self.target.target().boot_id
            || observe_service(
                &self.connection,
                "org.freedesktop.PolicyKit1",
                authority.polkit_uid,
            )? != observed
        {
            return Err(Error::TargetChanged);
        }
        self.target.recheck()?;
        Ok(observed)
    }
    pub fn connect() -> Result<Self> {
        let target = VerifiedTarget::enroll()?;
        let connection = connect_native()?;
        target.recheck()?;
        Ok(Self {
            connection,
            target,
            epoch: uuid::Uuid::new_v4(),
        })
    }
    pub fn authenticate(&self, header: &Header<'_>) -> Result<VerifiedCaller> {
        if header.message_type() != zbus::message::Type::MethodCall {
            return Err(Error::Authority);
        }
        let sender = header.sender().ok_or(Error::Authority)?.as_str();
        self.authenticate_sender(sender)
    }
    /// Only the native method adapter may pass its injected header's sender.
    pub(crate) fn authenticate_sender(&self, sender: &str) -> Result<VerifiedCaller> {
        self.target.recheck()?;
        let snapshot = capture(&self.connection, sender)?;
        requesting_user(&snapshot.caller)?;
        if snapshot.caller.boot_id != self.target.target().boot_id {
            return Err(Error::TargetChanged);
        }
        self.target.recheck()?;
        Ok(VerifiedCaller {
            snapshot,
            broker_epoch: self.epoch,
        })
    }
    pub fn recheck(&self, caller: &VerifiedCaller) -> Result<()> {
        self.target.recheck()?;
        if caller.broker_epoch != self.epoch {
            return Err(Error::TargetChanged);
        }
        compare(
            &caller.snapshot,
            &capture(&self.connection, &caller.identity().sender)?,
        )?;
        requesting_user(caller.identity())?;
        self.target.recheck()
    }
}

fn connect_native() -> Result<Connection> {
    connect_native_timeout(Duration::from_secs(2))
}
#[cfg(test)]
pub(crate) fn read_only_test_connection() -> Result<Connection> {
    connect_native()
}
pub(crate) fn connect_native_timeout(timeout: Duration) -> Result<Connection> {
    // Do not honor DBUS_SYSTEM_BUS_ADDRESS or connect to a caller-supplied bus.
    for path in ["/", "/run", "/run/dbus"] {
        let m = fs::symlink_metadata(path).map_err(|_| Error::Authority)?;
        if !m.is_dir() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(Error::Ownership);
        }
    }
    let m = fs::symlink_metadata("/run/dbus/system_bus_socket").map_err(|_| Error::Authority)?;
    if !m.file_type().is_socket() || m.uid() != 0 {
        return Err(Error::Ownership);
    }
    zbus::blocking::connection::Builder::address(ADDRESS)
        .map_err(|_| Error::Authority)?
        .method_timeout(timeout)
        .max_queued(32)
        .build()
        .map_err(|_| Error::Authority)
}
fn bus(connection: &Connection) -> Result<Proxy<'_>> {
    Proxy::new(
        connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .map_err(|_| Error::Authority)
}
fn credentials(bus: &Proxy<'_>, sender: &str) -> Result<(u32, u32)> {
    if !unique(sender) {
        return Err(Error::Authority);
    }
    let c: zbus::fdo::ConnectionCredentials = bus
        .call("GetConnectionCredentials", &(sender,))
        .map_err(|_| Error::TargetChanged)?;
    let uid = c.unix_user_id().ok_or(Error::Authority)?;
    let pid = c.process_id().ok_or(Error::Authority)?;
    if pid <= 1 {
        return Err(Error::Authority);
    }
    Ok((uid, pid))
}
fn unique(sender: &str) -> bool {
    let Some(rest) = sender.strip_prefix(':') else {
        return false;
    };
    !rest.is_empty()
        && sender.len() <= 128
        && rest.split('.').count() >= 2
        && rest.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        })
}
fn bus_id(bus: &Proxy<'_>) -> Result<String> {
    let id: String = bus.call("GetId", &()).map_err(|_| Error::TargetChanged)?;
    if id.len() != 32
        || !id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(Error::Authority);
    }
    Ok(id)
}
fn service(bus: &Proxy<'_>) -> Result<ServiceIdentity> {
    let sender: String = bus
        .call("GetNameOwner", &(LOGIN,))
        .map_err(|_| Error::Authority)?;
    let (uid, pid) = credentials(bus, &sender)?;
    if uid != 0 {
        return Err(Error::Authority);
    }
    let (start_ticks, boot_id) = process(pid, uid)?;
    Ok(ServiceIdentity {
        sender,
        uid,
        pid,
        start_ticks,
        boot_id,
    })
}
pub(crate) fn observe_service(
    connection: &Connection,
    name: &str,
    uid: u32,
) -> Result<ServiceIdentity> {
    if !matches!(
        name,
        "org.freedesktop.PolicyKit1" | "org.freedesktop.login1"
    ) {
        return Err(Error::Authority);
    }
    let bus = bus(connection)?;
    let sender: String = bus
        .call("GetNameOwner", &(name,))
        .map_err(|_| Error::Authority)?;
    let (actual_uid, pid) = credentials(&bus, &sender)?;
    if uid != actual_uid {
        return Err(Error::Ownership);
    }
    let (start_ticks, boot_id) = process(pid, uid)?;
    Ok(ServiceIdentity {
        sender,
        uid,
        pid,
        start_ticks,
        boot_id,
    })
}
pub(crate) fn start_polkit(connection: &Connection) -> Result<()> {
    // Native installed D-Bus service activation only; no authentication or action.
    let result: u32 = bus(connection)?
        .call("StartServiceByName", &("org.freedesktop.PolicyKit1", 0u32))
        .map_err(|_| Error::AuthRequired)?;
    if !matches!(result, 1 | 2) {
        return Err(Error::Authority);
    }
    Ok(())
}
fn capture(connection: &Connection, sender: &str) -> Result<Snapshot> {
    let bus = bus(connection)?;
    let (uid, pid) = credentials(&bus, sender)?;
    let bus_id = bus_id(&bus)?;
    let (start_ticks, boot_id) = process(pid, uid)?;
    let logind = service(&bus)?;
    if logind.boot_id != boot_id {
        return Err(Error::TargetChanged);
    }
    let session = session(connection, &logind.sender, pid, uid)?;
    if credentials(&bus, sender)? != (uid, pid)
        || process(pid, uid)? != (start_ticks, boot_id.clone())
        || self::bus_id(&bus)? != bus_id
        || service(&bus)? != logind
    {
        return Err(Error::TargetChanged);
    }
    Ok(Snapshot {
        caller: CallerIdentity {
            uid,
            pid,
            start_ticks,
            boot_id,
            bus_id,
            sender: sender.into(),
            session,
        },
        logind,
    })
}
fn compare(original: &Snapshot, current: &Snapshot) -> Result<()> {
    if original != current {
        return Err(Error::TargetChanged);
    }
    Ok(())
}
fn requesting_user(caller: &CallerIdentity) -> Result<()> {
    if caller.uid == 0 {
        return Err(Error::Authority);
    }
    if caller
        .session
        .as_ref()
        .is_some_and(|s| s.state == "closing")
    {
        return Err(Error::TargetChanged);
    }
    Ok(())
}

fn session(
    connection: &Connection,
    owner: &str,
    pid: u32,
    uid: u32,
) -> Result<Option<SessionIdentity>> {
    let manager = Proxy::new(
        connection,
        owner,
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .map_err(|_| Error::Authority)?;
    let path: OwnedObjectPath = match manager.call("GetSessionByPID", &(pid,)) {
        Ok(path) => path,
        Err(zbus::Error::MethodError(name, _, _))
            if name.as_str() == "org.freedesktop.login1.NoSessionForPID" =>
        {
            return Ok(None);
        }
        Err(_) => return Err(Error::Authority),
    };
    if !path
        .as_str()
        .starts_with("/org/freedesktop/login1/session/")
        || path.as_str().len() > 256
    {
        return Err(Error::Authority);
    }
    // Fresh direct Properties.Get calls avoid proxy property caches during recheck.
    let properties = Proxy::new(
        connection,
        owner,
        path.as_str(),
        "org.freedesktop.DBus.Properties",
    )
    .map_err(|_| Error::Authority)?;
    let get = |name: &str| -> Result<zbus::zvariant::OwnedValue> {
        properties
            .call("Get", &("org.freedesktop.login1.Session", name))
            .map_err(|_| Error::Authority)
    };
    let (actual_uid, _): (u32, OwnedObjectPath) =
        get("User")?.try_into().map_err(|_| Error::Authority)?;
    if actual_uid != uid {
        return Err(Error::Ownership);
    }
    let string = |name: &str, limit: usize| -> Result<String> {
        let s: String = get(name)?.try_into().map_err(|_| Error::Authority)?;
        if s.is_empty()
            || s.len() > limit
            || !s
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error::Authority);
        }
        Ok(s)
    };
    let result = SessionIdentity {
        id: string("Id", 128)?,
        remote: get("Remote")?.try_into().map_err(|_| Error::Authority)?,
        kind: string("Type", 32)?,
        class: string("Class", 32)?,
        state: string("State", 32)?,
        active: get("Active")?.try_into().map_err(|_| Error::Authority)?,
    };
    Ok(Some(result))
}
fn read_proc(path: &str, owner: u32) -> Result<String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::TargetChanged)?;
    let m = file.metadata()?;
    if !m.is_file() || m.uid() != owner {
        return Err(Error::Ownership);
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::TargetChanged)?;
    if bytes.len() > 65536 {
        return Err(Error::Authority);
    }
    String::from_utf8(bytes).map_err(|_| Error::Authority)
}
fn process(pid: u32, uid: u32) -> Result<(u64, String)> {
    if pid <= 1 {
        return Err(Error::Authority);
    }
    let directory = format!("/proc/{pid}");
    let m = fs::symlink_metadata(&directory).map_err(|_| Error::TargetChanged)?;
    if !m.is_dir() || m.uid() != uid {
        return Err(Error::Ownership);
    }
    let status = read_proc(&format!("{directory}/status"), uid)?;
    check_uids(&status, uid)?;
    let ticks = start_ticks(&read_proc(&format!("{directory}/stat"), uid)?, pid)?;
    let boot = read_proc("/proc/sys/kernel/random/boot_id", 0)?
        .trim()
        .to_owned();
    if !crate::uuid(&boot) || boot == uuid::Uuid::nil().to_string() {
        return Err(Error::Authority);
    }
    check_uids(&read_proc(&format!("{directory}/status"), uid)?, uid)?;
    if start_ticks(&read_proc(&format!("{directory}/stat"), uid)?, pid)? != ticks {
        return Err(Error::TargetChanged);
    }
    Ok((ticks, boot))
}
pub(crate) fn check_uids(status: &str, uid: u32) -> Result<()> {
    let mut lines = status.lines().filter_map(|s| s.strip_prefix("Uid:"));
    let fields: Vec<_> = lines
        .next()
        .ok_or(Error::Authority)?
        .split_whitespace()
        .collect();
    if lines.next().is_some()
        || fields.len() != 4
        || !fields.iter().all(|f| f.parse::<u32>() == Ok(uid))
    {
        return Err(Error::Ownership);
    }
    Ok(())
}
pub(crate) fn start_ticks(stat: &str, pid: u32) -> Result<u64> {
    let (prefix, rest) = stat.rsplit_once(')').ok_or(Error::Authority)?;
    let (actual, name) = prefix.split_once(" (").ok_or(Error::Authority)?;
    if actual.parse::<u32>() != Ok(pid) || name.len() > 4096 {
        return Err(Error::Authority);
    }
    let fields: Vec<_> = rest.split_whitespace().collect();
    let ticks: u64 = fields
        .get(19)
        .ok_or(Error::Authority)?
        .parse()
        .map_err(|_| Error::Authority)?;
    if ticks == 0 {
        return Err(Error::Authority);
    }
    Ok(ticks)
}

#[cfg(test)]
mod tests;
