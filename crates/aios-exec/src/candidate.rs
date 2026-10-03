//! Descriptor-pinned, sealed candidate snapshots from one installed template.
use crate::{Error, Result, canonical, digest, sha256, store};
use aios_state::{Catalog, Compiled};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::{self, File, Permissions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};
use uuid::Uuid;
const MANAGED: &str = "managed.json";
const MANIFEST: &str = "candidate.json";
const MAX_FILES: usize = 4096;
const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    pub path: String,
    pub mode: u32,
    pub size: u64,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateManifest {
    pub schema_version: u32,
    pub files: Vec<FileEntry>,
}
fn relative(name: &str) -> Result<Vec<&str>> {
    let parts: Vec<_> = name.split('/').collect();
    if name.is_empty()
        || name.len() > 512
        || parts.len() > 16
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || !p
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
    {
        return Err(Error::Invalid);
    }
    Ok(parts)
}
fn cstr(name: &str) -> Result<CString> {
    CString::new(name).map_err(|_| Error::Invalid)
}
fn child(parent: &File, name: &str, directory: bool) -> Result<File> {
    if name.contains('/') {
        return Err(Error::Invalid);
    }
    let c = cstr(name)?;
    let flags = libc::O_RDONLY
        | libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live parent FD and a NUL-terminated validated component; returned
    // descriptor is owned exactly once by File, and never inherited by children.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), c.as_ptr(), flags) };
    if fd < 0 {
        return Err(Error::Io);
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn owned(file: &File, owner: u32, mode: Option<u32>, directory: bool) -> Result<()> {
    let m = file.metadata()?;
    if m.uid() != owner
        || m.mode() & 0o022 != 0
        || (directory && !m.is_dir())
        || (!directory && !m.is_file())
        || mode.is_some_and(|v| m.mode() & 0o7777 != v)
    {
        return Err(Error::Ownership);
    }
    Ok(())
}
fn anchor(path: &Path, owner: u32, ancestors_root: bool) -> Result<File> {
    let mut fd = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    let parts = path
        .to_str()
        .ok_or(Error::Invalid)?
        .strip_prefix('/')
        .ok_or(Error::Invalid)?;
    let components = relative(parts)?;
    for (index, component) in components.iter().enumerate() {
        fd = child(&fd, component, true)?;
        if ancestors_root {
            // Nix's root-owned sticky store may be group-writable to nixbld.
            // Sticky semantics plus readonly root-owned children protect existing
            // installed objects; no other writable ancestor is accepted.
            if index == 1 && components[0] == "nix" && *component == "store" {
                let m = fd.metadata()?;
                if m.uid() != 0
                    || !m.is_dir()
                    || (m.mode() & 0o022 != 0 && m.mode() & libc::S_ISVTX == 0)
                {
                    return Err(Error::Ownership);
                }
            } else {
                owned(&fd, 0, None, true)?;
            }
        } else if index + 1 == components.len() {
            owned(&fd, owner, None, true)?;
        }
    }
    Ok(fd)
}
fn read(root: &File, entry: &FileEntry, owner: u32, copied: bool) -> Result<Vec<u8>> {
    let components = relative(&entry.path)?;
    let mut fd = root.try_clone()?;
    for name in &components[..components.len() - 1] {
        fd = child(&fd, name, true)?;
        owned(&fd, owner, Some(0o555), true)?;
    }
    let file = child(&fd, components.last().unwrap(), false)?;
    let mode = if entry.mode == 0o755 { 0o555 } else { 0o444 };
    owned(&file, owner, Some(mode), false)?;
    let before = file.metadata()?;
    if before.len() != entry.size || before.len() > MAX_FILE || (copied && before.nlink() != 1) {
        return Err(Error::Integrity);
    }
    let mut bytes = vec![];
    (&file).take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 != entry.size
        || sha256(&bytes) != entry.sha256
        || (
            before.dev(),
            before.ino(),
            before.len(),
            before.mtime(),
            before.mtime_nsec(),
            before.ctime(),
            before.ctime_nsec(),
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
    Ok(bytes)
}
fn inventory(root: &File, prefix: &str, result: &mut BTreeSet<String>, owner: u32) -> Result<()> {
    for entry in fs::read_dir(format!("/proc/self/fd/{}", root.as_raw_fd()))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid)?;
        relative(&name)?;
        let name = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if result.len() >= MAX_FILES * 17 {
            return Err(Error::Invalid);
        }
        if entry.file_type()?.is_dir() {
            let component = name.rsplit('/').next().unwrap();
            let dir = child(root, component, true)?;
            owned(&dir, owner, Some(0o555), true)?;
            inventory(&dir, &name, result, owner)?;
            // Empty directories are not an unlisted channel into the candidate.
            if !result.iter().any(|v| v.starts_with(&(name.clone() + "/"))) {
                return Err(Error::Integrity);
            }
        } else {
            if !entry.file_type()?.is_file() || !result.insert(name) {
                return Err(Error::Integrity);
            }
        }
    }
    Ok(())
}
fn validate_entries(files: &[FileEntry], allow_reserved: bool) -> Result<()> {
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(Error::Invalid);
    }
    let mut previous = "";
    let mut names = BTreeSet::new();
    let mut total = 0_u64;
    for entry in files {
        let parts = relative(&entry.path)?;
        if entry.path.as_str() <= previous
            || !digest(&entry.sha256)
            || !matches!(entry.mode, 0o644 | 0o755)
            || entry.size > MAX_FILE
            || (!allow_reserved
                && [MANAGED, MANIFEST, "template.json"].contains(&entry.path.as_str()))
        {
            return Err(Error::Invalid);
        }
        for n in 1..parts.len() {
            if names.contains(&parts[..n].join("/")) {
                return Err(Error::Invalid);
            }
        }
        total = total.checked_add(entry.size).ok_or(Error::Invalid)?;
        if total > MAX_TOTAL {
            return Err(Error::Invalid);
        }
        names.insert(entry.path.clone());
        previous = &entry.path;
    }
    if !names.contains("flake.nix")
        || !names.contains("flake.lock")
        || !names.contains("catalog.json")
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
/// Only obtained from a root-selected installed immutable package. No model path.
pub struct InstalledTemplate {
    root: File,
    manifest: TemplateManifest,
    template_sha256: String,
    catalog: Catalog,
    owner: u32,
    path: PathBuf,
    root_ancestors: bool,
    target: Option<crate::native::VerifiedTarget>,
    authority: Option<crate::native::InstalledAuthority>,
}
impl InstalledTemplate {
    pub fn from_installed() -> Result<Self> {
        let target = crate::native::VerifiedTarget::enroll()?;
        let authority = target.authority()?;
        Self::open(&authority.template_path, &authority.manifest_sha256)
    }
    pub fn open(package: &str, manifest_sha256: &str) -> Result<Self> {
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(Error::Authority);
        }
        if !store(package) || !digest(manifest_sha256) {
            return Err(Error::Invalid);
        }
        let target = crate::native::VerifiedTarget::enroll()?;
        let authority = target.authority()?;
        if package != authority.template_path || manifest_sha256 != authority.manifest_sha256 {
            return Err(Error::Integrity);
        }
        let mut value = Self::load(Path::new(package), manifest_sha256, 0, true)?;
        if value.catalog.content().base_template_revision != authority.base_template_revision
            || value.catalog.revision() != authority.catalog_revision
            || value.catalog.content().lock_sha256 != authority.lock_sha256
        {
            return Err(Error::Integrity);
        }
        value.target = Some(target);
        value.authority = Some(authority);
        value.verify()?;
        Ok(value)
    }
    fn load(path: &Path, expected: &str, owner: u32, root_ancestors: bool) -> Result<Self> {
        let root = anchor(path, owner, root_ancestors)?;
        owned(&root, owner, Some(0o555), true)?;
        let file = child(&root, "template.json", false)?;
        owned(&file, owner, Some(0o444), false)?;
        if file.metadata()?.len() > 1024 * 1024 {
            return Err(Error::Invalid);
        }
        let mut bytes = vec![];
        (&file).take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if sha256(&bytes) != expected {
            return Err(Error::Integrity);
        }
        let manifest: TemplateManifest = serde_json::from_slice(&bytes)?;
        if manifest.schema_version != 1 || canonical(&manifest)? != bytes {
            return Err(Error::Invalid);
        }
        validate_entries(&manifest.files, false)?;
        let mut observed = BTreeSet::new();
        inventory(&root, "", &mut observed, owner)?;
        let mut wanted: BTreeSet<_> = manifest.files.iter().map(|p| p.path.clone()).collect();
        wanted.insert("template.json".into());
        if observed != wanted {
            return Err(Error::Integrity);
        }
        for entry in &manifest.files {
            read(&root, entry, owner, false)?;
        }
        let catalog_bytes = read(
            &root,
            manifest
                .files
                .iter()
                .find(|f| f.path == "catalog.json")
                .unwrap(),
            owner,
            false,
        )?;
        let catalog = Catalog::from_installed(&catalog_bytes).map_err(|_| Error::Integrity)?;
        let lock = manifest
            .files
            .iter()
            .find(|f| f.path == "flake.lock")
            .unwrap();
        if lock.sha256 != catalog.content().lock_sha256 {
            return Err(Error::Integrity);
        }
        Ok(Self {
            root,
            manifest,
            template_sha256: expected.into(),
            catalog,
            owner,
            path: path.into(),
            root_ancestors,
            target: None,
            authority: None,
        })
    }
    fn verify(&self) -> Result<()> {
        if let Some(target) = &self.target {
            if self.authority.as_ref() != Some(&target.authority()?) {
                return Err(Error::TargetChanged);
            }
        }
        let current = anchor(&self.path, self.owner, self.root_ancestors)?;
        if (current.metadata()?.dev(), current.metadata()?.ino())
            != (self.root.metadata()?.dev(), self.root.metadata()?.ino())
        {
            return Err(Error::TargetChanged);
        }
        owned(&self.root, self.owner, Some(0o555), true)?;
        let bytes = canonical(&self.manifest)?;
        let entry = FileEntry {
            path: "template.json".into(),
            mode: 0o644,
            size: bytes.len() as u64,
            sha256: self.template_sha256.clone(),
        };
        read(&self.root, &entry, self.owner, false)?;
        let mut actual = BTreeSet::new();
        inventory(&self.root, "", &mut actual, self.owner)?;
        let mut expected: BTreeSet<_> =
            self.manifest.files.iter().map(|p| p.path.clone()).collect();
        expected.insert("template.json".into());
        if actual != expected {
            return Err(Error::Integrity);
        }
        for entry in &self.manifest.files {
            read(&self.root, entry, self.owner, false)?;
        }
        Ok(())
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    pub fn digest(&self) -> &str {
        &self.template_sha256
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidateManifest {
    pub schema_version: u32,
    pub template_sha256: String,
    pub base_template_revision: String,
    pub catalog_revision: String,
    pub lock_sha256: String,
    pub managed_sha256: String,
    pub files: Vec<FileEntry>,
}
/// Non-deserializable registered capability. The builder receives a broker-issued
/// ID; the broker selects this path and verifies sealed bytes again at use time.
pub struct Candidate {
    manifest: CandidateManifest,
    digest: String,
    path: PathBuf,
}
impl Candidate {
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn manifest(&self) -> &CandidateManifest {
        &self.manifest
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
pub struct CandidateStore {
    root: File,
    path: PathBuf,
    owner: u32,
    root_ancestors: bool,
    target: Option<crate::native::VerifiedTarget>,
}
impl CandidateStore {
    /// Administrator creates this root-owned 0755 directory via the NixOS module.
    pub fn open() -> Result<Self> {
        if unsafe { libc::getuid() } != 0 || unsafe { libc::geteuid() } != 0 {
            return Err(Error::Authority);
        }
        let target = crate::native::VerifiedTarget::enroll()?;
        let mut value = Self::load(Path::new("/var/lib/aios/candidates"), 0, true)?;
        value.target = Some(target);
        value.verify_anchor()?;
        Ok(value)
    }
    fn load(path: &Path, owner: u32, root_ancestors: bool) -> Result<Self> {
        let root = anchor(path, owner, root_ancestors)?;
        owned(&root, owner, Some(0o755), true)?;
        Ok(Self {
            root,
            path: path.into(),
            owner,
            root_ancestors,
            target: None,
        })
    }
    fn verify_anchor(&self) -> Result<()> {
        if let Some(target) = &self.target {
            target.recheck()?;
        }
        let current = anchor(&self.path, self.owner, self.root_ancestors)?;
        if (current.metadata()?.dev(), current.metadata()?.ino())
            != (self.root.metadata()?.dev(), self.root.metadata()?.ino())
        {
            return Err(Error::TargetChanged);
        }
        owned(&self.root, self.owner, Some(0o755), true)
    }
    pub fn prepare(&self, template: &InstalledTemplate, managed: &Compiled) -> Result<Candidate> {
        self.verify_anchor()?;
        template.verify()?;
        if template.owner != self.owner {
            return Err(Error::Ownership);
        }
        let checked = template
            .catalog
            .compile(&managed.bytes)
            .map_err(|_| Error::Invalid)?;
        if checked.digest != managed.digest || checked.state != managed.state {
            return Err(Error::Integrity);
        }
        let staging = format!(".staging-{}", Uuid::new_v4());
        let c = cstr(&staging)?;
        // SAFETY: parent descriptor is live and staging name is generated here.
        if unsafe { libc::mkdirat(self.root.as_raw_fd(), c.as_ptr(), 0o700) } != 0 {
            return Err(Error::Io);
        }
        let stage = child(&self.root, &staging, true)?;
        let result = (|| {
            let mut files = vec![];
            for entry in &template.manifest.files {
                let bytes = read(&template.root, entry, self.owner, false)?;
                write(&stage, &entry.path, &bytes, entry.mode, self.owner)?;
                files.push(entry.clone());
            }
            write(&stage, MANAGED, &checked.bytes, 0o644, self.owner)?;
            files.push(FileEntry {
                path: MANAGED.into(),
                mode: 0o644,
                size: checked.bytes.len() as u64,
                sha256: checked.digest.clone(),
            });
            files.sort_by(|a, b| a.path.cmp(&b.path));
            let manifest = CandidateManifest {
                schema_version: 1,
                template_sha256: template.template_sha256.clone(),
                base_template_revision: template.catalog.content().base_template_revision.clone(),
                catalog_revision: template.catalog.revision().into(),
                lock_sha256: template.catalog.content().lock_sha256.clone(),
                managed_sha256: checked.digest,
                files,
            };
            let bytes = canonical(&manifest)?;
            let hash = sha256(&bytes);
            write(&stage, MANIFEST, &bytes, 0o644, self.owner)?;
            seal(&stage, self.owner)?;
            verify(&stage, &manifest, &hash, self.owner)?;
            template.verify()?;
            self.verify_anchor()?;
            let target = cstr(&hash)?;
            // Atomic no-replace publication; no candidate can overwrite another.
            let rc = unsafe {
                libc::renameat2(
                    self.root.as_raw_fd(),
                    c.as_ptr(),
                    self.root.as_raw_fd(),
                    target.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            };
            if rc != 0 {
                if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
                    return Err(Error::Io);
                }
                let existing = child(&self.root, &hash, true)?;
                verify(&existing, &manifest, &hash, self.owner)?;
                cleanup(
                    &PathBuf::from(format!("/proc/self/fd/{}", self.root.as_raw_fd()))
                        .join(&staging),
                )?;
            }
            self.root.sync_all()?;
            Ok(Candidate {
                manifest,
                digest: hash.clone(),
                path: self.path.join(hash),
            })
        })();
        if result.is_err() {
            let _ = cleanup(
                &PathBuf::from(format!("/proc/self/fd/{}", self.root.as_raw_fd())).join(&staging),
            );
        }
        result
    }
    pub fn verify(&self, candidate: &Candidate) -> Result<()> {
        self.verify_anchor()?;
        if !digest(&candidate.digest) || candidate.path != self.path.join(&candidate.digest) {
            return Err(Error::Integrity);
        }
        verify(
            &child(&self.root, &candidate.digest, true)?,
            &candidate.manifest,
            &candidate.digest,
            self.owner,
        )
    }
}
fn write(root: &File, name: &str, bytes: &[u8], mode: u32, owner: u32) -> Result<()> {
    let parts = relative(name)?;
    let mut fd = root.try_clone()?;
    for part in &parts[..parts.len() - 1] {
        let component = cstr(part)?;
        let rc = unsafe { libc::mkdirat(fd.as_raw_fd(), component.as_ptr(), 0o700) };
        if rc != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(Error::Io);
        }
        fd = child(&fd, part, true)?;
        owned(&fd, owner, Some(0o700), true)?;
    }
    let name = cstr(parts.last().unwrap())?;
    let raw = unsafe {
        libc::openat(
            fd.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if raw < 0 {
        return Err(Error::Io);
    }
    let mut file = unsafe { File::from_raw_fd(raw) };
    file.write_all(bytes)?;
    file.set_permissions(Permissions::from_mode(if mode == 0o755 {
        0o555
    } else {
        0o444
    }))?;
    file.sync_all()?;
    fd.sync_all()?;
    Ok(())
}
fn seal(root: &File, owner: u32) -> Result<()> {
    for entry in fs::read_dir(format!("/proc/self/fd/{}", root.as_raw_fd()))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid)?;
        if entry.file_type()?.is_dir() {
            let dir = child(root, &name, true)?;
            owned(&dir, owner, Some(0o700), true)?;
            seal(&dir, owner)?;
        } else if !entry.file_type()?.is_file() {
            return Err(Error::Integrity);
        }
    }
    root.set_permissions(Permissions::from_mode(0o555))?;
    root.sync_all()?;
    Ok(())
}
fn verify(root: &File, manifest: &CandidateManifest, hash: &str, owner: u32) -> Result<()> {
    owned(root, owner, Some(0o555), true)?;
    validate_entries(&manifest.files, true)?;
    if manifest.schema_version != 1 || sha256(&canonical(manifest)?) != hash {
        return Err(Error::Integrity);
    }
    let bytes = canonical(manifest)?;
    let entry = FileEntry {
        path: MANIFEST.into(),
        mode: 0o644,
        size: bytes.len() as u64,
        sha256: hash.into(),
    };
    read(root, &entry, owner, true)?;
    let mut inventory_set = BTreeSet::new();
    inventory(root, "", &mut inventory_set, owner)?;
    let mut wanted: BTreeSet<_> = manifest.files.iter().map(|f| f.path.clone()).collect();
    wanted.insert(MANIFEST.into());
    if wanted != inventory_set {
        return Err(Error::Integrity);
    }
    for entry in &manifest.files {
        read(root, entry, owner, true)?;
    }
    Ok(())
}
fn cleanup(path: &Path) -> Result<()> {
    // Only a unique staging directory created by this invocation is passed here.
    // Parent is owned by the broker. No published digest is removed on failure.
    if !path
        .file_name()
        .is_some_and(|v| v.to_string_lossy().starts_with(".staging-"))
    {
        return Err(Error::Invalid);
    }
    fn writable(path: &Path) -> std::io::Result<()> {
        fs::set_permissions(path, Permissions::from_mode(0o700))?;
        for e in fs::read_dir(path)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                writable(&e.path())?;
            }
        }
        Ok(())
    }
    writable(path)?;
    fs::remove_dir_all(path)?;
    Ok(())
}
#[cfg(test)]
pub(crate) mod tests;
pub(crate) fn root_ledger_directory(path: &Path) -> Result<File> {
    let fd = anchor(path, 0, true)?;
    owned(&fd, 0, Some(0o700), true)?;
    Ok(fd)
}
