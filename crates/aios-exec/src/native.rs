//! Fixed native target and installed-authority intake. No client paths or IDs.
use crate::ledger::Target;
use crate::{Error, Result, canonical, digest, store, uuid};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Enrollment {
    schema_version: u32,
    os_id: String,
    os_version: String,
    installation_uuid: String,
    dmi_uuid: String,
    guest_role: String,
    disk_serial: String,
    disk_device: String,
    root_partition: String,
    root_filesystem: String,
    management_channel: String,
}
impl Enrollment {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.os_id != "nixos"
            || self.os_version != "26.05"
            || !uuid(&self.installation_uuid)
            || !uuid(&self.dmi_uuid)
            || self.installation_uuid == uuid::Uuid::nil().to_string()
            || self.dmi_uuid == uuid::Uuid::nil().to_string()
            || !matches!(
                self.guest_role.as_str(),
                "development" | "production" | "acceptance"
            )
            || !matches!(
                self.management_channel.as_str(),
                "ssh-development" | "local-product"
            )
            || (self.guest_role == "development" && self.management_channel != "ssh-development")
            || self.root_filesystem != "btrfs"
            || [&self.disk_device, &self.root_partition, &self.disk_serial]
                .iter()
                .any(|s| {
                    s.is_empty()
                        || s.len() > 64
                        || !s
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                })
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstalledAuthority {
    pub schema_version: u32,
    pub template_path: String,
    pub manifest_sha256: String,
    pub base_template_revision: String,
    pub catalog_revision: String,
    pub lock_sha256: String,
}
impl InstalledAuthority {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !store(&self.template_path)
            || ![
                &self.manifest_sha256,
                &self.base_template_revision,
                &self.catalog_revision,
                &self.lock_sha256,
            ]
            .iter()
            .all(|v| digest(v))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
fn root() -> Result<()> {
    if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
        return Err(Error::Authority);
    }
    let map = read(
        Path::new("/proc/self/uid_map"),
        Path::new("/proc"),
        false,
        4096,
    )
    .map_err(|_| Error::Authority)?;
    if !full_root_mapping(&map) {
        return Err(Error::Authority);
    }
    Ok(())
}
fn full_root_mapping(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .is_ok_and(|map| map.split_whitespace().collect::<Vec<_>>() == ["0", "0", "4294967295"])
}
fn normalized(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(Error::Invalid);
    }
    let mut result = PathBuf::from("/");
    for c in path.components() {
        match c {
            Component::RootDir => {}
            Component::Normal(n) => result.push(n),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return Err(Error::Invalid);
                }
            }
            _ => return Err(Error::Invalid),
        }
    }
    Ok(result)
}
/// Every traversed object is administrator/kernel owned. Only the sticky Nix
/// store can be writable by another UID; its existing readonly objects stay safe.
pub(crate) fn resolved(path: &Path, scope: &Path) -> Result<PathBuf> {
    let mut selected = normalized(path)?;
    for _ in 0..32 {
        let mut cursor = PathBuf::from("/");
        let parts: Vec<_> = selected
            .components()
            .filter_map(|c| {
                if let Component::Normal(n) = c {
                    Some(n.to_owned())
                } else {
                    None
                }
            })
            .collect();
        let mut redirected = false;
        for (i, part) in parts.iter().enumerate() {
            cursor.push(part);
            let before = fs::symlink_metadata(&cursor)?;
            if before.uid() != 0 {
                return Err(Error::Ownership);
            }
            if before.file_type().is_symlink() {
                // Only the fixed kernel self alias is permitted in procfs.
                // Generic FD/root magic links cannot redirect configuration.
                if cursor.starts_with("/proc")
                    && !(cursor == Path::new("/proc/self")
                        && scope == Path::new("/proc")
                        && path.starts_with("/proc/self"))
                {
                    return Err(Error::Integrity);
                }
                let link = fs::read_link(&cursor)?;
                if cursor == Path::new("/proc/self")
                    && link != PathBuf::from(std::process::id().to_string())
                {
                    return Err(Error::Integrity);
                }
                let after = fs::symlink_metadata(&cursor)?;
                if (
                    before.dev(),
                    before.ino(),
                    before.ctime(),
                    before.ctime_nsec(),
                ) != (after.dev(), after.ino(), after.ctime(), after.ctime_nsec())
                {
                    return Err(Error::TargetChanged);
                }
                let mut next = if link.is_absolute() {
                    link
                } else {
                    cursor.parent().ok_or(Error::Invalid)?.join(link)
                };
                for remaining in &parts[i + 1..] {
                    next.push(remaining);
                }
                selected = normalized(&next)?;
                redirected = true;
                break;
            }
            if before.is_dir() {
                let sticky_store =
                    cursor == Path::new("/nix/store") && before.mode() & libc::S_ISVTX != 0;
                if before.mode() & 0o022 != 0 && !sticky_store {
                    return Err(Error::Ownership);
                }
            } else if i + 1 != parts.len() {
                return Err(Error::Ownership);
            }
        }
        if !redirected {
            if !selected.starts_with(scope) {
                return Err(Error::Ownership);
            }
            return Ok(selected);
        }
    }
    Err(Error::Integrity)
}
pub(crate) fn read(path: &Path, scope: &Path, immutable: bool, max: u64) -> Result<Vec<u8>> {
    let selected = resolved(path, scope)?;
    let before = fs::symlink_metadata(&selected)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(&selected)?;
    let info = file.metadata()?;
    if !info.is_file()
        || info.uid() != 0
        || info.mode() & 0o022 != 0
        || (immutable && info.mode() & 0o7777 != 0o444)
        || (info.dev(), info.ino()) != (before.dev(), before.ino())
    {
        return Err(Error::Ownership);
    }
    let mut data = Vec::new();
    (&file).take(max + 1).read_to_end(&mut data)?;
    let after = file.metadata()?;
    if data.len() as u64 > max
        || (
            info.dev(),
            info.ino(),
            info.len(),
            info.mtime(),
            info.mtime_nsec(),
            info.ctime(),
            info.ctime_nsec(),
        ) != (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec(),
        )
    {
        return Err(Error::Integrity);
    }
    Ok(data)
}
fn text(path: &str, scope: &str) -> Result<String> {
    String::from_utf8(read(Path::new(path), Path::new(scope), false, 16384)?)
        .map(|s| s.trim().to_owned())
        .map_err(|_| Error::Invalid)
}
pub(crate) fn running() -> Result<PathBuf> {
    let value = resolved(Path::new("/run/current-system"), Path::new("/nix/store"))?;
    if !store(value.to_str().ok_or(Error::Invalid)?) {
        return Err(Error::Integrity);
    }
    Ok(value)
}
pub(crate) fn installed<T: for<'a> Deserialize<'a> + Serialize>(name: &str) -> Result<T> {
    let bytes = read(
        &running()?.join("etc/aios").join(name),
        Path::new("/nix/store"),
        true,
        65536,
    )?;
    let value: T = serde_json::from_slice(&bytes)?;
    if canonical(&value)? != bytes {
        return Err(Error::Integrity);
    }
    Ok(value)
}
fn os_value(data: &str, key: &str) -> Result<String> {
    let mut values = data
        .lines()
        .filter_map(|l| l.strip_prefix(&format!("{key}=")));
    let v = values.next().ok_or(Error::Invalid)?;
    if values.next().is_some() {
        return Err(Error::Invalid);
    }
    let result = if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        &v[1..v.len() - 1]
    } else {
        v
    };
    if result.is_empty() || result.contains(['"', '\n', '\r', '\\']) {
        return Err(Error::Invalid);
    }
    Ok(result.into())
}
#[derive(Clone, Debug)]
struct Observed {
    target: Target,
    os_id: String,
    os_version: String,
    partition: String,
    filesystem: String,
    disk_parent: String,
}
fn check(enrollment: &Enrollment, observed: &Observed) -> Result<()> {
    enrollment.validate()?;
    observed.target.validate()?;
    let t = &observed.target;
    if observed.os_id != enrollment.os_id
        || observed.os_version != enrollment.os_version
        || t.installation_uuid != enrollment.installation_uuid
        || t.dmi_uuid != enrollment.dmi_uuid
        || t.role != enrollment.guest_role
        || t.disk_serial != enrollment.disk_serial
        || t.management_channel != enrollment.management_channel
        || observed.partition != format!("/dev/{}", enrollment.root_partition)
        || observed.filesystem != enrollment.root_filesystem
        || observed.disk_parent != enrollment.disk_device
    {
        return Err(Error::TargetChanged);
    }
    Ok(())
}
fn check_snapshot(enrollment: &Enrollment, observed: &Observed, previous: &Target) -> Result<()> {
    check(enrollment, observed)?;
    if &observed.target != previous {
        return Err(Error::TargetChanged);
    }
    Ok(())
}
fn observe(e: &Enrollment) -> Result<Observed> {
    let os = text("/etc/os-release", "/nix/store")?;
    let partition = resolved(
        &PathBuf::from("/dev/disk/by-uuid").join(&e.installation_uuid),
        Path::new("/dev"),
    )?;
    if !fs::symlink_metadata(&partition)?
        .file_type()
        .is_block_device()
    {
        return Err(Error::Integrity);
    }
    let mounts = String::from_utf8(read(
        Path::new("/proc/self/mountinfo"),
        Path::new("/proc"),
        false,
        1024 * 1024,
    )?)
    .map_err(|_| Error::Invalid)?;
    let mut roots = mounts.lines().filter_map(|line| {
        let (left, right) = line.split_once(" - ")?;
        let l: Vec<_> = left.split_whitespace().collect();
        let r: Vec<_> = right.split_whitespace().collect();
        if l.get(4) == Some(&"/") && r.len() >= 2 {
            Some((r[0].to_owned(), r[1].to_owned()))
        } else {
            None
        }
    });
    let (filesystem, mounted) = roots.next().ok_or(Error::Invalid)?;
    if roots.next().is_some() {
        return Err(Error::Integrity);
    }
    let mounted = resolved(Path::new(&mounted), Path::new("/dev"))?;
    if mounted != partition {
        return Err(Error::TargetChanged);
    }
    let disk = resolved(
        &PathBuf::from("/sys/class/block").join(&e.disk_device),
        Path::new("/sys/devices"),
    )?;
    let part = resolved(
        &PathBuf::from("/sys/class/block").join(&e.root_partition),
        Path::new("/sys/devices"),
    )?;
    if part.parent() != Some(disk.as_path()) {
        return Err(Error::TargetChanged);
    }
    Ok(Observed {
        os_id: os_value(&os, "ID")?,
        os_version: os_value(&os, "VERSION_ID")?,
        partition: partition.to_str().ok_or(Error::Invalid)?.into(),
        filesystem,
        disk_parent: e.disk_device.clone(),
        target: Target {
            installation_uuid: text("/etc/aios/installation-uuid", "/nix/store")?,
            dmi_uuid: text("/sys/devices/virtual/dmi/id/product_uuid", "/sys/devices")?
                .to_ascii_lowercase(),
            boot_id: text("/proc/sys/kernel/random/boot_id", "/proc")?,
            machine_id: text("/etc/machine-id", "/etc")?,
            role: text("/etc/aios/guest-role", "/nix/store")?,
            disk_serial: text(
                &format!("/sys/class/block/{}/serial", e.disk_device),
                "/sys/devices",
            )?,
            management_channel: text("/etc/aios/management-channel", "/nix/store")?,
        },
    })
}
/// Non-deserializable capability minted only by actual root native observations.
#[derive(Clone, Debug)]
pub struct VerifiedTarget {
    enrollment: Enrollment,
    target: Target,
}

/// Read-only fingerprint from an administrator-owned immutable store object.
/// This evidence does not confer approval or permission to execute the object.
#[derive(Clone, Debug, Serialize)]
pub struct StoreArtifact {
    pub path: PathBuf,
    pub sha256: String,
    pub size: u64,
    pub executable: bool,
}
impl VerifiedTarget {
    pub fn enroll() -> Result<Self> {
        root()?;
        let enrollment: Enrollment = installed("target-authority.json")?;
        enrollment.validate()?;
        let observed = observe(&enrollment)?;
        check(&enrollment, &observed)?;
        let value = Self {
            enrollment,
            target: observed.target,
        };
        value.recheck()?;
        Ok(value)
    }
    pub fn target(&self) -> &Target {
        &self.target
    }
    pub(crate) fn system_mounts(&self) -> Result<Vec<crate::health::Mount>> {
        self.recheck()?;
        let bytes = read(
            Path::new("/proc/1/mountinfo"),
            Path::new("/proc"),
            false,
            1024 * 1024,
        )?;
        let mounts =
            crate::health::mounts(&bytes, &format!("/dev/{}", self.enrollment.root_partition))?;
        if let Some(boot) = mounts.iter().find(|m| m.path == "/boot") {
            let selected = resolved(Path::new(&boot.source), Path::new("/dev"))?;
            if !fs::symlink_metadata(&selected)?
                .file_type()
                .is_block_device()
            {
                return Err(Error::Integrity);
            }
            let part = selected.file_name().ok_or(Error::Integrity)?;
            let part = resolved(
                &Path::new("/sys/class/block").join(part),
                Path::new("/sys/devices"),
            )?;
            let disk = resolved(
                &Path::new("/sys/class/block").join(&self.enrollment.disk_device),
                Path::new("/sys/devices"),
            )?;
            if part.parent() != Some(disk.as_path()) {
                return Err(Error::TargetChanged);
            }
        }
        self.recheck()?;
        Ok(mounts)
    }
    pub fn recheck(&self) -> Result<()> {
        root()?;
        let current: Enrollment = installed("target-authority.json")?;
        if current != self.enrollment {
            return Err(Error::TargetChanged);
        }
        let observed = observe(&current)?;
        check_snapshot(&current, &observed, &self.target)
    }
    /// Compiled runtime adapters use this protected traversal; clients cannot
    /// supply paths to it through the product protocol.
    pub fn resolve_store_object(&self, path: &Path) -> Result<PathBuf> {
        self.recheck()?;
        let selected = resolved(path, Path::new("/nix/store"))?;
        self.recheck()?;
        Ok(selected)
    }
    pub fn store_artifact(&self, path: &Path, max: u64, executable: bool) -> Result<StoreArtifact> {
        if max == 0 || max > 256 * 1024 * 1024 {
            return Err(Error::Invalid);
        }
        self.recheck()?;
        let selected = resolved(path, Path::new("/nix/store"))?;
        let before = fs::symlink_metadata(&selected)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(&selected)?;
        let info = file.metadata()?;
        if !info.is_file()
            || info.uid() != 0
            || info.mode() & 0o222 != 0
            || (executable && info.mode() & 0o111 == 0)
            || info.len() > max
            || (before.dev(), before.ino()) != (info.dev(), info.ino())
        {
            return Err(Error::Ownership);
        }
        let mut hasher = Sha256::new();
        let mut buffer = [0; 65536];
        let mut count = 0u64;
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            count = count.checked_add(n as u64).ok_or(Error::Integrity)?;
            if count > max {
                return Err(Error::Integrity);
            }
            hasher.update(&buffer[..n]);
        }
        let after = file.metadata()?;
        if count != info.len()
            || (
                info.dev(),
                info.ino(),
                info.len(),
                info.mtime(),
                info.mtime_nsec(),
                info.ctime(),
                info.ctime_nsec(),
            ) != (
                after.dev(),
                after.ino(),
                after.len(),
                after.mtime(),
                after.mtime_nsec(),
                after.ctime(),
                after.ctime_nsec(),
            )
            || fs::symlink_metadata(&selected)?.ino() != info.ino()
        {
            return Err(Error::TargetChanged);
        }
        self.recheck()?;
        Ok(StoreArtifact {
            path: selected,
            sha256: format!("{:x}", hasher.finalize()),
            size: count,
            executable,
        })
    }
    pub fn store_text(&self, path: &Path) -> Result<String> {
        let artifact = self.store_artifact(path, 65536, false)?;
        let data = read(path, Path::new("/nix/store"), false, 65536)?;
        if crate::sha256(&data) != artifact.sha256
            || resolved(path, Path::new("/nix/store"))? != artifact.path
        {
            return Err(Error::TargetChanged);
        }
        self.recheck()?;
        String::from_utf8(data).map_err(|_| Error::Invalid)
    }
    pub fn verify_system_enrollment(&self, closure: &str) -> Result<()> {
        if !store(closure) {
            return Err(Error::Invalid);
        }
        self.recheck()?;
        let path = Path::new(closure).join("etc/aios/target-authority.json");
        let bytes = read(&path, Path::new("/nix/store"), true, 65536)?;
        let enrollment: Enrollment = serde_json::from_slice(&bytes)?;
        enrollment.validate()?;
        if enrollment != self.enrollment || canonical(&enrollment)? != bytes {
            return Err(Error::TargetChanged);
        }
        self.recheck()
    }
    pub fn systemd_boot_payload_sha256(&self, name: &str, max: u64) -> Result<String> {
        let file = name.strip_prefix("/EFI/nixos/").ok_or(Error::Invalid)?;
        if file.is_empty()
            || file.len() > 256
            || !file
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
            || matches!(file, "." | "..")
            || max == 0
            || max > 256 * 1024 * 1024
        {
            return Err(Error::Invalid);
        }
        self.recheck()?;
        let bytes = read(
            &Path::new("/boot/EFI/nixos").join(file),
            Path::new("/boot/EFI/nixos"),
            false,
            max,
        )?;
        let digest = crate::sha256(&bytes);
        self.recheck()?;
        Ok(digest)
    }
    /// Fixed installed systemd-boot files only; no general privileged read API.
    pub fn systemd_boot_file(&self, entry: Option<&str>) -> Result<Vec<u8>> {
        self.recheck()?;
        let path = match entry {
            None => PathBuf::from("/boot/loader/loader.conf"),
            Some(name) => {
                let number = name
                    .strip_prefix("nixos-generation-")
                    .and_then(|s| s.strip_suffix(".conf"))
                    .ok_or(Error::Invalid)?;
                if number.is_empty()
                    || number.len() > 10
                    || number.starts_with('0')
                    || !number.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(Error::Invalid);
                }
                Path::new("/boot/loader/entries").join(name)
            }
        };
        let data = read(&path, Path::new("/boot/loader"), false, 65536)?;
        self.recheck()?;
        Ok(data)
    }
    pub fn firmware_boot_override(&self, once: bool) -> Result<Option<Vec<u8>>> {
        self.recheck()?;
        let root = Path::new("/sys/firmware/efi/efivars");
        resolved(root, Path::new("/sys"))?;
        let name = if once {
            "LoaderEntryOneShot"
        } else {
            "LoaderEntryDefault"
        };
        let path = root.join(format!("{name}-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"));
        let value = match fs::symlink_metadata(&path) {
            Ok(_) => Some(read(&path, root, false, 4096)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(Error::Io),
        };
        self.recheck()?;
        Ok(value)
    }
    pub(crate) fn authority(&self) -> Result<InstalledAuthority> {
        self.recheck()?;
        let authority: InstalledAuthority = installed("template-authority.json")?;
        authority.validate()?;
        self.recheck()?;
        Ok(authority)
    }
}
#[cfg(test)]
mod tests;
