//! Native OS observations and fixed process effect adapters. Brokers own
//! authorization; this crate neither issues permission nor invokes inference.
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path, process::{Command, Stdio}, thread, time::{Duration, Instant}};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemInfo {
    pub os_id: String,
    pub os_version: String,
    pub current_closure: String,
    pub generation: Option<u64>,
    pub boot_id: String,
    pub architecture: String,
    pub virtualization: String,
}

fn bounded(path: &Path, max: u64) -> Result<String, ErrorCode> {
    let mut value = String::new();
    File::open(path).map_err(|_| ErrorCode::PermissionDenied)?.take(max + 1).read_to_string(&mut value).map_err(|_| ErrorCode::InvalidArgument)?;
    if value.len() as u64 > max { return Err(ErrorCode::ResourceExhausted); }
    Ok(value)
}

fn os_release(value: &str) -> Result<(String, String), ErrorCode> {
    let mut id = None; let mut version = None;
    for line in value.lines() {
        if let Some((key, raw)) = line.split_once('=') {
            let slot = match key { "ID" => &mut id, "VERSION_ID" => &mut version, _ => continue };
            if slot.is_some() { return Err(ErrorCode::InvalidArgument); }
            let text = raw.trim_matches('"');
            if text.is_empty() || text.len() > 128 || !text.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
                return Err(ErrorCode::InvalidArgument);
            }
            *slot = Some(text.to_owned());
        }
    }
    Ok((id.ok_or(ErrorCode::PartialResult)?, version.ok_or(ErrorCode::PartialResult)?))
}

fn boot_id(value: &str) -> Result<String, ErrorCode> {
    let text = value.trim();
    if text.len() != 36 || !text.bytes().enumerate().all(|(i,b)| {
        if [8,13,18,23].contains(&i) { b == b'-' } else { b.is_ascii_digit() || (b'a'..=b'f').contains(&b) }
    }) { return Err(ErrorCode::InvalidArgument); }
    Ok(text.to_owned())
}

fn generation(target: &Path) -> Option<u64> {
    target.file_name()?.to_str()?.strip_prefix("system-")?.strip_suffix("-link")?.parse().ok()
}

fn virtualization() -> Result<String, ErrorCode> {
    // Upstream supplies this read-only detector; arguments cannot come from a call.
    let mut child = Command::new("/run/current-system/sw/bin/systemd-detect-virt")
        .arg("--vm").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|_| ErrorCode::UnsupportedCapability)?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|_| ErrorCode::PartialResult)? {
            Some(status) => {
                let mut output = String::new();
                child.stdout.take().ok_or(ErrorCode::PartialResult)?.take(129).read_to_string(&mut output).map_err(|_| ErrorCode::PartialResult)?;
                let value = output.trim();
                if output.len() > 128 || value.is_empty() || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                    return Err(ErrorCode::PartialResult);
                }
                if !status.success() && !(status.code() == Some(1) && value == "none") { return Err(ErrorCode::PartialResult); }
                return Ok(value.to_owned());
            },
            None if started.elapsed() >= Duration::from_secs(2) => {
                let _ = child.kill(); let _ = child.wait();
                return Err(ErrorCode::DeadlineExceeded);
            },
            None => thread::sleep(Duration::from_millis(10)),
        }
    }
}

pub fn observe_system_info() -> ProviderResult<SystemInfo> {
    system_info_with(virtualization)
}

/// System-bus brokers can observe systemd directly while retaining an execve
/// denial. No caller-provided virtualization assertion or endpoint is accepted.
pub fn observe_system_info_native() -> ProviderResult<SystemInfo> {
    system_info_with(services::read_virtualization)
}

fn system_info_with(detector: fn() -> Result<String, ErrorCode>) -> ProviderResult<SystemInfo> {
    let observed_at = OffsetDateTime::now_utc().format(&Rfc3339).expect("valid UTC timestamp");
    let observation = (|| {
        let (os_id, os_version) = os_release(&bounded(Path::new("/etc/os-release"), 4096)?)?;
        if os_id != "nixos" { return Err(ErrorCode::UnsupportedCapability); }
        let current = std::fs::canonicalize("/run/current-system").map_err(|_| ErrorCode::TargetNotFound)?;
        let current_closure = current.to_str().filter(|s| current.parent() == Some(Path::new("/nix/store")) && s.len() < 512).ok_or(ErrorCode::TargetChanged)?.to_owned();
        let generation = if std::fs::canonicalize("/nix/var/nix/profiles/system").ok().as_ref() == Some(&current) {
            std::fs::read_link("/nix/var/nix/profiles/system").ok().and_then(|p| generation(&p))
        } else { None };
        Ok(SystemInfo { os_id, os_version, current_closure, generation,
            boot_id: boot_id(&bounded(Path::new("/proc/sys/kernel/random/boot_id"), 128)?)?,
            architecture: std::env::consts::ARCH.to_owned(), virtualization: detector()? })
    })();
    let (status, complete, data, error) = match observation {
        Ok(info) if info.generation.is_some() => (ResultStatus::Ok, true, Some(info), None),
        Ok(info) => (ResultStatus::Partial, false, Some(info), Some(ProviderError { code: ErrorCode::PartialResult, message: "System profile generation could not be resolved; running closure was observed separately".into(), retryable: true })),
        Err(code) => (ResultStatus::Error, false, None, Some(ProviderError { code, message: "Required system observation unavailable; no healthy state inferred".into(), retryable: true })),
    };
    ProviderResult { schema_version: 1, status, observed_at,
        source: Source { provider: "aios-system".into(), provider_version: env!("CARGO_PKG_VERSION").into() },
        // Persistent, caller-scoped evidence handles require the state service.
        evidence_ids: Vec::new(), complete, next_cursor: None, data, error }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn os_metadata_duplicate_or_malformed_is_not_healthy() {
        assert_eq!(os_release("ID=nixos\nVERSION_ID=\"26.05\"\n").unwrap(), ("nixos".into(), "26.05".into()));
        for raw in ["ID=nixos\nID=other\nVERSION_ID=26.05", "ID=nixos", "ID=$(id)\nVERSION_ID=26"] { assert!(os_release(raw).is_err()); }
    }
    #[test]
    fn boot_and_generation_are_actual_identifiers() {
        assert!(boot_id("68107942-3f74-4194-bcb3-3b3dd6ab16f0\n").is_ok());
        assert!(boot_id("current").is_err());
        assert_eq!(generation(Path::new("system-12-link")), Some(12));
        assert_eq!(generation(Path::new("system-link")), None);
    }
}

pub mod services;
pub mod journal;
pub mod processes;

pub mod devices;
