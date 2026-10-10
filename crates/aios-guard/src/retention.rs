use crate::{Candidate, Error, Plan, Result};
use serde::Deserialize;
use std::{
    fs::{self, File},
    os::unix::fs::{MetadataExt, symlink},
    path::{Path, PathBuf},
};

const POLICY_SUFFIX: &str = "etc/aios/transaction-policy.json";
const ROOT: &str = "/nix/var/nix/gcroots/aios-known-good";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    schema_version: u32,
    keep_known_good_generations: usize,
}

fn policy(plan: &Plan) -> Result<Policy> {
    let closure = match &plan.candidate {
        Candidate::System { closure } => &closure.path,
        Candidate::ModelOnly { .. } => return Err(Error::InvalidPlan),
    };
    let path = Path::new(closure).join(POLICY_SUFFIX);
    let resolved = fs::canonicalize(&path).map_err(|_| Error::Integrity)?;
    let resolved = resolved.to_str().ok_or(Error::Integrity)?;
    if !crate::valid_store(resolved) {
        return Err(Error::Integrity);
    }
    let bytes = fs::read(resolved).map_err(|_| Error::Integrity)?;
    if bytes.len() > 4096 {
        return Err(Error::Integrity);
    }
    let value: Policy = serde_json::from_slice(&bytes).map_err(|_| Error::Integrity)?;
    if value.schema_version != 1 || !(3..=100).contains(&value.keep_known_good_generations) {
        return Err(Error::Integrity);
    }
    Ok(value)
}

fn private_directory(path: &Path, owner: u32) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Integrity)?;
    if !metadata.is_dir()
        || metadata.uid() != owner
        || metadata.mode() & 0o777 != 0o700
        || fs::canonicalize(path).map_err(|_| Error::Integrity)? != path
    {
        return Err(Error::Integrity);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| Error::Adapter)
}

fn entries(root: &Path, owner: u32) -> Result<Vec<(std::time::SystemTime, PathBuf)>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root).map_err(|_| Error::Integrity)? {
        let entry = entry.map_err(|_| Error::Integrity)?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_str().ok_or(Error::Integrity)?;
        let metadata = fs::symlink_metadata(&path).map_err(|_| Error::Integrity)?;
        if uuid::Uuid::parse_str(name).map(|value| value.to_string() != name).unwrap_or(true)
            || !metadata.file_type().is_symlink()
            || metadata.uid() != owner
        {
            return Err(Error::Integrity);
        }
        let target = fs::read_link(&path).map_err(|_| Error::Integrity)?;
        if !target.is_absolute() || !crate::valid_store(target.to_str().ok_or(Error::Integrity)?) {
            return Err(Error::Integrity);
        }
        result.push((metadata.modified().map_err(|_| Error::Integrity)?, path));
    }
    result.sort_by(|left, right| left.cmp(right));
    Ok(result)
}

fn retain_closure_at(
    root: &Path,
    transaction_id: &str,
    closure: &str,
    keep: usize,
    owner: u32,
) -> Result<()> {
    private_directory(root, owner)?;
    if uuid::Uuid::parse_str(transaction_id)
        .map(|value| value.hyphenated().to_string() != transaction_id)
        .unwrap_or(true)
        || !crate::valid_store(closure)
        || !(2..=100).contains(&keep)
    {
        return Err(Error::Integrity);
    }
    let current = root.join(transaction_id);
    match fs::symlink_metadata(&current) {
        Ok(metadata) => {
            if !metadata.file_type().is_symlink()
                || metadata.uid() != owner
                || fs::read_link(&current).map_err(|_| Error::Integrity)? != Path::new(closure)
            {
                return Err(Error::Integrity);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            symlink(closure, &current).map_err(|_| Error::Adapter)?;
            sync_directory(root)?;
        }
        Err(_) => return Err(Error::Integrity),
    }
    let all = entries(root, owner)?;
    for (_, path) in all.iter().filter(|(_, path)| {
        path != &current
            && fs::read_link(path)
                .is_ok_and(|target| target == Path::new(closure))
    }) {
        fs::remove_file(path).map_err(|_| Error::Adapter)?;
    }
    let all = entries(root, owner)?;
    let remove = all.len().saturating_sub(keep);
    for (_, path) in all
        .into_iter()
        .filter(|(_, path)| path != &current)
        .take(remove)
    {
        fs::remove_file(path).map_err(|_| Error::Adapter)?;
    }
    sync_directory(root)
}

fn retain_at(root: &Path, plan: &Plan, keep: usize, owner: u32) -> Result<()> {
    let candidate = match &plan.candidate {
        Candidate::System { closure } => &closure.path,
        Candidate::ModelOnly { .. } => return Ok(()),
    };
    let active = fs::canonicalize("/nix/var/nix/profiles/system").map_err(|_| Error::Integrity)?;
    let active = active.to_str().ok_or(Error::Integrity)?;
    if active != candidate && active != plan.prior.profile.path {
        return Err(Error::Integrity);
    }
    retain_closure_at(root, &plan.transaction_id, active, keep, owner)
}

pub(crate) fn retain_committed(plan: &Plan) -> Result<()> {
    let value = policy(plan)?;
    retain_at(Path::new(ROOT), plan, value.keep_known_good_generations, 0)
}

#[cfg(test)]
mod tests {
    use super::retain_closure_at;
    use crate::Error;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt},
        path::PathBuf,
        thread,
        time::Duration,
    };

    const A: &str = "/nix/store/00000000000000000000000000000000-system-a";
    const B: &str = "/nix/store/11111111111111111111111111111111-system-b";
    const C: &str = "/nix/store/22222222222222222222222222222222-system-c";
    const ID1: &str = "00000000-0000-4000-8000-000000000001";
    const ID2: &str = "00000000-0000-4000-8000-000000000002";
    const ID3: &str = "00000000-0000-4000-8000-000000000003";

    #[test]
    fn policy_target_must_be_a_store_object() {
        assert!(crate::valid_store(
            "/nix/store/00000000000000000000000000000000-etc-aios-transaction-policy.json"
        ));
        assert!(!crate::valid_store(
            "/nix/store/00000000000000000000000000000000-system/etc/aios/transaction-policy.json"
        ));
    }
    const ID4: &str = "00000000-0000-4000-8000-000000000004";

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("aios-known-good-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        root
    }

    #[test]
    fn keeps_distinct_recent_generations_and_refreshes_duplicates() {
        let root = root();
        let owner = fs::metadata(&root).unwrap().uid();
        retain_closure_at(&root, ID1, A, 2, owner).unwrap();
        thread::sleep(Duration::from_millis(2));
        retain_closure_at(&root, ID2, A, 2, owner).unwrap();
        thread::sleep(Duration::from_millis(2));
        retain_closure_at(&root, ID3, B, 2, owner).unwrap();
        thread::sleep(Duration::from_millis(2));
        retain_closure_at(&root, ID4, C, 2, owner).unwrap();

        let mut retained = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        retained.sort();
        assert_eq!(retained, [ID3, ID4]);
        assert_eq!(fs::read_link(root.join(ID3)).unwrap(), PathBuf::from(B));
        assert_eq!(fs::read_link(root.join(ID4)).unwrap(), PathBuf::from(C));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_foreign_entries_without_pruning_them() {
        let root = root();
        let owner = fs::metadata(&root).unwrap().uid();
        fs::write(root.join("foreign"), b"do not remove").unwrap();
        assert_eq!(
            retain_closure_at(&root, ID1, A, 2, owner),
            Err(Error::Integrity)
        );
        assert_eq!(fs::read(root.join("foreign")).unwrap(), b"do not remove");
        fs::remove_dir_all(root).unwrap();
    }
}
