//! Durable preparation/plan register. Activation belongs to the independent guard.
use crate::{Error, Result, canonical, digest, sha256, store, uuid};
use aios_state::{Intent, Preview};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    time::Duration,
};
fn sql_revision(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| Error::Ledger)
}
fn next_revision(value: u64) -> Result<i64> {
    sql_revision(value)?.checked_add(1).ok_or(Error::Ledger)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub installation_uuid: String,
    pub dmi_uuid: String,
    pub boot_id: String,
    pub machine_id: String,
    pub role: String,
    pub disk_serial: String,
    pub management_channel: String,
}
impl Target {
    pub fn validate(&self) -> Result<()> {
        if ![&self.installation_uuid, &self.dmi_uuid, &self.boot_id]
            .iter()
            .all(|s| uuid(s))
            || self.machine_id.len() != 32
            || !self
                .machine_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
            || (self.role == "development" && self.management_channel != "ssh-development")
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requester {
    pub uid: u32,
    pub logind_session: String,
    pub bus_sender: String,
}
impl Requester {
    fn validate(&self) -> Result<()> {
        if self.uid == 0
            || self.logind_session.is_empty()
            || self.logind_session.len() > 128
            || !self
                .logind_session
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !self.bus_sender.starts_with(':')
            || self.bus_sender.len() > 128
            || !self
                .bus_sender
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b":._-".contains(&b))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub intended_manifest_sha256: String,
    pub running_closure: String,
    pub profile_closure: String,
    pub boot_selected_closure: String,
    pub boot_metadata_sha256: String,
}
impl Baseline {
    fn validate(&self) -> Result<()> {
        if !digest(&self.intended_manifest_sha256)
            || !digest(&self.boot_metadata_sha256)
            || ![
                &self.running_closure,
                &self.profile_closure,
                &self.boot_selected_closure,
            ]
            .iter()
            .all(|s| store(s))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationMode {
    Act,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub mode: PreparationMode,
    pub target: Target,
    pub requester: Requester,
    pub intent_text: String,
    pub intent: Intent,
    pub policy_revision: String,
    pub capability_revision: String,
    pub baseline: Baseline,
    pub candidate_sha256: String,
    pub template_sha256: String,
    pub preview: Preview,
    pub prepared_at_monotonic_ms: u64,
    pub preparation_expires_monotonic_ms: u64,
}
impl PreparedPlan {
    pub fn validate(&self) -> Result<()> {
        self.target.validate()?;
        self.requester.validate()?;
        self.baseline.validate()?;
        if self.schema_version != 1
            || !uuid(&self.plan_id)
            || self.intent_text.trim().is_empty()
            || self.intent_text.len() > 8192
            || self.intent_text.contains('\0')
            || ![
                &self.policy_revision,
                &self.capability_revision,
                &self.candidate_sha256,
                &self.template_sha256,
            ]
            .iter()
            .all(|s| digest(s))
            || self.preview.schema_version != 1
            || self.preview.baseline_manifest_sha256 != self.baseline.intended_manifest_sha256
            || !digest(&self.preview.candidate_manifest_sha256)
            || self.preview.final_authorization_ready
            || self.preview.candidate_closure.is_some()
            || self.preview.user_data_deleted
            || self.preview.build_bytes.is_some()
            || self.preview.download_bytes.is_some()
            || self.preview.reboot_required.is_some()
            || self.preview.retained_dependency_paths.is_some()
            || self
                .preparation_expires_monotonic_ms
                .checked_sub(self.prepared_at_monotonic_ms)
                .is_none_or(|d| d == 0 || d > 300000)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(sha256(&canonical(self)?))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ResourcePermission {
    pub requester_uid: u32,
    pub boot_id: String,
    pub candidate_sha256: String,
    pub plan_digest: String,
    pub max_build_bytes: u64,
    pub max_download_bytes: u64,
    pub recovery_reserve_bytes: u64,
    pub expires_monotonic_ms: u64,
    pub approved_cache: String,
}
impl ResourcePermission {
    fn validate(&self, plan: &PreparedPlan, plan_hash: &str, now: u64) -> Result<()> {
        if self.requester_uid != plan.requester.uid
            || self.boot_id != plan.target.boot_id
            || self.candidate_sha256 != plan.candidate_sha256
            || self.plan_digest != plan_hash
            || self.expires_monotonic_ms <= now
            || self.expires_monotonic_ms > plan.preparation_expires_monotonic_ms
            || self.max_build_bytes == 0
            || self.recovery_reserve_bytes < 8 * 1024 * 1024 * 1024
            || self.approved_cache != "https://cache.nixos.org"
        {
            return Err(Error::ResourcePermissionRequired);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildResult {
    pub candidate_sha256: String,
    pub derivation: String,
    pub closure: String,
    pub inventory_sha256: String,
    pub added_paths: Vec<String>,
    pub removed_paths: Vec<String>,
    pub nar_bytes: u64,
    pub measured_download_bytes: Option<u64>,
}
impl BuildResult {
    fn validate(&self, plan: &PreparedPlan) -> Result<()> {
        if self.candidate_sha256 != plan.candidate_sha256
            || !store(&self.derivation)
            || !self.derivation.ends_with(".drv")
            || !store(&self.closure)
            || !digest(&self.inventory_sha256)
            || self.added_paths.len() > 16384
            || self.removed_paths.len() > 16384
            || !self
                .added_paths
                .iter()
                .chain(self.removed_paths.iter())
                .all(|p| store(p))
            || !self.added_paths.windows(2).all(|p| p[0] < p[1])
            || !self.removed_paths.windows(2).all(|p| p[0] < p[1])
            || self
                .added_paths
                .iter()
                .any(|p| self.removed_paths.contains(p))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
/// This value is created by the privileged adapter after independently checking
/// the registered candidate, expected derivation, roots, identity and Nix output.
/// No public/deserialized build result can become this capability.
pub struct VerifiedBuild {
    result: BuildResult,
    baselines_unchanged: bool,
}
/// Minted only by the qualified worker supervisor after PID/start/cgroup checks.
/// Observation timeout is not proof of termination.
pub struct VerifiedWorkerStop {
    plan_id: String,
    candidate_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalPlan {
    pub prepared_plan_sha256: String,
    pub candidate_sha256: String,
    pub build: BuildResult,
    pub semantic_preview: Preview,
    pub reboot_required: bool,
    pub ordered_steps: Vec<Step>,
    pub frozen_at_monotonic_ms: u64,
    pub approval_expires_monotonic_ms: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    AuthorizeExactPlan,
    Precheck,
    ArmIndependentGuard,
    TestExactClosure,
    VerifyHealthAndCapabilities,
    SetExactProfile,
    SelectBoot,
    VerifyPointers,
    DurableCommit,
    DisarmGuard,
}
const STEPS: [Step; 10] = [
    Step::AuthorizeExactPlan,
    Step::Precheck,
    Step::ArmIndependentGuard,
    Step::TestExactClosure,
    Step::VerifyHealthAndCapabilities,
    Step::SetExactProfile,
    Step::SelectBoot,
    Step::VerifyPointers,
    Step::DurableCommit,
    Step::DisarmGuard,
];
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Received,
    Validating,
    Planned,
    Building,
    Built,
    AwaitingApproval,
    Cancelled,
    Rejected,
    Failed,
}
impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Received => "RECEIVED",
            Self::Validating => "VALIDATING",
            Self::Planned => "PLANNED",
            Self::Building => "BUILDING",
            Self::Built => "BUILT",
            Self::AwaitingApproval => "AWAITING_APPROVAL",
            Self::Cancelled => "CANCELLED",
            Self::Rejected => "REJECTED",
            Self::Failed => "FAILED",
        }
    }
    fn terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Rejected | Self::Failed)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub plan_id: String,
    pub state: State,
    pub revision: u64,
    pub cancel_requested: bool,
    pub final_plan_sha256: Option<String>,
}
pub struct Ledger {
    connection: Connection,
    durable_file: Option<File>,
    directory: Option<File>,
    target: Option<crate::native::VerifiedTarget>,
}
impl Ledger {
    /// No path/connection selector is exposed by the product. The administrator
    /// module creates the 0700 root directory; SQLite cannot follow a DB symlink.
    pub fn open() -> Result<Self> {
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(Error::Authority);
        }
        let target = crate::native::VerifiedTarget::enroll()?;
        let path = std::path::Path::new("/var/lib/aios/transactions");
        let directory = super::candidate::root_ledger_directory(path)?;
        let proc = std::path::PathBuf::from(format!(
            "/proc/self/fd/{}",
            std::os::fd::AsRawFd::as_raw_fd(&directory)
        ));
        let db = proc.join("ledger.sqlite");
        let (file, created) = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&db)
        {
            Ok(file) => (file, true),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&db)?,
                false,
            ),
            Err(e) => return Err(e.into()),
        };
        if !created && file.metadata()?.len() == 0 {
            return Err(Error::Ledger);
        }
        let meta = file.metadata()?;
        if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o7777 != 0o600 || meta.nlink() != 1
        {
            return Err(Error::Ownership);
        }
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags(db, flags)?;
        target.recheck()?;
        let mut ledger = Self::initialize(connection)?;
        ledger.durable_file = Some(file);
        ledger.directory = Some(directory);
        ledger.target = Some(target);
        ledger.sync()?;
        Ok(ledger)
    }
    fn initialize(connection: Connection) -> Result<Self> {
        connection.busy_timeout(Duration::from_millis(100))?;
        connection.execute_batch(
            "PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE;",
        )?;
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='broker_schema')",
            [],
            |r| r.get(0),
        )?;
        if !exists {
            let occupied: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name NOT LIKE 'sqlite_%')",
                [],
                |r| r.get(0),
            )?;
            if occupied {
                return Err(Error::Ledger);
            }
            connection.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE broker_schema(version INTEGER NOT NULL); INSERT INTO broker_schema VALUES(1);
                CREATE TABLE plans(id TEXT PRIMARY KEY,subject INTEGER NOT NULL,prepared BLOB NOT NULL,digest TEXT NOT NULL,state TEXT NOT NULL,revision INTEGER NOT NULL,cancel INTEGER NOT NULL DEFAULT 0,resource BLOB,result BLOB,final BLOB,final_digest TEXT);
                CREATE UNIQUE INDEX one_active ON plans((1)) WHERE state NOT IN ('CANCELLED','REJECTED','FAILED');
                CREATE TABLE events(id TEXT NOT NULL,revision INTEGER NOT NULL,state TEXT NOT NULL,kind TEXT NOT NULL,PRIMARY KEY(id,revision)); COMMIT;")?;
        } else {
            let versions: Vec<i64> = connection
                .prepare("SELECT version FROM broker_schema")?
                .query_map([], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?;
            if versions != [1] {
                return Err(Error::Ledger);
            }
            for name in ["plans", "events", "one_active"] {
                let present: bool = connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                    [name],
                    |r| r.get(0),
                )?;
                if !present {
                    return Err(Error::Ledger);
                }
            }
        }
        Ok(Self {
            connection,
            durable_file: None,
            directory: None,
            target: None,
        })
    }
    fn sync(&self) -> Result<()> {
        if let Some(file) = &self.durable_file {
            file.sync_all()?;
        }
        if let Some(dir) = &self.directory {
            dir.sync_all()?;
        }
        Ok(())
    }
    fn verify_target(&self) -> Result<()> {
        if let Some(target) = &self.target {
            target.recheck()?;
        }
        Ok(())
    }
    fn verify_plan_target(&self, plan: &PreparedPlan) -> Result<()> {
        self.verify_target()?;
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.target() != &plan.target)
        {
            return Err(Error::TargetChanged);
        }
        Ok(())
    }
    pub fn register(
        &mut self,
        plan: &PreparedPlan,
        candidate: &crate::candidate::Candidate,
        store: &crate::candidate::CandidateStore,
    ) -> Result<Status> {
        self.verify_target()?;
        if self
            .target
            .as_ref()
            .is_some_and(|target| target.target() != &plan.target)
        {
            return Err(Error::TargetChanged);
        }
        store.verify(candidate)?;
        let manifest = candidate.manifest();
        if candidate.digest() != plan.candidate_sha256
            || manifest.template_sha256 != plan.template_sha256
            || manifest.managed_sha256 != plan.preview.candidate_manifest_sha256
            || manifest.catalog_revision != plan.preview.candidate_manifest.catalog_revision
            || manifest.lock_sha256 != plan.preview.lock_sha256
            || manifest.base_template_revision
                != plan.preview.candidate_manifest.base_template_revision
            || sha256(&canonical(&plan.preview.candidate_manifest)?) != manifest.managed_sha256
        {
            return Err(Error::Integrity);
        }
        let hash = plan.digest()?;
        let bytes = canonical(plan)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT digest FROM plans WHERE id=?1",
                [&plan.plan_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != hash {
                return Err(Error::Conflict);
            }
            tx.commit()?;
            return self.status(&plan.plan_id, plan.requester.uid);
        }
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM plans WHERE state NOT IN ('CANCELLED','REJECTED','FAILED'))",[],|r|r.get(0))?;
        if active {
            return Err(Error::Conflict);
        }
        tx.execute("INSERT INTO plans(id,subject,prepared,digest,state,revision) VALUES(?1,?2,?3,?4,'RECEIVED',0)",params![plan.plan_id,plan.requester.uid,bytes,hash])?;
        tx.execute(
            "INSERT INTO events VALUES(?1,0,'RECEIVED','registered')",
            [&plan.plan_id],
        )?;
        tx.commit()?;
        self.sync()?;
        self.status(&plan.plan_id, plan.requester.uid)
    }
    pub fn get_plan(&self, id: &str, uid: u32) -> Result<PreparedPlan> {
        self.verify_target()?;
        if !uuid(id) {
            return Err(Error::Invalid);
        }
        let (subject, bytes, hash): (u32, Vec<u8>, String) = self
            .connection
            .query_row(
                "SELECT subject,prepared,digest FROM plans WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or(Error::NotFound)?;
        if uid != subject {
            return Err(Error::Authority);
        }
        let plan: PreparedPlan = serde_json::from_slice(&bytes)?;
        if plan.plan_id != id
            || plan.requester.uid != subject
            || plan.digest()? != hash
            || canonical(&plan)? != bytes
        {
            return Err(Error::Integrity);
        }
        Ok(plan)
    }
    pub fn status(&self, id: &str, uid: u32) -> Result<Status> {
        self.get_plan(id, uid)?;
        let (state, revision, cancel, hash): (String, i64, bool, Option<String>) =
            self.connection.query_row(
                "SELECT state,revision,cancel,final_digest FROM plans WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
        Ok(Status {
            plan_id: id.into(),
            state: serde_json::from_value(serde_json::Value::String(state))?,
            revision: u64::try_from(revision).map_err(|_| Error::Ledger)?,
            cancel_requested: cancel,
            final_plan_sha256: hash,
        })
    }
    fn transition(&mut self, id: &str, expected: &Status, next: State, kind: &str) -> Result<()> {
        self.verify_target()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("UPDATE plans SET state=?2,revision=revision+1 WHERE id=?1 AND state=?3 AND revision=?4",params![id,next.name(),expected.state.name(),sql_revision(expected.revision)?])?;
        if changed != 1 {
            return Err(Error::State);
        }
        tx.execute(
            "INSERT INTO events VALUES(?1,?2,?3,?4)",
            params![id, next_revision(expected.revision)?, next.name(), kind],
        )?;
        tx.commit()?;
        self.sync()
    }
    pub fn mark_planned(&mut self, id: &str, uid: u32) -> Result<Status> {
        let s = self.status(id, uid)?;
        if s.state == State::Planned {
            return Ok(s);
        }
        if !matches!(s.state, State::Received | State::Validating) {
            return Err(Error::State);
        }
        if s.state == State::Received {
            self.transition(id, &s, State::Validating, "validation-started")?;
        }
        let s = self.status(id, uid)?;
        self.transition(id, &s, State::Planned, "sealed-candidate-registered")?;
        self.status(id, uid)
    }
    pub fn start_build(
        &mut self,
        id: &str,
        uid: u32,
        target: &Target,
        baseline: &Baseline,
        permission: &ResourcePermission,
        now: u64,
    ) -> Result<Status> {
        let plan = self.get_plan(id, uid)?;
        self.verify_plan_target(&plan)?;
        if &plan.target != target || &plan.baseline != baseline {
            return Err(Error::TargetChanged);
        }
        if now < plan.prepared_at_monotonic_ms || now >= plan.preparation_expires_monotonic_ms {
            return Err(Error::Expired);
        }
        permission.validate(&plan, &plan.digest()?, now)?;
        let s = self.status(id, uid)?;
        if s.state == State::Building {
            if s.cancel_requested {
                return Err(Error::State);
            }
            let prior: Vec<u8> =
                self.connection
                    .query_row("SELECT resource FROM plans WHERE id=?1", [id], |r| r.get(0))?;
            if prior != canonical(permission)? {
                return Err(Error::Conflict);
            }
            return Ok(s);
        }
        if s.state != State::Planned || s.cancel_requested {
            return Err(Error::State);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed=tx.execute("UPDATE plans SET state='BUILDING',revision=revision+1,resource=?2 WHERE id=?1 AND state='PLANNED' AND revision=?3",params![id,canonical(permission)?,sql_revision(s.revision)?])?;
        if changed != 1 {
            return Err(Error::State);
        }
        tx.execute(
            "INSERT INTO events VALUES(?1,?2,'BUILDING','resource-permission-recorded')",
            params![id, next_revision(s.revision)?],
        )?;
        tx.commit()?;
        self.sync()?;
        self.status(id, uid)
    }
    pub fn request_cancel(&mut self, id: &str, uid: u32) -> Result<Status> {
        let s = self.status(id, uid)?;
        if s.state.terminal() {
            return Ok(s);
        }
        if s.state == State::Building {
            if !s.cancel_requested {
                let tx = self
                    .connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)?;
                if tx.execute("UPDATE plans SET cancel=1,revision=revision+1 WHERE id=?1 AND state='BUILDING' AND revision=?2",params![id,sql_revision(s.revision)?])?!=1{return Err(Error::State);}
                tx.execute("INSERT INTO events VALUES(?1,?2,'BUILDING','cancel-requested-await-worker-stop')",params![id,next_revision(s.revision)?])?;
                tx.commit()?;
                self.sync()?;
            }
        } else {
            self.transition(id, &s, State::Cancelled, "cancelled-before-system-effects")?;
        }
        self.status(id, uid)
    }
    pub fn acknowledge_worker_stopped(
        &mut self,
        id: &str,
        uid: u32,
        proof: &VerifiedWorkerStop,
    ) -> Result<Status> {
        let plan = self.get_plan(id, uid)?;
        self.verify_plan_target(&plan)?;
        if proof.plan_id != id || proof.candidate_sha256 != plan.candidate_sha256 {
            return Err(Error::Integrity);
        }
        let s = self.status(id, uid)?;
        if s.state != State::Building || !s.cancel_requested {
            return Err(Error::State);
        }
        self.transition(id, &s, State::Cancelled, "worker-stopped-no-system-effects")?;
        self.status(id, uid)
    }
    pub fn record_build(
        &mut self,
        id: &str,
        uid: u32,
        verified: &VerifiedBuild,
        target: &Target,
        baseline: &Baseline,
    ) -> Result<Status> {
        let plan = self.get_plan(id, uid)?;
        self.verify_plan_target(&plan)?;
        let s = self.status(id, uid)?;
        if s.cancel_requested {
            return Err(Error::State);
        }
        if &plan.target != target || &plan.baseline != baseline || !verified.baselines_unchanged {
            return Err(Error::TargetChanged);
        }
        verified.result.validate(&plan)?;
        if matches!(s.state, State::Built | State::AwaitingApproval) {
            let bytes: Vec<u8> =
                self.connection
                    .query_row("SELECT result FROM plans WHERE id=?1", [id], |r| r.get(0))?;
            if bytes != canonical(&verified.result)? {
                return Err(Error::Conflict);
            }
            return Ok(s);
        }
        if s.state != State::Building {
            return Err(Error::State);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE plans SET result=?2,state='BUILT',revision=revision+1 WHERE id=?1 AND state='BUILDING' AND revision=?3",params![id,canonical(&verified.result)?,sql_revision(s.revision)?])?!=1{return Err(Error::State);}
        tx.execute(
            "INSERT INTO events VALUES(?1,?2,'BUILT','registered-output-independently-verified')",
            params![id, next_revision(s.revision)?],
        )?;
        tx.commit()?;
        self.sync()?;
        self.status(id, uid)
    }
    pub fn freeze(
        &mut self,
        id: &str,
        uid: u32,
        now: u64,
        reboot_required: bool,
    ) -> Result<FinalPlan> {
        let plan = self.get_plan(id, uid)?;
        self.verify_plan_target(&plan)?;
        let s = self.status(id, uid)?;
        if s.state == State::AwaitingApproval {
            let prior = self.final_plan(id, uid)?;
            if prior.reboot_required != reboot_required {
                return Err(Error::Conflict);
            }
            return Ok(prior);
        }
        if s.state != State::Built {
            return Err(Error::State);
        }
        if now < plan.prepared_at_monotonic_ms || now >= plan.preparation_expires_monotonic_ms {
            return Err(Error::Expired);
        }
        let bytes: Vec<u8> =
            self.connection
                .query_row("SELECT result FROM plans WHERE id=?1", [id], |r| r.get(0))?;
        let build: BuildResult = serde_json::from_slice(&bytes)?;
        build.validate(&plan)?;
        let final_plan = FinalPlan {
            prepared_plan_sha256: plan.digest()?,
            candidate_sha256: plan.candidate_sha256,
            build,
            semantic_preview: plan.preview,
            reboot_required,
            ordered_steps: STEPS.to_vec(),
            frozen_at_monotonic_ms: now,
            approval_expires_monotonic_ms: now.checked_add(300000).ok_or(Error::Invalid)?,
        };
        let bytes = canonical(&final_plan)?;
        let hash = sha256(&bytes);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE plans SET final=?2,final_digest=?3,state='AWAITING_APPROVAL',revision=revision+1 WHERE id=?1 AND state='BUILT' AND revision=?4",params![id,bytes,hash,sql_revision(s.revision)?])?!=1{return Err(Error::State);}
        tx.execute("INSERT INTO events VALUES(?1,?2,'AWAITING_APPROVAL','exact-plan-frozen-not-authorized')",params![id,next_revision(s.revision)?])?;
        tx.commit()?;
        self.sync()?;
        self.final_plan(id, uid)
    }
    pub fn final_plan(&self, id: &str, uid: u32) -> Result<FinalPlan> {
        let prepared = self.get_plan(id, uid)?;
        let (bytes, hash): (Vec<u8>, String) = self
            .connection
            .query_row(
                "SELECT final,final_digest FROM plans WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or(Error::NotFound)?;
        let plan: FinalPlan = serde_json::from_slice(&bytes)?;
        plan.build.validate(&prepared)?;
        if sha256(&bytes) != hash
            || canonical(&plan)? != bytes
            || plan.prepared_plan_sha256 != prepared.digest()?
            || plan.candidate_sha256 != prepared.candidate_sha256
            || canonical(&plan.semantic_preview)? != canonical(&prepared.preview)?
            || plan.ordered_steps != STEPS
            || plan
                .approval_expires_monotonic_ms
                .checked_sub(plan.frozen_at_monotonic_ms)
                != Some(300000)
        {
            return Err(Error::Integrity);
        }
        Ok(plan)
    }
    /// This preparation core cannot mint an activation receipt or execute. The
    /// qualified approval/guard adapter must be connected before this can succeed.
    pub fn execute(&self, id: &str, uid: u32, hash: &str) -> Result<()> {
        let plan = self.get_plan(id, uid)?;
        self.verify_plan_target(&plan)?;
        let s = self.status(id, uid)?;
        if s.state != State::AwaitingApproval || s.final_plan_sha256.as_deref() != Some(hash) {
            return Err(Error::State);
        }
        self.final_plan(id, uid)?;
        Err(Error::ActivationUnavailable)
    }
    pub fn history(&self, id: &str, uid: u32) -> Result<Vec<(u64, State, String)>> {
        let status = self.status(id, uid)?;
        let rows = self
            .connection
            .prepare("SELECT revision,state,kind FROM events WHERE id=?1 ORDER BY revision")?
            .query_map([id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if i64::try_from(rows.len()).map_err(|_| Error::Ledger)? != next_revision(status.revision)?
            || rows.iter().enumerate().any(|(i, row)| row.0 != i as i64)
            || rows.last().is_none_or(|row| row.1 != status.state.name())
        {
            return Err(Error::Integrity);
        }
        rows.into_iter()
            .map(|(n, s, k)| {
                Ok((
                    u64::try_from(n).map_err(|_| Error::Ledger)?,
                    serde_json::from_value(serde_json::Value::String(s))?,
                    k,
                ))
            })
            .collect()
    }
}
#[cfg(test)]
mod tests;
