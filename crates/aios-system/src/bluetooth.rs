//! Fixed BlueZ and rfkill observations. Hardware addresses and object paths are
//! used only to derive caller-scoped handles and are never returned.
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, path::Path};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedObjectPath, OwnedValue},
};
const NAME: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
type Properties = HashMap<String, OwnedValue>;
type Interfaces = HashMap<String, Properties>;
type Objects = HashMap<OwnedObjectPath, Interfaces>;

#[derive(Debug, Serialize)]
pub struct BluetoothDevice {
    pub device_id: String,
    pub name: String,
    pub connected: bool,
}
#[derive(Debug, Serialize)]
pub struct BluetoothAdapter {
    pub adapter_id: String,
    pub name: String,
    pub powered: bool,
    pub rfkill: String,
    pub devices: Vec<BluetoothDevice>,
}
#[derive(Debug, Serialize)]
pub struct BluetoothData {
    pub adapters: Vec<BluetoothAdapter>,
}
fn property<T: TryFrom<OwnedValue>>(values: &Properties, name: &str) -> Result<T, ErrorCode> {
    values
        .get(name)
        .ok_or(ErrorCode::PartialResult)?
        .try_clone()
        .map_err(|_| ErrorCode::PartialResult)?
        .try_into()
        .map_err(|_| ErrorCode::TargetChanged)
}
fn handle(scope: &[u8], kind: &str, path: &str, identity: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"aios-bluetooth-handle-v1\0");
    h.update(scope);
    h.update([0]);
    h.update(kind.as_bytes());
    h.update([0]);
    h.update(path.as_bytes());
    h.update([0]);
    h.update(identity.as_bytes());
    format!("bt-{:x}", h.finalize())
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
fn bit(path: &Path) -> Result<bool, ErrorCode> {
    match super::bounded(path, 8)?.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(ErrorCode::TargetChanged),
    }
}
fn rfkill() -> Result<HashMap<String, String>, ErrorCode> {
    let Ok(entries) = fs::read_dir("/sys/class/rfkill") else {
        return Ok(HashMap::new());
    };
    let mut states = HashMap::new();
    for (index, entry) in entries.enumerate() {
        if index >= 64 {
            return Err(ErrorCode::ResourceExhausted);
        }
        let path = entry.map_err(|_| ErrorCode::PartialResult)?.path();
        if super::bounded(&path.join("type"), 32)?.trim() != "bluetooth" {
            continue;
        }
        let name = super::bounded(&path.join("name"), 128)?.trim().to_owned();
        if name.is_empty() || name.chars().any(char::is_control) {
            return Err(ErrorCode::TargetChanged);
        }
        let blocked = bit(&path.join("soft"))? || bit(&path.join("hard"))?;
        if states
            .insert(name, if blocked { "blocked" } else { "unblocked" }.into())
            .is_some()
        {
            return Err(ErrorCode::TargetChanged);
        }
    }
    Ok(states)
}
fn collect(scope: &[u8], selected: Option<&str>) -> Result<(BluetoothData, bool), ErrorCode> {
    let c = Connection::system().map_err(|_| ErrorCode::UnsupportedCapability)?;
    let initial = owner(&c)?;
    let manager = Proxy::new(
        &c,
        initial.as_str(),
        "/",
        "org.freedesktop.DBus.ObjectManager",
    )
    .map_err(|_| ErrorCode::UnsupportedCapability)?;
    let objects: Objects = manager
        .call("GetManagedObjects", &())
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    if objects.len() > 512 {
        return Err(ErrorCode::ResourceExhausted);
    }
    let kills = rfkill()?;
    let mut adapters = Vec::new();
    let mut paths = HashMap::new();
    for (path, interfaces) in &objects {
        let Some(values) = interfaces.get(ADAPTER) else {
            continue;
        };
        if adapters.len() >= 32 {
            return Err(ErrorCode::ResourceExhausted);
        }
        let address: String = property(values, "Address")?;
        let name: String = property(values, "Alias").or_else(|_| property(values, "Name"))?;
        if address.len() > 64
            || name.is_empty()
            || name.len() > 256
            || name.chars().any(char::is_control)
        {
            return Err(ErrorCode::TargetChanged);
        }
        let id = handle(scope, "adapter", path.as_str(), &address);
        let kernel_name = path
            .as_str()
            .rsplit('/')
            .next()
            .ok_or(ErrorCode::TargetChanged)?;
        let kill = kills
            .get(kernel_name)
            .cloned()
            .unwrap_or_else(|| "unknown".into());
        paths.insert(path.clone(), (adapters.len(), id.clone()));
        adapters.push(BluetoothAdapter {
            adapter_id: id,
            name,
            powered: property(values, "Powered")?,
            rfkill: kill,
            devices: vec![],
        });
    }
    if adapters.is_empty() {
        return Err(ErrorCode::UnsupportedCapability);
    }
    for (path, interfaces) in &objects {
        let Some(values) = interfaces.get(DEVICE) else {
            continue;
        };
        let adapter: OwnedObjectPath = property(values, "Adapter")?;
        let Some((index, _)) = paths.get(&adapter) else {
            return Err(ErrorCode::TargetChanged);
        };
        if adapters[*index].devices.len() >= 100 {
            return Err(ErrorCode::ResourceExhausted);
        }
        let address: String = property(values, "Address")?;
        let name: String = property(values, "Alias").or_else(|_| property(values, "Name"))?;
        if address.len() > 64
            || name.is_empty()
            || name.len() > 256
            || name.chars().any(char::is_control)
        {
            return Err(ErrorCode::TargetChanged);
        }
        adapters[*index].devices.push(BluetoothDevice {
            device_id: handle(scope, "device", path.as_str(), &address),
            name,
            connected: property(values, "Connected")?,
        });
    }
    if let Some(id) = selected {
        adapters.retain(|adapter| adapter.adapter_id == id);
        if adapters.is_empty() {
            return Err(ErrorCode::TargetNotFound);
        }
    }
    let rfkill_complete = adapters.iter().all(|adapter| adapter.rfkill != "unknown");
    if owner(&c)? != initial {
        return Err(ErrorCode::TargetChanged);
    }
    Ok((BluetoothData { adapters }, rfkill_complete))
}
pub fn observe(scope: &[u8], selected: Option<&str>) -> ProviderResult<BluetoothData> {
    let observed_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("valid UTC timestamp");
    let result = collect(scope, selected);
    let(status,complete,data,error)=match result{Ok((data,complete))=>(if complete{ResultStatus::Ok}else{ResultStatus::Partial},complete,Some(data),(!complete).then(||ProviderError{code:ErrorCode::PartialResult,message:"Bluetooth adapter is available but rfkill state is unavailable".into(),retryable:true})),Err(code)=>(ResultStatus::Error,false,Some(BluetoothData{adapters:vec![]}),Some(ProviderError{code,message:"BlueZ adapter observation is unavailable; an empty adapter list is not reported healthy".into(),retryable:true}))};
    ProviderResult {
        schema_version: 1,
        status,
        observed_at,
        source: Source {
            provider: "bluez-rfkill-dbus".into(),
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
    fn handles_are_caller_scoped_and_addresses_are_not_exposed() {
        let a = handle(b"a", "adapter", "/org/bluez/hci0", "AA:BB:CC:DD:EE:FF");
        assert_eq!(a.len(), 67);
        assert_ne!(
            a,
            handle(b"b", "adapter", "/org/bluez/hci0", "AA:BB:CC:DD:EE:FF")
        );
        assert!(!a.contains("AA"));
    }
}
