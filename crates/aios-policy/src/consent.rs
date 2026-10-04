//! Broker-owned native read consent. No JSON decision, boolean, inherited
//! executable path or client-provided token can construct the authority below.
//! The provider must authenticate the originating client and native desktop,
//! freshly resolve resources on each poll, and drop pending consent on Stop.
use super::*;
use std::{fs, io::{Read, Write}, net::Shutdown, os::unix::{fs::{MetadataExt, FileTypeExt}, net::UnixStream},
    path::PathBuf, process::{Child, Command, Stdio}};

#[derive(Clone, Serialize)]
pub struct SelectedWindow {
    pub handle: String, pub identity_sha256: String, pub name: String, pub window: String,
}
/// Native display adapter output, never deserialized from a request. The
/// graphical identity digest includes logind, compositor and socket bindings.
pub struct NativeDesktop {
    pub uid: u32, pub boot_id: String, pub session_id: String,
    pub identity_sha256: String, pub socket_name: String,
}
pub struct ReadPresentation {
    pub target: String, pub profile: String, pub goal: String,
    pub windows: Vec<SelectedWindow>, pub evidence: Vec<String>,
}
pub struct ReadProposal {
    intent: Intent, scope: Scope, desktop: NativeDesktop, presentation: ReadPresentation,
    revision: String, incarnation: Uuid, nonce: Uuid, issued_ms: u64, expires_ms: u64, digest: String,
}
/// Single-use, local native result. Only the owned pinned child transport can
/// create this value. Consuming it rechecks identity, scope, policy and time.
pub struct NativeReadDecision { proposal: ReadProposal, cancelled: Arc<AtomicBool> }
/// Revocation only, suitable for the independent control loop. This closes
/// the prompt's actual channel without waiting for native resource queries.
pub struct ReadCancellation { stream: UnixStream, cancelled: Arc<AtomicBool> }
impl ReadCancellation {
    pub fn cancel(&self) {self.cancelled.store(true,Ordering::Release);let _=self.stream.shutdown(Shutdown::Both);}
}
/// Pollable transport: caller keeps control requests responsive. Drop closes
/// the anonymous channel and reaps only this owned child, revoking pending UI.
pub struct ReadConfirmation {
    proposal: Option<ReadProposal>, stream: UnixStream, child: Child,
    received: Vec<u8>, eof: bool, native_seen: bool, finished: bool,
    cancelled: Arc<AtomicBool>,
}
fn text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(|c|
        (c.is_control() && c != '\n' && c != '\t') || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}
fn session_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
fn fresh(proposal: &ReadProposal, policy: &Policy, subject: &Subject, current: &impl CurrentResources, now: u64) -> Result<()> {
    subject.validate()?;
    if &proposal.intent.subject != subject { return Err(ErrorCode::PermissionDenied); }
    if policy.boot_id != subject.boot_id || policy.incarnation != proposal.incarnation { return Err(ErrorCode::ApprovalExpired); }
    if policy.revision != proposal.revision { return Err(ErrorCode::PolicyChanged); }
    if now < proposal.issued_ms || now >= proposal.expires_ms || proposal.nonce.is_nil() { return Err(ErrorCode::ApprovalExpired); }
    for r in &proposal.scope.resources {
        if current.resolve(&r.field, &r.kind, &r.handle)? != r.identity_sha256 { return Err(ErrorCode::TargetChanged); }
    }
    let after=boottime_ms()?;
    if after < proposal.issued_ms || after >= proposal.expires_ms {return Err(ErrorCode::ApprovalExpired);}
    Ok(())
}
impl Policy {
    /// First native path deliberately authorizes only snapshots of explicitly
    /// selected windows. Derived snapshot/node lineage is a separate provider
    /// contract; ui.find and mutations cannot borrow this authority.
    pub fn propose_graphical_read(&self, intent: Intent, desktop: NativeDesktop,
        presentation: ReadPresentation, expiry_ms: u64) -> Result<ReadProposal> {
        if !matches!(intent.mode, Mode::Ask | Mode::Diagnose) || intent.subject.uid == 0
            || intent.subject.uid != desktop.uid || intent.subject.boot_id != desktop.boot_id
            || desktop.boot_id != self.boot_id { return Err(ErrorCode::PermissionDenied); }
        if !session_id(&desktop.session_id) || !hash(&desktop.identity_sha256)
            || !desktop.socket_name.starts_with("wayland-") || !bounded(&desktop.socket_name)
            || desktop.socket_name.contains('/') || !text(&presentation.target, 256)
            || !text(&presentation.profile, 256) || !text(&presentation.goal, 4096)
            || digest(&presentation.goal)? != intent.goal_sha256
            || presentation.windows.is_empty() || presentation.windows.len() > 16
            || presentation.evidence.len() > 16 || presentation.evidence.iter().any(|s| !text(s,128))
            || expiry_ms == 0 || expiry_ms > MAX_EXPIRY_MS { return Err(ErrorCode::InvalidArgument); }
        let mut scope = Scope { actions: ["ui.snapshot".into()].into(), ..Default::default() };
        scope.resources.insert(Resource { field: "selected_session".into(), kind: "graphical-session".into(),
            handle: desktop.session_id.clone(), identity_sha256: desktop.identity_sha256.clone() });
        for window in &presentation.windows {
            if !bounded(&window.handle) || !hash(&window.identity_sha256) || !text(&window.name,256)
                || !text(&window.window,512) || scope.apps.insert(window.handle.clone(),window.identity_sha256.clone()).is_some() {
                return Err(ErrorCode::InvalidArgument);
            }
            scope.resources.insert(Resource { field: "window_handle".into(), kind: "scope-owner-expiry".into(),
                handle: window.handle.clone(), identity_sha256: window.identity_sha256.clone() });
        }
        scope.validate()?;
        if requirement("ui.snapshot", intent.mode, Risk::R0)? != Requirement::ReadScope { return Err(ErrorCode::PolicyChanged); }
        let issued_ms = boottime_ms()?;
        let expires_ms = issued_ms.checked_add(expiry_ms).ok_or(ErrorCode::InvalidArgument)?;
        let mut proposal = ReadProposal { intent, scope, desktop, presentation, revision: self.revision.clone(),
            incarnation: self.incarnation, nonce: Uuid::new_v4(), issued_ms, expires_ms, digest: String::new() };
        proposal.digest = digest(&(&proposal.intent.subject, &proposal.intent.request_id, &proposal.intent.goal_sha256,
            proposal.intent.mode, &proposal.scope, &proposal.revision, proposal.incarnation, proposal.nonce,
            proposal.issued_ms, proposal.expires_ms, proposal.wire(false)))?;
        Ok(proposal)
    }
    pub fn consume_graphical_read(&self, decision: NativeReadDecision, subject: &Subject,
        current: &impl CurrentResources) -> Result<ReadGrant> {
        if decision.cancelled.load(Ordering::Acquire) {return Err(ErrorCode::Cancelled);}
        let p = decision.proposal;
        fresh(&p, self, subject, current, boottime_ms()?)?;
        if decision.cancelled.load(Ordering::Acquire) {return Err(ErrorCode::Cancelled);}
        let plan_sha256 = digest(&(&p.intent.subject, &p.intent.request_id, &p.intent.goal_sha256,
            p.intent.mode, &p.scope, &p.revision, p.issued_ms, p.expires_ms))?;
        Ok(ReadGrant { subject: p.intent.subject, request_id: p.intent.request_id, mode: p.intent.mode,
            goal_sha256: p.intent.goal_sha256, scope: p.scope, plan_sha256, policy_revision: p.revision,
            incarnation: p.incarnation, nonce: p.nonce, issued_ms: p.issued_ms, expires_ms: p.expires_ms, revoked: decision.cancelled })
    }
}
impl ReadProposal {
    fn wire(&self, include_digest: bool) -> Value {
        serde_json::json!({"schema_version":1,"kind":"read_scope","digest":if include_digest {self.digest.as_str()} else {""},
            "uid":self.desktop.uid,"session_id":self.desktop.session_id,"target":self.presentation.target,
            "profile":self.presentation.profile,"mode":self.intent.mode,"goal":self.presentation.goal,
            "apps":self.presentation.windows,"actions":self.scope.actions,"issued_ms":self.issued_ms,
            "expires_ms":self.expires_ms,"evidence":self.presentation.evidence})
    }
    pub fn confirmation_digest(&self) -> &str { &self.digest }
    pub fn expires_ms(&self) -> u64 { self.expires_ms }
    /// Fresh native identities/resources must be checked before launching.
    /// No environment variable or request chooses the program or platform.
    pub fn launch(self, policy: &Policy, subject: &Subject, current: &impl CurrentResources) -> Result<ReadConfirmation> {
        fresh(&self, policy, subject, current, boottime_ms()?)?;
        if self.desktop.uid != unsafe {libc::geteuid()} {return Err(ErrorCode::PermissionDenied);}
        let launcher = option_env!("AIOS_CONSENT_UI").ok_or(ErrorCode::UnsupportedCapability)?;
        let native = option_env!("AIOS_CONSENT_NATIVE").ok_or(ErrorCode::UnsupportedCapability)?;
        for path in [launcher,native] {
            if !PathBuf::from(path).starts_with("/nix/store") || fs::canonicalize(path).map_err(|_|ErrorCode::TargetChanged)? != PathBuf::from(path) {
                return Err(ErrorCode::PermissionDenied);
            }
        }
        let runtime = PathBuf::from(format!("/run/user/{}",self.desktop.uid));
        let meta = fs::symlink_metadata(&runtime).map_err(|_|ErrorCode::TargetChanged)?;
        if !meta.is_dir() || meta.uid() != self.desktop.uid || meta.mode() & 0o077 != 0
            || fs::canonicalize(&runtime).map_err(|_|ErrorCode::TargetChanged)? != runtime {return Err(ErrorCode::PermissionDenied);}
        let socket = fs::symlink_metadata(runtime.join(&self.desktop.socket_name)).map_err(|_|ErrorCode::TargetChanged)?;
        if !socket.file_type().is_socket() || socket.uid() != self.desktop.uid {return Err(ErrorCode::TargetChanged);}
        let data = canonical_json(&self.wire(true))?;
        if data.is_empty() || data.len() > 65536 {return Err(ErrorCode::ResourceExhausted);}
        let (parent,child) = UnixStream::pair().map_err(|_|ErrorCode::TargetChanged)?;
        parent.set_write_timeout(Some(std::time::Duration::from_millis(250))).map_err(|_|ErrorCode::TargetChanged)?;
        let process = Command::new(launcher).env_clear().env("XDG_RUNTIME_DIR",&runtime)
            .env("WAYLAND_DISPLAY",&self.desktop.socket_name).env("DBUS_SESSION_BUS_ADDRESS",format!("unix:path={}/bus",runtime.display()))
            .env("QT_QPA_PLATFORM","wayland").env("LANG","C.UTF-8")
            // Keep the native permission surface usable by ordinary assistive
            // technology even when no screen reader is active. The AIOS
            // provider separately excludes this pinned executable before any
            // content/action query; accessibility does not grant AIOS input.
            .env("QT_LINUX_ACCESSIBILITY_ALWAYS_ON","1")
            .stdin(Stdio::from(std::os::fd::OwnedFd::from(child))).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map_err(|_|ErrorCode::TargetChanged)?;
        let frame = (data.len() as u32).to_be_bytes();
        let mut pending = ReadConfirmation {proposal:Some(self),stream:parent,
            child:process, received:Vec::new(), eof:false,native_seen:false,finished:false,cancelled:Arc::new(AtomicBool::new(false))};
        pending.stream.write_all(&frame).and_then(|_|pending.stream.write_all(&data)).map_err(|_|ErrorCode::TargetChanged)?;
        pending.stream.set_nonblocking(true).map_err(|_|ErrorCode::TargetChanged)?;
        pending.observe_child()?;
        Ok(pending)
    }
}
impl ReadConfirmation {
    pub fn cancellation(&self)->Result<ReadCancellation>{
        Ok(ReadCancellation{stream:self.stream.try_clone().map_err(|_|ErrorCode::TargetChanged)?,cancelled:self.cancelled.clone()})
    }
    fn observe_child(&mut self) -> Result<()> {
        if let Ok(exe) = fs::read_link(format!("/proc/{}/exe",self.child.id())) {
            if option_env!("AIOS_CONSENT_NATIVE").is_some_and(|p| exe == PathBuf::from(p)) {self.native_seen=true;}
        }
        Ok(())
    }
    /// Returns no decision until the exact pinned child successfully exits and
    /// its bounded canonical response is complete. A result is never a token.
    pub fn poll(&mut self, policy: &Policy, subject: &Subject, current: &impl CurrentResources) -> Result<Option<NativeReadDecision>> {
        let result = self.poll_inner(policy,subject,current);
        if result.is_err() {self.withdraw();}
        result
    }
    fn poll_inner(&mut self, policy: &Policy, subject: &Subject, current: &impl CurrentResources) -> Result<Option<NativeReadDecision>> {
        if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        let p = self.proposal.as_ref().ok_or(ErrorCode::ApprovalExpired)?;
        fresh(p,policy,subject,current,boottime_ms()?)?;
        self.observe_child()?;
        let mut bytes = [0u8;256];
        loop {
            match self.stream.read(&mut bytes) {
                Ok(0) => {self.eof=true;break;},
                Ok(n) => {self.received.extend_from_slice(&bytes[..n]); if self.received.len() > 192 {return Err(ErrorCode::InvalidArgument);}},
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(ErrorCode::TargetChanged),
            }
        }
        let Some(exit) = self.child.try_wait().map_err(|_|ErrorCode::TargetChanged)? else {return Ok(None);};
        self.finished=true;
        if !exit.success() || !self.native_seen {return Err(ErrorCode::TargetChanged);}
        if !self.eof {return Ok(None);}
        let p = self.proposal.take().ok_or(ErrorCode::ApprovalExpired)?;
        fresh(&p,policy,subject,current,boottime_ms()?)?;
        if self.cancelled.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        if !allowed(&self.received,&p.digest)? {return Err(ErrorCode::PermissionDenied);}
        Ok(Some(NativeReadDecision {proposal:p,cancelled:self.cancelled.clone()}))
    }
    pub fn withdraw(&mut self) {
        self.cancelled.store(true,Ordering::Release);self.cleanup();
    }
    fn cleanup(&mut self) {
        self.proposal.take();
        let _ = self.stream.shutdown(Shutdown::Both);
        if !self.finished {let _ = self.child.kill();let _ = self.child.wait();self.finished=true;}
        self.received.fill(0); self.received.clear();
    }
}
impl Drop for ReadConfirmation {fn drop(&mut self) {
    // Successful delivery transfers the cancellation state into the grant.
    // Normal supervisor destruction must not cancel that transferred grant.
    if self.proposal.is_some(){self.cancelled.store(true,Ordering::Release);}
    self.cleanup();
}}
fn allowed(bytes: &[u8], expected: &str) -> Result<bool> {
    // Byte equality is stricter than duplicate-collapsing JSON parsing. Qt
    // emits sorted compact keys with a single newline and no other fields.
    let yes = format!("{{\"decision\":\"allow\",\"digest\":\"{expected}\"}}\n");
    let no = format!("{{\"decision\":\"cancel\",\"digest\":\"{expected}\"}}\n");
    if bytes == yes.as_bytes() {Ok(true)} else if bytes == no.as_bytes() {Ok(false)} else {Err(ErrorCode::InvalidArgument)}
}

#[cfg(test)] mod tests;
