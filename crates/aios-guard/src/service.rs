//! Independent transaction service. The only input is a root-owned handoff
//! already bound to a consumed native authorization and broker ledger entry.
use crate::{
    ActionValidator, Adapter, Api, ApiHealth, Candidate, Engine, Error, HealthPolicy, Heartbeat,
    Ledger, Plan, Prior, Result, State, UnitHealth,
    native::NativeIntake,
    runtime::{NativeAdapter, managed_sha256, user_units},
};
use aios_exec::{
    guard::GuardHandoff,
    ledger::{Ledger as BrokerLedger, State as BrokerState},
};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

const MAX_CONTROL: usize = 64 * 1024;

fn executor<T>(value: aios_exec::Result<T>) -> Result<T> {
    value.map_err(|error| match error {
        aios_exec::Error::Authority => Error::Authority,
        aios_exec::Error::TargetChanged => Error::TargetMismatch,
        aios_exec::Error::Ownership | aios_exec::Error::Integrity => Error::Integrity,
        aios_exec::Error::Ledger => Error::Ledger,
        aios_exec::Error::Conflict => Error::Conflict,
        aios_exec::Error::State => Error::State,
        _ => Error::Adapter,
    })
}

fn stage<T>(name: &'static str, result: Result<T>) -> Result<T> {
    result.map_err(|error| {
        eprintln!(
            "{}",
            json!({"schema_version":1,"guard_stage_failed":name,"reason":format!("{error:?}")})
        );
        error
    })
}

fn plan(handoff: &GuardHandoff, broker: &BrokerLedger, intake: &NativeIntake) -> Result<Plan> {
    let status = stage(
        "plan-broker-status",
        executor(broker.status(&handoff.transaction_id, handoff.requester_uid)),
    )?;
    if status.state != BrokerState::Authorized
        || status.final_plan_sha256.as_deref() != Some(handoff.final_plan_sha256.as_str())
    {
        return Err(Error::Authority);
    }
    let prepared = stage(
        "plan-prepared",
        executor(broker.get_plan(&handoff.transaction_id, handoff.requester_uid)),
    )?;
    let final_plan = stage(
        "plan-final",
        executor(broker.final_plan(&handoff.transaction_id, handoff.requester_uid)),
    )?;
    if final_plan.reboot_required
        || final_plan.semantic_preview.candidate_closure.is_some()
        || final_plan.candidate_sha256 != prepared.candidate_sha256
    {
        return Err(Error::InvalidPlan);
    }
    let evidence = intake.evidence();
    let candidate = stage(
        "plan-candidate-closure",
        intake.closure_for(&final_plan.build.closure),
    )?;
    let mounts = evidence
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
    let action_validators = final_plan
        .semantic_preview
        .validators
        .iter()
        .map(|validator| {
            let value = serde_json::to_value(validator).map_err(|_| Error::InvalidPlan)?;
            if value.get("kind").and_then(|kind| kind.as_str())
                == Some("power_profile_supported_and_applied")
            {
                let policy = &final_plan.semantic_preview.candidate_manifest.power_policy;
                let profile = |value| {
                    serde_json::to_value(value)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .ok_or(Error::InvalidPlan)
                };
                Ok(ActionValidator::PowerProfileSupportedAndApplied {
                    profile_on_ac: profile(&policy.profile_on_ac)?,
                    profile_on_battery: profile(&policy.profile_on_battery)?,
                })
            } else {
                serde_json::from_value(value).map_err(|_| Error::InvalidPlan)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let (baseline_user_units, required_user_units) = stage(
        "plan-user-units",
        user_units(handoff.requester_uid, &candidate.path),
    )?;
    let prior_running = stage(
        "plan-prior-running",
        intake.closure_for(&prepared.baseline.running_closure),
    )?;
    let prior_profile = stage(
        "plan-prior-profile",
        intake.closure_for(&prepared.baseline.profile_closure),
    )?;
    let prior_boot = stage(
        "plan-prior-boot",
        intake.closure_for(&prepared.baseline.boot_selected_closure),
    )?;
    let prior_managed_sha256 = stage(
        "plan-prior-managed",
        managed_sha256(&prepared.baseline.running_closure),
    )?;
    let plan = Plan {
        schema_version: 1,
        transaction_id: handoff.transaction_id.clone(),
        identity: crate::Identity {
            installation_uuid: prepared.target.installation_uuid.clone(),
            dmi_uuid: prepared.target.dmi_uuid.clone(),
            machine_id: prepared.target.machine_id.clone(),
            boot_id: prepared.target.boot_id.clone(),
            role: prepared.target.role.clone(),
            disk_serial: prepared.target.disk_serial.clone(),
            management_channel: prepared.target.management_channel.clone(),
        },
        source_digest: prepared.candidate_sha256,
        candidate_digest: handoff.final_plan_sha256.clone(),
        nixpkgs_revision: crate::NIXPKGS_REVISION.into(),
        prior: Prior {
            running: prior_running,
            profile: prior_profile,
            boot: prior_boot,
            managed_sha256: prior_managed_sha256,
            model: None,
        },
        candidate: Candidate::System { closure: candidate },
        managed_sha256: final_plan.semantic_preview.candidate_manifest_sha256,
        retained_guard_sha256: evidence.guard.sha256.clone(),
        guard_timeout_seconds: 180,
        health: HealthPolicy {
            required_mounts: mounts,
            baseline_units,
            required_apis: vec![
                ApiHealth { api: Api::Executor, uid: None, healthy: true },
                ApiHealth { api: Api::Graph, uid: None, healthy: true },
            ],
            baseline_user_units,
            required_user_units,
            action_validators,
        },
    };
    stage("plan-validation", plan.validate())?;
    stage("plan-artifacts", intake.verify_plan_artifacts(&plan))?;
    Ok(plan)
}

fn runtime_directory(id: &str) -> Result<PathBuf> {
    let path = Path::new("/run/aios-guard").join(id);
    let metadata = fs::symlink_metadata(&path).map_err(|_| Error::Integrity)?;
    if !metadata.is_dir()
        || metadata.uid() != 0
        || metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(&path).map_err(|_| Error::Integrity)? != path
    {
        return Err(Error::Integrity);
    }
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
    stream.write_all(&(bytes.len() as u32).to_be_bytes()).map_err(|_| Error::Adapter)?;
    stream.write_all(&bytes).map_err(|_| Error::Adapter)
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Control {
    Status { transaction_id: String },
    Heartbeat { heartbeat: Heartbeat },
    Recover { transaction_id: String },
}

fn control(
    stream: &mut UnixStream,
    engine: &mut Engine,
    guard: &mut Ledger,
    adapter: &mut NativeAdapter,
    plan: &Plan,
) -> Result<Option<State>> {
    peer(stream)?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_| Error::Adapter)?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_| Error::Adapter)?;
    let request: Control = serde_json::from_slice(&read_frame(stream)?).map_err(|_| Error::InvalidPlan)?;
    match request {
        Control::Status { transaction_id } if transaction_id == plan.transaction_id => {
            write_frame(
                stream,
                &json!({"schema_version":1,"transaction_id":plan.transaction_id,
                    "state":engine.state(),"plan_digest":plan.digest()?,
                    "candidate_digest":plan.candidate_digest,
                    "identity":plan.identity,"nonce":engine.challenge().transport_value()}),
            )?;
            Ok(None)
        }
        Control::Heartbeat { heartbeat } => {
            let now = adapter.boot_time_ms()?;
            engine.heartbeat(heartbeat, now)?;
            let state = engine.commit(guard, adapter, now)?;
            write_frame(stream, &json!({"schema_version":1,"state":state}))?;
            Ok(Some(state))
        }
        Control::Recover { transaction_id } if transaction_id == plan.transaction_id => {
            let state = engine.recover(guard, adapter)?;
            write_frame(stream, &json!({"schema_version":1,"state":state}))?;
            Ok(Some(state))
        }
        _ => Err(Error::Heartbeat),
    }
}

fn broker_terminal(broker: &mut BrokerLedger, handoff: &GuardHandoff, state: State) -> Result<()> {
    let next = match state {
        State::Committed => BrokerState::Committed,
        State::RolledBack => BrokerState::RolledBack,
        State::Rejected => BrokerState::Rejected,
        State::RecoveryRequired => BrokerState::RecoveryRequired,
        _ => return Err(Error::State),
    };
    executor(broker.guard_complete(
        &handoff.transaction_id,
        handoff.requester_uid,
        next,
    ))?;
    Ok(())
}

fn recover_existing(
    handoff: &GuardHandoff,
    broker: &mut BrokerLedger,
    guard: &mut Ledger,
) -> Result<()> {
    let persisted = guard.plan(&handoff.transaction_id)?;
    let mut adapter = NativeAdapter::recovering(persisted)?;
    match Engine::reconcile(guard, &handoff.transaction_id, &mut adapter) {
        Ok(state) => broker_terminal(broker, handoff, state),
        Err(Error::RecoveryRequired) => {
            broker_terminal(broker, handoff, State::RecoveryRequired)
        }
        Err(error) => Err(error),
    }
}

pub fn run(id: &str) -> Result<()> {
    if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
        return Err(Error::Authority);
    }
    let handoff = stage("handoff", executor(aios_exec::guard::read(id)))?;
    let mut broker = stage("broker-ledger", executor(BrokerLedger::open()))?;
    let connection = stage("guard-database", executor(BrokerLedger::guard_database()))?;
    let mut guard = stage("guard-ledger", Ledger::new(connection))?;
    if stage("guard-ledger-lookup", guard.contains(id))? {
        return stage(
            "recovery-reconciliation",
            recover_existing(&handoff, &mut broker, &mut guard),
        );
    }
    let intake = stage("native-intake", NativeIntake::capture())?;
    let plan = stage("plan-assembly", plan(&handoff, &broker, &intake))?;
    let mut adapter = match stage("adapter-construction", NativeAdapter::new(plan.clone())) {
        Ok(adapter) => adapter,
        Err(_) => {
            // No guard effect or durable guard intent exists yet. Close the
            // consumed authorization rather than leaving an AUTHORIZED plan
            // wedged after an intervening target or baseline change.
            broker_terminal(&mut broker, &handoff, State::Rejected)?;
            return Ok(());
        }
    };
    let mut engine = stage("guard-registration", Engine::register(plan.clone(), &mut guard))?;
    let now = stage("guard-clock", adapter.boot_time_ms())?;
    if let Err(error) = engine.arm(&mut guard, &mut adapter, now) {
        if engine.state() == State::Rejected {
            broker_terminal(&mut broker, &handoff, State::Rejected)?;
        } else if engine.state() == State::RecoveryRequired {
            broker_terminal(&mut broker, &handoff, State::RecoveryRequired)?;
        }
        return Err(error);
    }
    let test_time = adapter.boot_time_ms()?;
    match engine.test(&mut guard, &mut adapter, test_time) {
        Ok(State::Verifying) => {}
        Ok(State::RolledBack) => {
            broker_terminal(&mut broker, &handoff, State::RolledBack)?;
            return Ok(());
        }
        Ok(State::RecoveryRequired) | Err(Error::RecoveryRequired) => {
            broker_terminal(&mut broker, &handoff, State::RecoveryRequired)?;
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
            Ok((mut stream, _)) => match control(&mut stream, &mut engine, &mut guard, &mut adapter, &plan) {
                Ok(Some(state)) => {
                    broker_terminal(&mut broker, &handoff, state)?;
                    return Ok(());
                }
                Ok(None) | Err(Error::Heartbeat) | Err(Error::Authority) | Err(Error::InvalidPlan) => {}
                Err(error) => return Err(error),
            },
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err(Error::Adapter),
        }
        let now = adapter.boot_time_ms()?;
        match engine.tick(&mut guard, &mut adapter, now) {
            Ok(State::RolledBack) => {
                broker_terminal(&mut broker, &handoff, State::RolledBack)?;
                return Ok(());
            }
            Ok(State::RecoveryRequired) | Err(Error::RecoveryRequired) => {
                broker_terminal(&mut broker, &handoff, State::RecoveryRequired)?;
                return Err(Error::RecoveryRequired);
            }
            Ok(_) => {}
            Err(error) => return Err(error),
        }
        thread::sleep(Duration::from_millis(50));
    }
}
