//! Fixed NetworkManager observations. Caller input can select only an opaque
//! interface handle; it cannot supply a bus name, object path or endpoint.
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedObjectPath,
};

const NAME: &str = "org.freedesktop.NetworkManager";
const ROOT: &str = "/org/freedesktop/NetworkManager";
const MANAGER: &str = "org.freedesktop.NetworkManager";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";

#[derive(Debug, Serialize)]
pub struct NetworkInterface {
    pub interface_id: String,
    pub name: String,
    pub link_state: String,
    pub connectivity: String,
    pub dns_state: String,
}
#[derive(Debug, Serialize)]
pub struct NetworkData {
    pub interfaces: Vec<NetworkInterface>,
    pub endpoint_reachability: String,
    pub wifi_supported: bool,
    pub wifi_enabled: Option<bool>,
    pub management_transport_protected: bool,
    pub unsupported_fields: Vec<String>,
}

fn handle(scope: &[u8], path: &str, name: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"aios-network-interface-v1\0");
    h.update(scope);
    h.update([0]);
    h.update(path.as_bytes());
    h.update([0]);
    h.update(name.as_bytes());
    format!("net-{:x}", h.finalize())
}
fn owner(c: &Connection) -> Result<String, ErrorCode> {
    Proxy::new(
        c,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .map_err(|_| ErrorCode::UnsupportedCapability)?
    .call("GetNameOwner", &(NAME,))
    .map_err(|_| ErrorCode::UnsupportedCapability)
}
fn connectivity(value: u32) -> Result<(&'static str, &'static str), ErrorCode> {
    match value {
        0 => Ok(("unknown", "unknown")),
        1 => Ok(("none", "unreachable")),
        2 | 3 => Ok(("limited", "unreachable")),
        4 => Ok(("full", "reachable")),
        _ => Err(ErrorCode::TargetChanged),
    }
}
fn link(value: u32) -> Result<&'static str, ErrorCode> {
    match value {
        0 => Ok("unknown"),
        10 | 20 | 30 => Ok("down"),
        40..=90 => Ok("unknown"),
        100 => Ok("up"),
        110 => Ok("unknown"),
        120 => Ok("down"),
        _ => Err(ErrorCode::TargetChanged),
    }
}
fn dns(c: &Connection, path: &OwnedObjectPath, up: bool) -> (&'static str, bool) {
    if path.as_str() == "/" {
        return (if up { "failed" } else { "unknown" }, false);
    }
    let Ok(p) = Proxy::new(
        c,
        NAME,
        path.as_str(),
        "org.freedesktop.NetworkManager.IP4Config",
    ) else {
        return ("unknown", false);
    };
    match p.get_property::<Vec<u32>>("Nameservers") {
        Ok(v) if !v.is_empty() => ("available", true),
        Ok(_) => ("failed", true),
        Err(_) => ("unknown", false),
    }
}
fn collect(scope: &[u8], selected: Option<&str>) -> Result<NetworkData, ErrorCode> {
    let c = Connection::system().map_err(|_| ErrorCode::UnsupportedCapability)?;
    let initial = owner(&c)?;
    let manager = Proxy::new(&c, initial.as_str(), ROOT, MANAGER)
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    let state: u32 = manager
        .get_property("State")
        .map_err(|_| ErrorCode::PartialResult)?;
    if state > 70 {
        return Err(ErrorCode::TargetChanged);
    }
    let raw_connectivity: u32 = manager
        .get_property("Connectivity")
        .map_err(|_| ErrorCode::PartialResult)?;
    let (global, endpoint) = connectivity(raw_connectivity)?;
    let paths: Vec<OwnedObjectPath> = manager
        .call("GetAllDevices", &())
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    if paths.len() > 100 {
        return Err(ErrorCode::ResourceExhausted);
    }
    let mut interfaces = Vec::new();
    let mut wifi = false;
    let mut unsupported = Vec::new();
    for path in paths {
        let device = Proxy::new(&c, initial.as_str(), path.as_str(), DEVICE)
            .map_err(|_| ErrorCode::PartialResult)?;
        let name: String = device
            .get_property("Interface")
            .map_err(|_| ErrorCode::PartialResult)?;
        if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
            return Err(ErrorCode::TargetChanged);
        }
        let device_type: u32 = device
            .get_property("DeviceType")
            .map_err(|_| ErrorCode::PartialResult)?;
        wifi |= device_type == 2;
        let device_state: u32 = device
            .get_property("State")
            .map_err(|_| ErrorCode::PartialResult)?;
        let link = link(device_state)?;
        let up = link == "up";
        let ip4: OwnedObjectPath = device
            .get_property("Ip4Config")
            .map_err(|_| ErrorCode::PartialResult)?;
        let (dns_state, dns_complete) = dns(&c, &ip4, up);
        let id = handle(scope, path.as_str(), &name);
        if selected.is_none_or(|wanted| wanted == id) {
            if !dns_complete {
                unsupported.push("dns".into());
            }
            interfaces.push(NetworkInterface {
                interface_id: id,
                name,
                link_state: link.into(),
                connectivity: if up { global.into() } else { "none".into() },
                dns_state: dns_state.into(),
            });
        }
    }
    if selected.is_some() && interfaces.is_empty() {
        return Err(ErrorCode::TargetNotFound);
    }
    let wifi_enabled = if wifi {
        Some(
            manager
                .get_property("WirelessEnabled")
                .map_err(|_| ErrorCode::PartialResult)?,
        )
    } else {
        unsupported.push("wifi.radio".into());
        None
    };
    if owner(&c)? != initial {
        return Err(ErrorCode::TargetChanged);
    }
    unsupported.sort();
    unsupported.dedup();
    Ok(NetworkData {
        interfaces,
        endpoint_reachability: endpoint.into(),
        wifi_supported: wifi,
        wifi_enabled,
        management_transport_protected: true,
        unsupported_fields: unsupported,
    })
}
pub fn observe(scope: &[u8], selected: Option<&str>) -> ProviderResult<NetworkData> {
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("valid UTC timestamp");
    let result = collect(scope, selected);
    let (status, complete, data, error) = match result {
        Ok(data) => {
            let complete = data.unsupported_fields.is_empty();
            (
                if complete {
                    ResultStatus::Ok
                } else {
                    ResultStatus::Partial
                },
                complete,
                Some(data),
                (!complete).then(|| ProviderError {
                    code: ErrorCode::PartialResult,
                    message: "Unavailable network domains remain explicit".into(),
                    retryable: true,
                }),
            )
        }
        Err(code) => (
            ResultStatus::Error,
            false,
            Some(NetworkData {
                interfaces: vec![],
                endpoint_reachability: "unknown".into(),
                wifi_supported: false,
                wifi_enabled: None,
                management_transport_protected: true,
                unsupported_fields: vec!["networkmanager".into()],
            }),
            Some(ProviderError {
                code,
                message:
                    "NetworkManager observation is unavailable; no healthy network state inferred"
                        .into(),
                retryable: true,
            }),
        ),
    };
    ProviderResult {
        schema_version: 1,
        status,
        observed_at,
        source: Source {
            provider: "networkmanager-dbus".into(),
            provider_version: env!("CARGO_PKG_VERSION").into(),
        },
        evidence_ids: vec![],
        complete,
        next_cursor: None,
        data,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn states_are_distinct_and_unknown_values_refuse() {
        assert_eq!(connectivity(4).unwrap(), ("full", "reachable"));
        assert_eq!(connectivity(3).unwrap(), ("limited", "unreachable"));
        assert_eq!(link(100).unwrap(), "up");
        assert_eq!(link(20).unwrap(), "down");
        assert_eq!(link(110).unwrap(), "unknown");
        assert_eq!(link(120).unwrap(), "down");
        assert!(connectivity(9).is_err() && link(999).is_err());
    }
    #[test]
    fn handles_bind_caller_and_native_identity() {
        assert_eq!(
            handle(b"a", "/device/1", "eth0"),
            handle(b"a", "/device/1", "eth0")
        );
        assert_ne!(
            handle(b"a", "/device/1", "eth0"),
            handle(b"b", "/device/1", "eth0")
        );
    }
}
