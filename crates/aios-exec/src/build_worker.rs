//! Low-authority worker for one fixed, sealed managed-state candidate shape.
//! Root authenticates over the private socket; the worker can build and retain
//! closures but has no activation or boot-selection API.
use crate::{candidate::CandidateStore, digest, sha256, store, uuid};
use crate::ledger::{BuildResult as LedgerBuildResult, PreparedPlan, ResourcePermission};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, PermissionsExt},
            net::{UnixListener, UnixStream},
            process::CommandExt,
        },
    },
    path::Path,
    process::{Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const RUNTIME: &str = "/run/aios-build";
const SOCKET: &str = "/run/aios-build/worker.sock";
const STATE: &str = "/var/lib/aios/build";
const ROOTS: &str = "/var/lib/aios/build/roots";
const CACHE: &str = "/var/lib/aios/build/cache";
const CANDIDATES: &str = "/var/lib/aios/candidates";
const MAX_REQUEST: usize = 16 * 1024;
const MAX_REPLY: usize = 8 * 1024 * 1024;
const MAX_PATH_INFO: usize = 8 * 1024 * 1024;
const MAX_STDERR: usize = 64 * 1024;
const MAX_RESOURCE_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
pub(crate) const MAX_BUILD_BYTES: u64 = 16 * 1024 * 1024 * 1024;
pub(crate) const MAX_DOWNLOAD_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub(crate) const MIN_RECOVERY_RESERVE: u64 = 8 * 1024 * 1024 * 1024;
const BUILD_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
enum Error {
    Authority,
    Identity,
    Integrity,
    Invalid,
    Candidate,
    PinnedInput,
    Io,
    Baseline,
    TargetChanged,
    Resource,
    Timeout,
    Build {
        stderr_sha256: String,
        stderr_tail: String,
    },
    NixOutput {
        stage: &'static str,
        stdout_sha256: String,
    },
    IntegrityAt(&'static str),
}
impl From<io::Error> for Error {
    fn from(_: io::Error) -> Self {
        Self::Io
    }
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Status {
        schema_version: u32,
        request_id: String,
    },
    Build {
        schema_version: u32,
        request_id: String,
        plan_id: String,
        candidate_sha256: String,
        template_sha256: String,
        managed_sha256: String,
        baseline_closure: String,
        max_build_bytes: u64,
        max_download_bytes: u64,
        recovery_reserve_bytes: u64,
        approved_cache: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildResult {
    candidate_sha256: String,
    derivation: String,
    closure: String,
    inventory_sha256: String,
    added_paths: Vec<String>,
    removed_paths: Vec<String>,
    nar_bytes: u64,
    measured_download_bytes: Option<u64>,
    gc_root: String,
    prior_gc_root: String,
}

fn builder_uid() -> Result<u32> {
    let value = unsafe { libc::getpwnam(c"aios-builder".as_ptr()) };
    if value.is_null() {
        return Err(Error::Identity);
    }
    let uid = unsafe { (*value).pw_uid };
    if uid == 0 {
        return Err(Error::Identity);
    }
    Ok(uid)
}

fn account() -> Result<u32> {
    let uid = builder_uid()?;
    if unsafe { libc::getuid() } != uid || unsafe { libc::geteuid() } != uid {
        return Err(Error::Identity);
    }
    Ok(uid)
}

fn directory(path: &str, uid: u32, mode: u32) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o777 != mode
        || fs::canonicalize(path).map_err(|_| Error::Identity)? != Path::new(path)
    {
        return Err(Error::Identity);
    }
    Ok(())
}

fn root_directory(uid: u32) -> Result<()> {
    directory(STATE, uid, 0o700)?;
    for path in [ROOTS, CACHE] {
        match fs::create_dir(path) {
            Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o700))?,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        directory(path, uid, 0o700)?;
    }
    Ok(())
}

fn peer(stream: &UnixStream) -> Result<libc::ucred> {
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &mut size,
        )
    } != 0
        || size as usize != std::mem::size_of::<libc::ucred>()
    {
        return Err(Error::Identity);
    }
    let credentials = unsafe { credentials.assume_init() };
    if credentials.pid <= 0 || credentials.uid != 0 {
        return Err(Error::Authority);
    }
    Ok(credentials)
}

fn frame_read(stream: &mut UnixStream, bound: usize) -> Result<Vec<u8>> {
    let mut length = [0; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > bound {
        return Err(Error::Invalid);
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn frame_write(stream: &mut UnixStream, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| Error::Invalid)?;
    if bytes.len() > MAX_REPLY {
        return Err(Error::Resource);
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    Ok(())
}

fn available(path: &str) -> Result<u64> {
    let path = std::ffi::CString::new(path).map_err(|_| Error::Invalid)?;
    let mut value = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(path.as_ptr(), value.as_mut_ptr()) } != 0 {
        return Err(Error::Io);
    }
    let value = unsafe { value.assume_init() };
    Ok(value.f_bavail.saturating_mul(value.f_frsize))
}

fn run(program: &str, arguments: &[&str], timeout: Duration, bound: usize) -> Result<Vec<u8>> {
    let canonical = fs::canonicalize(program)?;
    let metadata = fs::metadata(program)?;
    if !Path::new(program).starts_with("/nix/store")
        || !canonical.starts_with("/nix/store")
        || !metadata.is_file()
        || metadata.uid() != 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(Error::Integrity);
    }
    let mut child = Command::new(program)
        .args(arguments)
        .env_clear()
        .env("HOME", "/var/empty")
        .env("LANG", "C.UTF-8")
        .env("XDG_CACHE_HOME", CACHE)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let stdout = child.stdout.take().ok_or(Error::Io)?;
    let stderr = child.stderr.take().ok_or(Error::Io)?;
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        stdout
            .take(bound as u64 + 1)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let error_reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        stderr
            .take(MAX_STDERR as u64 + 1)
            .read_to_end(&mut output)
            .map(|_| output)
    });
    let started = Instant::now();
    let status: ExitStatus = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            // The worker created this process group; killing the group closes
            // inherited output descriptors before the bounded reader is joined.
            unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            let _ = error_reader.join();
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let output = reader.join().map_err(|_| Error::Io)??;
    let errors = error_reader.join().map_err(|_| Error::Io)??;
    if output.len() > bound || errors.len() > MAX_STDERR {
        return Err(Error::Resource);
    }
    if !status.success() {
        let text = String::from_utf8_lossy(&errors);
        let sanitized: String = text
            .chars()
            .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
            .collect();
        let stderr_tail: String = sanitized
            .chars()
            .rev()
            .take(4096)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        return Err(Error::Build {
            stderr_sha256: sha256(&errors),
            stderr_tail,
        });
    }
    Ok(output)
}

fn lines(bytes: &[u8], maximum: usize) -> Result<Vec<String>> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Invalid)?;
    let values: Vec<_> = text.lines().map(str::to_owned).collect();
    if values.is_empty() || values.len() > maximum || values.iter().any(|line| line.is_empty()) {
        return Err(Error::Invalid);
    }
    Ok(values)
}

fn nix_output(stage: &'static str, output: &[u8]) -> Error {
    Error::NixOutput {
        stage,
        stdout_sha256: sha256(output),
    }
}

fn inventory(nix: &str, closure: &str) -> Result<(BTreeSet<String>, u64)> {
    let output = run(
        nix,
        &["path-info", "--json", "--recursive", closure],
        QUERY_TIMEOUT,
        MAX_PATH_INFO,
    )?;
    let value: Value =
        serde_json::from_slice(&output).map_err(|_| nix_output("inventory-json", &output))?;
    let entries = value
        .as_object()
        .ok_or_else(|| nix_output("inventory-object", &output))?;
    if entries.is_empty() || entries.len() > 16_384 {
        return Err(Error::Resource);
    }
    let mut paths = BTreeSet::new();
    let mut bytes = 0u64;
    for (path, metadata) in entries {
        if !store(path) || !paths.insert(path.into()) {
            return Err(Error::IntegrityAt("inventory-path"));
        }
        let size = metadata
            .get("narSize")
            .and_then(Value::as_u64)
            .ok_or_else(|| nix_output("inventory-size", &output))?;
        bytes = bytes.checked_add(size).ok_or(Error::Resource)?;
    }
    Ok((paths, bytes))
}

fn retain(nix_store: &str, root: &Path, closure: &str) -> Result<()> {
    let root_text = root.to_str().ok_or(Error::Invalid)?;
    match fs::symlink_metadata(root) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let _ = run(
                nix_store,
                &["--realise", closure, "--add-root", root_text, "--indirect"],
                QUERY_TIMEOUT,
                4096,
            )?;
        }
        Err(_) => return Err(Error::Io),
    }
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || fs::canonicalize(root)? != Path::new(closure)
    {
        return Err(Error::IntegrityAt("gc-root-state"));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build(
    plan_id: &str,
    candidate_sha256: &str,
    template_sha256: &str,
    managed_sha256: &str,
    baseline_closure: &str,
    max_build_bytes: u64,
    max_download_bytes: u64,
    recovery_reserve_bytes: u64,
    approved_cache: &str,
) -> Result<BuildResult> {
    if !uuid(plan_id)
        || !digest(candidate_sha256)
        || !digest(template_sha256)
        || !digest(managed_sha256)
        || !store(baseline_closure)
        || max_build_bytes == 0
        || max_build_bytes > MAX_BUILD_BYTES
        || max_download_bytes > MAX_DOWNLOAD_BYTES
        || recovery_reserve_bytes < MIN_RECOVERY_RESERVE
        || recovery_reserve_bytes > MAX_RESOURCE_BYTES
        || approved_cache != "https://cache.nixos.org"
    {
        return Err(Error::Invalid);
    }
    let candidate = CandidateStore::registered(candidate_sha256).map_err(|_| Error::Candidate)?;
    if candidate.manifest().template_sha256 != template_sha256
        || candidate.manifest().managed_sha256 != managed_sha256
        || candidate.path() != Path::new(CANDIDATES).join(candidate_sha256)
    {
        return Err(Error::Candidate);
    }
    let baseline = fs::canonicalize("/run/current-system")?;
    if baseline != Path::new(baseline_closure) {
        return Err(Error::Baseline);
    }
    let required = max_build_bytes
        .checked_add(recovery_reserve_bytes)
        .ok_or(Error::Resource)?;
    if available("/nix/store")? < required {
        return Err(Error::Resource);
    }
    let nix = option_env!("AIOS_NIX").ok_or(Error::Integrity)?;
    let nix_store = option_env!("AIOS_NIX_STORE").ok_or(Error::Integrity)?;
    let nixpkgs = option_env!("AIOS_NIXPKGS").ok_or(Error::Integrity)?;
    let nixpkgs_metadata = fs::symlink_metadata(nixpkgs)?;
    if !store(nixpkgs)
        || !nixpkgs_metadata.is_dir()
        || nixpkgs_metadata.uid() != 0
        || nixpkgs_metadata.mode() & 0o022 != 0
    {
        return Err(Error::PinnedInput);
    }
    let nixpkgs_reference = format!("path:{nixpkgs}");
    let reference = format!(
        "path:{CANDIDATES}/{candidate_sha256}#nixosConfigurations.aios-dev.config.system.build.toplevel"
    );
    let output = run(
        nix,
        &[
            "build",
            "--offline",
            "--no-link",
            "--no-write-lock-file",
            "--no-update-lock-file",
            "--override-input",
            "nixpkgs",
            &nixpkgs_reference,
            "--print-out-paths",
            &reference,
        ],
        BUILD_TIMEOUT,
        4096,
    )?;
    let outputs =
        lines(&output, 1).map_err(|_| nix_output("build-path", &output))?;
    let closure = outputs
        .into_iter()
        .next()
        .ok_or(Error::IntegrityAt("build-path-empty"))?;
    if !store(&closure) || fs::canonicalize(&closure)? != Path::new(&closure) {
        return Err(Error::IntegrityAt("build-path"));
    }
    let derivation_output = run(
        nix,
        &["path-info", "--derivation", &closure],
        QUERY_TIMEOUT,
        4096,
    )?;
    let derivations = lines(&derivation_output, 1)
        .map_err(|_| nix_output("derivation-path", &derivation_output))?;
    let derivation = derivations
        .into_iter()
        .next()
        .ok_or(Error::IntegrityAt("derivation-empty"))?;
    if !store(&derivation) || !derivation.ends_with(".drv") {
        return Err(Error::IntegrityAt("derivation"));
    }
    let (candidate_paths, nar_bytes) = inventory(nix, &closure)?;
    let (baseline_paths, _) = inventory(nix, baseline_closure)?;
    if nar_bytes > max_build_bytes {
        return Err(Error::Resource);
    }
    let rechecked = CandidateStore::registered(candidate_sha256)
        .map_err(|_| Error::IntegrityAt("candidate-reload"))?;
    if rechecked.manifest() != candidate.manifest() {
        return Err(Error::IntegrityAt("candidate-recheck"));
    }
    let inventory: Vec<_> = candidate_paths.iter().cloned().collect();
    let inventory_bytes =
        crate::canonical(&inventory).map_err(|_| Error::IntegrityAt("inventory-canonical"))?;
    if fs::canonicalize("/run/current-system")? != baseline {
        return Err(Error::TargetChanged);
    }
    let root = Path::new(ROOTS).join(format!("{plan_id}-candidate"));
    let prior_root = Path::new(ROOTS).join(format!("{plan_id}-prior"));
    retain(nix_store, &prior_root, baseline_closure)?;
    retain(nix_store, &root, &closure)?;
    let root_text = root.to_str().ok_or(Error::Invalid)?;
    let prior_root_text = prior_root.to_str().ok_or(Error::Invalid)?;
    let result = BuildResult {
        candidate_sha256: candidate_sha256.into(),
        derivation,
        closure,
        inventory_sha256: sha256(&inventory_bytes),
        added_paths: candidate_paths
            .difference(&baseline_paths)
            .cloned()
            .collect(),
        removed_paths: baseline_paths
            .difference(&candidate_paths)
            .cloned()
            .collect(),
        nar_bytes,
        measured_download_bytes: Some(0),
        gc_root: root_text.into(),
        prior_gc_root: prior_root_text.into(),
    };
    Ok(result)
}

fn serve(mut stream: UnixStream, uid: u32) -> Result<()> {
    peer(&stream)?;
    directory(RUNTIME, uid, 0o700)?;
    root_directory(uid)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let bytes = frame_read(&mut stream, MAX_REQUEST)?;
    let request: Request = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
    let reply = match request {
        Request::Status {
            schema_version,
            request_id,
        } if schema_version == 1 && uuid(&request_id) => json!({
            "schema_version": 1,
            "request_id": request_id,
            "ok": true,
            "data": {
                "uid": uid,
                "candidate_build_authority": true,
                "activation_authority": false,
                "candidate_template_selection": false,
                "nix_trusted_user": false,
                "network_egress": false
            }
        }),
        Request::Build {
            schema_version,
            request_id,
            plan_id,
            candidate_sha256,
            template_sha256,
            managed_sha256,
            baseline_closure,
            max_build_bytes,
            max_download_bytes,
            recovery_reserve_bytes,
            approved_cache,
        } if schema_version == 1 && uuid(&request_id) => match build(
            &plan_id,
            &candidate_sha256,
            &template_sha256,
            &managed_sha256,
            &baseline_closure,
            max_build_bytes,
            max_download_bytes,
            recovery_reserve_bytes,
            &approved_cache,
        ) {
            Ok(build) => json!({
                "schema_version": 1,
                "request_id": request_id,
                "ok": true,
                "build": build
            }),
            Err(error) => json!({
                "schema_version": 1,
                "request_id": request_id,
                "ok": false,
                "error": format!("{error:?}")
            }),
        },
        _ => json!({
            "schema_version": 1,
            "request_id": null,
            "ok": false,
            "error": "INVALID_REQUEST"
        }),
    };
    frame_write(&mut stream, &reply)
}

/// A capability proving that the authenticated single-flight worker returned a
/// complete failure response for this request. It carries no caller-controlled
/// error text and cannot be constructed outside this module.
pub(crate) struct VerifiedWorkerFailure {
    _private: (),
}

pub(crate) enum WorkerCompletion {
    Built(VerifiedWorkerBuild),
    Failed(VerifiedWorkerFailure),
}

/// Non-serializable capability returned only after the root broker has
/// authenticated the installed worker, exact response and retained roots.
pub(crate) struct VerifiedWorkerBuild {
    result: LedgerBuildResult,
}
impl VerifiedWorkerBuild {
    pub(crate) fn into_result(self) -> LedgerBuildResult {
        self.result
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildReply {
    schema_version: u32,
    request_id: String,
    ok: bool,
    build: Option<BuildResult>,
    error: Option<String>,
}

fn worker_identity(stream: &UnixStream, uid: u32) -> Result<libc::ucred> {
    let metadata = fs::symlink_metadata(SOCKET)?;
    if !metadata.file_type().is_socket()
        || metadata.uid() != uid
        || metadata.mode() & 0o777 != 0o600
    {
        return Err(Error::Identity);
    }
    let mut credentials = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(), &mut size)
    } != 0 || size as usize != std::mem::size_of::<libc::ucred>() {
        return Err(Error::Identity);
    }
    let credentials = unsafe { credentials.assume_init() };
    if credentials.pid <= 1 || credentials.uid != uid {
        return Err(Error::Identity);
    }
    let executable = fs::read_link(format!("/proc/{}/exe", credentials.pid))?;
    let executable_metadata = fs::metadata(&executable)?;
    if !executable.starts_with("/nix/store")
        || !executable.ends_with("bin/aios-buildd")
        || !executable_metadata.is_file()
        || executable_metadata.uid() != 0
        || executable_metadata.mode() & 0o022 != 0
    {
        return Err(Error::Identity);
    }
    let mut cgroup = vec![];
    File::open(format!("/proc/{}/cgroup", credentials.pid))?
        .take(64 * 1024 + 1).read_to_end(&mut cgroup)?;
    if cgroup.len() > 64 * 1024
        || !cgroup.split(|byte| *byte == b'\n').any(|line| {
            line.ends_with(b"/aios-build.service")
                || line.windows(b"/aios-build.service/".len())
                    .any(|window| window == b"/aios-build.service/")
        })
    {
        return Err(Error::Identity);
    }
    Ok(credentials)
}

fn retained_root(path: &str, expected: &str, uid: u32, suffix: &str) -> Result<()> {
    if path != format!("{ROOTS}/{suffix}") {
        return Err(Error::Integrity);
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_symlink()
        || metadata.uid() != uid
        || fs::read_link(path)?.to_str() != Some(expected)
    {
        return Err(Error::Integrity);
    }
    Ok(())
}

/// Invoke only the fixed installed worker protocol. The caller supplies a
/// ledger-validated permission rather than arbitrary paths or Nix arguments.
pub(crate) fn supervise(
    plan: &PreparedPlan,
    permission: &ResourcePermission,
) -> crate::Result<WorkerCompletion> {
    let run = || -> Result<WorkerCompletion> {
        plan.validate().map_err(|_| Error::Invalid)?;
        let candidate = CandidateStore::registered(&plan.candidate_sha256)
            .map_err(|_| Error::Candidate)?;
        if candidate.manifest().template_sha256 != plan.template_sha256 {
            return Err(Error::Candidate);
        }
        let uid = builder_uid()?;
        let mut stream = UnixStream::connect(SOCKET)?;
        let peer = worker_identity(&stream, uid)?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        stream.set_read_timeout(Some(BUILD_TIMEOUT + QUERY_TIMEOUT + Duration::from_secs(30)))?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = json!({
            "operation": "build",
            "schema_version": 1,
            "request_id": request_id,
            "plan_id": plan.plan_id,
            "candidate_sha256": plan.candidate_sha256,
            "template_sha256": plan.template_sha256,
            "managed_sha256": candidate.manifest().managed_sha256,
            "baseline_closure": plan.baseline.running_closure,
            "max_build_bytes": permission.max_build_bytes,
            "max_download_bytes": permission.max_download_bytes,
            "recovery_reserve_bytes": permission.recovery_reserve_bytes,
            "approved_cache": permission.approved_cache,
        });
        frame_write(&mut stream, &request)?;
        let bytes = frame_read(&mut stream, MAX_REPLY)?;
        let reply: BuildReply = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
        if reply.schema_version != 1 || reply.request_id != request_id {
            return Err(Error::Integrity);
        }
        let rechecked_peer = worker_identity(&stream, uid)?;
        if (rechecked_peer.pid, rechecked_peer.uid, rechecked_peer.gid)
            != (peer.pid, peer.uid, peer.gid)
        {
            return Err(Error::TargetChanged);
        }
        let rechecked = CandidateStore::registered(&plan.candidate_sha256)
            .map_err(|_| Error::Candidate)?;
        if rechecked.manifest() != candidate.manifest() {
            return Err(Error::TargetChanged);
        }
        if !reply.ok {
            if reply.build.is_some() || reply.error.as_deref().is_none_or(str::is_empty) {
                return Err(Error::Integrity);
            }
            return Ok(WorkerCompletion::Failed(VerifiedWorkerFailure {
                _private: (),
            }));
        }
        if reply.error.is_some() {
            return Err(Error::Integrity);
        }
        let result = reply.build.ok_or(Error::Integrity)?;
        retained_root(
            &result.prior_gc_root, &plan.baseline.running_closure, uid,
            &format!("{}-prior", plan.plan_id))?;
        retained_root(
            &result.gc_root, &result.closure, uid,
            &format!("{}-candidate", plan.plan_id))?;
        let result = LedgerBuildResult {
            candidate_sha256: result.candidate_sha256,
            derivation: result.derivation,
            closure: result.closure,
            inventory_sha256: result.inventory_sha256,
            added_paths: result.added_paths,
            removed_paths: result.removed_paths,
            nar_bytes: result.nar_bytes,
            measured_download_bytes: result.measured_download_bytes,
        };
        Ok(WorkerCompletion::Built(VerifiedWorkerBuild { result }))
    };
    run().map_err(|error| match error {
        Error::TargetChanged | Error::Baseline => crate::Error::TargetChanged,
        Error::Resource => crate::Error::ResourcePermissionRequired,
        Error::Timeout => crate::Error::Expired,
        Error::Io => crate::Error::Io,
        _ => crate::Error::Integrity,
    })
}

fn run_service() -> Result<()> {
    let uid = account()?;
    directory(RUNTIME, uid, 0o700)?;
    root_directory(uid)?;
    if let Ok(metadata) = fs::symlink_metadata(SOCKET) {
        if !metadata.file_type().is_socket()
            || metadata.uid() != uid
            || metadata.mode() & 0o777 != 0o600
            || UnixStream::connect(SOCKET).is_ok()
        {
            return Err(Error::Identity);
        }
        fs::remove_file(SOCKET)?;
    }
    let listener = UnixListener::bind(SOCKET)?;
    fs::set_permissions(SOCKET, fs::Permissions::from_mode(0o600))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = serve(stream, uid) {
                    eprintln!("aios-buildd: request refused: {error:?}");
                }
            }
            Err(_) => return Err(Error::Io),
        }
    }
    Err(Error::Io)
}

pub fn entry() -> std::process::ExitCode {
    match run_service() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aios-buildd: refused: {error:?}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_schema_rejects_unknown_and_unbounded_authority() {
        let valid = json!({
            "operation": "status",
            "schema_version": 1,
            "request_id": "123e4567-e89b-12d3-a456-426614174000"
        });
        assert!(serde_json::from_value::<Request>(valid.clone()).is_ok());
        let mut unknown = valid;
        unknown["command"] = Value::String("nix build arbitrary".into());
        assert!(serde_json::from_value::<Request>(unknown).is_err());
        assert!(
            serde_json::from_value::<Request>(json!({
                "operation": "build",
                "schema_version": 1,
                "request_id": "123e4567-e89b-12d3-a456-426614174000",
                "plan_id": "123e4567-e89b-12d3-a456-426614174001",
                "candidate_sha256": "0".repeat(64),
                "template_sha256": "1".repeat(64),
                "managed_sha256": "2".repeat(64),
                "baseline_closure": "/nix/store/00000000000000000000000000000000-base",
                "max_build_bytes": 1,
                "max_download_bytes": 0,
                "recovery_reserve_bytes": MIN_RECOVERY_RESERVE,
                "approved_cache": "https://cache.nixos.org",
                "activation": true
            }))
            .is_err()
        );
    }
}
