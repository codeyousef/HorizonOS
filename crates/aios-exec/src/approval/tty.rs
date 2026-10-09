//! Broker-owned TTY confirmation for exact final system plans.
//!
//! The caller supplies no decision field. The broker reopens the authenticated
//! caller's foreground terminal, renders bounded immutable plan data, and reads
//! one exact plan-bound phrase. This is separate from native polkit
//! authentication and cannot mint authority without that second check.
use super::{Binding, TrustedConfirmation, boottime_ms};
use crate::{Error, Result, caller::{SystemBus, VerifiedCaller}, canonical};
use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::{IsTerminal, Read, Write},
    os::{fd::AsRawFd, unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt}},
    sync::{Arc, atomic::{AtomicBool, Ordering}},
};

const MAX_PRESENTATION_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 256;
const MAX_INTERACTION_MS: u64 = 120_000;

fn display(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        if byte == b'\n' || (0x20..=0x7e).contains(&byte) {
            output.push(byte);
        } else {
            output.extend_from_slice(format!("\\x{byte:02x}").as_bytes());
        }
    }
    output
}

fn expected(verb: &str, plan_id: &str) -> String {
    format!("{verb} {plan_id}")
}

fn accepted(response: &[u8], phrase: &str) -> bool {
    let Ok(text) = std::str::from_utf8(response) else { return false; };
    text.trim_end_matches(['\r', '\n']) == phrase
}

fn open_terminal(caller: &VerifiedCaller) -> Result<File> {
    let identity = caller.identity();
    let session = identity.session.as_ref().ok_or(Error::AuthRequired)?;
    if identity.uid < 1000 || session.kind != "tty" || session.class != "user"
        || session.state != "active" || !session.active {
        return Err(Error::AuthRequired);
    }
    let terminal = OpenOptions::new().read(true).write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOCTTY | libc::O_NONBLOCK)
        .open(format!("/proc/{}/fd/0", identity.pid)).map_err(|_| Error::AuthRequired)?;
    let metadata = terminal.metadata().map_err(|_| Error::AuthRequired)?;
    if !terminal.is_terminal() || !metadata.file_type().is_char_device()
        || metadata.uid() != identity.uid {
        return Err(Error::AuthRequired);
    }
    let fd = terminal.as_raw_fd();
    let foreground = unsafe { libc::tcgetpgrp(fd) };
    let process_group = unsafe { libc::getpgid(identity.pid as i32) };
    let terminal_session = unsafe { libc::tcgetsid(fd) };
    let process_session = unsafe { libc::getsid(identity.pid as i32) };
    if foreground <= 1 || process_group != foreground || terminal_session <= 1
        || process_session != terminal_session {
        return Err(Error::AuthRequired);
    }
    Ok(terminal)
}

fn read_response(terminal: &mut File, deadline_ms: u64, cancelled: &AtomicBool) -> Result<Vec<u8>> {
    let now = boottime_ms()?;
    if now >= deadline_ms { return Err(Error::Expired); }
    let interaction_deadline = now.checked_add(MAX_INTERACTION_MS)
        .ok_or(Error::Invalid)?.min(deadline_ms);
    let mut pollfd = libc::pollfd { fd: terminal.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    loop {
        if cancelled.load(Ordering::Acquire) { return Err(Error::AuthRequired); }
        let now = boottime_ms()?;
        if now >= deadline_ms { return Err(Error::Expired); }
        if now >= interaction_deadline { return Err(Error::AuthRequired); }
        let timeout = i32::try_from((interaction_deadline - now).min(100))
            .map_err(|_| Error::Invalid)?;
        let result = unsafe { libc::poll(&mut pollfd, 1, timeout) };
        if result == 0 { continue; }
        if result < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted { continue; }
            return Err(Error::AuthRequired);
        }
        if pollfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0
            || pollfd.revents & libc::POLLIN == 0 { return Err(Error::AuthRequired); }
        let mut response = vec![0u8; MAX_RESPONSE_BYTES + 1];
        let count = match terminal.read(&mut response) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(_) => return Err(Error::AuthRequired),
        };
        response.truncate(count);
        return Ok(response);
    }
}

pub(super) fn confirm(bus: &SystemBus, caller: &VerifiedCaller, binding: &Binding,
    presentation: &Value, cancelled: &Arc<AtomicBool>) -> Result<TrustedConfirmation> {
    bus.recheck(caller)?;
    binding.validate_time(boottime_ms()?)?;
    let canonical = canonical(presentation)?;
    if canonical.is_empty() || canonical.len() > MAX_PRESENTATION_BYTES { return Err(Error::Invalid); }
    let mut terminal = open_terminal(caller)?;
    let rendered = display(&canonical);
    let phrase = expected("AUTHORIZE", &binding.plan_id);
    terminal.write_all(b"\nHorizon OS trusted system approval\n")?;
    terminal.write_all(&rendered)?;
    terminal.write_all(b"\nRecovery is limited exactly as shown above. Polkit administrator authentication follows.\n")?;
    terminal.write_all(format!("Type `{phrase}` to continue, or press Enter to cancel:\n> ").as_bytes())?;
    terminal.flush()?;
    let mut response = read_response(&mut terminal, binding.expires_at, cancelled)?;
    let allowed = accepted(&response, &phrase);
    response.fill(0);
    if cancelled.load(Ordering::Acquire) { return Err(Error::AuthRequired); }
    bus.recheck(caller)?;
    binding.validate_time(boottime_ms()?)?;
    if !allowed { return Err(Error::AuthRequired); }
    Ok(TrustedConfirmation { binding_sha256: binding.confirmation_digest()? })
}

pub(super) fn confirm_resource(
    bus: &SystemBus,
    caller: &VerifiedCaller,
    plan_id: &str,
    expires_at: u64,
    presentation: &Value,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    bus.recheck(caller)?;
    if boottime_ms()? >= expires_at { return Err(Error::Expired); }
    let canonical = canonical(presentation)?;
    if canonical.is_empty() || canonical.len() > MAX_PRESENTATION_BYTES {
        return Err(Error::Invalid);
    }
    let mut terminal = open_terminal(caller)?;
    let rendered = display(&canonical);
    let phrase = expected("BUILD", plan_id);
    terminal.write_all(b"\nHorizon OS candidate build approval\n")?;
    terminal.write_all(&rendered)?;
    terminal.write_all(b"\nThis permits only the resource limits and cache shown above; it does not permit activation.\n")?;
    terminal.write_all(format!("Type `{phrase}` to build, or press Enter to cancel:\n> ").as_bytes())?;
    terminal.flush()?;
    let mut response = read_response(&mut terminal, expires_at, cancelled)?;
    let allowed = accepted(&response, &phrase);
    response.fill(0);
    if cancelled.load(Ordering::Acquire) { return Err(Error::AuthRequired); }
    bus.recheck(caller)?;
    if boottime_ms()? >= expires_at { return Err(Error::Expired); }
    if !allowed { return Err(Error::AuthRequired); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::tests::binding;

    #[test]
    fn response_is_exact_and_plan_bound() {
        let binding = binding();
        let phrase = expected("AUTHORIZE", &binding.plan_id);
        assert!(accepted(format!("{phrase}\n").as_bytes(), &phrase));
        for response in ["", "allow\n", "approved=true\n", "AUTHORIZE other\n"] {
            assert!(!accepted(response.as_bytes(), &phrase));
        }
        assert!(accepted(b"BUILD exact-plan\n", "BUILD exact-plan"));
    }

    #[test]
    fn rendering_escapes_terminal_controls_and_non_ascii_bytes() {
        assert_eq!(display(b"safe\n\x1b[31m\xe2\x80\xae"), b"safe\n\\x1b[31m\\xe2\\x80\\xae");
    }

}
