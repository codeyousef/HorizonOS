//! Durable one-way handoff from consumed authorization to the independent guard.
use crate::{Error, Result, canonical, digest, uuid};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    time::Duration,
};
use zbus::blocking::{Connection, Proxy};

const ROOT: &str = "/var/lib/aios/transactions";
const MAX_HANDOFF: u64 = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardHandoff {
    pub schema_version: u32,
    pub transaction_id: String,
    pub requester_uid: u32,
    pub final_plan_sha256: String,
}
impl GuardHandoff {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !uuid(&self.transaction_id)
            || self.requester_uid == 0
            || !digest(&self.final_plan_sha256)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

fn root() -> Result<File> {
    let metadata = fs::symlink_metadata(ROOT)?;
    if !metadata.is_dir()
        || metadata.uid() != 0
        || metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(ROOT)? != Path::new(ROOT)
    {
        return Err(Error::Ownership);
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(ROOT)
        .map_err(Into::into)
}
fn names(id: &str) -> Result<(PathBuf, PathBuf)> {
    if !uuid(id) {
        return Err(Error::Invalid);
    }
    let root = Path::new(ROOT);
    Ok((root.join(format!("guard-{id}.json")), root.join(format!(".guard-{id}.tmp"))))
}
fn read_exact(path: &Path) -> Result<Vec<u8>> {
    let before = fs::symlink_metadata(path)?;
    if !before.is_file()
        || before.uid() != 0
        || before.mode() & 0o777 != 0o400
        || before.nlink() != 1
        || before.len() == 0
        || before.len() > MAX_HANDOFF
    {
        return Err(Error::Ownership);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let opened = file.metadata()?;
    if (opened.dev(), opened.ino(), opened.len()) != (before.dev(), before.ino(), before.len()) {
        return Err(Error::TargetChanged);
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != before.len()
        || (after.dev(), after.ino(), after.len(), after.mtime(), after.mtime_nsec())
            != (before.dev(), before.ino(), before.len(), before.mtime(), before.mtime_nsec())
    {
        return Err(Error::TargetChanged);
    }
    Ok(bytes)
}

pub fn read(id: &str) -> Result<GuardHandoff> {
    let _root = root()?;
    let (path, _) = names(id)?;
    let bytes = read_exact(&path)?;
    let handoff: GuardHandoff = serde_json::from_slice(&bytes)?;
    handoff.validate()?;
    if handoff.transaction_id != id || canonical(&handoff)? != bytes {
        return Err(Error::Integrity);
    }
    Ok(handoff)
}

fn persist(handoff: &GuardHandoff) -> Result<()> {
    handoff.validate()?;
    let directory = root()?;
    let (path, temporary) = names(&handoff.transaction_id)?;
    let bytes = canonical(handoff)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => {
            let existing = read_exact(&path)?;
            return if existing == bytes {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    match fs::symlink_metadata(&temporary) {
        Ok(metadata)
            if metadata.is_file()
                && metadata.uid() == 0
                && metadata.mode() & 0o777 == 0o600
                && metadata.nlink() == 1 =>
        {
            fs::remove_file(&temporary)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err(Error::Ownership),
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o400))?;
    file.sync_all()?;
    fs::hard_link(&temporary, &path)?;
    fs::remove_file(&temporary)?;
    directory.sync_all()?;
    Ok(())
}

fn start(id: &str) -> Result<()> {
    if !uuid(id) {
        return Err(Error::Invalid);
    }
    let connection = Connection::system().map_err(|_| Error::Io)?;
    let bus = Proxy::new(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .map_err(|_| Error::Io)?;
    let owner: String = bus
        .call("GetNameOwner", &("org.freedesktop.systemd1",))
        .map_err(|_| Error::Authority)?;
    let credentials: zbus::fdo::ConnectionCredentials = bus
        .call("GetConnectionCredentials", &(owner.as_str(),))
        .map_err(|_| Error::Authority)?;
    if credentials.unix_user_id() != Some(0) || credentials.process_id() != Some(1) {
        return Err(Error::Authority);
    }
    let manager = Proxy::new(
        &connection,
        owner,
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .map_err(|_| Error::Io)?;
    let unit = format!("aios-guard@{id}.service");
    let _: zbus::zvariant::OwnedObjectPath = manager
        .call("StartUnit", &(unit.as_str(), "replace"))
        .map_err(|_| Error::Io)?;
    Ok(())
}

pub(crate) fn handoff(
    ledger: &mut crate::ledger::Ledger,
    id: &str,
    uid: u32,
    hash: &str,
) -> Result<crate::ledger::Status> {
    let handoff = GuardHandoff {
        schema_version: 1,
        transaction_id: id.into(),
        requester_uid: uid,
        final_plan_sha256: hash.into(),
    };
    persist(&handoff)?;
    let status = ledger.authorize(id, uid, hash)?;
    start(id)?;
    Ok(status)
}

pub(crate) fn restart(handoff: &GuardHandoff) -> Result<()> {
    handoff.validate()?;
    start(&handoff.transaction_id)
}

const MAX_CONTROL: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagementIdentity {
    pub installation_uuid: String,
    pub dmi_uuid: String,
    pub machine_id: String,
    pub boot_id: String,
    pub role: String,
    pub disk_serial: String,
    pub management_channel: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagementHeartbeat {
    pub schema_version: u32,
    pub transaction_id: String,
    pub identity: ManagementIdentity,
    pub plan_digest: String,
    pub candidate_digest: String,
    pub nonce: String,
}
impl ManagementHeartbeat {
    fn validate(&self, id: &str) -> Result<()> {
        if self.schema_version != 1
            || self.transaction_id != id
            || !uuid(id)
            || !digest(&self.plan_digest)
            || !digest(&self.candidate_digest)
            || self.nonce.len() != 64
            || !self
                .nonce
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || ![
                &self.identity.installation_uuid,
                &self.identity.dmi_uuid,
                &self.identity.boot_id,
            ]
            .iter()
            .all(|value| uuid(value))
            || self.identity.machine_id.len() != 32
            || !self
                .identity
                .machine_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !matches!(
                self.identity.role.as_str(),
                "development" | "production" | "acceptance"
            )
            || self.identity.disk_serial.is_empty()
            || self.identity.disk_serial.len() > 64
            || !self
                .identity
                .disk_serial
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
            || !matches!(
                self.identity.management_channel.as_str(),
                "ssh-development" | "local-product"
            )
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

fn guard_stream(id: &str) -> Result<UnixStream> {
    if !uuid(id) {
        return Err(Error::Invalid);
    }
    let directory = Path::new("/run/aios-guard").join(id);
    let directory_metadata = fs::symlink_metadata(&directory)?;
    if !directory_metadata.is_dir()
        || directory_metadata.uid() != 0
        || directory_metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(&directory)? != directory
    {
        return Err(Error::Ownership);
    }
    let socket = directory.join("control.sock");
    let metadata = fs::symlink_metadata(&socket)?;
    if !metadata.file_type().is_socket()
        || metadata.uid() != 0
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(Error::Ownership);
    }
    let stream = UnixStream::connect(&socket)?;
    // A heartbeat performs the bounded synchronous exact-pointer commit. Keep
    // the broker connection alive through the guard's 180-second deadline so a
    // successful durable COMMITTED record is acknowledged and terminalized.
    stream.set_read_timeout(Some(Duration::from_secs(185)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            std::os::fd::AsRawFd::as_raw_fd(&stream),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &mut size,
        )
    } != 0
        || size as usize != std::mem::size_of::<libc::ucred>()
        || unsafe { credentials.assume_init() }.uid != 0
    {
        return Err(Error::Authority);
    }
    Ok(stream)
}

fn exchange(id: &str, request: &Value) -> Result<Value> {
    let bytes = canonical(request)?;
    if bytes.len() > MAX_CONTROL {
        return Err(Error::Invalid);
    }
    let mut stream = guard_stream(id)?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_CONTROL {
        return Err(Error::Integrity);
    }
    let mut reply = vec![0; length];
    stream.read_exact(&mut reply)?;
    serde_json::from_slice(&reply).map_err(Into::into)
}

pub(crate) fn status(id: &str) -> Result<Value> {
    exchange(id, &json!({"operation":"status","transaction_id":id}))
}

pub(crate) fn heartbeat(id: &str, raw: &str) -> Result<Value> {
    if raw.len() > MAX_CONTROL {
        return Err(Error::Invalid);
    }
    let heartbeat: ManagementHeartbeat = serde_json::from_str(raw)?;
    heartbeat.validate(id)?;
    exchange(id, &json!({"operation":"heartbeat","heartbeat":heartbeat}))
}

pub(crate) fn recover(id: &str) -> Result<Value> {
    exchange(id, &json!({"operation":"recover","transaction_id":id}))
}
