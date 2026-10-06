//! Deterministic policy shared by native brokers. None of the authority types
//! deserialize from client/model JSON. Native adapters must authenticate and
//! recheck the subject and resolve concrete resources before calling this API.
use aios_protocol::{contracts::{Action, ErrorCode, canonical_json}, registry::{self, ResourceResolver}};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc,atomic::{AtomicBool, Ordering}}};
use uuid::Uuid;
pub mod consent;

pub type Result<T> = std::result::Result<T, ErrorCode>;
pub const MAX_EXPIRY_MS: u64 = 300_000;
/// Suspend-inclusive monotonic time; caller-provided/wall clocks are not expiry authority.
pub fn boottime_ms() -> Result<u64> {
    let mut value = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) } != 0 || value.tv_sec < 0
        || !(0..1_000_000_000).contains(&value.tv_nsec) { return Err(ErrorCode::TargetChanged); }
    (value.tv_sec as u64).checked_mul(1000).and_then(|v| v.checked_add(value.tv_nsec as u64 / 1_000_000)).ok_or(ErrorCode::TargetChanged)
}
pub fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(canonical_json(&serde_json::to_value(value).map_err(|_| ErrorCode::InvalidArgument)?)?)))
}
pub fn registry_revision() -> String {
    // Includes the policy implementation contract and the fixed generated registry.
    format!("{:x}", Sha256::digest(format!("aios-policy-v4-native-process-task\n{}", aios_protocol::contracts::REGISTRY_SOURCE)))
}
fn hash(value: &str) -> bool { value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
fn uuid(value: &str) -> bool { Uuid::parse_str(value).is_ok_and(|v| !v.is_nil() && v.to_string() == value) }
fn bounded(value: &str) -> bool { !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control) }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Ask, Diagnose, Act, Automate }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Risk { R0, R1, R2, R3, R4 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement { ReadScope, TaskConsent, ExactApproval, ElevatedApproval }

/// Registry metadata is a lower bound. A broker can elevate risk after inspecting
/// concrete impact; neither a model nor a claimed tier can reduce it.
pub fn requirement(action_id: &str, mode: Mode, impact: Risk) -> Result<Requirement> {
    let c = registry::capability(action_id)?;
    let registered = match c.risk_class.as_str() {
        "R0" => Risk::R0, "R1" => Risk::R1, "R2" => Risk::R2, "R3" => Risk::R3, "R4" => Risk::R4,
        _ => return Err(ErrorCode::PolicyChanged),
    };
    let risk = registered.max(impact);
    if risk == Risk::R4 { return Err(ErrorCode::PermissionDenied); }
    if mode == Mode::Automate && (!c.supports_automation || risk >= Risk::R3) { return Err(ErrorCode::PermissionDenied); }
    if c.read_only {
        // A proposal/preview may carry R2 metadata but cannot execute effects.
        if impact != Risk::R0 { return Err(ErrorCode::PermissionDenied); }
        return Ok(Requirement::ReadScope);
    }
    if matches!(mode, Mode::Ask | Mode::Diagnose) { return Err(ErrorCode::PermissionDenied); }
    match risk {
        Risk::R0 => Err(ErrorCode::PolicyChanged), // Registry write mislabeled as R0.
        Risk::R1 => Ok(Requirement::TaskConsent),
        Risk::R2 => Ok(Requirement::ExactApproval),
        Risk::R3 => Ok(Requirement::ElevatedApproval),
        Risk::R4 => Err(ErrorCode::PermissionDenied),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Session { pub id: String, pub remote: bool, pub kind: String }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Client {
    Bus { sender: String, bus_id: String },
    Unix { connection_id: String },
}
/// Values originate only from the native bus/kernel/logind adapter. This is an
/// identity snapshot, not proof that a document or model message is user intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Subject {
    pub uid: u32, pub pid: u32, pub start_ticks: u64, pub boot_id: String,
    pub session: Option<Session>, pub client: Client,
}
impl Subject {
    fn validate(&self) -> Result<()> {
        if self.uid == u32::MAX || self.pid <= 1 || self.start_ticks == 0 || !uuid(&self.boot_id)
            || self.session.as_ref().is_some_and(|s| !bounded(&s.id) || !bounded(&s.kind)) { return Err(ErrorCode::PermissionDenied); }
        match &self.client {
            Client::Bus { sender, bus_id } if sender.starts_with(':') && bounded(sender)
                && bus_id.len() == 32 && bus_id.bytes().all(|b| b.is_ascii_hexdigit()) => Ok(()),
            Client::Unix { connection_id } if uuid(connection_id) => Ok(()),
            _ => Err(ErrorCode::PermissionDenied),
        }
    }
}

/// Created by the broker's authenticated Submit/typed human control route only.
/// There is deliberately no observation/model/automation constructor.
pub struct Intent { subject: Subject, request_id: String, goal_sha256: String, mode: Mode }

/// A broker-issued handle and the concrete identity resolved inside its scope.
/// Digests identify enrolled roots/apps/objects; raw path prefix matching never
/// substitutes for the provider's no-follow/beneath/current-object checks.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Resource { pub field: String, pub kind: String, pub handle: String, pub identity_sha256: String }
#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Scope {
    pub actions: BTreeSet<String>, pub roots: BTreeMap<String, String>,
    pub apps: BTreeMap<String, String>, pub resources: BTreeSet<Resource>,
}
impl Scope {
    fn validate(&self) -> Result<()> {
        if self.actions.is_empty() || self.actions.len() > 64 || self.roots.len() > 64
            || self.apps.len() > 64 || self.resources.len() > 256 { return Err(ErrorCode::ResourceExhausted); }
        for (id, identity) in self.roots.iter().chain(self.apps.iter()) {
            if !bounded(id) || !hash(identity) { return Err(ErrorCode::InvalidArgument); }
        }
        let mut references = BTreeSet::new();
        for r in &self.resources {
            if !bounded(&r.field) || !bounded(&r.kind) || !bounded(&r.handle) || !hash(&r.identity_sha256)
                || !references.insert((&r.field, &r.kind, &r.handle)) { return Err(ErrorCode::InvalidArgument); }
            if r.kind.contains("secret") { return Err(ErrorCode::SecretScopeDenied); }
            if r.kind == "root" && self.roots.get(&r.handle) != Some(&r.identity_sha256)
                || r.kind == "app" && self.apps.get(&r.handle) != Some(&r.identity_sha256) { return Err(ErrorCode::PermissionDenied); }
        }
        for id in &self.actions { registry::capability(id)?; }
        Ok(())
    }
}
/// Native providers freshly resolve handles and their owner/query/expiry/mount
/// bindings. Dynamic arguments must be checked against the original enrolled
/// scope; a policy success is not an execution/verification receipt.
pub trait CurrentResources {
    fn resolve(&self, field: &str, kind: &str, handle: &str) -> Result<String>;
    fn dynamic_arguments(&self, action_id: &str, arguments: &Value, scope: &Scope) -> Result<()>;
}
struct ScopedResolver<'a, R> { scope: &'a Scope, current: &'a R }
impl<R: CurrentResources> ResourceResolver for ScopedResolver<'_, R> {
    fn resolve(&self, field: &str, kind: &str, reference: &str) -> Result<()> {
        let enrolled = self.scope.resources.iter().find(|r| r.field == field && r.kind == kind && r.handle == reference)
            .ok_or(ErrorCode::PermissionDenied)?;
        if self.current.resolve(field, kind, reference)? != enrolled.identity_sha256 { return Err(ErrorCode::TargetChanged); }
        Ok(())
    }
    fn dynamic_arguments(&self, id: &str, args: &Value) -> Result<()> { self.current.dynamic_arguments(id, args, self.scope) }
}

/// Opaque, process-local authority: no serde, Debug, token export or persistence.
/// Stop/Forget/disconnect revoke it immediately. A new broker incarnation cannot
/// validate a retained grant, even within the same boot/policy revision.
pub struct ReadGrant {
    subject: Subject, request_id: String, mode: Mode, goal_sha256: String, scope: Scope,
    plan_sha256: String, policy_revision: String, incarnation: Uuid, nonce: Uuid,
    issued_ms: u64, expires_ms: u64, revoked: Arc<AtomicBool>,
}
impl ReadGrant { pub fn revoke(&self) { self.revoked.store(true, Ordering::Release); } }
impl Drop for ReadGrant { fn drop(&mut self) { self.revoke(); } }
pub struct Policy { boot_id: String, revision: String, incarnation: Uuid }
impl Policy {
    /// Revision must come from installed policy, never a request's claimed value.
    pub fn new(boot_id: String, installed_revision: String) -> Result<Self> {
        if !uuid(&boot_id) || !hash(&installed_revision) { return Err(ErrorCode::PolicyChanged); }
        Ok(Self { boot_id, revision: installed_revision, incarnation: Uuid::new_v4() })
    }
    pub fn authenticated_user_intent(&self, subject: Subject, request_id: String, text: &str, mode: Mode) -> Result<Intent> {
        subject.validate()?;
        if subject.boot_id != self.boot_id { return Err(ErrorCode::TargetChanged); }
        if !uuid(&request_id) || text.trim().is_empty() || text.len() > 60_000 || text.contains('\0') { return Err(ErrorCode::InvalidArgument); }
        if mode == Mode::Automate { return Err(ErrorCode::AuthRequired); }
        Ok(Intent { subject, request_id, goal_sha256: digest(&text)?, mode })
    }
    /// Only read authority can be minted through direct user intent. R1+ writes
    /// still need the separate native consent/exact-plan/polkit path.
    pub fn grant_reads(&self, intent: Intent, scope: Scope, now_ms: u64, expiry_ms: u64) -> Result<ReadGrant> {
        scope.validate()?;
        if intent.subject.boot_id != self.boot_id { return Err(ErrorCode::TargetChanged); }
        if expiry_ms == 0 || expiry_ms > MAX_EXPIRY_MS { return Err(ErrorCode::InvalidArgument); }
        for id in &scope.actions {
            if requirement(id, intent.mode, Risk::R0)? != Requirement::ReadScope { return Err(ErrorCode::AuthRequired); }
            if registry::capability(id)?.requires_interactive_session { return Err(ErrorCode::AuthRequired); }
        }
        let expires_ms = now_ms.checked_add(expiry_ms).ok_or(ErrorCode::InvalidArgument)?;
        let plan_sha256 = digest(&(&intent.subject, &intent.request_id, &intent.goal_sha256, intent.mode, &scope, &self.revision, now_ms, expires_ms))?;
        Ok(ReadGrant { subject: intent.subject, request_id: intent.request_id, goal_sha256: intent.goal_sha256,
            mode: intent.mode, scope, plan_sha256, policy_revision: self.revision.clone(), incarnation: self.incarnation,
            nonce: Uuid::new_v4(), issued_ms: now_ms, expires_ms, revoked: Arc::new(AtomicBool::new(false)) })
    }
    pub fn check_read(&self, grant: &ReadGrant, subject: &Subject, request_id: &str, action: &Action,
        current: &impl CurrentResources, now_ms: u64) -> Result<()> {
        subject.validate()?;
        // Foreign clients cannot revoke the legitimate owner's authority.
        if subject != &grant.subject || request_id != grant.request_id { return Err(ErrorCode::PermissionDenied); }
        if self.boot_id != subject.boot_id || grant.incarnation != self.incarnation { return Err(ErrorCode::ApprovalExpired); }
        if grant.policy_revision != self.revision { grant.revoke(); return Err(ErrorCode::PolicyChanged); }
        if now_ms < grant.issued_ms || now_ms >= grant.expires_ms || grant.revoked.load(Ordering::Acquire) || grant.nonce.is_nil() {
            grant.revoke(); return Err(ErrorCode::ApprovalExpired);
        }
        if digest(&(&grant.subject, &grant.request_id, &grant.goal_sha256, grant.mode, &grant.scope,
            &grant.policy_revision, grant.issued_ms, grant.expires_ms))? != grant.plan_sha256 { grant.revoke(); return Err(ErrorCode::PlanChanged); }
        if !grant.scope.actions.contains(action.action_id()) { return Err(ErrorCode::PermissionDenied); }
        if requirement(action.action_id(), grant.mode, Risk::R0)? != Requirement::ReadScope { return Err(ErrorCode::PermissionDenied); }
        if grant.scope.resources.iter().any(|r| r.kind == "graphical-session") {
            // Interactive authority retains every selected window and display
            // binding, even when this particular action references only one.
            for r in &grant.scope.resources {
                match current.resolve(&r.field, &r.kind, &r.handle) {
                    Ok(identity) if identity == r.identity_sha256 => {},
                    Ok(_) => { grant.revoke(); return Err(ErrorCode::TargetChanged); },
                    Err(error) => { grant.revoke(); return Err(error); },
                }
            }
            let live_now=boottime_ms()?;
            if live_now < grant.issued_ms || live_now >= grant.expires_ms || grant.revoked.load(Ordering::Acquire) {
                grant.revoke(); return Err(ErrorCode::ApprovalExpired);
            }
        }
        registry::validate_references(action, &ScopedResolver { scope: &grant.scope, current })
    }
}

/// Immutable native approval snapshot, not an approval. Human confirmation and
/// privileged authentication remain independent broker-owned requirements.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ApprovalBinding {
    subject: Subject, plan_id: String, plan_sha256: String, target_sha256: String,
    closure: String, impact_sha256: String, policy_revision: String, approval_action: String,
    frozen_ms: u64, expires_ms: u64,
}
impl ApprovalBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(subject: Subject, plan_id: String, plan_sha256: String, target_sha256: String,
        closure: String, impact_sha256: String, policy_revision: String, approval_action: String,
        frozen_ms: u64, expires_ms: u64) -> Result<Self> {
        subject.validate()?;
        if !uuid(&plan_id) || ![&plan_sha256, &target_sha256, &impact_sha256, &policy_revision].iter().all(|v| hash(v))
            || !bounded(&approval_action) || !store_closure(&closure)
            || expires_ms.checked_sub(frozen_ms).is_none_or(|n| n == 0 || n > MAX_EXPIRY_MS) { return Err(ErrorCode::InvalidArgument); }
        Ok(Self { subject, plan_id, plan_sha256, target_sha256, closure, impact_sha256,
            policy_revision, approval_action, frozen_ms, expires_ms })
    }
    pub fn confirmation_digest(&self) -> Result<String> { digest(self) }
    pub fn validate_time(&self, now_ms: u64) -> Result<()> {
        if now_ms < self.frozen_ms || now_ms >= self.expires_ms { Err(ErrorCode::ApprovalExpired) } else { Ok(()) }
    }
}
fn store_closure(value: &str) -> bool {
    let Some(component) = value.strip_prefix("/nix/store/") else { return false; };
    let Some((hash, name)) = component.split_once('-') else { return false; };
    hash.len() == 32 && hash.bytes().all(|c| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&c))
        && !name.is_empty() && component.len() <= 255 && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"+._?=-".contains(&c))
}

#[cfg(test)] mod tests;
