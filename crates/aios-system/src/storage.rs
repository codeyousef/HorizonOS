//! Read-only filesystem capacity and mount-source observations. The action takes
//! no path; visibility is derived from the authenticated caller UID.
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{ffi::{CStr, CString}, fs, os::unix::fs::MetadataExt, path::{Path, PathBuf}};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const MAX_MOUNTINFO: u64 = 1024 * 1024;
const MAX_MOUNTS: usize = 100;
const JS_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeMount {
    major: u32,
    minor: u32,
    path: String,
    filesystem: String,
    read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MountStatus {
    pub mount_id: String,
    pub mount_path: String,
    pub source_identity: String,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
    pub read_only: bool,
}

#[derive(Debug, Serialize)]
pub struct StorageData {
    pub mounts: Vec<MountStatus>,
}

fn unescape_mount(value: &str) -> Result<String, ErrorCode> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            if index + 3 >= bytes.len() { return Err(ErrorCode::PartialResult); }
            let octal = &bytes[index + 1..index + 4];
            if !octal.iter().all(|b| (b'0'..=b'7').contains(b)) { return Err(ErrorCode::PartialResult); }
            output.push((octal[0] - b'0') * 64 + (octal[1] - b'0') * 8 + (octal[2] - b'0'));
            index += 4;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    let value = String::from_utf8(output).map_err(|_| ErrorCode::PartialResult)?;
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) || !value.starts_with('/') {
        return Err(ErrorCode::PartialResult);
    }
    Ok(value)
}

fn parse_mountinfo(raw: &str) -> Result<Vec<NativeMount>, ErrorCode> {
    let mut mounts = Vec::new();
    for (line_count, line) in raw.lines().enumerate() {
        if line_count >= 4096 { return Err(ErrorCode::ResourceExhausted); }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let separator = fields.iter().position(|value| *value == "-").ok_or(ErrorCode::PartialResult)?;
        if separator < 6 || separator + 3 > fields.len() { return Err(ErrorCode::PartialResult); }
        let (major, minor) = fields[2].split_once(':').ok_or(ErrorCode::PartialResult)?;
        let major = major.parse().map_err(|_| ErrorCode::PartialResult)?;
        let minor = minor.parse().map_err(|_| ErrorCode::PartialResult)?;
        let path = unescape_mount(fields[4])?;
        let filesystem = fields[separator + 1];
        if filesystem.is_empty() || filesystem.len() > 64 || !filesystem.bytes().all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b)) {
            return Err(ErrorCode::PartialResult);
        }
        let read_only = fields[5].split(',').any(|option| option == "ro");
        mounts.push(NativeMount { major, minor, path, filesystem: filesystem.into(), read_only });
    }
    Ok(mounts)
}

fn account_home(uid: u32) -> Result<PathBuf, ErrorCode> {
    let entry = unsafe { libc::getpwuid(uid) };
    if entry.is_null() { return Err(ErrorCode::PermissionDenied); }
    let home = unsafe { CStr::from_ptr((*entry).pw_dir) }.to_str().map_err(|_| ErrorCode::PermissionDenied)?;
    let path = PathBuf::from(home);
    if !path.is_absolute() || path == Path::new("/") { return Err(ErrorCode::PermissionDenied); }
    Ok(path)
}

fn visible(path: &Path, home: &Path, uid: u32) -> bool {
    if [Path::new("/"), Path::new("/boot"), Path::new("/boot/efi"), Path::new("/home"), Path::new("/nix")].contains(&path) {
        return true;
    }
    let under_owner = path.starts_with(home)
        || path.starts_with("/run/media")
        || path.starts_with("/media")
        || path.starts_with("/mnt");
    if !under_owner { return false; }
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && metadata.uid() == uid && metadata.mode() & 0o002 == 0)
}

fn capacity(path: &Path) -> Result<(u64, u64), ErrorCode> {
    let bytes = path.as_os_str().as_encoded_bytes();
    let path = CString::new(bytes).map_err(|_| ErrorCode::PartialResult)?;
    let mut value = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), value.as_mut_ptr()) } != 0 { return Err(ErrorCode::PartialResult); }
    let value = unsafe { value.assume_init() };
    let unit = value.f_frsize as u64;
    let total = (value.f_blocks as u64).checked_mul(unit).ok_or(ErrorCode::ResourceExhausted)?;
    let free = (value.f_bavail as u64).checked_mul(unit).ok_or(ErrorCode::ResourceExhausted)?;
    if total > JS_SAFE_INTEGER || free > total { return Err(ErrorCode::ResourceExhausted); }
    Ok((total, free))
}

fn opaque_id(scope: &[u8], mount: &NativeMount) -> String {
    let mut digest = Sha256::new();
    digest.update(b"aios-storage-mount-v1\0");
    digest.update(scope);
    digest.update([0]);
    digest.update(mount.path.as_bytes());
    digest.update([0]);
    digest.update(mount.major.to_be_bytes());
    digest.update(mount.minor.to_be_bytes());
    digest.update(mount.filesystem.as_bytes());
    format!("mount-{:x}", digest.finalize())
}

fn observe_inner(uid: u32, scope: &[u8]) -> Result<StorageData, ErrorCode> {
    let home = account_home(uid)?;
    let raw = super::bounded(Path::new("/proc/self/mountinfo"), MAX_MOUNTINFO)?;
    let mut mounts = Vec::new();
    for mount in parse_mountinfo(&raw)? {
        let path = Path::new(&mount.path);
        if !visible(path, &home, uid) { continue; }
        if mounts.len() >= MAX_MOUNTS { return Err(ErrorCode::ResourceExhausted); }
        let (capacity_bytes, free_bytes) = capacity(path)?;
        let source_identity = if mount.major == 0 {
            format!("filesystem:{}", mount.filesystem)
        } else {
            format!("block:{}:{}:{}", mount.major, mount.minor, mount.filesystem)
        };
        mounts.push(MountStatus {
            mount_id: opaque_id(scope, &mount),
            mount_path: mount.path.clone(),
            source_identity,
            capacity_bytes,
            free_bytes,
            read_only: mount.read_only,
        });
    }
    mounts.sort_by(|a, b| a.mount_path.cmp(&b.mount_path));
    if !mounts.iter().any(|mount| mount.mount_path == "/") { return Err(ErrorCode::PartialResult); }
    Ok(StorageData { mounts })
}

pub fn read_system_mounts() -> Result<Vec<MountStatus>, ErrorCode> {
    let raw = super::bounded(Path::new("/proc/self/mountinfo"), MAX_MOUNTINFO)?;
    let mut mounts = Vec::new();
    for mount in parse_mountinfo(&raw)? {
        let path = Path::new(&mount.path);
        if ![Path::new("/"), Path::new("/boot"), Path::new("/boot/efi"), Path::new("/home"), Path::new("/nix")].contains(&path) {
            continue;
        }
        if mounts.len() >= MAX_MOUNTS { return Err(ErrorCode::ResourceExhausted); }
        let (capacity_bytes, free_bytes) = capacity(path)?;
        let source_identity = if mount.major == 0 {
            format!("filesystem:{}", mount.filesystem)
        } else {
            format!("block:{}:{}:{}", mount.major, mount.minor, mount.filesystem)
        };
        mounts.push(MountStatus {
            mount_id: opaque_id(b"system-graph", &mount),
            mount_path: mount.path,
            source_identity,
            capacity_bytes,
            free_bytes,
            read_only: mount.read_only,
        });
    }
    mounts.sort_by(|a, b| a.mount_path.cmp(&b.mount_path));
    if !mounts.iter().any(|mount| mount.mount_path == "/") { return Err(ErrorCode::PartialResult); }
    Ok(mounts)
}

pub fn observe(uid: u32, scope: &[u8]) -> ProviderResult<StorageData> {
    let observed_at = OffsetDateTime::now_utc().format(&Rfc3339).expect("valid UTC timestamp");
    match observe_inner(uid, scope) {
        Ok(data) => ProviderResult {
            schema_version: 1,
            status: ResultStatus::Ok,
            observed_at,
            source: Source { provider: "aios-system-native-storage".into(), provider_version: env!("CARGO_PKG_VERSION").into() },
            evidence_ids: Vec::new(), complete: true, next_cursor: None, data: Some(data), error: None,
        },
        Err(code) => ProviderResult {
            schema_version: 1,
            status: ResultStatus::Error,
            observed_at,
            source: Source { provider: "aios-system-native-storage".into(), provider_version: env!("CARGO_PKG_VERSION").into() },
            evidence_ids: Vec::new(), complete: false, next_cursor: None, data: None,
            error: Some(ProviderError { code, message: "Bounded native mount or capacity observation unavailable; no capacity inferred".into(), retryable: true }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_parser_distinguishes_block_and_virtual_sources() {
        let raw = "36 25 8:1 / / rw,relatime - ext4 /dev/vda1 rw\n37 36 0:29 / /mnt/user\\040disk ro,nosuid - tmpfs tmpfs ro\n";
        let mounts = parse_mountinfo(raw).unwrap();
        assert_eq!(mounts[0], NativeMount { major: 8, minor: 1, path: "/".into(), filesystem: "ext4".into(), read_only: false });
        assert_eq!(mounts[1], NativeMount { major: 0, minor: 29, path: "/mnt/user disk".into(), filesystem: "tmpfs".into(), read_only: true });
    }

    #[test]
    fn malformed_or_relative_mount_paths_are_refused() {
        for raw in [
            "36 25 8:1 / relative rw - ext4 /dev/vda1 rw\n",
            "36 25 nope / / rw - ext4 /dev/vda1 rw\n",
            "36 25 8:1 / / rw ext4 /dev/vda1 rw\n",
        ] {
            assert!(parse_mountinfo(raw).is_err());
        }
    }
}
