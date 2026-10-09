//! Root-only fixed activation adapter. Construction captures installed native
//! authority before any effect; callers cannot supply commands or store paths.
use crate::{
    activation::{self, RetainedNix},
    native::{self, NativeIntake},
    Adapter, Api, ApiHealth, Candidate, Effect, Error, Observation, Plan, Result, UnitHealth,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::{fs::{MetadataExt, OpenOptionsExt, PermissionsExt, symlink}, net::UnixStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use zbus::blocking::{Connection, Proxy};

const RECOVERY_ROOTS: &str = "/nix/var/nix/gcroots/aios-guard";
const GRAPH_SOCKET: &str = "/run/aios-state/owner.sock";
const MAX_GRAPH_REPLY: usize = 2 * 1024 * 1024;

/// The adapter is intentionally not serializable and has no public command API.
/// Its Plan has already crossed executor authorization before construction.
pub struct NativeAdapter {
    plan: Plan,
    nix: RetainedNix,
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

fn managed_sha256(running: &str) -> Result<String> {
    hash_file(&Path::new(running).join("etc/aios/managed.json"), 1024 * 1024)
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

fn observation() -> Result<Observation> {
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
        user_units: vec![],
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
    let directory = Path::new(RECOVERY_ROOTS).join(&plan.transaction_id);
    private_root(&directory)?;
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

impl NativeAdapter {
    pub fn new(plan: Plan) -> Result<Self> {
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(Error::Authority);
        }
        let intake = native_intake()?;
        intake.verify_plan_artifacts(&plan)?;
        let executable = PathBuf::from(&intake.evidence().nix_env.path);
        if executable.file_name().and_then(|name| name.to_str()) != Some("nix-env")
            || executable.parent().and_then(Path::parent).and_then(Path::to_str).is_none()
        {
            return Err(Error::Integrity);
        }
        let package = executable.parent().and_then(Path::parent).and_then(Path::to_str)
            .ok_or(Error::Integrity)?;
        let nix = RetainedNix::from_store_package(package)?;
        let observed = observation()?;
        if observed.identity != plan.identity
            || observed.running != plan.prior.running.path
            || observed.profile != plan.prior.profile.path
            || observed.boot != plan.prior.boot.path
            || observed.managed_sha256 != plan.prior.managed_sha256
        {
            return Err(Error::TargetMismatch);
        }
        Ok(Self { plan, nix })
    }

    fn command(&self, effect: &Effect) -> Result<()> {
        let Some(spec) = activation::command(&self.plan, effect, &self.nix)? else {
            return Err(Error::Adapter);
        };
        let mut command = Command::new(&spec.executable);
        command.args(&spec.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if spec.clear_environment { command.env_clear(); }
        command.envs(&spec.environment);
        let status = command.status().map_err(|_| Error::Adapter)?;
        if status.success() { Ok(()) } else { Err(Error::Adapter) }
    }
}

impl Adapter for NativeAdapter {
    fn observe(&mut self) -> Result<Observation> { observation() }

    fn apply(&mut self, effect: &Effect) -> Result<()> {
        match effect {
            Effect::RetainClosures => retain(&self.plan),
            Effect::ArmGuard { transaction_id, executable_sha256, .. }
                if transaction_id == &self.plan.transaction_id
                    && executable_sha256 == &self.plan.retained_guard_sha256 => Ok(()),
            Effect::TestSystem { .. } | Effect::RecoverSystem { .. }
                | Effect::SetProfile { .. } | Effect::InstallBoot { .. } => self.command(effect),
            Effect::PublishManaged { artifact_sha256 } => {
                let observed = observation()?;
                if &observed.managed_sha256 == artifact_sha256 { Ok(()) } else { Err(Error::TargetMismatch) }
            }
            Effect::DisarmGuard { transaction_id } if transaction_id == &self.plan.transaction_id => disarm(&self.plan),
            Effect::TestModel { .. } | Effect::RecoverModel { .. } | Effect::ArmGuard { .. }
                | Effect::DisarmGuard { .. } => Err(Error::Adapter),
        }
    }

    fn boot_time_ms(&mut self) -> Result<u64> { native::boot_time_ms() }
}
