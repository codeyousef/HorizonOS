//! Deterministic activation/recovery protocol. No client commands or shell API.
//! The real root activation/health adapter is not yet qualified or enabled.
pub mod activation;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, time::Duration};
use uuid::Uuid;

pub const NIXPKGS_REVISION: &str = "774debe7a0d1b496e35677ad955a1011c6ff74f3";
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPlan,
    TargetMismatch,
    Conflict,
    Ledger,
    State,
    Expired,
    Heartbeat,
    Health,
    Adapter,
    RecoveryRequired,
}
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self::Ledger
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::InvalidPlan
    }
}
fn hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn valid_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.hyphenated().to_string() == value)
}
fn valid_store(value: &str) -> bool {
    let Some(name) = value.strip_prefix("/nix/store/") else {
        return false;
    };
    let Some((digest, label)) = name.split_once('-') else {
        return false;
    };
    digest.len() == 32
        && digest
            .bytes()
            .all(|b| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&b))
        && !label.is_empty()
        && label.len() <= 192
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub installation_uuid: String,
    pub dmi_uuid: String,
    pub machine_id: String,
    pub boot_id: String,
    pub role: String,
    pub disk_serial: String,
    pub management_channel: String,
}
impl Identity {
    fn validate(&self) -> Result<()> {
        if ![&self.installation_uuid, &self.dmi_uuid, &self.boot_id]
            .iter()
            .all(|s| valid_uuid(s))
            || self.machine_id.len() != 32
            || !self
                .machine_id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || !matches!(
                self.role.as_str(),
                "development" | "production" | "acceptance"
            )
            || self.disk_serial.is_empty()
            || self.disk_serial.len() > 64
            || !self
                .disk_serial
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !matches!(
                self.management_channel.as_str(),
                "ssh-development" | "local-product"
            )
        {
            return Err(Error::InvalidPlan);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Closure {
    pub path: String,
    pub activation_sha256: String,
    pub kernel_sha256: String,
    pub initrd_sha256: String,
    pub boot_adapter_sha256: String,
}
impl Closure {
    fn validate(&self) -> Result<()> {
        if !valid_store(&self.path)
            || ![
                &self.activation_sha256,
                &self.kernel_sha256,
                &self.initrd_sha256,
                &self.boot_adapter_sha256,
            ]
            .iter()
            .all(|s| valid_hash(s))
        {
            return Err(Error::InvalidPlan);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelArtifact {
    pub store_path: String,
    pub manifest_sha256: String,
}
impl ModelArtifact {
    fn validate(&self) -> Result<()> {
        if !valid_store(&self.store_path) || !valid_hash(&self.manifest_sha256) {
            Err(Error::InvalidPlan)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prior {
    pub running: Closure,
    pub profile: Closure,
    pub boot: Closure,
    pub managed_sha256: String,
    pub model: Option<ModelArtifact>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Candidate {
    System { closure: Closure },
    ModelOnly { artifact: ModelArtifact },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitHealth {
    pub name: String,
    pub active: bool,
    pub failed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Api {
    Executor,
    Graph,
    Model,
    UserSession,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiHealth {
    pub api: Api,
    pub uid: Option<u32>,
    pub healthy: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthPolicy {
    pub required_mounts: Vec<String>,
    pub baseline_units: Vec<UnitHealth>,
    pub required_apis: Vec<ApiHealth>,
    pub required_user_units: Vec<UserUnit>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserUnit {
    pub uid: u32,
    pub name: String,
    pub expected_executable_sha256: String,
}
fn known_unit(value: &str) -> bool {
    matches!(
        value,
        "sshd.service"
            | "dbus.service"
            | "aios-state.service"
            | "aios-exec.service"
            | "aios-model.service"
            | "aios-observer.service"
            | "aios-build.service"
    )
}
impl HealthPolicy {
    fn validate(&self) -> Result<()> {
        let mut mounts = BTreeSet::new();
        let mut units = BTreeSet::new();
        let mut apis = BTreeSet::new();
        let mut users = BTreeSet::new();
        if self.required_mounts.len() > 6
            || !self.required_mounts.iter().all(|s| {
                matches!(s.as_str(), "/" | "/nix" | "/var" | "/home" | "/boot") && mounts.insert(s)
            })
            || !mounts.contains(&String::from("/"))
            || self.baseline_units.len() > 16
            || !self
                .baseline_units
                .iter()
                .all(|s| known_unit(&s.name) && !(s.active && s.failed) && units.insert(&s.name))
            || self.required_apis.len() > 16
            || !self.required_apis.iter().all(|s| {
                s.healthy
                    && (s.uid.is_some() == (s.api == Api::UserSession))
                    && apis.insert(format!("{:?}:{:?}", s.api, s.uid))
            })
            || self.required_user_units.len() > 16
            || !self.required_user_units.iter().all(|s| {
                s.uid > 0
                    && matches!(
                        s.name.as_str(),
                        "aios-sessiond.service" | "aios-indexd.service" | "aios-ui-agent.service"
                    )
                    && valid_hash(&s.expected_executable_sha256)
                    && users.insert((s.uid, &s.name))
            })
        {
            return Err(Error::InvalidPlan);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub transaction_id: String,
    pub identity: Identity,
    pub source_digest: String,
    pub candidate_digest: String,
    pub nixpkgs_revision: String,
    pub prior: Prior,
    pub candidate: Candidate,
    pub managed_sha256: String,
    pub retained_guard_sha256: String,
    pub guard_timeout_seconds: u32,
    pub health: HealthPolicy,
}
impl Plan {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !valid_uuid(&self.transaction_id)
            || self.nixpkgs_revision != NIXPKGS_REVISION
            || ![
                &self.source_digest,
                &self.candidate_digest,
                &self.managed_sha256,
                &self.prior.managed_sha256,
                &self.retained_guard_sha256,
            ]
            .iter()
            .all(|s| valid_hash(s))
            || !(60..=600).contains(&self.guard_timeout_seconds)
        {
            return Err(Error::InvalidPlan);
        }
        self.identity.validate()?;
        for closure in [&self.prior.running, &self.prior.profile, &self.prior.boot] {
            closure.validate()?;
        }
        if let Some(model) = &self.prior.model {
            model.validate()?;
        }
        match &self.candidate {
            Candidate::System { closure } => closure.validate()?,
            Candidate::ModelOnly { artifact } => {
                artifact.validate()?;
                if self.prior.model.is_none() {
                    return Err(Error::InvalidPlan);
                }
            }
        }
        self.health.validate()?;
        if !self
            .health
            .baseline_units
            .iter()
            .any(|u| u.name == "dbus.service" && u.active && !u.failed)
            || ![Api::Executor, Api::Graph].iter().all(|api| {
                self.health
                    .required_apis
                    .iter()
                    .any(|a| &a.api == api && a.uid.is_none() && a.healthy)
            })
            || (self.identity.management_channel == "ssh-development"
                && !self
                    .health
                    .baseline_units
                    .iter()
                    .any(|u| u.name == "sshd.service" && u.active && !u.failed))
            || (matches!(self.candidate, Candidate::ModelOnly { .. })
                && !self
                    .health
                    .required_apis
                    .iter()
                    .any(|a| a.api == Api::Model && a.healthy))
        {
            return Err(Error::InvalidPlan);
        }
        Ok(())
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 65536 {
            return Err(Error::InvalidPlan);
        }
        let plan: Self = serde_json::from_slice(bytes)?;
        plan.validate()?;
        Ok(plan)
    }
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(hash(&serde_json::to_vec(self)?))
    }
    pub fn needs_reboot(&self) -> bool {
        matches!(&self.candidate, Candidate::System { closure } if closure.kernel_sha256 != self.prior.running.kernel_sha256 || closure.initrd_sha256 != self.prior.running.initrd_sha256 || closure.boot_adapter_sha256 != self.prior.running.boot_adapter_sha256)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub identity: Identity,
    pub running: String,
    pub profile: String,
    pub boot: String,
    pub managed_sha256: String,
    pub model: Option<ModelArtifact>,
    pub system_bus: bool,
    pub mounts: Vec<String>,
    pub units: Vec<UnitHealth>,
    pub apis: Vec<ApiHealth>,
    pub user_units: Vec<UserUnit>,
    pub retained_guard_sha256: Option<String>,
}
impl Observation {
    fn target(&self, plan: &Plan) -> Result<()> {
        if self.identity == plan.identity {
            Ok(())
        } else {
            Err(Error::TargetMismatch)
        }
    }
    fn health(&self, plan: &Plan) -> Result<()> {
        self.target(plan)?;
        // Contradictory or repeated facts cannot supply a convenient healthy
        // entry while hiding a failing observation of the same resource.
        let mut mounts = BTreeSet::new();
        let mut units = BTreeSet::new();
        let mut apis = BTreeSet::new();
        let mut users = BTreeSet::new();
        if !self.mounts.iter().all(|m| mounts.insert(m))
            || !self
                .units
                .iter()
                .all(|u| !(u.active && u.failed) && units.insert(&u.name))
            || !self
                .apis
                .iter()
                .all(|a| apis.insert(format!("{:?}:{:?}", a.api, a.uid)))
            || !self
                .user_units
                .iter()
                .all(|u| users.insert((u.uid, &u.name)))
        {
            return Err(Error::Health);
        }
        if !self.system_bus
            || !plan
                .health
                .required_mounts
                .iter()
                .all(|m| self.mounts.contains(m))
            || !plan.health.baseline_units.iter().all(|b| {
                self.units
                    .iter()
                    .any(|u| u.name == b.name && (!b.active || u.active) && (b.failed || !u.failed))
            })
            || !plan
                .health
                .required_apis
                .iter()
                .all(|required| self.apis.contains(required))
            || !plan
                .health
                .required_user_units
                .iter()
                .all(|required| self.user_units.contains(required))
        {
            return Err(Error::Health);
        }
        let baseline_failed: BTreeSet<_> = plan
            .health
            .baseline_units
            .iter()
            .filter(|u| u.failed)
            .map(|u| &u.name)
            .collect();
        if self
            .units
            .iter()
            .any(|u| known_unit(&u.name) && u.failed && !baseline_failed.contains(&u.name))
        {
            return Err(Error::Health);
        }
        Ok(())
    }
    fn prior(&self, plan: &Plan) -> bool {
        self.running == plan.prior.running.path
            && self.profile == plan.prior.profile.path
            && self.boot == plan.prior.boot.path
            && self.managed_sha256 == plan.prior.managed_sha256
            && self.model == plan.prior.model
    }
    fn tested(&self, plan: &Plan) -> bool {
        self.profile == plan.prior.profile.path
            && self.boot == plan.prior.boot.path
            && self.managed_sha256 == plan.prior.managed_sha256
            && match &plan.candidate {
                Candidate::System { closure } => {
                    self.running == closure.path && self.model == plan.prior.model
                }
                Candidate::ModelOnly { artifact } => {
                    self.running == plan.prior.running.path && self.model.as_ref() == Some(artifact)
                }
            }
    }
    fn committed(&self, plan: &Plan) -> bool {
        self.managed_sha256 == plan.managed_sha256
            && match &plan.candidate {
                Candidate::System { closure } => {
                    self.running == closure.path
                        && self.profile == closure.path
                        && self.boot == closure.path
                        && self.model == plan.prior.model
                }
                Candidate::ModelOnly { artifact } => {
                    self.running == plan.prior.running.path
                        && self.profile == plan.prior.profile.path
                        && self.boot == plan.prior.boot.path
                        && self.model.as_ref() == Some(artifact)
                }
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Prechecking,
    AwaitingReboot,
    GuardArmed,
    TestActivating,
    Verifying,
    Committing,
    Committed,
    Recovering,
    RolledBack,
    RecoveryRequired,
    Rejected,
}
impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Prechecking => "PRECHECKING",
            Self::AwaitingReboot => "AWAITING_REBOOT",
            Self::GuardArmed => "GUARD_ARMED",
            Self::TestActivating => "TEST_ACTIVATING",
            Self::Verifying => "VERIFYING",
            Self::Committing => "COMMITTING",
            Self::Committed => "COMMITTED",
            Self::Recovering => "RECOVERING",
            Self::RolledBack => "ROLLED_BACK",
            Self::RecoveryRequired => "RECOVERY_REQUIRED",
            Self::Rejected => "REJECTED",
        }
    }
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Committed | Self::RolledBack | Self::Rejected | Self::RecoveryRequired
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    RetainClosures,
    ArmGuard {
        transaction_id: String,
        deadline_boot_ms: u64,
        executable_sha256: String,
    },
    TestSystem {
        closure: String,
    },
    TestModel {
        artifact: ModelArtifact,
    },
    SetProfile {
        closure: String,
    },
    InstallBoot {
        closure: String,
    },
    PublishManaged {
        artifact_sha256: String,
    },
    RecoverSystem {
        closure: String,
    },
    RecoverModel {
        artifact: ModelArtifact,
    },
    DisarmGuard {
        transaction_id: String,
    },
}
/// Root adapter operations are fixed by Effect; it exposes no shell/argv method.
/// A fixture adapter cannot establish real activation or health qualification.
pub trait Adapter {
    fn observe(&mut self) -> Result<Observation>;
    fn apply(&mut self, effect: &Effect) -> Result<()>;
    fn boot_time_ms(&mut self) -> Result<u64>;
}

pub struct Ledger {
    connection: Connection,
}
impl Ledger {
    /// Caller must own/validate the root directory/file before opening this DB.
    pub fn new(connection: Connection) -> Result<Self> {
        connection.busy_timeout(Duration::from_millis(100))?;
        connection.execute_batch("PRAGMA trusted_schema=OFF;")?;
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='guard_schema')",
            [],
            |r| r.get(0),
        )?;
        if exists {
            let versions: Vec<i64> = connection
                .prepare("SELECT version FROM guard_schema")?
                .query_map([], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            if versions != [1] {
                return Err(Error::Ledger);
            }
            for name in ["guard_transactions", "guard_events"] {
                let found: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [name],
                    |r| r.get(0),
                )?;
                if !found {
                    return Err(Error::Ledger);
                }
            }
        } else {
            let collision:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name IN ('guard_transactions','guard_events','guard_one_active'))",[],|r|r.get(0))?;
            if collision {
                return Err(Error::Ledger);
            }
            connection.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE guard_schema(version INTEGER NOT NULL);
                INSERT INTO guard_schema VALUES(1);
                CREATE TABLE guard_transactions(id TEXT PRIMARY KEY, plan BLOB NOT NULL, digest TEXT NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL, deadline INTEGER);
                CREATE UNIQUE INDEX guard_one_active ON guard_transactions((1)) WHERE state NOT IN ('COMMITTED','ROLLED_BACK','REJECTED');
                CREATE TABLE guard_events(id TEXT NOT NULL, revision INTEGER NOT NULL, state TEXT NOT NULL, event BLOB NOT NULL, PRIMARY KEY(id,revision)); COMMIT;")?;
        }
        connection.execute_batch("PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE;")?;
        Ok(Self { connection })
    }
    fn insert(&mut self, plan: &Plan) -> Result<()> {
        let digest = plan.digest()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM guard_transactions WHERE state NOT IN ('COMMITTED','ROLLED_BACK','REJECTED'))",[],|r|r.get(0))?;
        if active {
            return Err(Error::Conflict);
        }
        transaction.execute(
            "INSERT INTO guard_transactions VALUES(?1,?2,?3,'PRECHECKING',0,NULL)",
            params![plan.transaction_id, serde_json::to_vec(plan)?, digest],
        )?;
        transaction.execute(
            "INSERT INTO guard_events VALUES(?1,0,'PRECHECKING',?2)",
            params![plan.transaction_id, b"registered".as_slice()],
        )?;
        transaction.commit()?;
        Ok(())
    }
    pub fn state(&self, id: &str) -> Result<State> {
        let state: String = self.connection.query_row(
            "SELECT state FROM guard_transactions WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        serde_json::from_value(serde_json::Value::String(state)).map_err(|_| Error::Ledger)
    }
    pub fn plan(&self, id: &str) -> Result<Plan> {
        let (bytes, digest): (Vec<u8>, String) = self.connection.query_row(
            "SELECT plan,digest FROM guard_transactions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let plan = Plan::from_json(&bytes)?;
        if plan.transaction_id != id || plan.digest()? != digest {
            return Err(Error::Ledger);
        }
        Ok(plan)
    }
    fn event(
        &mut self,
        id: &str,
        expected: State,
        expected_revision: u64,
        next: State,
        effect: Option<&Effect>,
        deadline: Option<u64>,
    ) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (state, revision): (String, i64) = transaction.query_row(
            "SELECT state,revision FROM guard_transactions WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if state != expected.name() || u64::try_from(revision).ok() != Some(expected_revision) {
            return Err(Error::State);
        }
        let revision = revision.checked_add(1).ok_or(Error::Ledger)?;
        let bytes = serde_json::to_vec(&effect)?;
        transaction.execute("UPDATE guard_transactions SET state=?2,revision=?3,deadline=COALESCE(?4,deadline) WHERE id=?1",params![id,next.name(),revision,deadline.and_then(|x|i64::try_from(x).ok())])?;
        transaction.execute(
            "INSERT INTO guard_events VALUES(?1,?2,?3,?4)",
            params![id, revision, next.name(), bytes],
        )?;
        transaction.commit()?;
        Ok(())
    }
    fn revision(&self, id: &str) -> Result<u64> {
        let value: i64 = self.connection.query_row(
            "SELECT revision FROM guard_transactions WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        u64::try_from(value).map_err(|_| Error::Ledger)
    }
    pub fn event_count(&self, id: &str) -> Result<usize> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM guard_events WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        usize::try_from(count).map_err(|_| Error::Ledger)
    }
    pub fn contains(&self, id: &str) -> Result<bool> {
        Ok(self
            .connection
            .query_row("SELECT 1 FROM guard_transactions WHERE id=?1", [id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .is_some())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heartbeat {
    pub schema_version: u32,
    pub transaction_id: String,
    pub identity: Identity,
    pub plan_digest: String,
    pub candidate_digest: String,
    pub nonce: String,
}
/// Volatile challenge: never serialised in the plan, ledger or normal logs.
pub struct Challenge([u8; 64]);
impl Challenge {
    fn new() -> Self {
        let value = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let mut bytes = [0; 64];
        bytes.copy_from_slice(value.as_bytes());
        Self(bytes)
    }
    pub fn transport_value(&self) -> &str {
        std::str::from_utf8(&self.0).expect("hex challenge")
    }
    fn matches(&self, value: &str) -> bool {
        value.len() == 64
            && value
                .as_bytes()
                .iter()
                .zip(self.0)
                .fold(0u8, |diff, (a, b)| diff | (a ^ b))
                == 0
    }
}
impl Drop for Challenge {
    fn drop(&mut self) {
        for b in &mut self.0 {
            unsafe {
                std::ptr::write_volatile(b, 0);
            }
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct Engine {
    plan: Plan,
    state: State,
    revision: u64,
    deadline: u64,
    challenge: Challenge,
    heartbeat_at: Option<u64>,
}
impl Engine {
    pub fn register(plan: Plan, ledger: &mut Ledger) -> Result<Self> {
        plan.validate()?;
        ledger.insert(&plan)?;
        Ok(Self {
            plan,
            state: State::Prechecking,
            revision: 0,
            deadline: 0,
            challenge: Challenge::new(),
            heartbeat_at: None,
        })
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn challenge(&self) -> &Challenge {
        &self.challenge
    }
    fn transition(
        &mut self,
        ledger: &mut Ledger,
        next: State,
        effect: Option<&Effect>,
    ) -> Result<()> {
        ledger.event(
            &self.plan.transaction_id,
            self.state,
            self.revision,
            next,
            effect,
            if next == State::GuardArmed {
                Some(self.deadline)
            } else {
                None
            },
        )?;
        self.revision = self.revision.checked_add(1).ok_or(Error::Ledger)?;
        self.state = next;
        Ok(())
    }
    fn apply<A: Adapter>(
        &mut self,
        ledger: &mut Ledger,
        adapter: &mut A,
        next: State,
        effect: Effect,
    ) -> Result<()> {
        // The runtime adapter must repeat identity/provenance checks at the
        // privileged operation boundary as well; this rejects intervening drift.
        adapter.observe()?.target(&self.plan)?;
        self.transition(ledger, next, Some(&effect))?; // fsync-sensitive intent BEFORE effect
        adapter.apply(&effect)
    }
    pub fn arm<A: Adapter>(
        &mut self,
        ledger: &mut Ledger,
        adapter: &mut A,
        now: u64,
    ) -> Result<State> {
        if self.state != State::Prechecking {
            return Err(Error::State);
        }
        let observation = adapter.observe()?;
        if let Err(error) = observation.health(&self.plan) {
            self.transition(ledger, State::Rejected, None)?;
            return Err(error);
        }
        if !observation.prior(&self.plan) {
            self.transition(ledger, State::Rejected, None)?;
            return Err(Error::Conflict);
        }
        if self.plan.needs_reboot() {
            self.transition(ledger, State::AwaitingReboot, None)?;
            return Ok(self.state);
        }
        self.deadline = now
            .checked_add(u64::from(self.plan.guard_timeout_seconds) * 1000)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or(Error::InvalidPlan)?;
        self.apply(ledger, adapter, State::Prechecking, Effect::RetainClosures)?;
        let arm = Effect::ArmGuard {
            transaction_id: self.plan.transaction_id.clone(),
            deadline_boot_ms: self.deadline,
            executable_sha256: self.plan.retained_guard_sha256.clone(),
        };
        if self.apply(ledger, adapter, State::GuardArmed, arm).is_err() {
            self.transition(ledger, State::RecoveryRequired, None)?;
            return Err(Error::RecoveryRequired);
        }
        let observation = match adapter.observe() {
            Ok(value) => value,
            Err(_) => {
                self.transition(ledger, State::RecoveryRequired, None)?;
                return Err(Error::RecoveryRequired);
            }
        };
        let current = match adapter.boot_time_ms() {
            Ok(value) => value.max(now),
            Err(_) => {
                self.transition(ledger, State::RecoveryRequired, None)?;
                return Err(Error::RecoveryRequired);
            }
        };
        if observation.target(&self.plan).is_err()
            || current >= self.deadline
            || observation.retained_guard_sha256.as_ref() != Some(&self.plan.retained_guard_sha256)
        {
            self.transition(ledger, State::RecoveryRequired, None)?;
            return Err(Error::RecoveryRequired);
        }
        Ok(self.state)
    }
    fn live(&self, now: u64) -> Result<()> {
        if now >= self.deadline {
            Err(Error::Expired)
        } else {
            Ok(())
        }
    }
    pub fn test<A: Adapter>(
        &mut self,
        ledger: &mut Ledger,
        adapter: &mut A,
        now: u64,
    ) -> Result<State> {
        if self.state != State::GuardArmed {
            return Err(Error::State);
        }
        self.live(adapter.boot_time_ms()?.max(now))?;
        let observation = adapter.observe()?;
        observation.target(&self.plan)?;
        if !observation.prior(&self.plan)
            || observation.retained_guard_sha256.as_ref() != Some(&self.plan.retained_guard_sha256)
        {
            return self.recover(ledger, adapter);
        }
        let effect = match &self.plan.candidate {
            Candidate::System { closure } => Effect::TestSystem {
                closure: closure.path.clone(),
            },
            Candidate::ModelOnly { artifact } => Effect::TestModel {
                artifact: artifact.clone(),
            },
        };
        if self
            .apply(ledger, adapter, State::TestActivating, effect)
            .is_err()
        {
            return self.recover(ledger, adapter);
        }
        let observation = match adapter.observe() {
            Ok(value) => value,
            Err(_) => return self.recover(ledger, adapter),
        };
        let current = match adapter.boot_time_ms() {
            Ok(value) => value.max(now),
            Err(_) => return self.recover(ledger, adapter),
        };
        if self.live(current).is_err()
            || observation.health(&self.plan).is_err()
            || !observation.tested(&self.plan)
            || observation.retained_guard_sha256.as_ref() != Some(&self.plan.retained_guard_sha256)
        {
            return self.recover(ledger, adapter);
        }
        self.transition(ledger, State::Verifying, None)?;
        Ok(self.state)
    }
    /// Transport must authenticate host/developer authority before constructing a
    /// heartbeat. Payload fields or an open TCP port cannot establish that trust.
    pub fn heartbeat(&mut self, heartbeat: Heartbeat, now: u64) -> Result<()> {
        if self.state != State::Verifying {
            return Err(Error::State);
        }
        self.live(now)?;
        if heartbeat.schema_version != 1
            || heartbeat.transaction_id != self.plan.transaction_id
            || heartbeat.identity != self.plan.identity
            || heartbeat.plan_digest != self.plan.digest()?
            || heartbeat.candidate_digest != self.plan.candidate_digest
            || !self.challenge.matches(&heartbeat.nonce)
        {
            return Err(Error::Heartbeat);
        }
        self.heartbeat_at = Some(now);
        Ok(())
    }
    pub fn commit<A: Adapter>(
        &mut self,
        ledger: &mut Ledger,
        adapter: &mut A,
        now: u64,
    ) -> Result<State> {
        if self.state != State::Verifying {
            return Err(Error::State);
        }
        self.live(now)?;
        if !self
            .heartbeat_at
            .is_some_and(|t| now.checked_sub(t).is_some_and(|age| age <= 5000))
        {
            return Err(Error::Heartbeat);
        }
        let observation = adapter.observe()?;
        if self.live(adapter.boot_time_ms()?.max(now)).is_err()
            || observation.health(&self.plan).is_err()
            || !observation.tested(&self.plan)
            || observation.retained_guard_sha256.as_ref() != Some(&self.plan.retained_guard_sha256)
        {
            return self.recover(ledger, adapter);
        }
        self.transition(ledger, State::Committing, None)?;
        let mut effects = vec![];
        if let Candidate::System { closure } = &self.plan.candidate {
            effects.push(Effect::SetProfile {
                closure: closure.path.clone(),
            });
            effects.push(Effect::InstallBoot {
                closure: closure.path.clone(),
            });
        }
        effects.push(Effect::PublishManaged {
            artifact_sha256: self.plan.managed_sha256.clone(),
        });
        for effect in effects {
            let current = match adapter.boot_time_ms() {
                Ok(value) => value.max(now),
                Err(_) => return self.recover(ledger, adapter),
            };
            if self.live(current).is_err()
                || !self
                    .heartbeat_at
                    .is_some_and(|t| current.checked_sub(t).is_some_and(|age| age <= 5000))
            {
                return self.recover(ledger, adapter);
            }
            if self
                .apply(ledger, adapter, State::Committing, effect)
                .is_err()
            {
                return self.recover(ledger, adapter);
            }
        }
        let observation = match adapter.observe() {
            Ok(value) => value,
            Err(_) => return self.recover(ledger, adapter),
        };
        let current = match adapter.boot_time_ms() {
            Ok(value) => value.max(now),
            Err(_) => return self.recover(ledger, adapter),
        };
        if self.live(current).is_err()
            || !self
                .heartbeat_at
                .is_some_and(|t| current.checked_sub(t).is_some_and(|age| age <= 5000))
            || observation.health(&self.plan).is_err()
            || !observation.committed(&self.plan)
            || observation.retained_guard_sha256.as_ref() != Some(&self.plan.retained_guard_sha256)
        {
            return self.recover(ledger, adapter);
        }
        self.transition(ledger, State::Committed, None)?; // durable COMMITTED before disarm
        adapter.apply(&Effect::DisarmGuard {
            transaction_id: self.plan.transaction_id.clone(),
        })?;
        Ok(self.state)
    }
    pub fn tick<A: Adapter>(
        &mut self,
        ledger: &mut Ledger,
        adapter: &mut A,
        now: u64,
    ) -> Result<State> {
        if self.state.terminal() || self.state == State::AwaitingReboot {
            return Ok(self.state);
        }
        if self.deadline != 0 && now >= self.deadline {
            return self.recover(ledger, adapter);
        }
        Ok(self.state)
    }
    pub fn recover<A: Adapter>(&mut self, ledger: &mut Ledger, adapter: &mut A) -> Result<State> {
        if self.state.terminal() {
            return Err(Error::State);
        }
        self.transition(ledger, State::Recovering, None)?;
        let observation = match adapter.observe() {
            Ok(value) => value,
            Err(_) => {
                self.transition(ledger, State::RecoveryRequired, None)?;
                return Err(Error::RecoveryRequired);
            }
        };
        if observation.target(&self.plan).is_err() {
            self.transition(ledger, State::RecoveryRequired, None)?;
            return Err(Error::RecoveryRequired);
        }
        let mut effects = vec![];
        match &self.plan.candidate {
            Candidate::System { .. } => {
                effects.push(Effect::RecoverSystem {
                    closure: self.plan.prior.running.path.clone(),
                });
                effects.push(Effect::SetProfile {
                    closure: self.plan.prior.profile.path.clone(),
                });
                effects.push(Effect::InstallBoot {
                    closure: self.plan.prior.boot.path.clone(),
                });
            }
            Candidate::ModelOnly { .. } => effects.push(Effect::RecoverModel {
                artifact: self.plan.prior.model.clone().ok_or(Error::InvalidPlan)?,
            }),
        }
        effects.push(Effect::PublishManaged {
            artifact_sha256: self.plan.prior.managed_sha256.clone(),
        });
        for effect in effects {
            if self
                .apply(ledger, adapter, State::Recovering, effect)
                .is_err()
            {
                self.transition(ledger, State::RecoveryRequired, None)?;
                return Err(Error::RecoveryRequired);
            }
        }
        let observation = match adapter.observe() {
            Ok(value) => value,
            Err(_) => {
                self.transition(ledger, State::RecoveryRequired, None)?;
                return Err(Error::RecoveryRequired);
            }
        };
        if observation.health(&self.plan).is_err() || !observation.prior(&self.plan) {
            self.transition(ledger, State::RecoveryRequired, None)?;
            return Err(Error::RecoveryRequired);
        }
        self.transition(ledger, State::RolledBack, None)?;
        adapter.apply(&Effect::DisarmGuard {
            transaction_id: self.plan.transaction_id.clone(),
        })?;
        Ok(self.state)
    }
    /// A guard restart cannot resurrect its lost volatile challenge or replay an
    /// uncertain activation/commit. It reconciles through prior-state recovery.
    pub fn reconcile<A: Adapter>(ledger: &mut Ledger, id: &str, adapter: &mut A) -> Result<State> {
        let plan = ledger.plan(id)?;
        let state = ledger.state(id)?;
        if state.terminal() || state == State::AwaitingReboot {
            return Ok(state);
        }
        let revision = ledger.revision(id)?;
        let mut engine = Self {
            plan,
            state,
            revision,
            deadline: 0,
            challenge: Challenge::new(),
            heartbeat_at: None,
        };
        engine.recover(ledger, adapter)
    }
}
