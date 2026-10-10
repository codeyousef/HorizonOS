//! VM-only developer deployment guard.
//!
//! This is deliberately separate from product authorization. It consumes only
//! a root-owned handoff written by the installed developer helper, then reuses
//! the same native activation, health, timeout, rollback, and durable guard
//! engine as product transactions.
use crate::{
    Adapter, Api, ApiHealth, Candidate, Engine, Error, HealthPolicy, Heartbeat, Identity, Ledger,
    Plan, Prior, Result, State, UnitHealth,
    native::NativeIntake,
    runtime::{NativeAdapter, managed_sha256, user_units},
};
use aios_exec::ledger::Ledger as BrokerLedger;
use serde::Deserialize;
use serde_json::json;
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    thread,
    time::Duration,
};
use uuid::Uuid;

const ROOT: &str = "/var/lib/aios/development";
const MAX_HANDOFF: u64 = 16 * 1024;
const MAX_CONTROL: usize = 64 * 1024;

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Test,
    Commit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    schema_version: u32,
    transaction_id: String,
    registration_id: String,
    developer_uid: u32,
    operation: Operation,
    identity: Identity,
    source_digest: String,
    candidate_digest: String,
    candidate_closure: String,
}

fn uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.hyphenated().to_string() == value)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn store(value: &str) -> bool {
    let Some(name) = value.strip_prefix("/nix/store/") else {
        return false;
    };
    let Some((hash, label)) = name.split_once('-') else {
        return false;
    };
    hash.len() == 32
        && hash
            .bytes()
            .all(|byte| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&byte))
        && !label.is_empty()
        && label.len() <= 192
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn protected_directory(path: &Path, mode: u32) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Integrity)?;
    if !metadata.is_dir()
        || metadata.uid() != 0
        || metadata.mode() & 0o7777 != mode
        || fs::canonicalize(path).map_err(|_| Error::Integrity)? != path
    {
        return Err(Error::Integrity);
    }
    Ok(())
}

fn read_handoff(id: &str) -> Result<Handoff> {
    if !uuid(id) {
        return Err(Error::InvalidPlan);
    }
    protected_directory(Path::new("/var/lib"), 0o755)?;
    protected_directory(Path::new("/var/lib/aios"), 0o755)?;
    protected_directory(Path::new(ROOT), 0o700)?;
    let directory = Path::new(ROOT).join("handoffs");
    protected_directory(&directory, 0o700)?;
    let path = directory.join(format!("{id}.json"));
    let metadata = fs::symlink_metadata(&path).map_err(|_| Error::Integrity)?;
    if !metadata.is_file()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() == 0
        || metadata.len() > MAX_HANDOFF
    {
        return Err(Error::Integrity);
    }
    let mut file = File::open(&path).map_err(|_| Error::Integrity)?;
    let opened = file.metadata().map_err(|_| Error::Integrity)?;
    if (opened.dev(), opened.ino()) != (metadata.dev(), metadata.ino()) {
        return Err(Error::Integrity);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes).map_err(|_| Error::Integrity)?;
    let handoff: Handoff = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidPlan)?;
    if handoff.schema_version != 1
        || handoff.transaction_id != id
        || !uuid(&handoff.registration_id)
        || handoff.developer_uid == 0
        || !digest(&handoff.source_digest)
        || !digest(&handoff.candidate_digest)
        || !store(&handoff.candidate_closure)
        || handoff.identity.role != "development"
        || handoff.identity.disk_serial != "AIOS_DEV_ROOT"
        || handoff.identity.management_channel != "ssh-development"
    {
        return Err(Error::InvalidPlan);
    }
    Ok(handoff)
}

fn plan(handoff: &Handoff, intake: &NativeIntake) -> Result<Plan> {
    let evidence = intake.evidence();
    if handoff.identity != evidence.identity {
        return Err(Error::TargetMismatch);
    }
    let candidate = intake.closure_for(&handoff.candidate_closure)?;
    let required_mounts = evidence
        .health
        .mounts
        .iter()
        .filter(|mount| mount.writable)
        .map(|mount| mount.path.clone())
        .collect();
    let baseline_units = evidence
        .health
        .units
        .iter()
        .map(|unit| UnitHealth {
            name: unit.name.clone(),
            active: unit.active(),
            failed: unit.failed(),
        })
        .collect();
    let (baseline_user_units, required_user_units) =
        user_units(handoff.developer_uid, &candidate.path)?;
    let value = Plan {
        schema_version: 1,
        transaction_id: handoff.transaction_id.clone(),
        identity: evidence.identity.clone(),
        source_digest: handoff.source_digest.clone(),
        candidate_digest: handoff.candidate_digest.clone(),
        nixpkgs_revision: crate::NIXPKGS_REVISION.into(),
        prior: Prior {
            running: evidence.running.clone(),
            profile: evidence.profile.clone(),
            boot: evidence.boot.clone(),
            managed_sha256: managed_sha256(&evidence.running.path)?,
            model: None,
        },
        candidate: Candidate::System { closure: candidate },
        managed_sha256: managed_sha256(&handoff.candidate_closure)?,
        retained_guard_sha256: evidence.guard.sha256.clone(),
        guard_timeout_seconds: 180,
        health: HealthPolicy {
            required_mounts,
            baseline_units,
            required_apis: vec![
                ApiHealth {
                    api: Api::Executor,
                    uid: None,
                    healthy: true,
                },
                ApiHealth {
                    api: Api::Graph,
                    uid: None,
                    healthy: true,
                },
            ],
            baseline_user_units,
            required_user_units,
            action_validators: vec![],
        },
    };
    value.validate()?;
    intake.verify_plan_artifacts(&value)?;
    Ok(value)
}

fn runtime_directory(id: &str) -> Result<PathBuf> {
    let path = Path::new("/run/aios-dev-guard").join(id);
    protected_directory(&path, 0o700)?;
    Ok(path)
}

fn peer(stream: &UnixStream) -> Result<()> {
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            std::os::fd::AsRawFd::as_raw_fd(stream),
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
    Ok(())
}

fn read_frame(stream: &mut UnixStream) -> Result<Vec<u8>> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).map_err(|_| Error::Adapter)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_CONTROL {
        return Err(Error::InvalidPlan);
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(|_| Error::Adapter)?;
    Ok(bytes)
}
fn write_frame(stream: &mut UnixStream, value: &serde_json::Value) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Adapter)?;
    if bytes.len() > MAX_CONTROL {
        return Err(Error::Adapter);
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|_| Error::Adapter)?;
    stream.write_all(&bytes).map_err(|_| Error::Adapter)
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Control {
    Status { transaction_id: String },
    Heartbeat { heartbeat: Heartbeat },
    CompleteTest { transaction_id: String },
}

fn control(
    stream: &mut UnixStream,
    operation: Operation,
    engine: &mut Engine,
    guard: &mut Ledger,
    adapter: &mut NativeAdapter,
    plan: &Plan,
) -> Result<Option<State>> {
    peer(stream)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| Error::Adapter)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| Error::Adapter)?;
    let request: Control =
        serde_json::from_slice(&read_frame(stream)?).map_err(|_| Error::InvalidPlan)?;
    match request {
        Control::Status { transaction_id } if transaction_id == plan.transaction_id => {
            write_frame(
                stream,
                &json!({"schema_version":1,"transaction_id":plan.transaction_id,
                    "state":engine.state(),"plan_digest":plan.digest()?,
                    "candidate_digest":plan.candidate_digest,"identity":plan.identity,
                    "nonce":engine.challenge().transport_value()}),
            )?;
            Ok(None)
        }
        Control::Heartbeat { heartbeat } if operation == Operation::Commit => {
            let now = adapter.boot_time_ms()?;
            engine.heartbeat(heartbeat, now)?;
            let state = engine.commit(guard, adapter, now)?;
            write_frame(stream, &json!({"schema_version":1,"state":state}))?;
            Ok(Some(state))
        }
        Control::CompleteTest { transaction_id }
            if operation == Operation::Test && transaction_id == plan.transaction_id =>
        {
            let state = engine.recover(guard, adapter)?;
            write_frame(stream, &json!({"schema_version":1,"state":state}))?;
            Ok(Some(state))
        }
        _ => Err(Error::Heartbeat),
    }
}

fn reconcile(guard: &mut Ledger, id: &str) -> Result<()> {
    let persisted = guard.plan(id)?;
    let mut adapter = NativeAdapter::recovering(persisted)?;
    match Engine::reconcile(guard, id, &mut adapter) {
        Ok(State::Committed | State::RolledBack | State::Rejected) => Ok(()),
        Ok(State::RecoveryRequired) | Err(Error::RecoveryRequired) => Err(Error::RecoveryRequired),
        Ok(_) => Err(Error::State),
        Err(error) => Err(error),
    }
}

pub fn run(id: &str) -> Result<()> {
    if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
        return Err(Error::Authority);
    }
    let handoff = read_handoff(id)?;
    let connection = BrokerLedger::guard_database().map_err(|_| Error::Ledger)?;
    let mut guard = Ledger::new(connection)?;
    if guard.contains(id)? {
        return reconcile(&mut guard, id);
    }
    let intake = NativeIntake::capture()?;
    let plan = plan(&handoff, &intake)?;
    let mut adapter = NativeAdapter::new(plan.clone())?;
    let mut engine = Engine::register(plan.clone(), &mut guard)?;
    let now = adapter.boot_time_ms()?;
    engine.arm(&mut guard, &mut adapter, now)?;
    let test_time = adapter.boot_time_ms()?;
    match engine.test(&mut guard, &mut adapter, test_time) {
        Ok(State::Verifying) => {}
        Ok(State::RolledBack) => return Ok(()),
        Ok(State::RecoveryRequired) | Err(Error::RecoveryRequired) => {
            return Err(Error::RecoveryRequired);
        }
        Ok(_) => return Err(Error::State),
        Err(error) => return Err(error),
    }

    let directory = runtime_directory(id)?;
    let socket = directory.join("control.sock");
    match fs::symlink_metadata(&socket) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err(Error::Integrity),
    }
    let listener = UnixListener::bind(&socket).map_err(|_| Error::Adapter)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(|_| Error::Adapter)?;
    listener.set_nonblocking(true).map_err(|_| Error::Adapter)?;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => match control(
                &mut stream,
                handoff.operation,
                &mut engine,
                &mut guard,
                &mut adapter,
                &plan,
            ) {
                Ok(Some(_)) => return Ok(()),
                Ok(None)
                | Err(Error::Heartbeat)
                | Err(Error::Authority)
                | Err(Error::InvalidPlan) => {}
                Err(error) => return Err(error),
            },
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err(Error::Adapter),
        }
        let now = adapter.boot_time_ms()?;
        match engine.tick(&mut guard, &mut adapter, now) {
            Ok(State::RolledBack) => return Ok(()),
            Ok(State::RecoveryRequired) | Err(Error::RecoveryRequired) => {
                return Err(Error::RecoveryRequired);
            }
            Ok(_) => {}
            Err(error) => return Err(error),
        }
        thread::sleep(Duration::from_millis(50));
    }
}
