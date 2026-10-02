//! Fixed, read-only native systemd operations. No caller-selected D-Bus method.
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zbus::{blocking::{Connection, Proxy}, zvariant::OwnedObjectPath};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceJob { pub id: u32, pub object_path: String }

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceStatus {
    pub service_id: String,
    pub unit_name: String,
    pub scope: String,
    pub manager_owner: String,
    pub manager_version: String,
    pub boot_id: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub result: String,
    pub main_pid: u32,
    pub restart_count: u32,
    pub exec_main_code: i32,
    pub exec_main_status: i32,
    pub job: Option<ServiceJob>,
}

pub fn validate_service_name(name: &str) -> Result<(), ErrorCode> {
    if name.len() > 255 || !name.ends_with(".service") || name.len() <= 8 || name.starts_with('-') ||
        !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.:@-".contains(&b)) {
        return Err(ErrorCode::InvalidArgument);
    }
    Ok(())
}

pub(crate) fn dbus_error(error: zbus::Error) -> ErrorCode {
    match error {
        zbus::Error::InputOutput(e) if e.kind() == std::io::ErrorKind::TimedOut => ErrorCode::DeadlineExceeded,
        zbus::Error::MethodError(name, _, _) => match name.as_str() {
            "org.freedesktop.DBus.Error.AccessDenied" | "org.freedesktop.DBus.Error.AuthFailed" => ErrorCode::PermissionDenied,
            "org.freedesktop.systemd1.NoSuchUnit" | "org.freedesktop.login1.NoSessionForPID" => ErrorCode::TargetNotFound,
            "org.freedesktop.DBus.Error.NameHasNoOwner" | "org.freedesktop.DBus.Error.ServiceUnknown" => ErrorCode::UnsupportedCapability,
            _ => ErrorCode::PartialResult,
        },
        _ => ErrorCode::PartialResult,
    }
}

pub(crate) fn system_connection() -> Result<Connection, ErrorCode> {
    // Bound to the standard system bus, not DBUS_SYSTEM_BUS_ADDRESS supplied by a client.
    zbus::blocking::connection::Builder::address("unix:path=/run/dbus/system_bus_socket").map_err(dbus_error)?
        .method_timeout(Duration::from_millis(250)).build().map_err(dbus_error)
}

pub(crate) fn root_owner(connection: &Connection, service: &str) -> Result<String, ErrorCode> {
    let bus = Proxy::new(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").map_err(dbus_error)?;
    let owner: String = bus.call("GetNameOwner", &(service,)).map_err(dbus_error)?;
    let uid: u32 = bus.call("GetConnectionUnixUser", &(owner.as_str(),)).map_err(dbus_error)?;
    if uid != 0 || !owner.starts_with(':') { return Err(ErrorCode::PermissionDenied); }
    Ok(owner)
}

fn fresh_proxy<'a>(connection: &Connection, owner: &'a str, path: &'a str, interface: &'a str) -> Result<Proxy<'a>, ErrorCode> {
    zbus::blocking::proxy::Builder::new(connection).destination(owner).map_err(dbus_error)?
        .path(path).map_err(dbus_error)?.interface(interface).map_err(dbus_error)?
        .cache_properties(zbus::proxy::CacheProperties::No).build().map_err(dbus_error)
}

fn small(value: String) -> Result<String, ErrorCode> {
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)) {
        return Err(ErrorCode::PartialResult);
    }
    Ok(value)
}

pub fn read_service_status(name: &str, service_id: &str) -> Result<ServiceStatus, ErrorCode> {
    let started = Instant::now();
    macro_rules! property {
        ($proxy:expr, $name:expr) => {{
            if started.elapsed() >= Duration::from_millis(4500) { return Err(ErrorCode::DeadlineExceeded); }
            $proxy.get_property($name).map_err(dbus_error)?
        }};
    }
    validate_service_name(name)?;
    let connection = system_connection()?;
    let owner = root_owner(&connection, "org.freedesktop.systemd1")?;
    let manager = fresh_proxy(&connection, &owner, "/org/freedesktop/systemd1", "org.freedesktop.systemd1.Manager")?;
    let manager_version: String = property!(manager, "Version");
    if manager_version.is_empty() || manager_version.len() > 256 || manager_version.chars().any(char::is_control) { return Err(ErrorCode::PartialResult); }
    // GetUnit observes only existing loaded units. It never loads or starts a unit.
    let path: OwnedObjectPath = manager.call("GetUnit", &(name,)).map_err(dbus_error)?;
    let unit = fresh_proxy(&connection, &owner, path.as_str(), "org.freedesktop.systemd1.Unit")?;
    let service = fresh_proxy(&connection, &owner, path.as_str(), "org.freedesktop.systemd1.Service")?;
    let id: String = property!(unit, "Id");
    if id != name { return Err(ErrorCode::TargetChanged); }
    let load_state = small(property!(unit, "LoadState"))?;
    let before = (small(property!(unit, "ActiveState"))?, small(property!(unit, "SubState"))?);
    let invocation: Vec<u8> = property!(unit, "InvocationID");
    if invocation.len() != 16 { return Err(ErrorCode::PartialResult); }
    let (job_id, job_path): (u32, OwnedObjectPath) = property!(unit, "Job");
    if (job_id == 0 && job_path.as_str() != "/") ||
        (job_id != 0 && job_path.as_str() != format!("/org/freedesktop/systemd1/job/{job_id}")) {
        return Err(ErrorCode::PartialResult);
    }
    let result = small(property!(service, "Result"))?;
    let main_pid: u32 = property!(service, "MainPID");
    let restart_count = property!(service, "NRestarts");
    let exec_main_code = property!(service, "ExecMainCode");
    let exec_main_status = property!(service, "ExecMainStatus");
    let after = (small(property!(unit, "ActiveState"))?, small(property!(unit, "SubState"))?);
    let final_invocation: Vec<u8> = property!(unit, "InvocationID");
    let final_main_pid: u32 = property!(service, "MainPID");
    if before != after || invocation != final_invocation || main_pid != final_main_pid || root_owner(&connection, "org.freedesktop.systemd1")? != owner {
        return Err(ErrorCode::StaleEvidence);
    }
    let boot_id = crate::boot_id(&crate::bounded(std::path::Path::new("/proc/sys/kernel/random/boot_id"), 128)?)?;
    if started.elapsed() >= Duration::from_secs(5) { return Err(ErrorCode::DeadlineExceeded); }
    Ok(ServiceStatus { service_id: service_id.to_owned(), unit_name: id, scope: "system".into(), manager_owner: owner.clone(), manager_version,
        boot_id, load_state, active_state: before.0, sub_state: before.1, result, main_pid, restart_count,
        exec_main_code, exec_main_status, job: if job_id == 0 { None } else { Some(ServiceJob { id: job_id, object_path: job_path.to_string() }) } })
}

pub fn service_result(name: &str, service_id: &str) -> ProviderResult<ServiceStatus> {
    let observed_at = OffsetDateTime::now_utc().format(&Rfc3339).expect("valid UTC timestamp");
    let (status, complete, data, error) = match read_service_status(name, service_id) {
        Ok(value) => (ResultStatus::Ok, true, Some(value), None),
        Err(code) => (ResultStatus::Error, false, None, Some(ProviderError { code, message: "Service observation unavailable; no healthy state inferred".into(), retryable: false })),
    };
    let provider_version = data.as_ref().map(|v| v.manager_version.clone()).unwrap_or_else(|| "unavailable".into());
    ProviderResult { schema_version: 1, status, observed_at, source: Source { provider: "systemd".into(), provider_version },
        evidence_ids: Vec::new(), complete, next_cursor: None, data, error }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_names_cannot_be_paths_commands_or_options() {
        for name in ["/etc/ssh/sshd.service", "--help.service", "sshd.service;id", "sshd.service\n", "sshd", "a/../sshd.service"] {
            assert_eq!(validate_service_name(name), Err(ErrorCode::InvalidArgument));
        }
        assert!(validate_service_name("sshd.service").is_ok());
        assert!(validate_service_name("user@1000.service").is_ok());
    }
}
