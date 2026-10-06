use super::*;
use std::{io::{BufRead,BufReader,Write}, process::{Child,ChildStdout,Command,Stdio},
    sync::atomic::{AtomicBool,Ordering},thread,time::{Duration,Instant}};
static TERMINATED:AtomicBool=AtomicBool::new(false);
extern "C" fn terminated(_:libc::c_int){TERMINATED.store(true,Ordering::Relaxed);}

#[test]
#[ignore="fixed native child for pidfd SIGTERM verification"]
fn native_termination_child() {
    let mode=std::env::var("AIOS_TERMINATION_CHILD").unwrap();
    assert!(matches!(mode.as_str(),"graceful"|"ignore"|"exec"));
    unsafe {libc::signal(libc::SIGTERM,if mode=="ignore"{libc::SIG_IGN}else{terminated as *const () as libc::sighandler_t});}
    println!("AIOS_TERMINATION_READY");std::io::stdout().flush().unwrap();
    if mode=="exec" {
        use std::os::unix::process::CommandExt;
        let mut text=String::new();std::io::stdin().read_line(&mut text).unwrap();assert_eq!(text,"exec\n");
        panic!("fixed exec failed: {}",Command::new("/run/current-system/sw/bin/sleep").arg("1").exec());
    }
    let end=Instant::now()+Duration::from_millis(if mode=="ignore"{700}else{1500});
    while Instant::now()<end && !TERMINATED.load(Ordering::Relaxed){thread::sleep(Duration::from_millis(5));}
    if TERMINATED.load(Ordering::Relaxed){println!("AIOS_TERMINATION_ACK");}
}
struct Target {child:Child,output:BufReader<ChildStdout>}
impl Target {
    fn new(mode:&str)->Self {
        let mut child=Command::new(std::env::current_exe().unwrap())
            .args(["--ignored","--exact","processes::termination::tests::native_termination_child","--nocapture"])
            .env("AIOS_TERMINATION_CHILD",mode).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let mut output=BufReader::new(child.stdout.take().unwrap());let mut text=String::new();
        loop {text.clear();assert!(output.read_line(&mut text).unwrap()>0);if text.trim()=="AIOS_TERMINATION_READY"{break;}}
        Self{child,output}
    }
    fn finish(&mut self)->String {
        use std::io::Read;
        let mut text=String::new();self.output.read_to_string(&mut text).unwrap();
        assert!(self.child.wait().unwrap().success());text
    }
}
impl Drop for Target {fn drop(&mut self){
    // Every fixed fixture exits within 1.5s; no forced signal in cleanup.
    let _=self.child.wait();
}}
fn wait(attempt:&mut Attempt)->Receipt {
    let end=Instant::now()+Duration::from_secs(2);
    loop {if let Some(value)=attempt.poll(){return value;}
        assert!(Instant::now()<end);thread::sleep(Duration::from_millis(5));}
}

#[test]
fn native_sigterm_ack_and_retained_pidfd_exit_verified_without_replay() {
    let mut child=Target::new("graceful");let native=OwnProcess::open(child.child.id()).unwrap();
    let prepared=native.prepare_termination(1000).unwrap();let preview=prepared.preview().clone();
    assert_eq!(preview.identity,native.inspect().unwrap().identity);assert_eq!(preview.signal,"SIGTERM");
    assert!(!preview.reversible && !preview.automatic_escalation);
    let mut checks=0;
    // Synthetic broker authorization fixture only; no native approval claim.
    let mut attempt=prepared.execute(|p|{assert_eq!(p,&preview);checks+=1;Ok(())}).unwrap();assert_eq!(checks,2);
    let receipt=wait(&mut attempt);assert!(receipt.signal_requested && receipt.verified_exit && receipt.complete);assert_eq!(receipt.error,None);
    assert_eq!(attempt.poll(),Some(receipt));assert!(native.exited().unwrap());
    assert!(child.finish().contains("AIOS_TERMINATION_ACK"));
}
#[test]
fn native_ignoring_sigterm_times_out_truthfully_and_never_escalates() {
    let mut child=Target::new("ignore");let native=OwnProcess::open(child.child.id()).unwrap();
    let mut attempt=native.prepare_termination(80).unwrap().execute(|_|Ok(())).unwrap();
    let receipt=wait(&mut attempt);assert!(receipt.signal_requested);assert!(!receipt.verified_exit && !receipt.complete);
    assert_eq!(receipt.error,Some(ErrorCode::DeadlineExceeded));assert!(!native.exited().unwrap());
    assert_eq!(attempt.poll(),Some(receipt));drop(attempt);assert!(!native.exited().unwrap());
    assert!(!child.finish().contains("AIOS_TERMINATION_ACK"));assert!(native.exited().unwrap());
}
#[test]
fn native_authority_denial_revocation_and_second_check_have_no_signal_effect() {
    let mut child=Target::new("graceful");let native=OwnProcess::open(child.child.id()).unwrap();
    assert!(matches!(native.prepare_termination(100).unwrap().execute(|_|Err(ErrorCode::AuthRequired)),Err(ErrorCode::AuthRequired)));
    let prepared=native.prepare_termination(100).unwrap();prepared.cancellation().cancel();
    assert!(matches!(prepared.execute(|_|panic!("cancelled plan cannot request approval")),Err(ErrorCode::Cancelled)));
    let prepared=native.prepare_termination(100).unwrap();let cancellation=prepared.cancellation();
    assert!(matches!(prepared.execute(|_|{cancellation.cancel();Ok(())}),Err(ErrorCode::Cancelled)));
    let mut checks=0;
    assert!(matches!(native.prepare_termination(100).unwrap().execute(|_|{checks+=1;if checks==2{Err(ErrorCode::ApprovalExpired)}else{Ok(())}}),Err(ErrorCode::ApprovalExpired)));
    assert!(!native.exited().unwrap());assert!(!child.finish().contains("AIOS_TERMINATION_ACK"));
}
#[test]
fn native_exec_drift_after_authority_check_refused_before_signal() {
    let mut child=Target::new("exec");let native=OwnProcess::open(child.child.id()).unwrap();
    let initial=native.inspect().unwrap().identity;let prepared=native.prepare_termination(100).unwrap();
    let result=prepared.execute(|_|{
        child.child.stdin.as_mut().unwrap().write_all(b"exec\n").unwrap();
        let end=Instant::now()+Duration::from_secs(1);
        while native.inspect().is_ok(){assert!(Instant::now()<end);thread::sleep(Duration::from_millis(1));}Ok(())
    });
    assert!(matches!(result,Err(ErrorCode::TargetChanged)));
    assert!(!native.exited().unwrap());
    assert_eq!(std::fs::read_to_string(format!("/proc/{}/stat",initial.pid)).unwrap().rsplit(')').next().unwrap().split_whitespace().nth(19).unwrap().parse::<u64>().unwrap(),initial.start_time_ticks);
    child.finish();
}
#[test]
fn native_self_invalid_budget_exit_and_post_signal_stop_are_explicit() {
    let own=OwnProcess::open(std::process::id()).unwrap();assert!(matches!(own.prepare_termination(100),Err(ErrorCode::PermissionDenied)));
    let mut child=Target::new("ignore");let native=OwnProcess::open(child.child.id()).unwrap();
    for budget in [0,30_001]{assert!(matches!(native.prepare_termination(budget),Err(ErrorCode::InvalidArgument)));}
    let prepared=native.prepare_termination(1000).unwrap();let cancellation=prepared.cancellation();
    let mut attempt=prepared.execute(|_|Ok(())).unwrap();cancellation.cancel();let receipt=wait(&mut attempt);
    assert_eq!(receipt.error,Some(ErrorCode::Cancelled));assert!(receipt.signal_requested && !receipt.complete && !receipt.verified_exit);
    assert!(!native.exited().unwrap());child.finish();
    assert!(matches!(native.prepare_termination(100),Err(ErrorCode::TargetNotFound)));
}

#[test]
fn native_late_poll_does_not_claim_verification_within_budget() {
    let mut child=Target::new("graceful");let native=OwnProcess::open(child.child.id()).unwrap();
    let mut attempt=native.prepare_termination(20).unwrap().execute(|_|Ok(())).unwrap();
    assert!(child.finish().contains("AIOS_TERMINATION_ACK"));thread::sleep(Duration::from_millis(30));
    let receipt=attempt.poll().unwrap();assert!(receipt.signal_requested && receipt.verified_exit);
    assert!(!receipt.complete);assert_eq!(receipt.error,Some(ErrorCode::DeadlineExceeded));
}
