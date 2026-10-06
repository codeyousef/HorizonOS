//! Graph observations are evidence, never authorization or live preconditions.
//! Persisted monotonic time is meaningful only in the observation's own boot.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub mod store;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BootId(String);
impl BootId {
    pub fn parse(value: &str) -> Option<Self> {
        let valid = value.len() == 36 && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) { b == b'-' }
            else { b.is_ascii_digit() || (b'a'..=b'f').contains(&b) }
        });
        valid.then(|| Self(value.to_owned()))
    }
    fn valid(&self) -> bool { Self::parse(&self.0).is_some() }
}

/// Native UID is part of process identity; neither a PID nor a boot alone is
/// sufficient. This constructor accepts observations, not a grant to inspect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessIdentity { uid: u32, boot: BootId, pid: u32, start_ticks: u64 }
impl ProcessIdentity {
    pub fn observed(uid: u32, boot: BootId, pid: u32, start_ticks: u64) -> Option<Self> {
        (boot.valid() && pid > 0 && start_ticks > 0).then_some(Self { uid, boot, pid, start_ticks })
    }
    pub fn stable_key(&self) -> String {
        // Length-delimited canonical JSON prevents concatenation ambiguity.
        let bytes = serde_json::to_vec(&("process", self.uid, &self.boot, self.pid, self.start_ticks)).expect("fixed serializable tuple");
        format!("process:{:x}", Sha256::digest(bytes))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessClass { Process, Ui, Service, Network, Audio, StorageFree, Inventory, Documentation }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationTime { pub boot: BootId, pub realtime_ns: u64, pub monotonic_ns: u64 }

/// Revision inputs come from authoritative providers. Absent revisions remain
/// unknown; equality between two absent values must not establish freshness.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceRevision {
    pub generation: Option<String>,
    pub profile: Option<String>,
    pub closure_hash: Option<String>,
    pub document_hash: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness { Current, Stale, Unknown }
fn same_known(before: &Option<String>, after: &Option<String>) -> Freshness {
    match (before, after) {
        (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => {
            if a == b { Freshness::Current } else { Freshness::Stale }
        }
        _ => Freshness::Unknown,
    }
}
fn combined(a: Freshness, b: Freshness) -> Freshness {
    if a == Freshness::Stale || b == Freshness::Stale { Freshness::Stale }
    else if a == Freshness::Unknown || b == Freshness::Unknown { Freshness::Unknown }
    else { Freshness::Current }
}

/// Diagnostic cache eligibility only. Execution must acquire critical
/// preconditions live through the provider; Current never grants an effect.
pub fn freshness(class: FreshnessClass, observed: &ObservationTime, now: &ObservationTime,
    before: &SourceRevision, after: &SourceRevision) -> Freshness {
    if !observed.boot.valid() || !now.boot.valid() { return Freshness::Unknown; }
    match class {
        FreshnessClass::Inventory => combined(same_known(&before.generation, &after.generation), same_known(&before.profile, &after.profile)),
        FreshnessClass::Documentation => combined(same_known(&before.closure_hash, &after.closure_hash), same_known(&before.document_hash, &after.document_hash)),
        other => {
            if observed.boot != now.boot { return Freshness::Stale; }
            let Some(age) = now.monotonic_ns.checked_sub(observed.monotonic_ns) else { return Freshness::Unknown; };
            let seconds = match other {
                FreshnessClass::Process | FreshnessClass::Ui => 2,
                FreshnessClass::Service | FreshnessClass::Network | FreshnessClass::Audio => 5,
                FreshnessClass::StorageFree => 30,
                _ => unreachable!(),
            };
            if age <= seconds * 1_000_000_000 { Freshness::Current } else { Freshness::Stale }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn boot(n: u8) -> BootId { BootId::parse(&format!("{n:08}-1111-4111-8111-111111111111")).unwrap() }
    fn time(boot: BootId, mono: u64) -> ObservationTime { ObservationTime { boot, monotonic_ns: mono, realtime_ns: 1 } }
    #[test]
    fn reused_pid_other_user_start_and_boot_have_distinct_keys() {
        let base = ProcessIdentity::observed(1000, boot(1), 42, 99).unwrap().stable_key();
        for other in [ProcessIdentity::observed(1001, boot(1), 42, 99), ProcessIdentity::observed(1000, boot(2), 42, 99),
            ProcessIdentity::observed(1000, boot(1), 42, 100), ProcessIdentity::observed(1000, boot(1), 43, 99)] {
            assert_ne!(base, other.unwrap().stable_key());
        }
        assert!(ProcessIdentity::observed(1000, boot(1), 0, 99).is_none());
        assert!(ProcessIdentity::observed(1000, boot(1), 42, 0).is_none());
    }
    #[test]
    fn persisted_monotonic_time_cannot_cross_boot_or_run_backwards() {
        let empty = SourceRevision::default();
        assert_eq!(freshness(FreshnessClass::Service, &time(boot(1), 10), &time(boot(2), 11), &empty, &empty), Freshness::Stale);
        assert_eq!(freshness(FreshnessClass::Service, &time(boot(1), 10), &time(boot(1), 9), &empty, &empty), Freshness::Unknown);
        let forged: BootId = serde_json::from_str("\"unverified-boot\"").unwrap();
        assert!(ProcessIdentity::observed(1000, forged.clone(), 42, 99).is_none());
        assert_eq!(freshness(FreshnessClass::Ui, &time(forged.clone(), 1), &time(forged, 1), &empty, &empty), Freshness::Unknown);
    }
    #[test]
    fn wall_clock_change_does_not_extend_native_observation_lifetime() {
        let empty = SourceRevision::default();
        for (class, seconds) in [(FreshnessClass::Process, 2), (FreshnessClass::Ui, 2), (FreshnessClass::Service, 5),
            (FreshnessClass::Network, 5), (FreshnessClass::Audio, 5), (FreshnessClass::StorageFree, 30)] {
            let old = time(boot(1), 1);
            let mut current = time(boot(1), 1 + seconds * 1_000_000_000);
            current.realtime_ns = 0;
            assert_eq!(freshness(class, &old, &current, &empty, &empty), Freshness::Current);
            current.monotonic_ns += 1;
            assert_eq!(freshness(class, &old, &current, &empty, &empty), Freshness::Stale);
        }
    }
    #[test]
    fn missing_revisions_are_unknown_and_manual_profile_change_is_stale() {
        let empty = SourceRevision::default(); let t = time(boot(1), 1);
        let observed = SourceRevision { generation: Some("generation-1".into()), profile: Some("profile-1".into()),
            closure_hash: Some("closure-1".into()), document_hash: Some("document-1".into()) };
        for class in [FreshnessClass::Inventory, FreshnessClass::Documentation] {
            assert_eq!(freshness(class, &t, &t, &empty, &empty), Freshness::Unknown);
            assert_eq!(freshness(class, &t, &t, &observed, &observed), Freshness::Current);
        }
        let mut changed = observed.clone(); changed.profile = Some("manual-profile".into());
        assert_eq!(freshness(FreshnessClass::Inventory, &t, &t, &observed, &changed), Freshness::Stale);
        changed = observed.clone(); changed.document_hash = None;
        assert_eq!(freshness(FreshnessClass::Documentation, &t, &t, &observed, &changed), Freshness::Unknown);
    }
}
