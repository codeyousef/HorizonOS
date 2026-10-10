//! Authenticated Executor1 control. Native identity is never request JSON.
use crate::{
    Error,
    approval::{AuthenticatedChallenge, Authorizer, ResourceChallenge, boottime_ms},
    baseline::NativeBaseline,
    caller::{CallerIdentity, VerifiedCaller},
    candidate::{CandidateStore, InstalledTemplate},
    ledger::{Ledger, PreparationMode, PreparedPlan, Requester, State, VerifiedBuild, VerifiedWorkerStop},
};
use aios_protocol::{MAX_FRAME_BYTES, MAX_TASK_BYTES, contracts::ErrorCode};
use aios_state::{DatabaseData, Intent, PreparationGrants};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use zbus::{DBusError, Message, message::Header, names::ErrorName};
mod system;
mod journal;

pub const NAME: &str = "org.aios.Executor1";
pub const PATH: &str = "/org/aios/Executor1";
/// Install before native bus/pool threads exist so every thread inherits it.
/// Systemd must allow the initial ExecStart; only the running broker is denied
/// program execution. x32 cannot bypass the fixed x86_64 filter.
pub fn restrict_execution() -> crate::Result<()> {
    let statement = |code, k| libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    };
    let jump = |k, jt, jf| libc::sock_filter {
        code: 0x15,
        jt,
        jf,
        k,
    };
    let deny = 0x00050000 | libc::EPERM as u32;
    let filter = [
        statement(0x20, 4),
        jump(0xc000003e, 1, 0),
        statement(0x06, 0x80000000),
        statement(0x20, 0),
        libc::sock_filter {
            code: 0x45,
            jt: 0,
            jf: 1,
            k: 0x40000000,
        },
        statement(0x06, deny),
        jump(libc::SYS_execve as u32, 0, 1),
        statement(0x06, deny),
        jump(libc::SYS_execveat as u32, 0, 1),
        statement(0x06, deny),
        statement(0x06, 0x7fff0000),
    ];
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_ptr().cast_mut(),
    };
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
        || unsafe { libc::prctl(libc::PR_SET_SECCOMP, 2, &program, 0, 0) } != 0
    {
        return Err(Error::Authority);
    }
    Ok(())
}
const OWNER_LIMIT: usize = 128;
const USER_LIMIT: usize = 16;
const TERMINAL_RETENTION_MS: u64 = 30 * 60 * 1000;

#[derive(Debug)]
pub struct BusError {
    code: ErrorCode,
    name: String,
}
impl From<ErrorCode> for BusError {
    fn from(code: ErrorCode) -> Self {
        let encoded = serde_json::to_value(code).expect("finite error code");
        Self {
            code,
            name: format!("org.aios.Error.{}", encoded.as_str().expect("error string")),
        }
    }
}
impl From<Error> for BusError {
    fn from(error: Error) -> Self {
        let code = match error {
            Error::Invalid => ErrorCode::InvalidArgument,
            Error::Authority | Error::Ownership => ErrorCode::PermissionDenied,
            Error::TargetChanged => ErrorCode::TargetChanged,
            Error::Conflict | Error::State => ErrorCode::Conflict,
            Error::NotFound => ErrorCode::TargetNotFound,
            Error::Expired => ErrorCode::ApprovalExpired,
            Error::AuthRequired | Error::ResourcePermissionRequired => ErrorCode::AuthRequired,
            Error::ActivationUnavailable => ErrorCode::UnsupportedCapability,
            Error::Integrity | Error::Ledger => ErrorCode::RecoveryRequired,
            Error::Io => ErrorCode::PartialResult,
            Error::Health => ErrorCode::HealthCheckFailed,
        };
        code.into()
    }
}
impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.code)
    }
}
impl std::error::Error for BusError {}
impl DBusError for BusError {
    fn name(&self) -> ErrorName<'_> {
        ErrorName::try_from(self.name.as_str()).expect("finite error name")
    }
    fn description(&self) -> Option<&str> {
        Some("Request could not be fulfilled within authenticated scope")
    }
    fn create_reply(&self, header: &Header<'_>) -> zbus::Result<Message> {
        Message::error(header, self.name())?.build(&(self.description().unwrap(),))
    }
}
type Result<T> = std::result::Result<T, BusError>;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrepareRequest {
    schema_version: u32,
    request_id: String,
    operation: PrepareOperation,
    mode: PreparationMode,
    intent_text: String,
    intent: Intent,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum PrepareOperation {
    Prepare,
}
fn parse_request(bytes: &str) -> Result<PrepareRequest> {
    if bytes.len() > MAX_TASK_BYTES {
        return Err(ErrorCode::ResourceExhausted.into());
    }
    let value = aios_protocol::validation::strict_json(bytes.as_bytes())?;
    let request: PrepareRequest =
        serde_json::from_str(bytes).map_err(|_| ErrorCode::InvalidArgument)?;
    if request.schema_version != 1 {
        return Err(ErrorCode::UnsupportedSchema.into());
    }
    aios_protocol::validation::validate(
        include_str!("../../../schemas/api/executor-prepare-request.json"),
        &value,
    )?;
    if !crate::uuid(&request.request_id)
        || request.intent_text.trim().is_empty()
        || request.intent_text.len() > 8192
        || request.intent_text.contains('\0')
    {
        return Err(ErrorCode::InvalidArgument.into());
    }
    Ok(request)
}
fn references(id: &str, hash: Option<&str>) -> Result<()> {
    if id.len() > 128 || !crate::uuid(id) || hash.is_some_and(|value| !crate::digest(value)) {
        return Err(ErrorCode::InvalidArgument.into());
    }
    Ok(())
}
fn caller_matches(expected: &CallerIdentity, actual: &CallerIdentity) -> Result<()> {
    if expected.uid != actual.uid
        || expected.boot_id != actual.boot_id
        || expected.bus_id != actual.bus_id
        || expected.session != actual.session
    {
        return Err(ErrorCode::PermissionDenied.into());
    }
    Ok(())
}
fn envelope(operation: &str, data: Value) -> Value {
    json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":operation,"data":data})
}

struct Owner {
    caller: VerifiedCaller,
    nonce: String,
    request_sha256: String,
    created_ms: u64,
    intended_source: String,
}
struct Runtime {
    authorizer: Authorizer,
    ledger: Ledger,
    candidates: CandidateStore,
    owners: BTreeMap<String, Owner>,
    reads: system::ReadState,
    pending_authorizations: BTreeMap<String, Arc<AtomicBool>>,
}
impl Runtime {
    fn open() -> crate::Result<Self> {
        // Startup diagnostics contain fixed stages and finite internal errors,
        // never caller requests, plans or evidence.
        let diagnostic = |stage: &str, error: Error| {
            eprintln!("{}", json!({"schema_version":1,"error":"BROKER_RUNTIME_INIT_FAILED",
                "stage":stage,"reason":format!("{error:?}")}));
            error
        };
        let authorizer = Authorizer::open().map_err(|e| diagnostic("native-authorizer", e))?;
        let mut ledger = Ledger::open().map_err(|e| diagnostic("durable-ledger", e))?;
        // A new bus connection cannot resurrect old volatile caller authority.
        // Durable pre-effect plans are cancelled; BUILDING keeps its slot until
        // a verified worker-stop adapter reconciles it. Already-authorized
        // transactions are handed back to their independent retained guards.
        ledger
            .cancel_abandoned_pre_effects()
            .map_err(|e| diagnostic("orphan-reconciliation", e))?;
        for (id, uid, hash) in ledger
            .authorized_handoffs()
            .map_err(|e| diagnostic("guard-reconciliation", e))?
        {
            let handoff =
                crate::guard::read(&id).map_err(|e| diagnostic("guard-handoff", e))?;
            if handoff.requester_uid != uid || handoff.final_plan_sha256 != hash {
                return Err(diagnostic("guard-handoff", Error::Integrity));
            }
            crate::guard::restart(&handoff)
                .map_err(|e| diagnostic("guard-restart", e))?;
        }
        let candidates = CandidateStore::open().map_err(|e| diagnostic("candidate-store", e))?;
        Ok(Self {
            authorizer,
            ledger,
            candidates,
            owners: BTreeMap::new(),
            pending_authorizations: BTreeMap::new(),
            reads: system::ReadState::default(),
        })
    }
    fn owner(&self, id: &str, caller: &VerifiedCaller) -> Result<&Owner> {
        references(id, None)?;
        let owner = self.owners.get(id).ok_or(ErrorCode::PermissionDenied)?;
        caller_matches(owner.caller.identity(), caller.identity())?;
        self.authorizer.bus().recheck(caller)?;
        Ok(owner)
    }
    fn management(
        &self,
        id: &str,
        caller: &VerifiedCaller,
    ) -> Result<crate::guard::GuardHandoff> {
        references(id, None)?;
        let identity = caller.identity();
        if !identity.session.as_ref().is_some_and(|session| {
            session.remote
                && session.active
                && session.class == "user"
                && session.state == "active"
        }) || self.authorizer.bus().target().target().management_channel != "ssh-development"
        {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let handoff = crate::guard::read(id)?;
        if handoff.requester_uid != identity.uid {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let status = self.ledger.status(id, identity.uid)?;
        if status.state != State::Authorized
            || status.final_plan_sha256.as_deref()
                != Some(handoff.final_plan_sha256.as_str())
        {
            return Err(ErrorCode::Conflict.into());
        }
        self.authorizer.bus().recheck(caller)?;
        Ok(handoff)
    }
    fn cleanup(&mut self) -> crate::Result<()> {
        self.authorizer.bus().target().recheck()?;
        self.reads.cleanup()?;
        let now = boottime_ms()?;
        let mut expired = vec![];
        for (id, owner) in &self.owners {
            let status = self.ledger.status(id, owner.caller.identity().uid)?;
            if matches!(
                status.state,
                State::Committed
                    | State::RolledBack
                    | State::Cancelled
                    | State::Rejected
                    | State::Failed
            ) {
                if now.saturating_sub(owner.created_ms) > TERMINAL_RETENTION_MS {
                    expired.push(id.clone());
                }
            } else if status.state == State::Planned {
                let plan = self.ledger.get_plan(id, owner.caller.identity().uid)?;
                if now >= plan.preparation_expires_monotonic_ms {
                    self.ledger
                        .request_cancel(id, owner.caller.identity().uid)?;
                }
            }
        }
        self.authorizer.bus().target().recheck()?;
        for id in expired {
            self.owners.remove(&id);
        }
        Ok(())
    }
    fn prepared(&self, id: &str, uid: u32) -> Result<Value> {
        let prepared = self.ledger.get_plan(id, uid)?;
        let owner = self.owners.get(id).ok_or(ErrorCode::PermissionDenied)?;
        let status = self.ledger.status(id, uid)?;
        if let Some(hash) = status.final_plan_sha256.as_deref() {
            let plan = self.ledger.final_plan(id, uid)?;
            return Ok(envelope(
                "final_plan",
                json!({"plan_id":id,"plan_sha256":hash,"plan":plan,
                "status":status,"intended_manifest_source":owner.intended_source,
                "build_started":true,"final_authorization_ready":true,
                "system_effects_performed":false}),
            ));
        }
        Ok(envelope(
            "prepared_plan",
            json!({"plan_id":id,"plan_sha256":prepared.digest()?,"plan":prepared,
            "status":status,"intended_manifest_source":owner.intended_source,
            "build_started":status.state==State::Building,
            "final_authorization_ready":false,"system_effects_performed":false}),
        ))
    }
    fn prepare(&mut self, caller: &VerifiedCaller, raw: &str) -> Result<Value> {
        let request = parse_request(raw)?;
        let request_hash = crate::sha256(&crate::canonical(&request)?);
        self.cleanup()?;
        for (id, owner) in &self.owners {
            if owner.caller.identity() == caller.identity() && owner.nonce == request.request_id {
                self.owner(id, caller)?;
                if owner.request_sha256 != request_hash {
                    return Err(ErrorCode::Conflict.into());
                }
                return self.prepared(id, caller.identity().uid);
            }
        }
        if self.owners.len() >= OWNER_LIMIT
            || self
                .owners
                .values()
                .filter(|o| o.caller.identity().uid == caller.identity().uid)
                .count()
                >= USER_LIMIT
        {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        let session = caller
            .identity()
            .session
            .as_ref()
            .ok_or(ErrorCode::AuthRequired)?;
        let template = InstalledTemplate::from_installed()?;
        let baseline = NativeBaseline::capture(&template)?;
        // Missing native data/grant adapters never become an absent database or
        // an unfree acknowledgement supplied by a client.
        let preview = template
            .catalog()
            .prepare(
                &baseline.managed,
                request.intent.clone(),
                DatabaseData::Unknown,
                &PreparationGrants::default(),
            )
            .map_err(|e| match e {
                aios_state::Error::DataReviewRequired
                | aios_state::Error::UnfreeAcknowledgementRequired => {
                    BusError::from(ErrorCode::AuthRequired)
                }
                aios_state::Error::ProtectedTransport => ErrorCode::PermissionDenied.into(),
                _ => ErrorCode::InvalidArgument.into(),
            })?;
        let managed = template
            .catalog()
            .compile(&crate::canonical(&preview.candidate_manifest)?)
            .map_err(|_| ErrorCode::InvalidArgument)?;
        let candidate = self.candidates.prepare(&template, &managed)?;
        let now = boottime_ms()?;
        let plan = PreparedPlan {
            schema_version: 1,
            plan_id: uuid::Uuid::new_v4().to_string(),
            mode: request.mode,
            target: self.authorizer.bus().target().target().clone(),
            requester: Requester {
                uid: caller.identity().uid,
                logind_session: session.id.clone(),
                bus_sender: caller.identity().sender.clone(),
            },
            intent_text: request.intent_text,
            intent: request.intent,
            policy_revision: self.authorizer.policy_revision()?,
            capability_revision: crate::sha256(
                aios_protocol::contracts::REGISTRY_SOURCE.as_bytes(),
            ),
            baseline: baseline.baseline.clone(),
            candidate_sha256: candidate.digest().into(),
            template_sha256: template.digest().into(),
            preview,
            prepared_at_monotonic_ms: now,
            preparation_expires_monotonic_ms: now
                .checked_add(300000)
                .ok_or(ErrorCode::ResourceExhausted)?,
        };
        baseline.recheck()?;
        self.authorizer.bus().recheck(caller)?;
        self.ledger.register(&plan, &candidate, &self.candidates)?;
        let id = plan.plan_id.clone();
        if let Err(error) = baseline
            .recheck()
            .and_then(|_| self.authorizer.bus().recheck(caller))
        {
            self.ledger.request_cancel(&id, caller.identity().uid)?;
            return Err(error.into());
        }
        self.ledger.mark_planned(&id, caller.identity().uid)?;
        self.owners.insert(
            id.clone(),
            Owner {
                caller: caller.clone(),
                nonce: request.request_id,
                request_sha256: request_hash,
                created_ms: now,
                intended_source: baseline.intended_source,
            },
        );
        self.prepared(&id, caller.identity().uid)
    }
    fn approval_reference(&self, caller: &VerifiedCaller, id: &str, hash: &str) -> Result<()> {
        references(id, Some(hash))?;
        self.owner(id, caller)?;
        let status = self.ledger.status(id, caller.identity().uid)?;
        if status.state == State::Cancelled {
            return Err(ErrorCode::Cancelled.into());
        }
        let final_hash = status.final_plan_sha256.as_deref().ok_or(ErrorCode::AuthRequired)?;
        if hash != final_hash {
            return Err(ErrorCode::PlanChanged.into());
        }
        Ok(())
    }
    fn dispatch(&mut self, caller: &VerifiedCaller, operation: Operation) -> Result<Value> {
        match operation {
            Operation::SystemCapabilities(scope) => system::capabilities(scope),
            Operation::SystemAction(scope, expected, request) => self.reads.dispatch(caller, scope, expected, &request),
            Operation::JournalEvidence(id) => self.reads.journal.evidence(caller,&id),
            Operation::JournalResolveService(unit) => self.reads.journal.resolve_service(caller,&unit),
            Operation::JournalResolveUserService(unit) => self.reads.journal.resolve_user_service(caller,&unit),
            Operation::GraphStatus => system::graph_status(),
            Operation::Capabilities => Ok(envelope(
                "capabilities",
                json!({"interface":NAME,"prepare":true,"private_plans":true,
                "native_caller_verified":true,"authorize":true,"execute":true,"rollback":true,
                "management_heartbeat":true,"resource_consent":true,"trusted_confirmation":true,
                "max_request_bytes":MAX_TASK_BYTES,"max_reply_bytes":MAX_FRAME_BYTES}),
            )),
            Operation::Prepare(raw) => self.prepare(caller, &raw),
            Operation::GetPlan(id) => {
                self.owner(&id, caller)?;
                self.prepared(&id, caller.identity().uid)
            }
            Operation::GetTransaction(id) => {
                self.owner(&id, caller)?;
                let status = self.ledger.status(&id, caller.identity().uid)?;
                let system_effects_performed = matches!(
                    status.state,
                    State::Authorized
                        | State::Committed
                        | State::RolledBack
                        | State::RecoveryRequired
                );
                Ok(envelope(
                    "transaction",
                    json!({"status":status,
                    "history":self.ledger.history(&id,caller.identity().uid)?,
                    "system_effects_performed":system_effects_performed}),
                ))
            }
            Operation::Cancel(id) => {
                self.owner(&id, caller)?;
                if let Some(cancelled) = self.pending_authorizations.remove(&id) {
                    cancelled.store(true, Ordering::Release);
                }
                let status = self.ledger.request_cancel(&id, caller.identity().uid)?;
                Ok(envelope(
                    "cancellation",
                    json!({"complete":status.state==State::Cancelled,"status":status,"system_effects_performed":false}),
                ))
            }
            Operation::GuardStatus(id) => {
                self.management(&id, caller)?;
                Ok(envelope("guard_status", crate::guard::status(&id)?))
            }
            Operation::GuardHeartbeat(id, heartbeat) => {
                self.management(&id, caller)?;
                Ok(envelope(
                    "guard_heartbeat",
                    crate::guard::heartbeat(&id, &heartbeat)?,
                ))
            }
            Operation::Execute(id, hash) => {
                self.approval_reference(caller, &id, &hash)?;
                if self
                    .ledger
                    .final_plan(&id, caller.identity().uid)?
                    .reboot_required
                {
                    return Err(ErrorCode::UnsupportedCapability.into());
                }
                let authorization =
                    self.authorizer
                        .consume(&self.ledger, caller, &id, &hash)?;
                if authorization.plan_id() != id || authorization.plan_hash() != hash {
                    return Err(ErrorCode::TargetChanged.into());
                }
                let status =
                    crate::guard::handoff(&mut self.ledger, &id, caller.identity().uid, &hash)?;
                Ok(envelope(
                    "guard_handoff",
                    json!({"plan_id":id,"plan_sha256":hash,"status":status,
                    "independent_guard_started":true}),
                ))
            }
            Operation::Rollback(id) => {
                self.owner(&id, caller)?;
                if self.ledger.status(&id, caller.identity().uid)?.state != State::Authorized {
                    return Err(ErrorCode::Conflict.into());
                }
                Ok(envelope("guard_recovery", crate::guard::recover(&id)?))
            }
        }
    }
}

enum Operation {
    JournalEvidence(String),
    JournalResolveService(String),
    JournalResolveUserService(String),
    SystemCapabilities(system::Scope),
    SystemAction(system::Scope, &'static str, String),
    GraphStatus,
    Capabilities,
    GuardStatus(String),
    GuardHeartbeat(String, String),
    Prepare(String),
    GetPlan(String),
    GetTransaction(String),
    Cancel(String),
    Execute(String, String),
    Rollback(String),
}
struct Admission(Arc<AtomicUsize>);
impl Drop for Admission {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Clone)]
pub struct Executor {
    runtime: Arc<Mutex<Runtime>>,
    active: Arc<AtomicUsize>,
}
impl Executor {
    fn admit(&self) -> Result<Admission> {
        if self.active.fetch_add(1, Ordering::AcqRel) >= 16 {
            self.active.fetch_sub(1, Ordering::AcqRel);
            return Err(ErrorCode::ResourceExhausted.into());
        }
        Ok(Admission(self.active.clone()))
    }
    async fn call(&self, header: Header<'_>, operation: Operation) -> Result<String> {
        let _admission = self.admit()?;
        if header.message_type() != zbus::message::Type::MethodCall {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let sender = header
            .sender()
            .ok_or(ErrorCode::PermissionDenied)?
            .as_str()
            .to_owned();
        let runtime = self.runtime.clone();
        blocking::unblock(move || {
            let mut runtime = runtime.lock().map_err(|_| ErrorCode::ResourceExhausted)?;
            let caller = runtime.authorizer.bus().authenticate_sender(&sender)?;
            let result = runtime.dispatch(&caller, operation);
            runtime.authorizer.bus().recheck(&caller)?;
            let encoded =
                serde_json::to_string(&result?).map_err(|_| ErrorCode::InvalidArgument)?;
            if encoded.len() > MAX_FRAME_BYTES {
                return Err(ErrorCode::ResourceExhausted.into());
            }
            Ok(encoded)
        })
        .await
    }
    async fn authorize_call(&self, header: Header<'_>, id: String, hash: String) -> Result<String> {
        enum Challenge {
            System(crate::approval::InteractiveChallenge),
            Resource(ResourceChallenge),
        }
        enum Authenticated {
            System(AuthenticatedChallenge),
            Resource(crate::ledger::ResourcePermission),
        }
        let _admission = self.admit()?;
        if header.message_type() != zbus::message::Type::MethodCall {
            return Err(ErrorCode::PermissionDenied.into());
        }
        let sender = header.sender().ok_or(ErrorCode::PermissionDenied)?
            .as_str().to_owned();
        let runtime_shared = self.runtime.clone();
        blocking::unblock(move || {
            let (caller, challenge, cancelled) = {
                let mut runtime = runtime_shared.lock()
                    .map_err(|_| ErrorCode::ResourceExhausted)?;
                let caller = runtime.authorizer.bus().authenticate_sender(&sender)?;
                references(&id, Some(&hash))?;
                runtime.owner(&id, &caller)?;
                if runtime.pending_authorizations.contains_key(&id) {
                    return Err(ErrorCode::Conflict.into());
                }
                let status = runtime.ledger.status(&id, caller.identity().uid)?;
                let challenge = if status.final_plan_sha256.is_some() {
                    runtime.approval_reference(&caller, &id, &hash)?;
                    Challenge::System(runtime.authorizer.challenge(
                        &runtime.ledger, &caller, &id, &hash)?)
                } else {
                    Challenge::Resource(runtime.authorizer.resource_challenge(
                        &runtime.ledger, &caller, &id, &hash)?)
                };
                let cancelled = Arc::new(AtomicBool::new(false));
                runtime.pending_authorizations.insert(id.clone(), cancelled.clone());
                (caller, challenge, cancelled)
            };
            let authenticated = match challenge {
                Challenge::System(challenge) => challenge.authenticate(&caller, &cancelled)
                    .map(Authenticated::System),
                Challenge::Resource(challenge) => challenge.authenticate(&caller, &cancelled)
                    .map(Authenticated::Resource),
            };
            let mut runtime = runtime_shared.lock()
                .map_err(|_| ErrorCode::ResourceExhausted)?;
            let current = runtime.pending_authorizations.remove(&id);
            if current.as_ref().is_none_or(|flag| !Arc::ptr_eq(flag, &cancelled))
                || cancelled.load(Ordering::Acquire) {
                return Err(ErrorCode::AuthRequired.into());
            }
            let authenticated = authenticated?;
            let current = runtime.authorizer.bus().authenticate_sender(&sender)?;
            if current.identity() != caller.identity() {
                return Err(ErrorCode::TargetChanged.into());
            }
            runtime.owner(&id, &current)?;
            let result = match authenticated {
                Authenticated::System(authenticated) => {
                    let Runtime { authorizer, ledger, .. } = &mut *runtime;
                    let status = authorizer.finish(
                        ledger, &current, &id, &hash, authenticated)?;
                    authorizer.bus().recheck(&current)?;
                    envelope("authorization", json!({
                        "plan_id": id,
                        "plan_sha256": hash,
                        "status": status,
                        "system_effects_performed": false,
                    }))
                }
                Authenticated::Resource(authenticated) => {
                    let permission = runtime.authorizer.finish_resource(
                        &runtime.ledger, &current, &id, &hash, authenticated)?;
                    let plan = runtime.ledger.get_plan(&id, current.identity().uid)?;
                    let template = InstalledTemplate::from_installed()?;
                    let baseline = NativeBaseline::capture(&template)?;
                    if baseline.baseline != plan.baseline {
                        return Err(ErrorCode::TargetChanged.into());
                    }
                    runtime.ledger.start_build(
                        &id, current.identity().uid, &plan.target, &plan.baseline,
                        &permission, boottime_ms()?)?;
                    let worker_runtime = runtime_shared.clone();
                    let worker_plan = plan.clone();
                    let worker_permission = permission.clone();
                    std::thread::spawn(move || {
                        let completion = crate::build_worker::supervise(
                            &worker_plan, &worker_permission);
                        let completion = match completion {
                            Ok(completion) => completion,
                            Err(_) => {
                                eprintln!("{}", json!({
                                    "schema_version": 1,
                                    "error": "BUILD_WORKER_UNVERIFIED_COMPLETION",
                                    "state": "BUILDING",
                                }));
                                return;
                            }
                        };
                        let Ok(mut runtime) = worker_runtime.lock() else { return; };
                        match completion {
                            crate::build_worker::WorkerCompletion::Failed(failure) => {
                                let stop = VerifiedWorkerStop::from_failed_worker(
                                    &worker_plan, &failure);
                                match runtime.ledger.record_build_failure(
                                    &worker_plan.plan_id, worker_plan.requester.uid, &stop)
                                {
                                    Ok(_) => eprintln!("{}", json!({
                                        "schema_version": 1,
                                        "error": "BUILD_FAILED",
                                        "state": "FAILED",
                                    })),
                                    Err(_) => eprintln!("{}", json!({
                                        "schema_version": 1,
                                        "error": "BUILD_FAILURE_NOT_COMMITTED",
                                        "state": "BUILDING",
                                    })),
                                }
                            }
                            crate::build_worker::WorkerCompletion::Built(worker) => {
                                let stop = VerifiedWorkerStop::from_completed_worker(
                                    &worker_plan, &worker);
                                if runtime.ledger.status(
                                    &worker_plan.plan_id, worker_plan.requester.uid)
                                    .is_ok_and(|status| status.cancel_requested)
                                {
                                    let _ = runtime.ledger.acknowledge_worker_stopped(
                                        &worker_plan.plan_id, worker_plan.requester.uid, &stop);
                                    return;
                                }
                                let Ok(template) = InstalledTemplate::from_installed() else { return; };
                                let Ok(baseline) = NativeBaseline::capture(&template) else { return; };
                                let unchanged = baseline.baseline == worker_plan.baseline
                                    && runtime.authorizer.bus().target().recheck().is_ok();
                                let verified = VerifiedBuild::from_worker(worker, unchanged);
                                match runtime.ledger.record_build(
                                    &worker_plan.plan_id, worker_plan.requester.uid, &verified,
                                    &worker_plan.target, &worker_plan.baseline)
                                    .and_then(|_| runtime.ledger.freeze(
                                        &worker_plan.plan_id, worker_plan.requester.uid,
                                        boottime_ms()?, false).map(|_| ()))
                                {
                                    Ok(()) => {}
                                    Err(_) => eprintln!("{}", json!({
                                        "schema_version": 1,
                                        "error": "BUILD_RESULT_NOT_COMMITTED",
                                        "state": "BUILDING",
                                    })),
                                }
                            }
                        }
                    });
                    envelope("build", json!({
                        "plan_id": id,
                        "prepared_plan_sha256": hash,
                        "status": runtime.ledger.status(&id, current.identity().uid)?,
                        "resource_permission": {
                            "max_build_bytes": permission.max_build_bytes,
                            "max_download_bytes": permission.max_download_bytes,
                            "recovery_reserve_bytes": permission.recovery_reserve_bytes,
                            "approved_cache": permission.approved_cache,
                            "network_egress": false,
                        },
                        "system_effects_performed": false,
                    }))
                }
            };
            let encoded = serde_json::to_string(&result)
                .map_err(|_| ErrorCode::InvalidArgument)?;
            if encoded.len() > MAX_FRAME_BYTES {
                return Err(ErrorCode::ResourceExhausted.into());
            }
            Ok(encoded)
        }).await
    }
}
#[zbus::interface(name = "org.aios.Executor1")]
impl Executor {
    async fn get_capabilities(&self, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.call(header, Operation::Capabilities).await
    }
    async fn prepare(&self, plan_json: &str, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if plan_json.len() > MAX_TASK_BYTES {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        self.call(header, Operation::Prepare(plan_json.into()))
            .await
    }
    async fn get_plan(&self, plan_id: &str, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.call(header, Operation::GetPlan(plan_id.into())).await
    }
    async fn authorize(
        &self,
        plan_id: &str,
        plan_hash: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.authorize_call(header, plan_id.into(), plan_hash.into()).await
    }
    async fn execute(
        &self,
        plan_id: &str,
        plan_hash: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.call(header, Operation::Execute(plan_id.into(), plan_hash.into()))
            .await
    }
    async fn guard_status(
        &self,
        transaction_id: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.call(header, Operation::GuardStatus(transaction_id.into()))
            .await
    }
    async fn guard_heartbeat(
        &self,
        transaction_id: &str,
        heartbeat_json: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        if heartbeat_json.len() > MAX_TASK_BYTES {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        self.call(
            header,
            Operation::GuardHeartbeat(transaction_id.into(), heartbeat_json.into()),
        )
        .await
    }
    async fn get_transaction(
        &self,
        transaction_id: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.call(header, Operation::GetTransaction(transaction_id.into()))
            .await
    }
    async fn cancel_transaction(
        &self,
        transaction_id: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.call(header, Operation::Cancel(transaction_id.into()))
            .await
    }
    async fn request_rollback(
        &self,
        transaction_id: &str,
        #[zbus(header)] header: Header<'_>,
    ) -> Result<String> {
        self.call(header, Operation::Rollback(transaction_id.into()))
            .await
    }
}

/// The registered service owns fixed native bus names. No caller bus/address
/// selector, test authority flag, name replacement or private broadcast exists.
pub fn serve() -> crate::Result<()> {
    let runtime = Runtime::open()?;
    let connection = runtime.authorizer.bus().connection();
    let runtime = Arc::new(Mutex::new(runtime));
    let executor = Executor {
        runtime: runtime.clone(),
        active: Arc::new(AtomicUsize::new(0)),
    };
    for registered in [
        connection.object_server().at(system::SYSTEM_PATH, system::System { executor:executor.clone() }),
        connection.object_server().at(system::PACKAGES_PATH, system::Packages { executor:executor.clone() }),
    ] {
        if !registered.map_err(|_|Error::Authority)? { return Err(Error::Conflict); }
    }
    if !connection
        .object_server()
        .at(PATH, executor)
        .map_err(|_| Error::Authority)?
    {
        return Err(Error::Conflict);
    }
    for name in [system::NAME, NAME] {
        let reply = connection.request_name_with_flags(name, zbus::fdo::RequestNameFlags::DoNotQueue.into())
            .map_err(|_|Error::Authority)?;
        if reply != zbus::fdo::RequestNameReply::PrimaryOwner { return Err(Error::Conflict); }
    }
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let bus = zbus::blocking::Proxy::new(
            &connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .map_err(|_| Error::Authority)?;
        for name in [system::NAME, NAME] {
            let owner: String = bus.call("GetNameOwner", &(name,)).map_err(|_|Error::Authority)?;
            if connection.unique_name().map(|s|s.as_str()) != Some(owner.as_str()) { return Err(Error::TargetChanged); }
        }
        runtime.lock().map_err(|_| Error::Conflict)?.cleanup()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broker_threads_and_their_children_cannot_execute_programs() {
        std::thread::spawn(|| {
            restrict_execution().unwrap();
            std::thread::spawn(|| {
                let file = c"/broker-must-never-execute";
                let arguments = [file.as_ptr(), std::ptr::null()];
                let environment = [std::ptr::null()];
                assert_eq!(
                    unsafe {
                        libc::execve(file.as_ptr(), arguments.as_ptr(), environment.as_ptr())
                    },
                    -1
                );
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EPERM)
                );
                assert_eq!(
                    unsafe {
                        libc::syscall(
                            libc::SYS_execveat,
                            libc::AT_FDCWD,
                            file.as_ptr(),
                            arguments.as_ptr(),
                            environment.as_ptr(),
                            0,
                        )
                    },
                    -1
                );
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EPERM)
                );
                assert_eq!(
                    unsafe {
                        libc::syscall(
                            0x40000000_i64 + 520,
                            std::ptr::null::<u8>(),
                            std::ptr::null::<u8>(),
                            std::ptr::null::<u8>(),
                        )
                    },
                    -1
                );
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(libc::EPERM)
                );
            })
            .join()
            .unwrap();
        })
        .join()
        .unwrap();
    }
    fn valid() -> String {
        json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":"prepare","mode":"act","intent_text":"Install Kate","intent":{"action":"install_package","package_id":"kate"}}).to_string()
    }
    #[test]
    fn prepare_is_strict_typed_intent_not_a_serialized_plan_or_approval() {
        let bytes = valid();
        assert!(parse_request(&bytes).is_ok());
        for key in [
            "uid",
            "target",
            "baseline",
            "candidate_sha256",
            "approved",
            "grants",
            "database_data",
            "template_path",
        ] {
            let mut value: Value = serde_json::from_str(&bytes).unwrap();
            value[key] = json!(true);
            assert!(parse_request(&value.to_string()).is_err());
        }
        assert!(
            parse_request(&bytes.replace(
                "\"schema_version\":1",
                "\"schema_version\":1,\"schema_version\":1"
            ))
            .is_err()
        );
        assert_eq!(
            parse_request(&bytes.replace("\"schema_version\":1", "\"schema_version\":2"))
                .err()
                .unwrap()
                .code,
            ErrorCode::UnsupportedSchema
        );
        assert!(parse_request(&bytes.replace("\"act\"", "\"ask\"")).is_err());
        assert!(parse_request(&bytes.replace("\"install_package\"", "\"shell\"")).is_err());
        assert_eq!(
            parse_request(&" ".repeat(MAX_TASK_BYTES + 1))
                .err()
                .unwrap()
                .code,
            ErrorCode::ResourceExhausted
        );
    }
    #[test]
    fn plan_ownership_survives_process_reconnect_only_within_the_exact_session() {
        let original = CallerIdentity {
            uid: 1000,
            pid: 44,
            start_ticks: 123,
            boot_id: uuid::Uuid::new_v4().to_string(),
            bus_id: "a".repeat(32),
            sender: ":1.8".into(),
            session: Some(crate::caller::SessionIdentity {
                id: "42".into(),
                remote: true,
                kind: "tty".into(),
                class: "user".into(),
                state: "active".into(),
                active: true,
            }),
        };
        assert!(caller_matches(&original, &original).is_ok());
        let mut reconnect = original.clone();
        reconnect.pid += 1;
        reconnect.start_ticks += 1;
        reconnect.sender = ":1.9".into();
        assert!(caller_matches(&original, &reconnect).is_ok());
        for field in 0..4 {
            let mut changed = reconnect.clone();
            match field {
                0 => changed.uid += 1,
                1 => changed.boot_id = uuid::Uuid::new_v4().to_string(),
                2 => changed.bus_id = "b".repeat(32),
                _ => changed.session.as_mut().unwrap().id = "43".into(),
            }
            assert!(caller_matches(&original, &changed).is_err(), "field {field}");
        }
    }
}
