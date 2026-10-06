//! Single native SIGTERM attempt with retained pidfd exit verification.
//!
//! This is an effect adapter, not an approval issuer. The calling broker must
//! bind this exact preview to its original task and native R2 approval, and
//! revalidate that authority at both callbacks. There is no public IPC route
//! to this adapter until that integration is implemented and qualified.
use super::{OwnProcess, Identity, Result, error};
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{os::fd::{AsRawFd, FromRawFd, OwnedFd}, sync::{Arc,atomic::{AtomicBool,Ordering}}};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Preview {
    pub identity: Identity,
    pub signal: &'static str,
    pub verification_timeout_ms: u32,
    pub reversible: bool,
    pub automatic_escalation: bool,
}
/// A stop request only. It cannot create signal authority or a new target.
#[derive(Clone)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation { pub fn cancel(&self) {self.0.store(true,Ordering::Release);} }

/// Opaque retained native target and immutable preview; no Deserialize or
/// constructor from PID/start/UID claims. Preparing sends no signal.
pub struct Prepared { native: OwnProcess, preview: Preview, cancel: Cancellation }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub preview: Preview,
    pub signal_requested: bool,
    pub verified_exit: bool,
    pub complete: bool,
    pub error: Option<ErrorCode>,
}
/// Owns verification after the irreversible kernel effect. Dropping this value
/// never kills the target. A failed/expired verification never retries a signal.
pub struct Attempt { prepared: Prepared, started_ms: u64, terminal: Option<Receipt> }
fn now()->Result<u64> {
    let mut t=libc::timespec{tv_sec:0,tv_nsec:0};
    if unsafe{libc::clock_gettime(libc::CLOCK_BOOTTIME,&mut t)}!=0 || t.tv_sec<0 || !(0..1_000_000_000).contains(&t.tv_nsec){return Err(ErrorCode::TargetChanged);}
    (t.tv_sec as u64).checked_mul(1000).and_then(|x|x.checked_add(t.tv_nsec as u64/1_000_000)).ok_or(ErrorCode::TargetChanged)
}
impl OwnProcess {
    pub fn prepare_termination(&self, verification_timeout_ms:u32)->Result<Prepared> {
        if verification_timeout_ms==0 || verification_timeout_ms>30_000 {return Err(ErrorCode::InvalidArgument);}
        // A provider cannot terminate itself (including its control threads).
        if self.identity.pid==std::process::id(){return Err(ErrorCode::PermissionDenied);}
        let identity=self.inspect()?.identity;
        let directory=self.directory.try_clone().map_err(error)?;
        let fd=unsafe{libc::fcntl(self.pidfd.as_raw_fd(),libc::F_DUPFD_CLOEXEC,3)};
        if fd<0{return Err(error(std::io::Error::last_os_error()));}
        let native=OwnProcess{directory,pidfd:unsafe{OwnedFd::from_raw_fd(fd)},identity:identity.clone()};
        native.inspect()?;
        Ok(Prepared{native,preview:Preview{identity,signal:"SIGTERM",verification_timeout_ms,
            reversible:false,automatic_escalation:false},cancel:Cancellation(Arc::new(AtomicBool::new(false)))})
    }
}
impl Prepared {
    pub fn preview(&self)->&Preview {&self.preview}
    pub fn cancellation(&self)->Cancellation {self.cancel.clone()}
    fn live(&self)->Result<()> {
        if self.cancel.0.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        if self.native.inspect()?.identity!=self.preview.identity{return Err(ErrorCode::TargetChanged);}Ok(())
    }
    /// Trusted native broker callback: check original caller, task grant,
    /// exact approval, policy/boot/closure, resource and expiry. It must not
    /// prompt or infer permission from model output. It is called twice so
    /// target reinspection cannot silently outlive volatile authority.
    /// Tests explicitly use synthetic authority callbacks; those are not
    /// installed approval or consent evidence.
    pub fn execute(self, mut revalidate:impl FnMut(&Preview)->Result<()>)->Result<Attempt> {
        self.live()?;revalidate(&self.preview)?;
        self.live()?;revalidate(&self.preview)?;
        if self.cancel.0.load(Ordering::Acquire){return Err(ErrorCode::Cancelled);}
        let started_ms=now()?;
        // Always signal the retained pidfd. Never fall back to kill(PID), a
        // process group, shell, SIGKILL, or a different delivery modality.
        if unsafe{libc::syscall(libc::SYS_pidfd_send_signal,self.native.pidfd.as_raw_fd(),libc::SIGTERM,
            std::ptr::null::<libc::siginfo_t>(),0u32)}<0{return Err(error(std::io::Error::last_os_error()));}
        Ok(Attempt{prepared:self,started_ms,terminal:None})
    }
}
impl Attempt {
    fn finish(&mut self,verified_exit:bool,error:Option<ErrorCode>)->Receipt {
        let receipt=Receipt{preview:self.prepared.preview.clone(),signal_requested:true,verified_exit,
            complete:verified_exit && error.is_none(),error};self.terminal=Some(receipt.clone());receipt
    }
    /// Polling never holds the caller's task lock or sleeps. Verification uses
    /// the original pidfd, so zombies and PID reuse cannot fake a live target.
    /// After a sent signal, cancellation/timeouts/errors retain truthful
    /// partial-effect state; they are not reported as a rollback/no mutation.
    pub fn poll(&mut self)->Option<Receipt> {
        if let Some(receipt)=&self.terminal{return Some(receipt.clone());}
        let exited=match self.prepared.native.exited() {
            Ok(value)=>value,
            Err(code)=>return Some(self.finish(false,Some(code))),
        };
        let expired=match now(){
            Ok(time) if time>=self.started_ms=>time-self.started_ms>=u64::from(self.prepared.preview.verification_timeout_ms),
            _=>return Some(self.finish(false,Some(ErrorCode::TargetChanged))),
        };
        // A late poll can prove current exit, but cannot prove verification
        // happened within the promised budget. Preserve both facts.
        if expired{return Some(self.finish(exited,Some(ErrorCode::DeadlineExceeded)));}
        if exited{return Some(self.finish(true,None));}
        if self.prepared.cancel.0.load(Ordering::Acquire){return Some(self.finish(false,Some(ErrorCode::Cancelled)));}
        None
    }
}

#[cfg(test)] mod tests;
