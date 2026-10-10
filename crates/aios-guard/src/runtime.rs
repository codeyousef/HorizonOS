//! Root-only fixed activation adapter. Construction captures installed native
//! authority before any effect; callers cannot supply commands or store paths.
use crate::{
    ActionValidator, Adapter, Api, ApiHealth, Candidate, Effect, Error, Observation, Plan, Result,
    UnitHealth, UserUnit,
    activation::{self, RetainedNix},
    native::{self, NativeIntake},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt, symlink},
        net::UnixStream,
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
use zbus::{blocking::{Connection, Proxy}, zvariant::OwnedObjectPath};

const RECOVERY_ROOTS: &str = "/nix/var/nix/gcroots/aios-guard";
const GRAPH_SOCKET: &str = "/run/aios-state/owner.sock";
const MAX_GRAPH_REPLY: usize = 2 * 1024 * 1024;

/// The adapter is intentionally not serializable and has no public command API.
/// Its Plan has already crossed executor authorization before construction.
pub struct NativeAdapter {
    plan: Plan,
    nix: RetainedNix,
    guard: GuardServiceIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GuardServiceIdentity {
    owner: String,
    unit: String,
    invocation: Vec<u8>,
    pid: u32,
    developer: bool,
}

fn guard_fragment_path(path: &Path, developer: bool) -> bool {
    let unit = if developer {
        "lib/systemd/system/aios-dev-guard@.service"
    } else {
        "lib/systemd/system/aios-guard@.service"
    };
    path.starts_with("/nix/store/") && path.ends_with(unit)
}
impl GuardServiceIdentity {
    fn capture(transaction_id: &str, developer: bool) -> Result<Self> {
        let pid = unsafe { libc::getpid() };
        let pid = u32::try_from(pid).map_err(|_| Error::Integrity)?;
        let prefix = if developer {
            "aios-dev-guard"
        } else {
            "aios-guard"
        };
        let unit = format!("{prefix}@{transaction_id}.service");
        let cgroup = fs::read(format!("/proc/{pid}/cgroup")).map_err(|_| Error::Integrity)?;
        let suffix = format!("/{unit}");
        if cgroup.len() > 65536
            || !cgroup
                .split(|byte| *byte == b'\n')
                .any(|line| line.ends_with(suffix.as_bytes()))
        {
            return Err(Error::Authority);
        }
        let connection = Connection::system().map_err(|_| Error::Authority)?;
        let bus = Proxy::new(
            &connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .map_err(|_| Error::Authority)?;
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
            owner.as_str(),
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .map_err(|_| Error::Authority)?;
        let path: OwnedObjectPath = manager
            .call("GetUnit", &(unit.as_str(),))
            .map_err(|_| Error::Authority)?;
        let properties = Proxy::new(
            &connection,
            owner.as_str(),
            path.as_str(),
            "org.freedesktop.DBus.Properties",
        )
        .map_err(|_| Error::Authority)?;
        let get = |interface: &str, property: &str| -> Result<zbus::zvariant::OwnedValue> {
            properties
                .call("Get", &(interface, property))
                .map_err(|_| Error::Authority)
        };
        let id: String = get("org.freedesktop.systemd1.Unit", "Id")?
            .try_into()
            .map_err(|_| Error::Authority)?;
        let active: String = get("org.freedesktop.systemd1.Unit", "ActiveState")?
            .try_into()
            .map_err(|_| Error::Authority)?;
        let invocation: Vec<u8> = get("org.freedesktop.systemd1.Unit", "InvocationID")?
            .try_into()
            .map_err(|_| Error::Authority)?;
        let fragment: String = get("org.freedesktop.systemd1.Unit", "FragmentPath")?
            .try_into()
            .map_err(|_| Error::Authority)?;
        let fragment = fs::canonicalize(&fragment).map_err(|_| Error::Integrity)?;
        let main_pid: u32 = get("org.freedesktop.systemd1.Service", "MainPID")?
            .try_into()
            .map_err(|_| Error::Authority)?;
        let fragment_metadata =
            fs::symlink_metadata(&fragment).map_err(|_| Error::Integrity)?;
        let failed_check = if id != unit {
            Some("unit-id")
        } else if !matches!(active.as_str(), "activating" | "active") {
            Some("active-state")
        } else if invocation.len() != 16 || invocation.iter().all(|byte| *byte == 0) {
            Some("invocation-id")
        } else if main_pid != pid {
            Some("main-pid")
        } else if !guard_fragment_path(&fragment, developer) {
            Some("fragment-path")
        } else if !fragment_metadata.is_file()
            || fragment_metadata.uid() != 0
            || fragment_metadata.mode() & 0o022 != 0
        {
            Some("fragment-metadata")
        } else {
            None
        };
        if let Some(check) = failed_check {
            eprintln!(
                "{}",
                serde_json::json!({
                    "schema_version": 1,
                    "service_identity_check_failed": check,
                })
            );
            return Err(Error::Authority);
        }
        drop(properties);
        drop(manager);
        drop(bus);
        drop(connection);
        Ok(Self {
            owner,
            unit,
            invocation,
            pid,
            developer,
        })
    }
    fn verify(&self, transaction_id: &str) -> Result<()> {
        if &Self::capture(transaction_id, self.developer)? == self {
            Ok(())
        } else {
            Err(Error::TargetMismatch)
        }
    }
}

fn native_intake() -> Result<NativeIntake> {
    NativeIntake::capture()
}

fn hash_file(path: &Path, bound: u64) -> Result<String> {
    let before = fs::symlink_metadata(path).map_err(|_| Error::Integrity)?;
    if !before.is_file()
        || before.uid() != 0
        || before.mode() & 0o022 != 0
        || before.len() > bound
    {
        return Err(Error::Integrity);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| Error::Integrity)?;
    let opened = file.metadata().map_err(|_| Error::Integrity)?;
    if (opened.dev(), opened.ino(), opened.len()) != (before.dev(), before.ino(), before.len()) {
        return Err(Error::Integrity);
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 16384];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|_| Error::Integrity)?;
        if count == 0 { break; }
        total = total.checked_add(count as u64).ok_or(Error::Integrity)?;
        if total > bound { return Err(Error::Integrity); }
        digest.update(&buffer[..count]);
    }
    let after = file.metadata().map_err(|_| Error::Integrity)?;
    if total != before.len()
        || (after.dev(), after.ino(), after.len(), after.mtime(), after.mtime_nsec(), after.ctime(), after.ctime_nsec())
            != (before.dev(), before.ino(), before.len(), before.mtime(), before.mtime_nsec(), before.ctime(), before.ctime_nsec())
    {
        return Err(Error::Integrity);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(crate) fn managed_sha256(running: &str) -> Result<String> {
    let manifest = fs::canonicalize(Path::new(running).join("etc/aios/managed.json"))
        .map_err(|_| Error::Integrity)?;
    if !manifest.starts_with("/nix/store/") {
        return Err(Error::Integrity);
    }
    hash_file(&manifest, 1024 * 1024)
}
const USER_SERVICES: [(&str, &str); 3] = [
    ("aios-sessiond.service", "aios-sessiond"),
    ("aios-processd.service", "aios-processd"),
    ("aios-ui-agent.service", "aios-ui-agent"),
];

fn user_service_hash(uid: u32, unit: &str, binary: &str) -> Result<Option<String>> {
    let cgroup = PathBuf::from(format!(
        "/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service/app.slice/{unit}/cgroup.procs"
    ));
    let raw = match fs::read_to_string(cgroup) {
        Ok(value) => value,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Integrity),
    };
    if raw.len() > 4096 {
        return Err(Error::Integrity);
    }
    let mut saw_process = false;
    for (index, line) in raw.lines().enumerate() {
        if index >= 64 {
            return Err(Error::Integrity);
        }
        let pid = line.parse::<u32>().map_err(|_| Error::Integrity)?;
        saw_process = true;
        let executable = match fs::canonicalize(format!("/proc/{pid}/exe")) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(_) => return Err(Error::Integrity),
        };
        if executable.file_name().and_then(|name| name.to_str()) == Some(binary) {
            return Ok(Some(hash_file(&executable, 256 * 1024 * 1024)?));
        }
    }
    if saw_process {
        Err(Error::Health)
    } else {
        Ok(None)
    }
}

pub(crate) fn user_units(uid: u32, candidate: &str) -> Result<(Vec<UserUnit>, Vec<UserUnit>)> {
    let mut baseline = Vec::with_capacity(USER_SERVICES.len());
    let mut required = Vec::with_capacity(USER_SERVICES.len());
    for (name, binary) in USER_SERVICES {
        let Some(baseline_hash) = user_service_hash(uid, name, binary)? else {
            continue;
        };
        let executable = fs::canonicalize(Path::new(candidate).join("sw/bin").join(binary))
            .map_err(|_| Error::Integrity)?;
        if !executable.starts_with("/nix/store/") {
            return Err(Error::Integrity);
        }
        baseline.push(UserUnit {
            uid,
            name: name.into(),
            expected_executable_sha256: baseline_hash,
        });
        required.push(UserUnit {
            uid,
            name: name.into(),
            expected_executable_sha256: hash_file(&executable, 256 * 1024 * 1024)?,
        });
    }
    Ok((baseline, required))
}

fn observed_user_units(plan: &Plan) -> Result<Vec<UserUnit>> {
    let mut observed = Vec::with_capacity(plan.health.baseline_user_units.len());
    for required in &plan.health.baseline_user_units {
        let binary = USER_SERVICES
            .iter()
            .find_map(|(unit, binary)| (*unit == required.name).then_some(*binary))
            .ok_or(Error::InvalidPlan)?;
        let Some(expected_executable_sha256) =
            user_service_hash(required.uid, &required.name, binary)?
        else {
            continue;
        };
        observed.push(UserUnit {
            uid: required.uid,
            name: required.name.clone(),
            expected_executable_sha256,
        });
    }
    Ok(observed)
}

fn graph_health() -> bool {
    fn call() -> std::io::Result<()> {
        let mut stream = UnixStream::connect(GRAPH_SOCKET)?;
        stream.set_read_timeout(Some(Duration::from_millis(250)))?;
        stream.set_write_timeout(Some(Duration::from_millis(250)))?;
        let request = br#"{"kind":"status"}"#;
        stream.write_all(&(request.len() as u32).to_be_bytes())?;
        stream.write_all(request)?;
        let mut length = [0u8; 4];
        stream.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length) as usize;
        if length == 0 || length > MAX_GRAPH_REPLY { return Err(std::io::ErrorKind::InvalidData.into()); }
        let mut bytes = vec![0u8; length];
        stream.read_exact(&mut bytes)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| std::io::ErrorKind::InvalidData)?;
        if value.get("ok") != Some(&serde_json::Value::Bool(true)) {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        Ok(())
    }
    call().is_ok()
}

fn executor_health() -> bool {
    fn call() -> std::result::Result<(), zbus::Error> {
        let connection = Connection::system()?;
        let proxy = Proxy::new(
            &connection,
            "org.aios.Executor1",
            "/org/aios/Executor1",
            "org.freedesktop.DBus.Introspectable",
        )?;
        let xml: String = proxy.call("Introspect", &())?;
        if !xml.contains("interface name=\"org.aios.Executor1\"") {
            return Err(zbus::Error::Failure("executor interface absent".into()));
        }
        Ok(())
    }
    call().is_ok()
}

fn unit_active(name: &str) -> bool {
    fn call(name: &str) -> std::result::Result<bool, zbus::Error> {
        let connection = Connection::system()?;
        let manager = Proxy::new(
            &connection,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )?;
        let path: OwnedObjectPath = manager.call("GetUnit", &(name,))?;
        let properties = Proxy::new(
            &connection,
            "org.freedesktop.systemd1",
            path.as_str(),
            "org.freedesktop.DBus.Properties",
        )?;
        let state: zbus::zvariant::OwnedValue =
            properties.call("Get", &("org.freedesktop.systemd1.Unit", "ActiveState"))?;
        let state: String = state
            .try_into()
            .map_err(|_| zbus::Error::Failure("invalid ActiveState".into()))?;
        Ok(state == "active")
    }
    call(name).unwrap_or(false)
}

fn action_healthy(validator: &ActionValidator, running: &str) -> bool {
    match validator {
        ActionValidator::DesktopCapability {
            binaries,
            desktop_ids,
            ..
        } => {
            let root = Path::new(running);
            binaries
                .iter()
                .all(|binary| root.join("sw/bin").join(binary).exists())
                && desktop_ids
                    .iter()
                    .all(|desktop| root.join("sw/share/applications").join(desktop).is_file())
        }
        ActionValidator::PostgresqlUnixReadiness => {
            unit_active("postgresql.service")
                && UnixStream::connect("/run/postgresql/.s.PGSQL.5432").is_ok()
        }
        ActionValidator::PostgresqlStopped => {
            !unit_active("postgresql.service")
                && !Path::new("/run/postgresql/.s.PGSQL.5432").exists()
        }
        // The fixed runtime power executor is not yet installed. Refuse to
        // claim a persisted preference was applied to hardware.
        ActionValidator::PowerProfileSupportedAndApplied { .. } => false,
    }
}

fn observation(plan: &Plan) -> Result<Observation> {
    let intake = native_intake()?;
    let evidence = intake.evidence();
    let units = evidence.health.units.iter().map(|unit| UnitHealth {
        name: unit.name.clone(), active: unit.active(), failed: unit.failed(),
    }).collect();
    let mounts = evidence.health.mounts.iter().filter(|mount| mount.writable)
        .map(|mount| mount.path.clone()).collect();
    Ok(Observation {
        identity: evidence.identity.clone(),
        running: evidence.running.path.clone(),
        profile: evidence.profile.path.clone(),
        boot: evidence.boot.path.clone(),
        managed_sha256: managed_sha256(&evidence.running.path)?,
        model: None,
        system_bus: evidence.health.core_baseline_healthy,
        mounts,
        units,
        apis: vec![
            ApiHealth { api: Api::Executor, uid: None, healthy: executor_health() },
            ApiHealth { api: Api::Graph, uid: None, healthy: graph_health() },
        ],
        user_units: observed_user_units(plan)?,
        validated_actions: if evidence.running.path
            == match &plan.candidate {
                Candidate::System { closure } => closure.path.as_str(),
                Candidate::ModelOnly { .. } => plan.prior.running.path.as_str(),
            }
        {
            plan.health
                .action_validators
                .iter()
                .filter(|validator| action_healthy(validator, &evidence.running.path))
                .cloned()
                .collect()
        } else {
            vec![]
        },
        retained_guard_sha256: Some(evidence.guard.sha256.clone()),
    })
}

fn private_root(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Integrity)?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(path).map_err(|_| Error::Integrity)? != path
    {
        return Err(Error::Integrity);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path).and_then(|file| file.sync_all()).map_err(|_| Error::Adapter)
}

fn retain(plan: &Plan) -> Result<()> {
    private_root(Path::new(RECOVERY_ROOTS))?;
    let directory = Path::new(RECOVERY_ROOTS).join(&plan.transaction_id);
    fs::create_dir(&directory).map_err(|_| Error::Adapter)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(|_| Error::Adapter)?;
    private_root(&directory)?;
    let mut roots: Vec<(&str, &str)> = vec![
        ("prior-running", &plan.prior.running.path),
        ("prior-profile", &plan.prior.profile.path),
        ("prior-boot", &plan.prior.boot.path),
    ];
    if let Candidate::System { closure } = &plan.candidate {
        roots.push(("candidate", &closure.path));
    }
    for (name, target) in roots {
        symlink(target, directory.join(name)).map_err(|_| Error::Adapter)?;
    }
    sync_directory(&directory)?;
    sync_directory(Path::new(RECOVERY_ROOTS))
}

fn disarm(plan: &Plan) -> Result<()> {
    crate::retention::retain_committed(plan)?;
    let directory = Path::new(RECOVERY_ROOTS).join(&plan.transaction_id);
    match fs::symlink_metadata(&directory) {
        Ok(_) => private_root(&directory)?,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(Error::Integrity),
    }
    for name in ["prior-running", "prior-profile", "prior-boot", "candidate"] {
        let path = directory.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(path).map_err(|_| Error::Adapter)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            _ => return Err(Error::Integrity),
        }
    }
    fs::remove_dir(&directory).map_err(|_| Error::Adapter)?;
    sync_directory(Path::new(RECOVERY_ROOTS))
}

fn terminate_group(child: &mut Child) {
    if let Ok(pid) = i32::try_from(child.id()) {
        // Every fixed effect is started in its own process group. Kill that
        // group, not the retained guard, before deterministic recovery.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    } else {
        let _ = child.kill();
    }
    let _ = child.wait();
}
fn run_command(command: &mut Command) -> Result<()> {
    const EFFECT_TIMEOUT_MS: u64 = 60_000;
    command.process_group(0);
    let deadline = native::boot_time_ms()?
        .checked_add(EFFECT_TIMEOUT_MS).ok_or(Error::Expired)?;
    let mut child = command.spawn().map_err(|_| Error::Adapter)?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(_)) => return Err(Error::Adapter),
            Ok(None) => {}
            Err(_) => {
                terminate_group(&mut child);
                return Err(Error::Adapter);
            }
        }
        let now = match native::boot_time_ms() {
            Ok(now) => now,
            Err(error) => {
                terminate_group(&mut child);
                return Err(error);
            }
        };
        if now >= deadline {
            terminate_group(&mut child);
            return Err(Error::Expired);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn primary_gid(uid: u32) -> Result<u32> {
    let mut record = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut result = std::ptr::null_mut();
    let mut buffer = [0i8; 16 * 1024];
    let status = unsafe {
        libc::getpwuid_r(
            uid,
            record.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() {
        return Err(Error::Authority);
    }
    let record = unsafe { record.assume_init() };
    if record.pw_uid != uid || record.pw_gid == 0 {
        return Err(Error::Authority);
    }
    Ok(record.pw_gid)
}

fn user_runtime(uid: u32) -> Result<PathBuf> {
    let directory = PathBuf::from(format!("/run/user/{uid}"));
    let metadata = fs::symlink_metadata(&directory).map_err(|_| Error::Integrity)?;
    if !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || fs::canonicalize(&directory).map_err(|_| Error::Integrity)? != directory
    {
        return Err(Error::Integrity);
    }
    let bus = fs::symlink_metadata(directory.join("bus")).map_err(|_| Error::Integrity)?;
    if !bus.file_type().is_socket() || bus.uid() != uid {
        return Err(Error::Integrity);
    }
    Ok(directory)
}


fn adapter_stage<T>(name: &'static str, result: Result<T>) -> Result<T> {
    result.map_err(|error| {
        eprintln!(
            "{}",
            serde_json::json!({
                "schema_version": 1,
                "adapter_stage_failed": name,
                "reason": format!("{error:?}"),
            })
        );
        error
    })
}

impl NativeAdapter {
    fn construct(plan: Plan, recovery: bool, developer: bool) -> Result<Self> {
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(Error::Authority);
        }
        let intake = adapter_stage("intake", native_intake())?;
        if recovery {
            adapter_stage("recovery-artifacts", intake.verify_recovery_artifacts(&plan))?;
        } else {
            adapter_stage("plan-artifacts", intake.verify_plan_artifacts(&plan))?;
        }
        let Candidate::System { closure: candidate } = &plan.candidate else {
            return Err(Error::Adapter);
        };
        if adapter_stage("candidate-managed", managed_sha256(&candidate.path))?
            != plan.managed_sha256
        {
            return Err(Error::Integrity);
        }
        let executable = adapter_stage(
            "prior-nix-resolution",
            fs::canonicalize(Path::new(&plan.prior.running.path).join("sw/bin/nix-env"))
                .map_err(|_| Error::Integrity),
        )?;
        if !executable.starts_with("/nix/store/") {
            return Err(Error::Integrity);
        }
        adapter_stage("prior-nix-hash", hash_file(&executable, 64 * 1024 * 1024))?;
        let package = executable
            .parent()
            .and_then(Path::parent)
            .and_then(Path::to_str)
            .ok_or(Error::Integrity)?;
        let nix = adapter_stage(
            "retained-nix",
            RetainedNix::from_store_package(package),
        )?;
        let observed = adapter_stage("observation", observation(&plan))?;
        if observed.identity != plan.identity
            || (!recovery
                && (observed.running != plan.prior.running.path
                    || observed.profile != plan.prior.profile.path
                    || observed.boot != plan.prior.boot.path
                    || observed.managed_sha256 != plan.prior.managed_sha256))
            || (recovery
                && ![plan.prior.managed_sha256.as_str(), plan.managed_sha256.as_str()]
                    .contains(&observed.managed_sha256.as_str()))
        {
            return Err(Error::TargetMismatch);
        }
        let guard = adapter_stage(
            "service-identity",
            GuardServiceIdentity::capture(&plan.transaction_id, developer),
        )?;
        Ok(Self { plan, nix, guard })
    }

    pub fn new(plan: Plan) -> Result<Self> {
        Self::construct(plan, false, false)
    }

    pub(crate) fn recovering(plan: Plan) -> Result<Self> {
        Self::construct(plan, true, false)
    }

    pub(crate) fn new_for_developer(plan: Plan) -> Result<Self> {
        Self::construct(plan, false, true)
    }

    pub(crate) fn recovering_for_developer(plan: Plan) -> Result<Self> {
        Self::construct(plan, true, true)
    }

    fn command(&self, effect: &Effect) -> Result<()> {
        let Some(spec) = activation::command(&self.plan, effect, &self.nix)? else {
            return Err(Error::Adapter);
        };
        let mut command = Command::new(&spec.executable);
        command.args(&spec.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if spec.clear_environment { command.env_clear(); }
        command.envs(&spec.environment);
        run_command(&mut command)
    }

    fn activate_user_units(&self, candidate: bool) -> Result<()> {
        let units = if candidate {
            &self.plan.health.required_user_units
        } else {
            &self.plan.health.baseline_user_units
        };
        if units.is_empty() {
            return Ok(());
        }
        let closure = if candidate {
            match &self.plan.candidate {
                Candidate::System { closure } => closure.path.as_str(),
                Candidate::ModelOnly { .. } => return Err(Error::InvalidPlan),
            }
        } else {
            self.plan.prior.running.path.as_str()
        };
        let executable = fs::canonicalize(Path::new(closure).join("sw/bin/systemctl"))
            .map_err(|_| Error::Integrity)?;
        if !executable.starts_with("/nix/store/")
            || executable.file_name().and_then(|name| name.to_str()) != Some("systemctl")
        {
            return Err(Error::Integrity);
        }
        hash_file(&executable, 64 * 1024 * 1024)?;
        let mut grouped = BTreeMap::<u32, Vec<&str>>::new();
        for unit in units {
            grouped.entry(unit.uid).or_default().push(&unit.name);
        }
        for (uid, names) in grouped {
            let gid = primary_gid(uid)?;
            let runtime = user_runtime(uid)?;
            let address = format!("unix:path={}/bus", runtime.display());
            for args in [
                vec!["--user", "daemon-reload"],
                std::iter::once("--user")
                    .chain(std::iter::once("restart"))
                    .chain(names.iter().copied())
                    .collect(),
            ] {
                let mut command = Command::new(&executable);
                command.args(args)
                    .env_clear()
                    .env("HOME", "/")
                    .env("LANG", "C.UTF-8")
                    .env("XDG_RUNTIME_DIR", &runtime)
                    .env("DBUS_SESSION_BUS_ADDRESS", &address)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                // The child must not retain the guard's root supplementary,
                // real, effective or saved credentials. These three syscalls
                // are async-signal-safe between fork and exec.
                unsafe {
                    command.pre_exec(move || {
                        if libc::setgroups(0, std::ptr::null()) != 0
                            || libc::setresgid(gid, gid, gid) != 0
                            || libc::setresuid(uid, uid, uid) != 0
                        {
                            return Err(std::io::Error::last_os_error());
                        }
                        Ok(())
                    });
                }
                run_command(&mut command)?;
            }
        }
        Ok(())
    }
}

impl Adapter for NativeAdapter {
    fn observe(&mut self) -> Result<Observation> {
        self.guard.verify(&self.plan.transaction_id)?;
        observation(&self.plan)
    }

    fn apply(&mut self, effect: &Effect) -> Result<()> {
        self.guard.verify(&self.plan.transaction_id)?;
        match effect {
            Effect::RetainClosures => retain(&self.plan),
            Effect::ArmGuard { transaction_id, executable_sha256, .. }
                if transaction_id == &self.plan.transaction_id
                    && executable_sha256 == &self.plan.retained_guard_sha256 => Ok(()),
            Effect::TestSystem { .. } => {
                self.command(effect)?;
                self.guard.verify(&self.plan.transaction_id)?;
                self.activate_user_units(true)
            }
            Effect::RecoverSystem { .. } => {
                self.command(effect)?;
                self.guard.verify(&self.plan.transaction_id)?;
                self.activate_user_units(false)
            }
            Effect::SetProfile { .. } | Effect::InstallBoot { .. } => self.command(effect),
            Effect::PublishManaged { artifact_sha256 } => {
                let observed = observation(&self.plan)?;
                if &observed.managed_sha256 == artifact_sha256 { Ok(()) } else { Err(Error::TargetMismatch) }
            }
            Effect::DisarmGuard { transaction_id } if transaction_id == &self.plan.transaction_id => disarm(&self.plan),
            Effect::TestModel { .. } | Effect::RecoverModel { .. } | Effect::ArmGuard { .. }
                | Effect::DisarmGuard { .. } => Err(Error::Adapter),
        }
    }

    fn boot_time_ms(&mut self) -> Result<u64> { native::boot_time_ms() }
}

#[cfg(test)]
mod tests {
    use super::guard_fragment_path;
    use std::path::Path;

    #[test]
    fn guard_fragment_requires_exact_store_unit_kind() {
        let product = Path::new(
            "/nix/store/abc-aios-guard/lib/systemd/system/aios-guard@.service",
        );
        let developer = Path::new(
            "/nix/store/def-aios-dev-deploy/lib/systemd/system/aios-dev-guard@.service",
        );
        assert!(guard_fragment_path(product, false));
        assert!(guard_fragment_path(developer, true));
        assert!(!guard_fragment_path(product, true));
        assert!(!guard_fragment_path(developer, false));
        assert!(!guard_fragment_path(
            Path::new("/etc/systemd/system/aios-guard@.service"),
            false,
        ));
    }
}
