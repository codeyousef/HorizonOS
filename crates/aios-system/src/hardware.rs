//! Bounded hardware inventory assembled from fixed native CPU and udev/sysfs
//! sources. Caller data is used only to scope opaque device handles.
use crate::devices::{self, BlockDevice};
use aios_protocol::contracts::{ErrorCode, ProviderError, ProviderResult, ResultStatus, Source};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Serialize)]
pub struct HardwareProperty {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct HardwareItem {
    pub device_id: String,
    pub device_class: String,
    pub name: String,
    pub properties: Vec<HardwareProperty>,
}

#[derive(Debug, Serialize)]
pub struct HardwareData {
    pub items: Vec<HardwareItem>,
    pub unsupported_fields: Vec<String>,
}

fn scoped_id(scope: &[u8], kind: &str, identity: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"aios-hardware-handle-v1\0");
    digest.update(scope);
    digest.update([0]);
    digest.update(kind.as_bytes());
    digest.update([0]);
    digest.update(identity);
    format!("hw-{:x}", digest.finalize())
}

fn cpu(scope: &[u8]) -> Result<(HardwareItem, Vec<String>), ErrorCode> {
    let raw = super::bounded(Path::new("/proc/cpuinfo"), 1024 * 1024)?;
    let mut count = 0_u32;
    let mut model = None;
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        match key.trim() {
            "processor" => count = count.checked_add(1).ok_or(ErrorCode::ResourceExhausted)?,
            "model name" if model.is_none() => {
                let value = value.trim();
                if !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control) {
                    model = Some(value.to_owned());
                }
            }
            _ => {}
        }
    }
    if count == 0 { return Err(ErrorCode::PartialResult); }
    let architecture = std::env::consts::ARCH.to_owned();
    let mut properties = vec![
        HardwareProperty { key: "architecture".into(), value: architecture.clone() },
        HardwareProperty { key: "logical_processors".into(), value: count.to_string() },
    ];
    let mut unsupported = Vec::new();
    let name = if let Some(model) = model {
        properties.push(HardwareProperty { key: "model_name".into(), value: model.clone() });
        model
    } else {
        unsupported.push("cpu.model_name".into());
        format!("{architecture} CPU")
    };
    Ok((HardwareItem {
        device_id: scoped_id(scope, "cpu", architecture.as_bytes()),
        device_class: "cpu".into(),
        name,
        properties,
    }, unsupported))
}

fn stable_device_identity(device: &BlockDevice, boot_id: &str) -> Vec<u8> {
    if let Some(wwn) = &device.wwn {
        return format!("wwn\0{wwn}").into_bytes();
    }
    if let Some(serial) = &device.serial {
        return format!("serial\0{}\0{serial}", device.bus.as_deref().unwrap_or("unknown")).into_bytes();
    }
    format!("boot\0{boot_id}\0{}\0{}:{}", device.devpath, device.major, device.minor).into_bytes()
}
fn udisks_properties(connection:&zbus::blocking::Connection,device:&BlockDevice)->Result<Vec<HardwareProperty>,ErrorCode>{
    if !device.sysname.bytes().all(|byte|byte.is_ascii_alphanumeric()||byte==b'_'){return Err(ErrorCode::UnsupportedCapability);}
    let path=format!("/org/freedesktop/UDisks2/block_devices/{}",device.sysname);
    let proxy=zbus::blocking::Proxy::new(connection,"org.freedesktop.UDisks2",path.as_str(),"org.freedesktop.UDisks2.Block")
        .map_err(|_|ErrorCode::UnsupportedCapability)?;
    let number:u64=proxy.get_property("DeviceNumber").map_err(|_|ErrorCode::UnsupportedCapability)?;
    if number!=libc::makedev(device.major,device.minor){return Err(ErrorCode::TargetChanged);}
    let size:u64=proxy.get_property("Size").map_err(|_|ErrorCode::PartialResult)?;
    if device.capacity_bytes.is_some_and(|capacity|capacity!=size){return Err(ErrorCode::TargetChanged);}
    let hint_system:bool=proxy.get_property("HintSystem").map_err(|_|ErrorCode::PartialResult)?;
    let usage:String=proxy.get_property("IdUsage").map_err(|_|ErrorCode::PartialResult)?;
    let mut properties=vec![
        HardwareProperty{key:"udisks2_device_number_verified".into(),value:"true".into()},
        HardwareProperty{key:"udisks2_hint_system".into(),value:hint_system.to_string()},
        HardwareProperty{key:"udisks2_size_bytes".into(),value:size.to_string()},
    ];
    if !usage.is_empty()&&usage.len()<=64&&!usage.chars().any(char::is_control){
        properties.push(HardwareProperty{key:"udisks2_id_usage".into(),value:usage});
    }
    Ok(properties)
}


fn storage_item(scope: &[u8], boot_id: &str, device: &BlockDevice,udisks:Option<&zbus::blocking::Connection>) -> (HardwareItem,bool) {
    let name = match (&device.vendor, &device.model) {
        (Some(vendor), Some(model)) => format!("{vendor} {model}"),
        (_, Some(model)) => model.clone(),
        _ => device.sysname.clone(),
    };
    let mut properties = vec![
        HardwareProperty { key: "bus".into(), value: device.bus.clone().unwrap_or_else(|| "unknown".into()) },
        HardwareProperty { key: "capacity_bytes".into(), value: device.capacity_bytes.map_or_else(|| "unknown".into(), |v| v.to_string()) },
        HardwareProperty { key: "initialized".into(), value: device.initialized.to_string() },
        HardwareProperty { key: "read_only".into(), value: device.read_only.map_or_else(|| "unknown".into(), |v| v.to_string()) },
        HardwareProperty { key: "removable".into(), value: device.removable.map_or_else(|| "unknown".into(), |v| v.to_string()) },
        HardwareProperty { key: "serial_available".into(), value: device.serial.is_some().to_string() },
        HardwareProperty { key: "wwn_available".into(), value: device.wwn.is_some().to_string() },
    ];
    let udisks_supported=udisks.and_then(|connection|udisks_properties(connection,device).ok()).is_some_and(|mut values|{properties.append(&mut values);true});
    properties.sort_by(|a, b| a.key.cmp(&b.key));
    (HardwareItem {
        device_id: scoped_id(scope, "storage", &stable_device_identity(device, boot_id)),
        device_class: "storage".into(),
        name,
        properties,
    },udisks_supported)
}

pub fn observe(device_class: &str, scope: &[u8]) -> ProviderResult<HardwareData> {
    let observed_at = OffsetDateTime::now_utc().format(&Rfc3339).expect("valid UTC timestamp");
    let mut items = Vec::new();
    let mut unsupported_fields = Vec::new();
    let mut failed = false;
    if matches!(device_class, "all" | "cpu") {
        match cpu(scope) {
            Ok((item, unsupported)) => { items.push(item); unsupported_fields.extend(unsupported); }
            Err(_) => { unsupported_fields.push("cpu".into()); failed = true; }
        }
    }
    if matches!(device_class, "all" | "storage") {
        let boot = super::bounded(Path::new("/proc/sys/kernel/random/boot_id"), 128)
            .and_then(|value| super::boot_id(&value));
        match (boot, devices::read_block_devices()) {
            (Ok(boot), Ok(inventory)) => {
                failed |= !inventory.complete;
                let udisks=zbus::blocking::Connection::system().ok();
                for device in &inventory.devices{
                    let (item,supported)=storage_item(scope,&boot,device,udisks.as_ref());
                    if !supported{unsupported_fields.push("storage.udisks2".into());failed=true;}
                    items.push(item);
                }
            }
            _ => { unsupported_fields.push("storage".into()); failed = true; }
        }
    }
    let requested_unsupported = match device_class {
        "gpu" | "network" | "audio" | "bluetooth" => Some(device_class),
        "all" => {
            unsupported_fields.extend(["gpu", "network", "audio", "bluetooth", "battery"].map(str::to_owned));
            None
        }
        _ => None,
    };
    if let Some(class) = requested_unsupported { unsupported_fields.push(class.into()); failed = true; }
    unsupported_fields.sort();
    unsupported_fields.dedup();
    let complete = !failed && unsupported_fields.is_empty();
    let status = if complete { ResultStatus::Ok } else { ResultStatus::Partial };
    ProviderResult {
        schema_version: 1,
        status,
        observed_at,
        source: Source { provider: "aios-system-native-hardware".into(), provider_version: env!("CARGO_PKG_VERSION").into() },
        evidence_ids: Vec::new(),
        complete,
        next_cursor: None,
        data: Some(HardwareData { items, unsupported_fields }),
        error: (!complete).then(|| ProviderError {
            code: ErrorCode::PartialResult,
            message: "Unavailable hardware classes and properties remain explicitly unsupported".into(),
            retryable: true,
        }),
    }
}
