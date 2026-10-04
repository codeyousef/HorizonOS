//! Fixed, prompt-free kernel denial checks in the installed inference process.
//! No caller supplies a path, address, command, or requested check.
use nix::libc;
use serde::Serialize;
use std::{fs::OpenOptions, io, os::unix::fs::OpenOptionsExt, path::Path};

#[derive(Clone, Serialize)]
pub(crate) struct Denial {
    boundary: &'static str,
    errno: i32,
}

#[derive(Clone, Serialize)]
pub(crate) struct Proof {
    schema_version: u32,
    evidence_kind: &'static str,
    checks: Vec<Denial>,
}

fn denied(boundary: &'static str, errno: i32, allowed: &[i32]) -> io::Result<Denial> {
    if allowed.contains(&errno) {
        Ok(Denial { boundary, errno })
    } else {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "model isolation check failed"))
    }
}

fn execution() -> io::Result<Vec<Denial>> {
    // A nonexistent executable avoids replacing this process even if a filter
    // regresses. EPERM proves the filter ran; ENOENT is explicitly not a pass.
    let file = c"/inference-must-never-execute";
    let argv = [file.as_ptr(), std::ptr::null()];
    let env = [std::ptr::null()];
    let exec = unsafe { libc::execve(file.as_ptr(), argv.as_ptr(), env.as_ptr()) };
    if exec != -1 { return Err(io::Error::other("unexpected execution result")); }
    let first = denied("execve", io::Error::last_os_error().raw_os_error().unwrap_or(0), &[libc::EPERM])?;
    let exec = unsafe { libc::syscall(libc::SYS_execveat, libc::AT_FDCWD, file.as_ptr(), argv.as_ptr(), env.as_ptr(), 0) };
    if exec != -1 { return Err(io::Error::other("unexpected execution result")); }
    let second = denied("execveat", io::Error::last_os_error().raw_os_error().unwrap_or(0), &[libc::EPERM])?;
    Ok(vec![first, second])
}

fn network(boundary: &'static str, family: i32) -> io::Result<Denial> {
    let descriptor = unsafe { libc::socket(family, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if descriptor >= 0 {
        unsafe { libc::close(descriptor); }
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "model network socket was permitted"));
    }
    denied(boundary, io::Error::last_os_error().raw_os_error().unwrap_or(0), &[libc::EAFNOSUPPORT, libc::EPERM, libc::EACCES])
}

fn directory(boundary: &'static str, path: &str) -> io::Result<Denial> {
    match OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW).open(path) {
        // Opening succeeds without reading any entries; close and fail startup.
        Ok(file) => { drop(file); Err(io::Error::new(io::ErrorKind::PermissionDenied, "model protected directory was accessible")) }
        Err(error) => denied(boundary, error.raw_os_error().unwrap_or(0), &[libc::EACCES, libc::EPERM]),
    }
}

fn immutable(boundary: &'static str, path: &Path) -> io::Result<Denial> {
    // No create, truncate or write occurs, even if the access plan regresses.
    match OpenOptions::new().write(true).custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW).open(path) {
        Ok(file) => { drop(file); Err(io::Error::new(io::ErrorKind::PermissionDenied, "model immutable data was writable")) }
        Err(error) => denied(boundary, error.raw_os_error().unwrap_or(0), &[libc::EROFS, libc::EACCES, libc::EPERM]),
    }
}

pub(crate) fn installed(model: &Path) -> io::Result<Proof> {
    let mut checks = execution()?;
    checks.push(network("ipv4_socket", libc::AF_INET)?);
    checks.push(network("ipv6_socket", libc::AF_INET6)?);
    for (boundary, path) in [
        ("home", "/home"), ("root_home", "/root"), ("user_runtime_keyring", "/run/user"),
        ("system_logs", "/var/log"), ("runtime_logs", "/run/log"),
        ("system_bus", "/run/dbus"), ("nix_daemon", "/nix/var/nix/daemon-socket"),
    ] {
        checks.push(directory(boundary, path)?);
    }
    checks.push(immutable("model_manifest_write", &model.join("lock.json"))?);
    let runtime = Path::new("/etc/aios/model-runtime.json").canonicalize()?;
    checks.push(immutable("runtime_configuration_write", &runtime)?);
    Ok(Proof { schema_version: 1, evidence_kind: "actual-installed-kernel-denials", checks })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_paths_are_not_isolation_evidence() {
        assert!(denied("fixture", libc::ENOENT, &[libc::EACCES, libc::EPERM]).is_err());
        assert!(directory("fixture", "/this-is-not-isolation-evidence").is_err());
        let readable = directory("fixture", "/nix/store");
        assert!(readable.is_err());
    }
    #[test]
    fn production_execution_probe_uses_the_real_filter() {
        std::thread::spawn(|| {
            crate::service::restrict_execution().unwrap();
            assert_eq!(execution().unwrap().len(), 2);
        }).join().unwrap();
    }
}
