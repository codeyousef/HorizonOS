//! Fixed native system pointers. Running activation and selected system profile
//! are distinct observations. Neither proves the bootloader's selected entry,
//! an approved manifest, closure contents, or permission to activate anything.
use super::{native::{NativeTime, Error, Result}, store::{GraphStore, Node, Scope, SourceTruth, ProviderSnapshot, ProviderState}, ObservationTime, SourceRevision};
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::fs::MetadataExt, path::Path};

pub const RUNNING_PROVIDER: &str = "native-running-system-pointer";
pub const PROFILE_PROVIDER: &str = "native-selected-system-profile";
const CURRENT: &str = "/run/current-system";
const PROFILE: &str = "/nix/var/nix/profiles/system";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemPointers {
    pub running_closure: String,
    pub selected_profile_closure: Option<String>,
    pub selected_profile_generation: Option<u64>,
    pub running_profile_divergence: Option<bool>,
    /// The profile is not proof of a bootloader entry or of managed provenance.
    pub bootloader_entry: Option<String>,
    pub managed_transaction: Option<String>,
}

fn closure(value: &str) -> bool {
    let Some(name) = value.strip_prefix("/nix/store/") else { return false; };
    let Some((hash, name)) = name.split_once('-') else { return false; };
    hash.len() == 32 && hash.bytes().all(|b| b"0123456789abcdfghijklmnpqrsvwxyz".contains(&b))
        && name.starts_with("nixos-system-") && name.len() > 13 && name.len() <= 200
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"+._-".contains(&b))
}
fn generation(value: &Path) -> Option<u64> {
    // Fixed root-owned profile link: reject nested locators, signs and aliases.
    let value = value.to_str()?;
    let number = value.strip_prefix("system-")?.strip_suffix("-link")?;
    if number.is_empty() || number.starts_with('0') || !number.bytes().all(|b| b.is_ascii_digit()) { return None; }
    number.parse().ok()
}
fn pointer(path: &str) -> Result<(String, Option<u64>)> {
    let before = fs::symlink_metadata(path).map_err(|_| Error::Native(aios_protocol::contracts::ErrorCode::TargetNotFound))?;
    if !before.file_type().is_symlink() || before.uid() != 0 { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
    let raw = fs::read_link(path).map_err(|_| Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    let resolved = fs::canonicalize(path).map_err(|_| Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    let value = resolved.to_str().filter(|v| closure(v)).ok_or(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    let target = fs::symlink_metadata(&resolved).map_err(|_| Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    if !target.is_dir() || target.uid() != 0 || target.mode() & 0o022 != 0 { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
    let after = fs::symlink_metadata(path).map_err(|_| Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    if (before.dev(), before.ino(), before.ctime(), before.ctime_nsec()) != (after.dev(), after.ino(), after.ctime(), after.ctime_nsec())
        || fs::read_link(path).ok().as_ref() != Some(&raw) || fs::canonicalize(path).ok().as_ref() != Some(&resolved) {
        return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));
    }
    Ok((value.into(), if path == PROFILE { generation(&raw) } else { None }))
}

/// Reads only the two compiled native paths. No arbitrary profile, deserialized
/// path, command, effect, or caller assertion is accepted.
pub fn observe() -> Result<(ObservationTime, SystemPointers)> {
    let captured = NativeTime::observe()?.observation().clone();
    let running = pointer(CURRENT)?;
    let profile = match pointer(PROFILE) {
        Ok(value) => Some(value),
        Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetNotFound)) => None,
        Err(error) => return Err(error),
    };
    // Re-read both independently mutable pointers before publishing the pair.
    if pointer(CURRENT)? != running || match (&profile, pointer(PROFILE)) {
        (Some(before), Ok(after)) => before != &after,
        (None, Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetNotFound))) => false,
        _ => true,
    } { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
    let now = NativeTime::observe()?.observation().clone();
    if captured.boot != now.boot || now.monotonic_ns.checked_sub(captured.monotonic_ns).is_none_or(|n| n > 2_000_000_000) { return Err(Error::Expired); }
    let selected_profile_closure = profile.as_ref().map(|p| p.0.clone());
    let selected_profile_generation = profile.as_ref().and_then(|p| p.1);
    let running_profile_divergence = selected_profile_closure.as_ref().map(|p| p != &running.0);
    Ok((captured, SystemPointers { running_closure: running.0, selected_profile_closure, selected_profile_generation,
        running_profile_divergence, bootloader_entry: None, managed_transaction: None }))
}

pub struct NativeGenerationSnapshot { captured: ObservationTime, pointers: SystemPointers,
    running_token: Option<String>, profile_token: Option<String> }
impl NativeGenerationSnapshot {
    pub fn collect(store: &GraphStore) -> Result<Self> {
        if store.native_scope() != Scope::System { return Err(Error::WrongScope); }
        let result = (|| {
            let (captured, pointers) = observe()?;
            let revision = Self::revision(&pointers);
            let running_token = store.reconciliation_plan(RUNNING_PROVIDER.into(), captured.clone(), revision.clone())?.state.map(|s| s.token);
            let profile_token = store.reconciliation_plan(PROFILE_PROVIDER.into(), captured.clone(), revision)?.state.map(|s| s.token);
            Ok(Self { captured, pointers, running_token, profile_token })
        })();
        if result.is_err() { store.report_event_loss(); }
        result
    }
    fn revision(p: &SystemPointers) -> SourceRevision {
        SourceRevision { generation: Some(p.running_closure.clone()), profile: p.selected_profile_closure.clone(),
            closure_hash: None, document_hash: None }
    }
    pub fn pointers(&self) -> &SystemPointers { &self.pointers }
    pub fn captured(&self) -> &ObservationTime { &self.captured }
    pub fn apply(&self, store: &GraphStore) -> Result<(ProviderState, ProviderState)> {
        if store.native_scope() != Scope::System { return Err(Error::WrongScope); }
        let result = (|| {
            let (_, current) = observe()?;
            let now = NativeTime::observe()?.observation().clone();
            if current != self.pointers || self.captured.boot != now.boot
                || now.monotonic_ns.checked_sub(self.captured.monotonic_ns).is_none_or(|n| n > 2_000_000_000) {
                return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));
            }
            let revision = Self::revision(&self.pointers);
            let snapshot = |provider: &str, token: Option<String>, truth, id: &str, complete, properties| ProviderSnapshot {
                provider: provider.into(), expected_token: token, source_truth: truth, time: self.captured.clone(), source_revision: revision.clone(), complete,
                verified_absent_ids: vec![], nodes: vec![Node { id: id.into(), kind: "generation".into(), scope: Scope::System,
                    provider: provider.into(), stable_key: id.into(), properties,
                    source_truth: truth, realtime_ns: self.captured.realtime_ns }] };
            let running = store.apply_provider_snapshot(snapshot(RUNNING_PROVIDER, self.running_token.clone(), SourceTruth::Running, "generation:running-system", true,
                serde_json::json!({"running_closure":self.pointers.running_closure,"captured":self.captured})))?;
            let selected = store.apply_provider_snapshot(snapshot(PROFILE_PROVIDER, self.profile_token.clone(), SourceTruth::BootSelected,
                "generation:selected-system-profile", self.pointers.selected_profile_generation.is_some(),
                serde_json::json!({"selected_profile_closure":self.pointers.selected_profile_closure,"selected_profile_generation":self.pointers.selected_profile_generation,
                    "bootloader_entry":self.pointers.bootloader_entry,"managed_transaction":self.pointers.managed_transaction,"captured":self.captured})))?;
            if observe()?.1 != self.pointers { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
            Ok((running, selected))
        })();
        if result.is_err() { store.report_event_loss(); }
        result
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn paths_and_generations_are_native_identifiers_only() {
        assert!(closure("/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-fixture-26.05"));
        for value in ["/tmp/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-fixture", "/nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-nixos-system-fixture",
            "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-other", "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-nixos-system-x/../y"] { assert!(!closure(value)); }
        assert_eq!(generation(Path::new("system-12-link")), Some(12));
        for value in ["system-0-link", "system-01-link", "system-+1-link", "system--1-link", "../system-1-link", "system-18446744073709551616-link"] {
            assert_eq!(generation(Path::new(value)), None);
        }
    }
}
