//! Volatile, single-use exact-plan system authorization. No RPC or activation.
pub(crate) mod policy;
mod polkit;
mod tty;
use crate::{
    Error, Result,
    caller::{CallerIdentity, ServiceIdentity, SystemBus, VerifiedCaller},
    canonical,
    ledger::{Ledger, Target},
    sha256,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Minting requires the broker-owned foreground TTY confirmation adapter and
/// a separate fresh native polkit challenge. Neither model output nor a
/// caller-supplied decision can construct this value.
pub struct TrustedConfirmation {
    binding_sha256: String,
}
/// Internal receipt only; no serialization, debug output or public constructors.
/// The independent guard must still verify preconditions and execute exact effects.
pub struct VerifiedSystemAuthorization {
    binding: Binding,
}
impl VerifiedSystemAuthorization {
    pub fn plan_id(&self) -> &str {
        &self.binding.plan_id
    }
    pub fn plan_hash(&self) -> &str {
        &self.binding.plan_hash
    }
    pub fn closure(&self) -> &str {
        &self.binding.closure
    }
}
#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthorizationStatus {
    SystemAuthorized,
}
#[derive(Clone, PartialEq, Eq)]
struct Binding {
    plan_id: String,
    plan_hash: String,
    caller: CallerIdentity,
    target: Target,
    closure: String,
    impact_sha256: String,
    policy_revision: String,
    action: String,
    frozen_at: u64,
    expires_at: u64,
}
impl Binding {
    fn shared(&self) -> Result<aios_policy::ApprovalBinding> {
        let subject = aios_policy::Subject {
            uid: self.caller.uid, pid: self.caller.pid, start_ticks: self.caller.start_ticks,
            boot_id: self.caller.boot_id.clone(), session: self.caller.session.as_ref().map(|s| aios_policy::Session {
                id: s.id.clone(), remote: s.remote, kind: s.kind.clone() }),
            client: aios_policy::Client::Bus { sender: self.caller.sender.clone(), bus_id: self.caller.bus_id.clone() },
        };
        aios_policy::ApprovalBinding::new(subject, self.plan_id.clone(), self.plan_hash.clone(),
            sha256(&canonical(&self.target)?), self.closure.clone(), self.impact_sha256.clone(), self.policy_revision.clone(),
            self.action.clone(), self.frozen_at, self.expires_at).map_err(|_| Error::Integrity)
    }
    fn confirmation_digest(&self) -> Result<String> {
        // Final plan's canonical hash transitively binds prepared intent/actions,
        // target, arguments, baseline, exact result, impact, recovery and expiry.
        Ok(sha256(&canonical(&(
            self.shared()?.confirmation_digest().map_err(|_| Error::Integrity)?,
            self.plan_id.as_str(),
            self.plan_hash.as_str(),
            &self.target,
            self.policy_revision.as_str(),
            self.caller.uid,
            self.caller.pid,
            self.caller.start_ticks,
            self.caller.sender.as_str(),
            self.caller.bus_id.as_str(),
            self.caller.boot_id.as_str(),
            self.caller
                .session
                .as_ref()
                .map(|s| (&s.id, s.remote, &s.kind, &s.class, &s.state, s.active)),
            self.closure.as_str(),
            self.action.as_str(),
            self.frozen_at,
            self.expires_at,
        ))?))
    }
    fn validate_time(&self, now: u64) -> Result<()> {
        if self.expires_at.checked_sub(self.frozen_at) != Some(policy::EXPIRY_MS) {
            return Err(Error::Integrity);
        }
        if now < self.frozen_at || now >= self.expires_at {
            return Err(Error::Expired);
        }
        self.shared()?.validate_time(now).map_err(|_| Error::Expired)?;
        Ok(())
    }
}
struct Receipt {
    binding: Binding,
    issued_at: u64,
    nonce: uuid::Uuid,
    polkit_owner: ServiceIdentity,
}
#[derive(Default)]
struct Volatile {
    receipts: BTreeMap<String, Receipt>,
}
impl Volatile {
    fn issue(
        &mut self,
        binding: Binding,
        confirmation: TrustedConfirmation,
        native: polkit::NativeAuthentication,
        now: u64,
    ) -> Result<()> {
        binding.validate_time(now)?;
        if confirmation.binding_sha256 != binding.confirmation_digest()? {
            return Err(Error::Integrity);
        }
        self.receipts
            .retain(|_, receipt| now >= receipt.issued_at && now < receipt.binding.expires_at);
        if self.receipts.contains_key(&binding.plan_id) {
            return Err(Error::Conflict);
        }
        if self.receipts.len() >= 16 {
            return Err(Error::Conflict);
        }
        self.receipts.insert(
            binding.plan_id.clone(),
            Receipt {
                binding,
                issued_at: now,
                nonce: uuid::Uuid::new_v4(),
                polkit_owner: native.owner,
            },
        );
        Ok(())
    }
    fn consume(
        &mut self,
        current: &Binding,
        owner: &ServiceIdentity,
        now: u64,
    ) -> Result<VerifiedSystemAuthorization> {
        let receipt = self
            .receipts
            .get(&current.plan_id)
            .ok_or(Error::AuthRequired)?;
        // Foreign subjects cannot destroy the owner's receipt by probing its ID.
        if receipt.binding.caller != current.caller {
            return Err(Error::Authority);
        }
        if current.validate_time(now).is_err()
            || now < receipt.issued_at
            || receipt.binding != *current
            || receipt.polkit_owner != *owner
        {
            self.receipts.remove(&current.plan_id);
            return Err(Error::Expired);
        }
        let receipt = self
            .receipts
            .remove(&current.plan_id)
            .ok_or(Error::AuthRequired)?;
        if receipt.nonce.is_nil() {
            return Err(Error::Integrity);
        }
        Ok(VerifiedSystemAuthorization {
            binding: receipt.binding,
        })
    }
}
pub struct Authorizer {
    bus: SystemBus,
    volatile: Volatile,
}
impl Authorizer {
    pub(crate) fn bus(&self) -> &SystemBus {
        &self.bus
    }
    pub fn open() -> Result<Self> {
        let bus = SystemBus::connect()?;
        let policy = policy::InstalledPolicy::load(bus.target())?;
        bus.polkit_owner(&policy.authority)?;
        Ok(Self {
            bus,
            volatile: Volatile::default(),
        })
    }
    pub fn authenticate(&self, header: &zbus::message::Header<'_>) -> Result<VerifiedCaller> {
        self.bus.authenticate(header)
    }
    pub fn policy_revision(&self) -> Result<String> {
        Ok(policy::InstalledPolicy::load(self.bus.target())?.revision)
    }
    fn binding(
        &self,
        ledger: &Ledger,
        caller: &VerifiedCaller,
        id: &str,
        hash: &str,
    ) -> Result<(Binding, policy::InstalledPolicy, Value)> {
        self.bus.recheck(caller)?;
        let (prepared, plan, actual_hash) = ledger.approval_snapshot(id, caller.identity().uid)?;
        let policy = policy::InstalledPolicy::load(self.bus.target())?;
        let identity = caller.identity();
        if actual_hash != hash
            || prepared.policy_revision != policy.revision
            || prepared.target != *self.bus.target().target()
        {
            return Err(Error::TargetChanged);
        }
        if prepared.requester.uid != identity.uid
            || identity.session.as_ref().map(|s| s.id.as_str())
                != Some(prepared.requester.logind_session.as_str())
        {
            return Err(Error::Authority);
        }
        if prepared.preview.risk != aios_state::Risk::R2 {
            return Err(Error::Authority);
        }
        let authority = self.bus.target().authority()?;
        if prepared.template_sha256 != authority.manifest_sha256 {
            return Err(Error::TargetChanged);
        }
        let binding = Binding {
            plan_id: id.into(),
            plan_hash: hash.into(),
            caller: identity.clone(),
            target: prepared.target,
            closure: plan.build.closure,
            impact_sha256: sha256(&canonical(&plan.semantic_preview)?),
            policy_revision: policy.revision.clone(),
            action: if plan.reboot_required {
                policy::ELEVATED_ACTION
            } else {
                policy::ACTION
            }
            .into(),
            frozen_at: plan.frozen_at_monotonic_ms,
            expires_at: plan.approval_expires_monotonic_ms,
        };
        binding.validate_time(boottime_ms()?)?;
        self.bus.recheck(caller)?;
        let presentation = json!({
            "schema_version": 1,
            "kind": "exact_system_plan",
            "plan_id": binding.plan_id,
            "plan_sha256": binding.plan_hash,
            "target": {
                "installation_uuid": binding.target.installation_uuid,
                "boot_id": binding.target.boot_id,
                "role": binding.target.role,
            },
            "candidate_closure": binding.closure,
            "semantic_changes": {
                "added_packages": plan.semantic_preview.added_packages,
                "removed_packages": plan.semantic_preview.removed_packages,
                "changes": plan.semantic_preview.changes,
                "user_data_deleted": plan.semantic_preview.user_data_deleted,
                "database_data_may_remain": plan.semantic_preview.database_data_may_remain,
                "notes": plan.semantic_preview.notes,
            },
            "risk": plan.semantic_preview.risk,
            "reboot_required": plan.reboot_required,
            "recovery": {
                "kind": plan.semantic_preview.recovery,
                "limit": "Restores the exact prior configuration; application or database data may remain.",
            },
            "ordered_steps": plan.ordered_steps,
            "policy_revision": binding.policy_revision,
            "expires_monotonic_ms": binding.expires_at,
        });
        Ok((binding, policy, presentation))
    }
    pub fn authorize(
        &mut self,
        ledger: &Ledger,
        caller: &VerifiedCaller,
        id: &str,
        hash: &str,
        confirmation: TrustedConfirmation,
    ) -> Result<AuthorizationStatus> {
        let (binding, policy, _) = self.binding(ledger, caller, id, hash)?;
        if confirmation.binding_sha256 != binding.confirmation_digest()? {
            return Err(Error::Integrity);
        }
        if self.volatile.receipts.contains_key(id) {
            return Err(Error::Conflict);
        }
        let native = polkit::authenticate(&self.bus, caller, &policy, &binding)?;
        let (current, current_policy, _) = self.binding(ledger, caller, id, hash)?;
        if current != binding || current_policy.revision != policy.revision {
            return Err(Error::TargetChanged);
        }
        self.volatile
            .issue(binding, confirmation, native, boottime_ms()?)?;
        Ok(AuthorizationStatus::SystemAuthorized)
    }
    pub fn authorize_interactive(
        &mut self,
        ledger: &Ledger,
        caller: &VerifiedCaller,
        id: &str,
        hash: &str,
    ) -> Result<AuthorizationStatus> {
        let (binding, _, presentation) = self.binding(ledger, caller, id, hash)?;
        let confirmation = tty::confirm(&self.bus, caller, &binding, &presentation)?;
        self.authorize(ledger, caller, id, hash, confirmation)
    }
    pub fn consume(
        &mut self,
        ledger: &Ledger,
        caller: &VerifiedCaller,
        id: &str,
        hash: &str,
    ) -> Result<VerifiedSystemAuthorization> {
        let (binding, policy, _) = match self.binding(ledger, caller, id, hash) {
            Ok(context) => context,
            Err(error) => {
                if self
                    .volatile
                    .receipts
                    .get(id)
                    .is_some_and(|r| r.binding.caller == *caller.identity())
                {
                    self.volatile.receipts.remove(id);
                }
                return Err(error);
            }
        };
        let owner = self.bus.polkit_owner(&policy.authority)?;
        self.volatile.consume(&binding, &owner, boottime_ms()?)
    }
    /// Trusted broker shutdown/policy revocation only; nothing is persisted.
    pub fn revoke_all(&mut self) {
        self.volatile.receipts.clear();
    }
}
pub fn boottime_ms() -> Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut value) } != 0
        || value.tv_sec < 0
        || !(0..1000000000).contains(&value.tv_nsec)
    {
        return Err(Error::Io);
    }
    (value.tv_sec as u64)
        .checked_mul(1000)
        .and_then(|v| v.checked_add(value.tv_nsec as u64 / 1000000))
        .ok_or(Error::Invalid)
}

#[cfg(test)]
mod tests;
