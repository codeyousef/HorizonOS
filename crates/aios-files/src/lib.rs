use aios_protocol::contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem::MaybeUninit,
    os::{fd::{AsRawFd, FromRawFd}, unix::fs::MetadataExt},
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, ErrorCode>;
const MAX_PATH_BYTES: usize = 4096;
const MAX_HANDLES: usize = 4096;
const MAX_ROOTS: usize = 32;
const MAX_READ_BYTES: usize = 2 * 1024 * 1024;
const PROPOSAL_LIFETIME_MS: u64 = 5 * 60 * 1000;
const HANDLE_LIFETIME_MS: u64 = 5 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub uid: u32,
    pub boot_id: String,
    pub session_id: Option<String>,
    /// Digest of the broker-authenticated process and transport subject. Never
    /// accept this binding from a public file request or model output.
    pub client_binding_sha256: String,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access { Metadata, Content, Mutation }

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProposedRoot {
    pub root_id: String,
    pub display_path: String,
    pub identity_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Proposal {
    pub proposal_id: String,
    pub expires_at_boottime_ms: u64,
    pub roots: Vec<ProposedRoot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RootGrant {
    pub root_id: String,
    pub display_path: String,
    pub identity_sha256: String,
    pub allowed_access: Vec<Access>,
    pub expires_at_boottime_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScopedMetadata {
    pub file_handle: String,
    pub root_id: String,
    pub relative_path: String,
    pub identity_sha256: String,
    pub size_bytes: u64,
    pub modified_ns: i128,
    pub access: Access,
    pub expires_at_boottime_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RevocationReceipt {
    pub root_id: String,
    pub handles_revoked: usize,
    pub cached_records_purged: usize,
    pub access_blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    mount_id: u64,
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    size: u64,
    modified_ns: i128,
    changed_ns: i128,
}

struct Candidate { root: ProposedRoot, path: PathBuf, identity: Identity }
struct Pending { owner: Owner, expires: u64, roots: Vec<Candidate> }
struct Root { owner: Owner, path: PathBuf, identity: Identity, identity_sha256: String, allowed: HashSet<Access>, expires: u64, directory: File }
struct Handle { owner: Owner, root_id: String, relative: String, identity: Identity, access: Access, expires: u64 }

#[derive(Clone)]
struct CachedRecord { handle: String, value: String }

#[derive(Default)]
pub struct Manager {
    pending: HashMap<String, Pending>,
    roots: HashMap<String, Root>,
    handles: HashMap<String, Handle>,
    chunks: Vec<CachedRecord>,
    previews: Vec<CachedRecord>,
    snippets: Vec<CachedRecord>,
}

fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn owner_valid(owner: &Owner) -> bool {
    owner.uid > 0 && Uuid::parse_str(&owner.boot_id).is_ok()
        && owner.session_id.as_deref().is_none_or(|id| bounded(id, 128))
        && owner.client_binding_sha256.len() == 64
        && owner.client_binding_sha256.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn timespec_ns(seconds: i64, nanos: i64) -> Result<i128> {
    (seconds as i128).checked_mul(1_000_000_000).and_then(|v| v.checked_add(nanos as i128))
        .ok_or(ErrorCode::TargetChanged)
}

const STATX_MNT_ID: u32 = 0x0000_1000;
const AT_STATX_DONT_SYNC: i32 = 0x4000;
fn mount_id(file: &File) -> Result<u64> {
    let mut value=MaybeUninit::<nix::libc::statx>::zeroed();
    // SAFETY: `value` points to writable statx storage and the empty C string
    // requests metadata for the already-open descriptor via AT_EMPTY_PATH.
    let result=unsafe{nix::libc::syscall(nix::libc::SYS_statx,file.as_raw_fd(),c"".as_ptr(),
        nix::libc::AT_EMPTY_PATH|AT_STATX_DONT_SYNC,STATX_MNT_ID,value.as_mut_ptr())};
    if result!=0{return Err(ErrorCode::TargetChanged);}
    // SAFETY: a successful statx call initialized the supplied structure.
    let value=unsafe{value.assume_init()};
    if value.stx_mask&STATX_MNT_ID==0{return Err(ErrorCode::TargetChanged);}
    Ok(value.stx_mnt_id)
}

fn identity(file: &File) -> Result<Identity> {
    let before = file.metadata().map_err(|_| ErrorCode::TargetChanged)?;
    let id = Identity {
        mount_id: mount_id(file)?, device: before.dev(), inode: before.ino(), mode: before.mode(), uid: before.uid(), size: before.size(),
        modified_ns: timespec_ns(before.mtime(), before.mtime_nsec())?,
        changed_ns: timespec_ns(before.ctime(), before.ctime_nsec())?,
    };
    let after = file.metadata().map_err(|_| ErrorCode::TargetChanged)?;
    if (before.dev(),before.ino(),before.mode(),before.uid(),before.size(),before.mtime(),before.mtime_nsec(),before.ctime(),before.ctime_nsec())
        != (after.dev(),after.ino(),after.mode(),after.uid(),after.size(),after.mtime(),after.mtime_nsec(),after.ctime(),after.ctime_nsec()) {
        return Err(ErrorCode::TargetChanged);
    }
    Ok(id)
}

fn identity_sha256(value: &Identity) -> String {
    let mut digest = Sha256::new();
    for bytes in [value.mount_id.to_le_bytes().as_slice(), value.device.to_le_bytes().as_slice(), value.inode.to_le_bytes().as_slice(),
        value.mode.to_le_bytes().as_slice(), value.uid.to_le_bytes().as_slice(), value.size.to_le_bytes().as_slice(),
        value.modified_ns.to_le_bytes().as_slice(), value.changed_ns.to_le_bytes().as_slice()] { digest.update(bytes); }
    format!("{:x}",digest.finalize())
}
fn same_root(left:&Identity,right:&Identity)->bool{
    (left.mount_id,left.device,left.inode,left.mode,left.uid)==(right.mount_id,right.device,right.inode,right.mode,right.uid)
}


#[repr(C)]
struct OpenHow { flags: u64, mode: u64, resolve: u64 }
const RESOLVE_NO_XDEV: u64 = 0x01;
const RESOLVE_NO_MAGICLINKS: u64 = 0x02;
const RESOLVE_NO_SYMLINKS: u64 = 0x04;
const RESOLVE_BENEATH: u64 = 0x08;

fn openat2(directory: i32, path: &Path, flags: i32, resolve: u64) -> Result<File> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.is_empty() || bytes.len() > MAX_PATH_BYTES || bytes.contains(&0) { return Err(ErrorCode::InvalidArgument); }
    let path = CString::new(bytes).map_err(|_| ErrorCode::InvalidArgument)?;
    let how = OpenHow { flags: (flags | nix::libc::O_CLOEXEC) as u64, mode: 0, resolve };
    // SAFETY: `path` is a live NUL-terminated CString, `how` is fully initialized,
    // and the kernel either returns a new owned descriptor or a negative errno.
    let fd = unsafe { nix::libc::syscall(nix::libc::SYS_openat2, directory, path.as_ptr(), &how, std::mem::size_of::<OpenHow>()) };
    if fd < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(code) if matches!(code,nix::libc::EACCES|nix::libc::EPERM|nix::libc::EXDEV|nix::libc::ELOOP|nix::libc::ENOTDIR) => Err(ErrorCode::PermissionDenied),
            Some(nix::libc::ENOENT) => Err(ErrorCode::TargetNotFound),
            _ => Err(ErrorCode::TargetChanged),
        };
    }
    // SAFETY: successful openat2 returns one new descriptor whose ownership is
    // transferred exactly once to File and closed by File::drop.
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}

fn open_root(path: &Path) -> Result<(File,Identity)> {
    let file = openat2(nix::libc::AT_FDCWD,path,nix::libc::O_PATH|nix::libc::O_DIRECTORY|nix::libc::O_NOFOLLOW,
        RESOLVE_NO_MAGICLINKS|RESOLVE_NO_SYMLINKS)?;
    let id = identity(&file)?;
    if id.mode & nix::libc::S_IFMT != nix::libc::S_IFDIR || id.uid == 0 { return Err(ErrorCode::PermissionDenied); }
    Ok((file,id))
}

fn secret(relative: &Path) -> bool {
    relative.components().any(|component| {
        let Component::Normal(name) = component else { return true };
        let name = name.to_string_lossy().to_ascii_lowercase();
        matches!(name.as_str(), ".ssh"|".gnupg"|".password-store"|".aws"|".azure"|".kube"|".docker"|".mozilla"|".git-credentials"|".netrc"|".npmrc"|".pypirc"|
            "keyrings"|"kwalletd"|"google-chrome"|"chromium"|"credentials"|"secrets"|"wallet"|"wallets"|".aios"|"aios-private")
            || name.contains("password") || name.contains("credential") || name.contains("wallet") || name.contains("keyring")
            || name=="login data" || name=="cookies" || name == ".env" || name.starts_with(".env.")
            || ["id_rsa","id_ed25519","id_ecdsa","id_dsa"].iter().any(|prefix|name.starts_with(prefix))
            || [".pem",".key",".p12",".pfx",".kdbx",".wallet",".kwl"].iter().any(|suffix|name.ends_with(suffix))
    })
}

fn relative_valid(path: &Path) -> bool {
    !path.as_os_str().is_empty() && path.as_os_str().as_encoded_bytes().len() <= MAX_PATH_BYTES
        && !secret(path) && path.components().all(|part| matches!(part,Component::Normal(_)))
}

fn xdg_configuration(home_fd: &File, uid: u32) -> Option<String> {
    let file = openat2(home_fd.as_raw_fd(), Path::new(".config/user-dirs.dirs"),
        nix::libc::O_RDONLY | nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK,
        RESOLVE_BENEATH | RESOLVE_NO_XDEV | RESOLVE_NO_MAGICLINKS | RESOLVE_NO_SYMLINKS).ok()?;
    let before = identity(&file).ok()?;
    if before.uid != uid || before.mode & nix::libc::S_IFMT != nix::libc::S_IFREG
        || before.size > 16 * 1024 { return None; }
    let mut bytes = Vec::new();
    // Bound allocation even if a concurrently modified file grows after stat.
    (&file).take(16 * 1024 + 1).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 16 * 1024 || identity(&file).ok()? != before { return None; }
    String::from_utf8(bytes).ok()
}

fn parse_xdg(home: &Path, home_fd: &File, uid: u32) -> Vec<PathBuf> {
    let mut configured = HashMap::new();
    if let Some(text) = xdg_configuration(home_fd, uid) {
            for line in text.lines() {
                let Some((key,quoted))=line.split_once('=') else {continue};
                let key=match key{"XDG_DOCUMENTS_DIR"=>"XDG_DOCUMENTS_DIR","XDG_DOWNLOAD_DIR"=>"XDG_DOWNLOAD_DIR","XDG_DESKTOP_DIR"=>"XDG_DESKTOP_DIR",_=>continue};
                if !quoted.starts_with('"') || !quoted.ends_with('"') {continue;}
                let raw=&quoted[1..quoted.len()-1];
                if raw.contains('\\') || raw.contains('`') || raw.contains("$(") {continue;}
                let expanded=raw.strip_prefix("$HOME/").map(|rest|home.join(rest))
                    .or_else(||(raw=="$HOME").then(||home.to_owned()))
                    .or_else(||raw.starts_with('/').then(||PathBuf::from(raw)));
                if let Some(path)=expanded { configured.insert(key,path); }
            }
    }
    [("XDG_DOCUMENTS_DIR","Documents"),("XDG_DOWNLOAD_DIR","Downloads"),("XDG_DESKTOP_DIR","Desktop")].into_iter()
        .map(|(key,fallback)|configured.remove(key).unwrap_or_else(||home.join(fallback))).collect()
}

fn absolute_clean(path:&Path)->bool{
    let mut parts=path.components();
    matches!(parts.next(),Some(Component::RootDir))&&parts.all(|part|matches!(part,Component::Normal(_)))
}

impl Manager {
    pub fn propose_xdg_roots(&mut self, owner: Owner, home: &Path, now_ms: u64) -> Result<Proposal> {
        if !owner_valid(&owner) || !absolute_clean(home) { return Err(ErrorCode::InvalidArgument); }
        let (home_fd,home_identity)=open_root(home)?;
        if home_identity.uid != owner.uid { return Err(ErrorCode::PermissionDenied); }
        let home=home.to_owned();
        let mut seen=HashSet::new();let mut roots=Vec::new();
        for path in parse_xdg(&home, &home_fd, owner.uid) {
            if !path.starts_with(&home) || path==home || secret(path.strip_prefix(&home).map_err(|_|ErrorCode::PermissionDenied)?) {continue;}
            let Ok((file,id))=open_root(&path) else {continue};
            if id.uid!=owner.uid || id.device!=home_identity.device || id.mount_id!=home_identity.mount_id {continue;}
            let display=path.to_string_lossy().into_owned();let digest={let mut value=id.clone();value.size=0;value.modified_ns=0;value.changed_ns=0;identity_sha256(&value)};
            if !seen.insert((id.mount_id,id.device,id.inode)) {continue;}
            roots.push(Candidate{root:ProposedRoot{root_id:Uuid::new_v4().to_string(),display_path:display,identity_sha256:digest},path,identity:id});
            drop(file);
        }
        let proposal_id=Uuid::new_v4().to_string();let expires=now_ms.checked_add(PROPOSAL_LIFETIME_MS).ok_or(ErrorCode::InvalidArgument)?;
        let result=Proposal{proposal_id:proposal_id.clone(),expires_at_boottime_ms:expires,roots:roots.iter().map(|r|r.root.clone()).collect()};
        self.pending.retain(|_,proposal|proposal.expires>now_ms);
        if self.pending.len()>=64{return Err(ErrorCode::ResourceExhausted);}
        self.pending.insert(proposal_id,Pending{owner,expires,roots});Ok(result)
    }

    pub fn enroll(&mut self, owner: &Owner, proposal_id: &str, approved: &[String], allowed_access: &[Access], now_ms: u64) -> Result<Vec<RootGrant>> {
        if approved.is_empty() || approved.len()>3 || approved.iter().collect::<HashSet<_>>().len()!=approved.len()
            || allowed_access.is_empty() || allowed_access.len()>3 || allowed_access.iter().copied().collect::<HashSet<_>>().len()!=allowed_access.len(){return Err(ErrorCode::InvalidArgument);}
        let proposal=self.pending.get(proposal_id).ok_or(ErrorCode::PermissionDenied)?;
        if &proposal.owner!=owner{return Err(ErrorCode::PermissionDenied);}if now_ms>=proposal.expires{return Err(ErrorCode::ApprovalExpired);}
        if self.roots.len().checked_add(approved.len()).ok_or(ErrorCode::ResourceExhausted)?>MAX_ROOTS{return Err(ErrorCode::ResourceExhausted);}
        let expires=now_ms.checked_add(HANDLE_LIFETIME_MS).ok_or(ErrorCode::InvalidArgument)?;let mut prepared=Vec::new();
        for id in approved {
            let candidate=proposal.roots.iter().find(|root|&root.root.root_id==id).ok_or(ErrorCode::PermissionDenied)?;
            let (directory,current)=open_root(&candidate.path)?;
            if !same_root(&current,&candidate.identity){return Err(ErrorCode::TargetChanged);}
            let grant=RootGrant{root_id:id.clone(),display_path:candidate.root.display_path.clone(),identity_sha256:candidate.root.identity_sha256.clone(),allowed_access:allowed_access.to_vec(),expires_at_boottime_ms:expires};
            let root=Root{owner:owner.clone(),path:candidate.path.clone(),identity:current,identity_sha256:grant.identity_sha256.clone(),allowed:allowed_access.iter().copied().collect(),expires,directory};
            prepared.push((id.clone(),root,grant));
        }
        let mut result=Vec::with_capacity(prepared.len());
        // Consume only after owner, expiry, exact selection and live identities
        // pass. A foreign probe must not withdraw another client's proposal.
        self.pending.remove(proposal_id);
        for (id,root,grant) in prepared{self.roots.insert(id,root);result.push(grant);}
        Ok(result)
    }

    /// Re-resolve the exact proposed selection before displaying or consuming
    /// native consent. This observes identities; it issues no grant.
    pub fn review_enrollment(&self, owner: &Owner, proposal_id: &str, approved: &[String],
        allowed_access: &[Access], now_ms: u64) -> Result<Vec<ProposedRoot>> {
        if approved.is_empty() || approved.len()>3 || approved.iter().collect::<HashSet<_>>().len()!=approved.len()
            || allowed_access.is_empty() || allowed_access.len()>3
            || allowed_access.iter().copied().collect::<HashSet<_>>().len()!=allowed_access.len() {
            return Err(ErrorCode::InvalidArgument);
        }
        let proposal=self.pending.get(proposal_id).ok_or(ErrorCode::PermissionDenied)?;
        if &proposal.owner!=owner { return Err(ErrorCode::PermissionDenied); }
        if now_ms>=proposal.expires { return Err(ErrorCode::ApprovalExpired); }
        approved.iter().map(|id| {
            let candidate=proposal.roots.iter().find(|root|&root.root.root_id==id).ok_or(ErrorCode::PermissionDenied)?;
            let (_,current)=open_root(&candidate.path)?;
            if !same_root(&current,&candidate.identity) { return Err(ErrorCode::TargetChanged); }
            Ok(candidate.root.clone())
        }).collect()
    }

    fn root(&self, owner:&Owner, root_id:&str, now_ms:u64)->Result<&Root>{
        let root=self.roots.get(root_id).ok_or(ErrorCode::PermissionDenied)?;
        if &root.owner!=owner{return Err(ErrorCode::PermissionDenied);}if now_ms>=root.expires{return Err(ErrorCode::ApprovalExpired);}
        if !same_root(&identity(&root.directory)?,&root.identity){return Err(ErrorCode::TargetChanged);}
        // A retained descriptor must not keep a disappeared/replaced named
        // root visible to snippets merely because its old inode is still open.
        let (_,current)=open_root(&root.path).map_err(|_|ErrorCode::TargetChanged)?;
        if !same_root(&current,&root.identity){return Err(ErrorCode::TargetChanged);}Ok(root)
    }

    pub fn issue_handle(&mut self,owner:&Owner,root_id:&str,relative:&Path,access:Access,now_ms:u64)->Result<ScopedMetadata>{
        if !relative_valid(relative){return Err(ErrorCode::PermissionDenied);}if self.handles.len()>=MAX_HANDLES{return Err(ErrorCode::ResourceExhausted);}
        let root=self.root(owner,root_id,now_ms)?;if !root.allowed.contains(&access){return Err(ErrorCode::PermissionDenied);}
        let file=openat2(root.directory.as_raw_fd(),relative,nix::libc::O_RDONLY|nix::libc::O_NOFOLLOW,
            RESOLVE_BENEATH|RESOLVE_NO_XDEV|RESOLVE_NO_MAGICLINKS|RESOLVE_NO_SYMLINKS)?;
        let id=identity(&file)?;if id.uid!=owner.uid || id.mode&nix::libc::S_IFMT!=nix::libc::S_IFREG{return Err(ErrorCode::PermissionDenied);}
        let expires=now_ms.checked_add(HANDLE_LIFETIME_MS).ok_or(ErrorCode::InvalidArgument)?.min(root.expires);
        let handle=Uuid::new_v4().to_string();let relative=relative.to_string_lossy().into_owned();
        let value=ScopedMetadata{file_handle:handle.clone(),root_id:root_id.into(),relative_path:relative.clone(),identity_sha256:identity_sha256(&id),size_bytes:id.size,modified_ns:id.modified_ns,access,expires_at_boottime_ms:expires};
        self.handles.insert(handle,Handle{owner:owner.clone(),root_id:root_id.into(),relative,identity:id,access,expires});Ok(value)
    }

    fn reopen(&self,owner:&Owner,handle_id:&str,required:Access,now_ms:u64)->Result<File>{
        let handle=self.handles.get(handle_id).ok_or(ErrorCode::PermissionDenied)?;
        if &handle.owner!=owner{return Err(ErrorCode::PermissionDenied);}if now_ms>=handle.expires{return Err(ErrorCode::ApprovalExpired);}
        if handle.access!=required{return Err(ErrorCode::PermissionDenied);}
        let root=self.root(owner,&handle.root_id,now_ms)?;
        let file=openat2(root.directory.as_raw_fd(),Path::new(&handle.relative),nix::libc::O_RDONLY|nix::libc::O_NOFOLLOW,
            RESOLVE_BENEATH|RESOLVE_NO_XDEV|RESOLVE_NO_MAGICLINKS|RESOLVE_NO_SYMLINKS)?;
        if identity(&file)?!=handle.identity{return Err(ErrorCode::TargetChanged);}Ok(file)
    }

    pub fn metadata(&self,owner:&Owner,handle:&str,now_ms:u64)->Result<ScopedMetadata>{
        let file=self.reopen(owner,handle,Access::Metadata,now_ms)?;let record=self.handles.get(handle).ok_or(ErrorCode::PermissionDenied)?;let id=identity(&file)?;
        Ok(ScopedMetadata{file_handle:handle.into(),root_id:record.root_id.clone(),relative_path:record.relative.clone(),identity_sha256:identity_sha256(&id),size_bytes:id.size,modified_ns:id.modified_ns,access:record.access,expires_at_boottime_ms:record.expires})
    }

    pub fn read(&self,owner:&Owner,handle:&str,max_bytes:usize,now_ms:u64)->Result<Vec<u8>>{
        if max_bytes==0||max_bytes>MAX_READ_BYTES{return Err(ErrorCode::InvalidArgument);}let mut file=self.reopen(owner,handle,Access::Content,now_ms)?;
        let before=identity(&file)?;file.seek(SeekFrom::Start(0)).map_err(|_|ErrorCode::TargetChanged)?;
        let mut bytes=Vec::with_capacity(max_bytes.min(before.size as usize));file.take((max_bytes as u64)+1).read_to_end(&mut bytes).map_err(|_|ErrorCode::TargetChanged)?;
        if bytes.len()>max_bytes{return Err(ErrorCode::ResourceExhausted);}let file=self.reopen(owner,handle,Access::Content,now_ms)?;
        if identity(&file)?!=before{return Err(ErrorCode::TargetChanged);}
        // Selection is not permission to export recognizable raw private keys.
        // Check before returning bytes, even when the file was renamed to a
        // normal text filename. This complements mandatory path exclusions.
        if bytes.windows(b"PRIVATE KEY-----".len()).any(|part|part==b"PRIVATE KEY-----") {
            return Err(ErrorCode::SecretScopeDenied);
        }
        Ok(bytes)
    }

    pub fn revalidate_mutation(&self,owner:&Owner,handle:&str,now_ms:u64)->Result<()> { self.reopen(owner,handle,Access::Mutation,now_ms).map(drop) }

    pub fn cache_for_test(&mut self,handle:&str,chunk:&str,preview:&str,snippet:&str)->Result<()> {
        if !self.handles.contains_key(handle){return Err(ErrorCode::PermissionDenied);}
        self.chunks.push(CachedRecord{handle:handle.into(),value:chunk.into()});self.previews.push(CachedRecord{handle:handle.into(),value:preview.into()});self.snippets.push(CachedRecord{handle:handle.into(),value:snippet.into()});Ok(())
    }

    pub fn cached_snippet(&self,owner:&Owner,handle:&str,now_ms:u64)->Result<&str>{
        self.reopen(owner,handle,Access::Content,now_ms)?;
        self.snippets.iter().find(|entry|entry.handle==handle).map(|entry|entry.value.as_str()).ok_or(ErrorCode::TargetNotFound)
    }

    pub fn revoke(&mut self,owner:&Owner,root_id:&str)->Result<RevocationReceipt>{
        let root=self.roots.get(root_id).ok_or(ErrorCode::PermissionDenied)?;if &root.owner!=owner{return Err(ErrorCode::PermissionDenied);}
        self.roots.remove(root_id);let revoked:HashSet<String>=self.handles.iter().filter(|(_,handle)|handle.root_id==root_id).map(|(id,_)|id.clone()).collect();
        for id in &revoked{self.handles.remove(id);}let before=self.chunks.len()+self.previews.len()+self.snippets.len();
        self.chunks.retain(|entry|!revoked.contains(&entry.handle));self.previews.retain(|entry|!revoked.contains(&entry.handle));self.snippets.retain(|entry|!revoked.contains(&entry.handle));
        let after=self.chunks.len()+self.previews.len()+self.snippets.len();
        Ok(RevocationReceipt{root_id:root_id.into(),handles_revoked:revoked.len(),cached_records_purged:before-after,access_blocked:true})
    }

    pub fn active_roots(&self,owner:&Owner,now_ms:u64)->Vec<RootGrant>{
        self.roots.iter().filter(|(_,root)|&root.owner==owner&&now_ms<root.expires).map(|(id,root)|{let mut allowed_access=root.allowed.iter().copied().collect::<Vec<_>>();allowed_access.sort_by_key(|access|match access{Access::Metadata=>0,Access::Content=>1,Access::Mutation=>2});RootGrant{root_id:id.clone(),display_path:root.path.to_string_lossy().into_owned(),identity_sha256:root.identity_sha256.clone(),allowed_access,expires_at_boottime_ms:root.expires}}).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write, os::unix::fs::{PermissionsExt,symlink}};

    fn setup()->(tempfile::TempDir,Owner,Manager,Proposal){
        let temp=tempfile::tempdir().unwrap();let home=temp.path();fs::create_dir(home.join("Documents")).unwrap();fs::create_dir(home.join("Downloads")).unwrap();
        fs::write(home.join("Documents/note.txt"),b"safe text").unwrap();
        let uid=nix::unistd::geteuid().as_raw();let owner=Owner{uid,boot_id:Uuid::new_v4().to_string(),session_id:Some("fixture-session".into()),client_binding_sha256:"a".repeat(64)};
        let mut manager=Manager::default();let proposal=manager.propose_xdg_roots(owner.clone(),home,100).unwrap();(temp,owner,manager,proposal)
    }
    fn enroll()->(tempfile::TempDir,Owner,Manager,RootGrant){
        let (temp,owner,mut manager,proposal)=setup();let id=proposal.roots.iter().find(|root|root.display_path.ends_with("Documents")).unwrap().root_id.clone();
        let grant=manager.enroll(&owner,&proposal.proposal_id,&[id],&[Access::Metadata,Access::Content,Access::Mutation],101).unwrap().remove(0);(temp,owner,manager,grant)
    }

    #[test]
    fn only_existing_same_mount_xdg_roots_are_proposed_and_content_needs_consent(){
        let (_temp,owner,mut manager,proposal)=setup();assert_eq!(proposal.roots.len(),2);assert!(proposal.roots.iter().all(|root|root.display_path.ends_with("Documents")||root.display_path.ends_with("Downloads")));
        assert_eq!(manager.issue_handle(&owner,&proposal.roots[0].root_id,Path::new("note.txt"),Access::Content,102),Err(ErrorCode::PermissionDenied));
    }

    #[test]
    fn scoped_read_rechecks_identity_owner_expiry_and_secret_exclusions(){
        let (temp,owner,mut manager,root)=enroll();let handle=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Content,102).unwrap();
        assert_eq!(manager.read(&owner,&handle.file_handle,32,103).unwrap(),b"safe text");
        let mut foreign=owner.clone();foreign.uid+=1;assert_eq!(manager.read(&foreign,&handle.file_handle,32,103),Err(ErrorCode::PermissionDenied));
        assert_eq!(manager.read(&owner,&handle.file_handle,32,handle.expires_at_boottime_ms),Err(ErrorCode::ApprovalExpired));
        for path in [".ssh/id_rsa","project/.env","wallets/a.dat","cert.pem",".mozilla/firefox/logins.json",".aws/credentials","passwords.txt"]{assert_eq!(manager.issue_handle(&owner,&root.root_id,Path::new(path),Access::Content,104),Err(ErrorCode::PermissionDenied));}
        let path=temp.path().join("Documents/note.txt");
        fs::set_permissions(&path,fs::Permissions::from_mode(0o000)).unwrap();
        assert_eq!(manager.read(&owner,&handle.file_handle,32,105),Err(ErrorCode::PermissionDenied));
        fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();
        let mut file=File::options().write(true).truncate(true).open(path).unwrap();file.write_all(b"changed").unwrap();file.sync_all().unwrap();
        assert_eq!(manager.read(&owner,&handle.file_handle,32,106),Err(ErrorCode::TargetChanged));
    }

    #[test]
    fn openat2_denies_symlink_swap_escape_and_new_mount_policy_is_kernel_enforced(){
        let (temp,owner,mut manager,root)=enroll();symlink("/etc/passwd",temp.path().join("Documents/link")).unwrap();
        assert_eq!(manager.issue_handle(&owner,&root.root_id,Path::new("link"),Access::Content,102),Err(ErrorCode::PermissionDenied));
        assert_eq!(manager.issue_handle(&owner,&root.root_id,Path::new("../Downloads/anything"),Access::Content,102),Err(ErrorCode::PermissionDenied));
    }

    #[test]
    fn revocation_blocks_direct_and_cached_reads_before_purging_all_records(){
        let (_temp,owner,mut manager,root)=enroll();let handle=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Content,102).unwrap();
        manager.cache_for_test(&handle.file_handle,"chunk","preview","snippet").unwrap();assert_eq!(manager.cached_snippet(&owner,&handle.file_handle,103),Ok("snippet"));
        let receipt=manager.revoke(&owner,&root.root_id).unwrap();assert!(receipt.access_blocked);assert_eq!(receipt.handles_revoked,1);assert_eq!(receipt.cached_records_purged,3);
        assert_eq!(manager.read(&owner,&handle.file_handle,32,104),Err(ErrorCode::PermissionDenied));assert_eq!(manager.cached_snippet(&owner,&handle.file_handle,104),Err(ErrorCode::PermissionDenied));
    }

    #[test]
    fn another_same_uid_connection_or_session_cannot_reuse_authority(){
        let (_temp,owner,mut manager,root)=enroll();
        let handle=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Content,102).unwrap();
        manager.cache_for_test(&handle.file_handle,"chunk","preview","snippet").unwrap();
        let mut other=owner.clone();other.client_binding_sha256="b".repeat(64);
        for foreign in [other,{let mut value=owner.clone();value.session_id=Some("another-session".into());value}] {
            assert_eq!(manager.issue_handle(&foreign,&root.root_id,Path::new("note.txt"),Access::Content,103),Err(ErrorCode::PermissionDenied));
            assert_eq!(manager.read(&foreign,&handle.file_handle,32,103),Err(ErrorCode::PermissionDenied));
            assert_eq!(manager.cached_snippet(&foreign,&handle.file_handle,103),Err(ErrorCode::PermissionDenied));
            assert_eq!(manager.revoke(&foreign,&root.root_id),Err(ErrorCode::PermissionDenied));
            assert!(manager.active_roots(&foreign,103).is_empty());
        }
        assert_eq!(manager.read(&owner,&handle.file_handle,32,104).unwrap(),b"safe text");
    }

    #[test]
    fn xdg_configuration_is_bounded_descriptor_relative_and_never_follows_links(){
        let (temp,owner,mut manager,_)=setup();let home=temp.path();
        fs::create_dir(home.join(".config")).unwrap();fs::create_dir(home.join("Work")).unwrap();
        let config=home.join(".config/user-dirs.dirs");
        fs::write(&config,format!("XDG_DOCUMENTS_DIR=\"{}\"\nXDG_DOWNLOAD_DIR=\"$HOME\"\n",home.join("Work").display())).unwrap();
        let proposal=manager.propose_xdg_roots(owner.clone(),home,110).unwrap();
        assert_eq!(proposal.roots.len(),1);assert!(proposal.roots[0].display_path.ends_with("Work"));
        let (fd,_)=open_root(home).unwrap();
        fs::remove_file(&config).unwrap();symlink(home.join("Documents/note.txt"),&config).unwrap();
        assert!(xdg_configuration(&fd,owner.uid).is_none());
        fs::remove_file(&config).unwrap();fs::write(&config,vec![b'x';16*1024+1]).unwrap();
        assert!(xdg_configuration(&fd,owner.uid).is_none());
        fs::remove_file(&config).unwrap();
        let name=CString::new(config.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: live NUL-terminated name, fixed permission bits; creates only
        // the owned fixture FIFO. Discovery must refuse without blocking on it.
        assert_eq!(unsafe{nix::libc::mkfifo(name.as_ptr(),0o600)},0);
        assert!(xdg_configuration(&fd,owner.uid).is_none());
    }

    #[test]
    fn foreign_enrollment_probe_does_not_consume_the_owners_proposal(){
        let (_temp,owner,mut manager,proposal)=setup();
        let id=proposal.roots[0].root_id.clone();
        let mut foreign=owner.clone();foreign.client_binding_sha256="b".repeat(64);
        assert_eq!(manager.enroll(&foreign,&proposal.proposal_id,&[id.clone()],&[Access::Content],101),Err(ErrorCode::PermissionDenied));
        let roots=manager.enroll(&owner,&proposal.proposal_id,&[id.clone()],&[Access::Content],102).unwrap();
        assert_eq!(roots[0].root_id,id);
        assert_eq!(manager.enroll(&owner,&proposal.proposal_id,&[id],&[Access::Content],103),Err(ErrorCode::PermissionDenied));
    }

    #[test]
    fn selected_renamed_private_key_is_never_returned_as_content(){
        let (temp,owner,mut manager,root)=enroll();
        let synthetic=[b"-----BEGIN ".as_slice(),b"OPENSSH PRIVATE KEY-----\nsynthetic fixture only".as_slice()].concat();
        fs::write(temp.path().join("Documents/notes.txt"),synthetic).unwrap();
        let handle=manager.issue_handle(&owner,&root.root_id,Path::new("notes.txt"),Access::Content,102).unwrap();
        assert_eq!(manager.read(&owner,&handle.file_handle,128,103),Err(ErrorCode::SecretScopeDenied));
        for path in ["id_ecdsa","credentials.json","kwalletd6/anything","wallet.dat","Login Data"] {
            assert_eq!(manager.issue_handle(&owner,&root.root_id,Path::new(path),Access::Content,103),Err(ErrorCode::PermissionDenied));
        }
    }

    #[test]
    fn enrollment_review_is_observation_only_and_rejects_replaced_or_expired_roots(){
        let (temp,owner,mut manager,proposal)=setup();
        let id=proposal.roots.iter().find(|r|r.display_path.ends_with("Documents")).unwrap().root_id.clone();
        let reviewed=manager.review_enrollment(&owner,&proposal.proposal_id,&[id.clone()],&[Access::Content],101).unwrap();
        assert_eq!(reviewed[0].root_id,id);
        assert_eq!(manager.issue_handle(&owner,&id,Path::new("note.txt"),Access::Content,102),Err(ErrorCode::PermissionDenied));
        assert_eq!(manager.review_enrollment(&owner,&proposal.proposal_id,&[id.clone()],&[Access::Content],proposal.expires_at_boottime_ms),Err(ErrorCode::ApprovalExpired));
        fs::rename(temp.path().join("Documents"),temp.path().join("OldDocuments")).unwrap();
        fs::create_dir(temp.path().join("Documents")).unwrap();
        assert_eq!(manager.review_enrollment(&owner,&proposal.proposal_id,&[id],&[Access::Content],103),Err(ErrorCode::TargetChanged));
    }

    #[test]
    fn renamed_granted_root_blocks_retained_handles_and_cached_snippets(){
        let (temp,owner,mut manager,root)=enroll();
        let handle=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Content,102).unwrap();
        manager.cache_for_test(&handle.file_handle,"chunk","preview","snippet").unwrap();
        fs::rename(temp.path().join("Documents"),temp.path().join("OldDocuments")).unwrap();
        fs::create_dir(temp.path().join("Documents")).unwrap();
        fs::write(temp.path().join("Documents/note.txt"),b"replacement content").unwrap();
        assert_eq!(manager.read(&owner,&handle.file_handle,32,103),Err(ErrorCode::TargetChanged));
        assert_eq!(manager.cached_snippet(&owner,&handle.file_handle,103),Err(ErrorCode::TargetChanged));
    }

    #[test]
    fn metadata_and_mutation_capabilities_do_not_widen_each_other(){
        let (_temp,owner,mut manager,root)=enroll();let metadata=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Metadata,102).unwrap();
        assert!(manager.metadata(&owner,&metadata.file_handle,103).is_ok());assert_eq!(manager.read(&owner,&metadata.file_handle,32,103),Err(ErrorCode::PermissionDenied));
        let mutation=manager.issue_handle(&owner,&root.root_id,Path::new("note.txt"),Access::Mutation,103).unwrap();assert_eq!(manager.revalidate_mutation(&owner,&mutation.file_handle,104),Ok(()));
        assert_eq!(manager.read(&owner,&mutation.file_handle,32,104),Err(ErrorCode::PermissionDenied));
    }
}
