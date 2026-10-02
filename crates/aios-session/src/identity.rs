//! Originating client identity comes from kernel credentials and live logind.
use aios_protocol::contracts::ErrorCode;
use nix::{sys::socket::{getsockopt, sockopt::PeerCredentials}, unistd::geteuid};
use serde::Serialize;
use std::{fs, os::{unix::{fs::MetadataExt, net::UnixStream}}, time::Duration};
use zbus::{blocking::Proxy, zvariant::OwnedObjectPath};

fn lookup_error(error: zbus::Error) -> ErrorCode {
    match error {
        zbus::Error::MethodError(name, _, _) => match name.as_str() {
            "org.freedesktop.login1.NoSessionForPID" => ErrorCode::TargetNotFound,
            "org.freedesktop.DBus.Error.NameHasNoOwner" | "org.freedesktop.DBus.Error.ServiceUnknown" => ErrorCode::UnsupportedCapability,
            "org.freedesktop.DBus.Error.AccessDenied" | "org.freedesktop.DBus.Error.AuthFailed" => ErrorCode::PermissionDenied,
            _ => ErrorCode::PartialResult,
        },
        zbus::Error::InputOutput(error) if error.kind() == std::io::ErrorKind::TimedOut => ErrorCode::DeadlineExceeded,
        _ => ErrorCode::PartialResult,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Peer {
    pub uid: u32, pub pid: u32, pub start_ticks: u64, pub boot_id: String,
    pub logind_session: Option<String>, pub remote: bool, pub session_type: Option<String>,
    pub ui_enabled: bool,
    pub bus_sender: Option<String>, pub bus_id: Option<String>,
}

fn process(pid: u32, uid: u32) -> Result<(u64, String), ErrorCode> {
    let path = format!("/proc/{pid}");
    if fs::metadata(&path).map_err(|_| ErrorCode::TargetChanged)?.uid() != uid { return Err(ErrorCode::PermissionDenied); }
    let value = fs::read_to_string(format!("{path}/stat")).map_err(|_| ErrorCode::TargetChanged)?;
    let fields = value.rsplit_once(')').ok_or(ErrorCode::TargetChanged)?.1.split_whitespace().collect::<Vec<_>>();
    let ticks = fields.get(19).ok_or(ErrorCode::TargetChanged)?.parse().map_err(|_| ErrorCode::TargetChanged)?;
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|_| ErrorCode::TargetChanged)?.trim().to_owned();
    Ok((ticks, boot))
}

pub fn authenticate(stream: &UnixStream) -> Result<Peer, ErrorCode> {
    let credentials = getsockopt(stream, PeerCredentials).map_err(|_| ErrorCode::PermissionDenied)?;
    let uid = credentials.uid(); let pid = u32::try_from(credentials.pid()).map_err(|_| ErrorCode::PermissionDenied)?;
    authenticate_process(uid, pid)
}

pub(crate) fn authenticate_process(uid: u32, pid: u32) -> Result<Peer, ErrorCode> {
    if uid != geteuid().as_raw() || pid <= 1 { return Err(ErrorCode::PermissionDenied); }
    let (start_ticks, boot_id) = process(pid, uid)?;
    let mut peer = Peer { uid, pid, start_ticks, boot_id, logind_session: None, remote: false, session_type: None, ui_enabled: false, bus_sender: None, bus_id: None };
    // Failure to associate a process never guesses a desktop. All present APIs
    // are headless and read-only; interactive grants require later enrollment.
    match logind_session(pid, uid) {
        Ok(session) => { peer.logind_session = Some(session.0); peer.remote = session.1; peer.session_type = Some(session.2); },
        Err(ErrorCode::TargetNotFound | ErrorCode::UnsupportedCapability) => {},
        Err(error) => return Err(error),
    }
    if process(pid, uid)? != (peer.start_ticks, peer.boot_id.clone()) { return Err(ErrorCode::TargetChanged); }
    Ok(peer)
}

pub fn verify(stream: &UnixStream, original: &Peer) -> Result<(), ErrorCode> {
    let current = authenticate(stream)?;
    if current != *original { return Err(ErrorCode::TargetChanged); }
    Ok(())
}

fn logind_session(pid: u32, uid: u32) -> Result<(String, bool, String), ErrorCode> {
    let conn = zbus::blocking::connection::Builder::address("unix:path=/run/dbus/system_bus_socket").map_err(|_| ErrorCode::UnsupportedCapability)?
        .method_timeout(Duration::from_millis(500)).build().map_err(|_| ErrorCode::UnsupportedCapability)?;
    let bus = Proxy::new(&conn, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").map_err(|_| ErrorCode::UnsupportedCapability)?;
    let owner: String = bus.call("GetNameOwner", &("org.freedesktop.login1",)).map_err(lookup_error)?;
    let manager_uid: u32 = bus.call("GetConnectionUnixUser", &(owner.as_str(),)).map_err(|_| ErrorCode::PermissionDenied)?;
    if manager_uid != 0 { return Err(ErrorCode::PermissionDenied); }
    let manager = Proxy::new(&conn, owner.as_str(), "/org/freedesktop/login1", "org.freedesktop.login1.Manager").map_err(|_| ErrorCode::UnsupportedCapability)?;
    let path: OwnedObjectPath = manager.call("GetSessionByPID", &(pid,)).map_err(lookup_error)?;
    let session = Proxy::new(&conn, owner.as_str(), path.as_str(), "org.freedesktop.login1.Session").map_err(lookup_error)?;
    let (session_uid, _): (u32, OwnedObjectPath) = session.get_property("User").map_err(|_| ErrorCode::PermissionDenied)?;
    if session_uid != uid { return Err(ErrorCode::PermissionDenied); }
    let id: String = session.get_property("Id").map_err(|_| ErrorCode::PartialResult)?;
    let remote = session.get_property("Remote").map_err(|_| ErrorCode::PartialResult)?;
    let kind: String = session.get_property("Type").map_err(|_| ErrorCode::PartialResult)?;
    if id.is_empty() || id.len() > 128 || kind.len() > 32 { return Err(ErrorCode::PartialResult); }
    Ok((id, remote, kind))
}
