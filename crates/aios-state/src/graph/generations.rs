//! Fixed native system pointers. Running activation and selected system profile
//! are distinct observations. Neither proves the bootloader's selected entry,
//! an approved manifest, closure contents, or permission to activate anything.
use super::{native::{NativeTime, Error, Result}, store::{GraphStore, Node, Scope, SourceTruth, ProviderSnapshot, ProviderState}, ObservationTime, SourceRevision};
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::fs::MetadataExt, path::{Path,PathBuf}};

pub const RUNNING_PROVIDER: &str = "native-running-system-pointer";
pub const PROFILE_PROVIDER: &str = "native-selected-system-profile";
const CURRENT: &str = "/run/current-system";
const PROFILE: &str = "/nix/var/nix/profiles/system";

const MAX_PROFILE_ENTRIES:usize=1024;
const MAX_SYSTEM_GENERATIONS:usize=256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemProfileGeneration {
    pub generation:u64,
    pub closure:String,
    pub selected:bool,
    pub running:bool,
    pub ownership:String,
    pub management_attribution:Option<String>,
}
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
    pub profile_generations: Vec<SystemProfileGeneration>,
    pub profile_history_complete: bool,
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
fn system_generations(running:&str,selected:Option<(&str,u64)>)->Result<Vec<SystemProfileGeneration>> {
    let directory=Path::new("/nix/var/nix/profiles");
    let before=fs::symlink_metadata(directory).map_err(|_|Error::Native(aios_protocol::contracts::ErrorCode::TargetNotFound))?;
    if !before.is_dir() || before.uid()!=0 || before.mode()&0o022!=0 { return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged)); }
    let mut paths=Vec::<(u64,PathBuf)>::new();
    for (count,entry) in fs::read_dir(directory).map_err(|_|Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?.enumerate() {
        if count>=MAX_PROFILE_ENTRIES{return Err(Error::Native(aios_protocol::contracts::ErrorCode::ResourceExhausted));}
        let entry=entry.map_err(|_|Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
        let name=entry.file_name();let Some(text)=name.to_str() else{return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));};
        if text.starts_with("system-")&&text.ends_with("-link"){
            let number=generation(Path::new(text)).ok_or(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
            paths.push((number,entry.path()));
        }
    }
    if paths.len()>MAX_SYSTEM_GENERATIONS{return Err(Error::Native(aios_protocol::contracts::ErrorCode::ResourceExhausted));}
    paths.sort_by_key(|entry|entry.0);
    if paths.windows(2).any(|pair|pair[0].0==pair[1].0){return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));}
    let mut result=Vec::with_capacity(paths.len());
    for (number,path) in paths {
        let path=path.to_str().ok_or(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
        let (target,_)=pointer(path)?;
        result.push(SystemProfileGeneration{generation:number,selected:selected.is_some_and(|(closure,generation)|generation==number&&closure==target),
            running:target==running,closure:target,ownership:"declarative_system_profile".into(),management_attribution:None});
    }
    let after=fs::symlink_metadata(directory).map_err(|_|Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    if (before.dev(),before.ino(),before.ctime(),before.ctime_nsec())!=(after.dev(),after.ino(),after.ctime(),after.ctime_nsec()){
        return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));
    }
    Ok(result)
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
    let profile_generations=system_generations(&running.0,selected_profile_closure.as_deref().zip(selected_profile_generation))?;
    if system_generations(&running.0,selected_profile_closure.as_deref().zip(selected_profile_generation))?!=profile_generations {
        return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));
    }
    Ok((captured, SystemPointers { running_closure: running.0, selected_profile_closure, selected_profile_generation,
        running_profile_divergence, bootloader_entry: None, managed_transaction: None, profile_generations,profile_history_complete:true }))
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
                    "bootloader_entry":self.pointers.bootloader_entry,"managed_transaction":self.pointers.managed_transaction,
                    "profile_generations":self.pointers.profile_generations,"profile_history_complete":self.pointers.profile_history_complete,
                    "user_profiles_observed":false,"user_profile_reason":"requires_user_scoped_owner","captured":self.captured})))?;
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
