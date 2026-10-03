//! Fixed read-only system health observations. They convey no action authority.
use crate::{Error, Result, ledger::Target, native::VerifiedTarget};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::MetadataExt,
    path::Path,
    time::{Duration, Instant},
};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedObjectPath, OwnedValue},
};

pub const PROTECTED_UNITS: &[&str] = &[
    "sshd.service",
    "dbus.service",
    "aios-state.service",
    "aios-exec.service",
    "aios-model.service",
    "aios-observer.service",
    "aios-build.service",
];
pub const SYSTEM_MOUNTS: &[&str] = &["/", "/nix", "/var", "/home", "/boot"];
const MANAGER: &str = "org.freedesktop.systemd1";
const UNIT: &str = "org.freedesktop.systemd1.Unit";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mount {
    pub path: String,
    pub root: String,
    pub filesystem: String,
    pub source: String,
    pub device: String,
    pub writable: bool,
}
fn writable(options: &str) -> Result<bool> {
    let selected: Vec<_> = options
        .split(',')
        .filter(|s| matches!(*s, "ro" | "rw"))
        .collect();
    if selected.len() != 1 {
        return Err(Error::Integrity);
    }
    Ok(selected[0] == "rw")
}
pub(crate) fn mounts(bytes: &[u8], root_device: &str) -> Result<Vec<Mount>> {
    if bytes.len() > 1024 * 1024 {
        return Err(Error::Integrity);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Integrity)?;
    let mut result = vec![];
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for line in text.lines() {
        let (left, right) = line.split_once(" - ").ok_or(Error::Integrity)?;
        let l: Vec<_> = left.split_whitespace().collect();
        let r: Vec<_> = right.split_whitespace().collect();
        if l.len() < 6
            || r.len() != 3
            || l[0].parse::<u64>().ok().filter(|n| *n > 0).is_none()
            || l[1].parse::<u64>().is_err()
            || !ids.insert(l[0])
        {
            return Err(Error::Integrity);
        }
        if !SYSTEM_MOUNTS.contains(&l[4]) {
            continue;
        }
        if !paths.insert(l[4]) {
            return Err(Error::Integrity);
        }
        let dev: Vec<_> = l[2].split(':').collect();
        if dev.len() != 2 || dev.iter().any(|n| n.parse::<u32>().is_err()) {
            return Err(Error::Integrity);
        }
        let expected = match l[4] {
            "/" => "/@root",
            "/nix" => "/@nix",
            "/var" => "/@var",
            "/home" => "/@home",
            "/boot" => "/",
            _ => unreachable!(),
        };
        if l[3] != expected
            || (l[4] != "/boot" && (r[0] != "btrfs" || r[1] != root_device))
            || (l[4] == "/boot" && (r[0] != "vfat" || !r[1].starts_with("/dev/")))
        {
            return Err(Error::TargetChanged);
        }
        let local_rw = writable(l[5])?;
        let super_rw = writable(r[2])?;
        result.push(Mount {
            path: l[4].into(),
            root: l[3].into(),
            filesystem: r[0].into(),
            source: r[1].into(),
            device: l[2].into(),
            writable: local_rw && super_rw,
        });
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    if !result.iter().any(|m| m.path == "/") {
        return Err(Error::Health);
    }
    Ok(result)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnitState {
    pub id: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub invocation_id: Vec<u8>,
    pub job_id: u32,
}
impl UnitState {
    fn validate(&self) -> Result<()> {
        if !service_name(&self.id)
            || ![
                "stub",
                "loaded",
                "not-found",
                "bad-setting",
                "error",
                "merged",
                "masked",
            ]
            .contains(&self.load_state.as_str())
            || ![
                "active",
                "reloading",
                "inactive",
                "failed",
                "activating",
                "deactivating",
                "maintenance",
                "refreshing",
            ]
            .contains(&self.active_state.as_str())
            || self.sub_state.is_empty()
            || self.sub_state.len() > 64
            || !self
                .sub_state
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || self.invocation_id.len() != 16
        {
            return Err(Error::Integrity);
        }
        Ok(())
    }
    pub fn active(&self) -> bool {
        self.load_state == "loaded" && self.active_state == "active" && self.job_id == 0
    }
    pub fn failed(&self) -> bool {
        self.active_state == "failed" || matches!(self.load_state.as_str(), "error" | "bad-setting")
    }
}
fn service_name(name: &str) -> bool {
    name.len() <= 255
        && name.len() > 8
        && name.ends_with(".service")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.@:-".contains(&b))
}
fn unit_identity(requested: &str, id: &str, names: &[String]) -> Result<()> {
    let mut unique = BTreeSet::new();
    if !PROTECTED_UNITS.contains(&requested)
        || !service_name(id)
        || names.is_empty()
        || names.len() > 64
        || !names
            .iter()
            .all(|name| service_name(name) && unique.insert(name))
        || !names.iter().any(|name| name == requested)
        || !names.iter().any(|name| name == id)
    {
        return Err(Error::Integrity);
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UnitEvidence {
    pub name: String,
    /// None means GetUnit reported NoSuchUnit, not a healthy inactive service.
    pub state: Option<UnitState>,
}
impl UnitEvidence {
    pub fn active(&self) -> bool {
        self.state.as_ref().is_some_and(UnitState::active)
    }
    pub fn failed(&self) -> bool {
        self.state.as_ref().is_some_and(UnitState::failed)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ManagerEvidence {
    pub bus_id: String,
    pub owner: String,
    pub uid: u32,
    pub pid: u32,
    pub start_ticks: u64,
    pub executable: String,
    pub executable_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HealthEvidence {
    pub target: Target,
    pub manager: ManagerEvidence,
    pub mounts: Vec<Mount>,
    pub units: Vec<UnitEvidence>,
    pub core_baseline_healthy: bool,
    pub product_apis_verified: bool,
    pub user_service_activation_verified: bool,
    pub action_postconditions_verified: bool,
    pub authenticated_host_heartbeat_verified: bool,
}
/// Constructed from verified target, kernel mounts and authenticated PID1 only.
/// Serialized evidence cannot be promoted into this capability.
pub struct NativeHealth {
    target: VerifiedTarget,
    evidence: HealthEvidence,
    captured_at: Instant,
}
fn deadline(started: Instant) -> Result<()> {
    if started.elapsed() > Duration::from_secs(5) {
        Err(Error::Health)
    } else {
        Ok(())
    }
}
fn manager(connection: &Connection, target: &VerifiedTarget) -> Result<ManagerEvidence> {
    let bus = Proxy::new(
        connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .map_err(|_| Error::Health)?;
    let bus_id: String = bus.call("GetId", &()).map_err(|_| Error::Health)?;
    let owner: String = bus
        .call("GetNameOwner", &(MANAGER,))
        .map_err(|_| Error::Health)?;
    if bus_id.len() != 32
        || !bus_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !owner.starts_with(':')
        || owner.len() > 128
        || !owner[1..]
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(Error::Integrity);
    }
    let credentials: zbus::fdo::ConnectionCredentials = bus
        .call("GetConnectionCredentials", &(owner.as_str(),))
        .map_err(|_| Error::Health)?;
    if credentials.unix_user_id() != Some(0) || credentials.process_id() != Some(1) {
        return Err(Error::Authority);
    }
    let read = |name: &str| -> Result<String> {
        String::from_utf8(crate::native::read(
            Path::new(name),
            Path::new("/proc"),
            false,
            65536,
        )?)
        .map_err(|_| Error::Integrity)
    };
    crate::caller::check_uids(&read("/proc/1/status")?, 0)?;
    let start_ticks = crate::caller::start_ticks(&read("/proc/1/stat")?, 1)?;
    let artifact = target.store_artifact(
        Path::new("/run/current-system/systemd/lib/systemd/systemd"),
        64 * 1024 * 1024,
        true,
    )?;
    let actual = fs::read_link("/proc/1/exe")?;
    let process = fs::metadata("/proc/1/exe")?;
    let installed = fs::symlink_metadata(&artifact.path)?;
    if actual != artifact.path
        || (process.dev(), process.ino()) != (installed.dev(), installed.ino())
        || crate::caller::start_ticks(&read("/proc/1/stat")?, 1)? != start_ticks
    {
        return Err(Error::TargetChanged);
    }
    Ok(ManagerEvidence {
        bus_id,
        owner,
        uid: 0,
        pid: 1,
        start_ticks,
        executable: artifact.path.to_str().ok_or(Error::Integrity)?.into(),
        executable_sha256: artifact.sha256,
    })
}
fn units(connection: &Connection, owner: &str, started: Instant) -> Result<Vec<UnitEvidence>> {
    let proxy = Proxy::new(
        connection,
        owner,
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .map_err(|_| Error::Health)?;
    let mut result = vec![];
    for name in PROTECTED_UNITS {
        deadline(started)?;
        let path: OwnedObjectPath = match proxy.call("GetUnit", &(name,)) {
            Ok(path) => path,
            Err(zbus::Error::MethodError(error, _, _))
                if error.as_str() == "org.freedesktop.systemd1.NoSuchUnit" =>
            {
                result.push(UnitEvidence {
                    name: (*name).into(),
                    state: None,
                });
                continue;
            }
            Err(_) => return Err(Error::Health),
        };
        if !path.as_str().starts_with("/org/freedesktop/systemd1/unit/")
            || path.as_str().len() > 512
        {
            return Err(Error::Integrity);
        }
        let properties = Proxy::new(
            connection,
            owner,
            path.as_str(),
            "org.freedesktop.DBus.Properties",
        )
        .map_err(|_| Error::Health)?;
        let get = |name: &str| -> Result<OwnedValue> {
            deadline(started)?;
            properties
                .call("Get", &(UNIT, name))
                .map_err(|_| Error::Health)
        };
        let id: String = get("Id")?.try_into().map_err(|_| Error::Integrity)?;
        let names: Vec<String> = get("Names")?.try_into().map_err(|_| Error::Integrity)?;
        #[cfg(test)]
        println!(
            "AIOS_FIXED_UNIT_IDENTITY {}",
            serde_json::json!({"requested":name,"id":id,"names":names})
        );
        unit_identity(name, &id, &names)?;
        let (job_id, job_path): (u32, OwnedObjectPath) =
            get("Job")?.try_into().map_err(|_| Error::Integrity)?;
        if (job_id == 0 && job_path.as_str() != "/")
            || (job_id != 0
                && job_path.as_str() != format!("/org/freedesktop/systemd1/job/{job_id}"))
        {
            return Err(Error::Integrity);
        }
        let state = UnitState {
            id,
            load_state: get("LoadState")?.try_into().map_err(|_| Error::Integrity)?,
            active_state: get("ActiveState")?
                .try_into()
                .map_err(|_| Error::Integrity)?,
            sub_state: get("SubState")?.try_into().map_err(|_| Error::Integrity)?,
            invocation_id: get("InvocationID")?
                .try_into()
                .map_err(|_| Error::Integrity)?,
            job_id,
        };
        state.validate()?;
        result.push(UnitEvidence {
            name: (*name).into(),
            state: Some(state),
        });
    }
    deadline(started)?;
    Ok(result)
}
fn core_healthy(target: &Target, mounts: &[Mount], units: &[UnitEvidence]) -> bool {
    SYSTEM_MOUNTS
        .iter()
        .all(|path| mounts.iter().any(|m| m.path == *path && m.writable))
        && units.iter().any(|u| u.name == "dbus.service" && u.active())
        && (target.role != "development"
            || units.iter().any(|u| u.name == "sshd.service" && u.active()))
}
impl NativeHealth {
    pub fn capture() -> Result<Self> {
        let target = VerifiedTarget::enroll()?;
        let connection = crate::caller::connect_native_timeout(Duration::from_millis(250))?;
        let started = Instant::now();
        let before = manager(&connection, &target)?;
        let mounts = target.system_mounts()?;
        let observed = units(&connection, &before.owner, started)?;
        let final_units = units(&connection, &before.owner, started)?;
        if observed != final_units
            || mounts != target.system_mounts()?
            || before != manager(&connection, &target)?
        {
            return Err(Error::TargetChanged);
        }
        deadline(started)?;
        target.recheck()?;
        let evidence = HealthEvidence {
            target: target.target().clone(),
            manager: before,
            core_baseline_healthy: core_healthy(target.target(), &mounts, &observed),
            mounts,
            units: observed,
            product_apis_verified: false,
            user_service_activation_verified: false,
            action_postconditions_verified: false,
            authenticated_host_heartbeat_verified: false,
        };
        Ok(Self {
            target,
            evidence,
            captured_at: Instant::now(),
        })
    }
    pub fn evidence(&self) -> &HealthEvidence {
        &self.evidence
    }
    pub fn verify_against(&self, baseline: &Self) -> Result<()> {
        self.target.recheck()?;
        baseline.target.recheck()?;
        if self.captured_at.elapsed() > Duration::from_secs(1) {
            return Err(Error::Health);
        }
        compare(&baseline.evidence, &self.evidence)
    }
}
fn compare(baseline: &HealthEvidence, current: &HealthEvidence) -> Result<()> {
    if baseline.target != current.target {
        return Err(Error::TargetChanged);
    }
    if !baseline.core_baseline_healthy
        || !current.core_baseline_healthy
        || baseline.mounts != current.mounts
        || !baseline.units.iter().all(|b| {
            current.units.iter().any(|u| {
                u.name == b.name && (!b.active() || u.active()) && (b.failed() || !u.failed())
            })
        })
    {
        return Err(Error::Health);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
