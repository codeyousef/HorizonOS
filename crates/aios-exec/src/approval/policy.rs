//! Installed policy is independent authority, never the plan's claimed revision.
use crate::{Error, Result, canonical, native, sha256, store};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub(crate) const POLICY: &[u8] = include_bytes!("../../policy/system-approval.json");
pub(crate) const ACTIONS: &[u8] = include_bytes!("../../policy/org.aios.executor.policy");
pub(crate) const ACTION: &str = "org.aios.executor.activate-exact-plan";
pub(crate) const ELEVATED_ACTION: &str = "org.aios.executor.activate-exact-plan-elevated";
pub(crate) const EXPIRY_MS: u64 = 300000;
// The pinned KDE image includes translated UDisks policy larger than 256 KiB.
// Keep each read bounded independently of the 16 MiB aggregate inventory cap.
const MAX_POLICY_FILE: u64 = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Authority {
    pub schema_version: u32,
    pub policy_path: String,
    pub policy_sha256: String,
    pub action_path: String,
    pub action_sha256: String,
    pub polkit_uid: u32,
    pub polkit_package: String,
}
impl Authority {
    pub(crate) fn validate(&self) -> Result<()> {
        let policy_root = self
            .policy_path
            .strip_suffix("/share/aios/system-approval.json")
            .ok_or(Error::Invalid)?;
        let action_root = self
            .action_path
            .strip_suffix("/share/polkit-1/actions/org.aios.executor.policy")
            .ok_or(Error::Invalid)?;
        if self.schema_version != 1
            || !store(policy_root)
            || policy_root != action_root
            || !store(&self.polkit_package)
            || self.polkit_uid == 0
            || self.polkit_uid == u32::MAX
            || self.policy_sha256 != sha256(POLICY)
            || self.action_sha256 != sha256(ACTIONS)
        {
            return Err(Error::Integrity);
        }
        Ok(())
    }
    pub(crate) fn daemon(&self) -> String {
        format!("{}/lib/polkit-1/polkitd", self.polkit_package)
    }
}
/// This native value has no caller-supplied constructor or deserialization.
pub(crate) struct InstalledPolicy {
    pub authority: Authority,
    pub revision: String,
}
impl InstalledPolicy {
    pub(crate) fn load(target: &native::VerifiedTarget) -> Result<Self> {
        target.recheck()?;
        let authority: Authority = native::installed("approval-authority.json")?;
        authority.validate()?;
        if native::read(
            Path::new(&authority.policy_path),
            Path::new("/nix/store"),
            true,
            65536,
        )? != POLICY
            || native::read(
                Path::new(&authority.action_path),
                Path::new("/nix/store"),
                true,
                65536,
            )? != ACTIONS
        {
            return Err(Error::Integrity);
        }
        let revision = runtime_revision(&authority)?;
        target.recheck()?;
        Ok(Self {
            authority,
            revision,
        })
    }
}
#[derive(Serialize)]
struct Entry {
    path: String,
    resolved: Option<String>,
    sha256: Option<String>,
}
fn directory(path: &Path, entries: &mut Vec<Entry>, total: &mut usize) -> Result<()> {
    if !path.exists() {
        // Check protected ancestors even for an absent optional policy directory.
        let mut parent = path.parent().ok_or(Error::Invalid)?;
        while !parent.exists() {
            parent = parent.parent().ok_or(Error::Invalid)?;
        }
        native::resolved(parent, Path::new("/"))?;
        entries.push(Entry {
            path: path.to_str().ok_or(Error::Invalid)?.into(),
            resolved: None,
            sha256: None,
        });
        return Ok(());
    }
    let root = native::resolved(path, Path::new("/"))?;
    let before = fs::symlink_metadata(&root)?;
    if !before.is_dir() {
        return Err(Error::Integrity);
    }
    entries.push(Entry {
        path: path.to_str().ok_or(Error::Invalid)?.into(),
        resolved: Some(root.to_str().ok_or(Error::Invalid)?.into()),
        sha256: None,
    });
    let mut paths = fs::read_dir(&root)?
        .map(|e| e.map(|v| v.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    if paths.len() > 1024 {
        return Err(Error::Integrity);
    }
    for file in paths {
        let resolved = native::resolved(&file, Path::new("/"))?;
        let bytes = native::read(&file, Path::new("/"), false, MAX_POLICY_FILE)?;
        *total = total.checked_add(bytes.len()).ok_or(Error::Integrity)?;
        if *total > 16 * 1024 * 1024 || entries.len() >= 4096 {
            return Err(Error::Integrity);
        }
        entries.push(Entry {
            path: file.to_str().ok_or(Error::Invalid)?.into(),
            resolved: Some(resolved.to_str().ok_or(Error::Invalid)?.into()),
            sha256: Some(sha256(&bytes)),
        });
    }
    let after = fs::symlink_metadata(root)?;
    if (
        before.dev(),
        before.ino(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return Err(Error::TargetChanged);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the installed pinned KDE development image"]
    fn installed_udisks_policy_bounds() {
        let path = Path::new(
            "/run/current-system/sw/share/polkit-1/actions/org.freedesktop.UDisks2.policy",
        );
        assert_eq!(
            native::read(path, Path::new("/"), false, 262144),
            Err(Error::Integrity)
        );
        let bytes = native::read(path, Path::new("/"), false, MAX_POLICY_FILE).unwrap();
        assert!(bytes.len() > 262144 && bytes.len() as u64 <= MAX_POLICY_FILE);
        let mut entries = Vec::new();
        let mut total = 0;
        directory(path.parent().unwrap(), &mut entries, &mut total).unwrap();
        let selected = native::resolved(path, Path::new("/")).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.resolved.as_deref() == selected.to_str()
                    && entry.sha256.as_ref() == Some(&sha256(&bytes)))
        );
        println!(
            "AIOS_INSTALLED_POLICY_BOUND {}",
            serde_json::json!({
                "evidence_kind":"actual-installed-public-policy-read",
                "path":path.to_str().unwrap(),"size":bytes.len(),"sha256":sha256(&bytes),
                "previous_limit_rejected":true,"per_file_limit":MAX_POLICY_FILE,
                "inventory_bytes":total,"inventory_entries":entries.len(),
                "authorization_performed":false,"root_authority_minted":false
            })
        );
    }
}
fn runtime_revision(authority: &Authority) -> Result<String> {
    let mut entries = Vec::new();
    let mut total = 0;
    for root in [
        PathBuf::from("/etc/polkit-1/rules.d"),
        PathBuf::from("/run/current-system/sw/share/polkit-1/rules.d"),
        PathBuf::from("/run/current-system/sw/share/polkit-1/actions"),
        PathBuf::from(&authority.polkit_package).join("share/polkit-1/rules.d"),
        PathBuf::from(&authority.polkit_package).join("share/polkit-1/actions"),
    ] {
        directory(&root, &mut entries, &mut total)?;
    }
    for file in ["/etc/passwd", "/etc/group"] {
        let resolved = native::resolved(Path::new(file), Path::new("/"))?;
        let bytes = native::read(Path::new(file), Path::new("/"), false, 1048576)?;
        entries.push(Entry {
            path: file.into(),
            resolved: Some(resolved.to_str().ok_or(Error::Invalid)?.into()),
            sha256: Some(sha256(&bytes)),
        });
    }
    Ok(sha256(&canonical(&(
        authority,
        native::running()?.to_str().ok_or(Error::Invalid)?,
        entries,
    ))?))
}
