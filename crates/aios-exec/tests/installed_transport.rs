//! Explicit installed-image qualification. Ordinary unit runs never invoke RPC.
use serde_json::{Value, json};
use zbus::blocking::{Connection, Proxy};

fn value(proxy: &Proxy<'_>, method: &str, arguments: impl serde::Serialize + zbus::zvariant::DynamicType) -> Value {
    let response: String = proxy.call(method, &arguments).unwrap();
    serde_json::from_str(&response).unwrap()
}
fn denied<T: std::fmt::Debug>(result: zbus::Result<T>, code: &str) {
    let zbus::Error::MethodError(name, _, _) = result.unwrap_err() else { panic!("expected stable method denial") };
    assert_eq!(name.as_str(), format!("org.aios.Error.{code}"));
}

#[test]
#[ignore = "requires the enrolled installed Executor1 service and unprivileged logind user"]
fn installed_native_transport_is_private_typed_and_cancellable() {
    let uid = unsafe { libc::getuid() };
    assert!(uid >= 1000);
    let connection = Connection::system().unwrap();
    let bus = Proxy::new(&connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").unwrap();
    let owner: String = bus.call("GetNameOwner", &(aios_exec::bus::NAME,)).unwrap();
    let owner_uid: u32 = bus.call("GetConnectionUnixUser", &(&owner,)).unwrap();
    assert_eq!(owner_uid, 0);
    let owner_pid: u32 = bus.call("GetConnectionUnixProcessID", &(&owner,)).unwrap();
    let executable = std::fs::canonicalize("/run/current-system/sw/bin/aios-execd").unwrap();
    assert!(executable.starts_with("/nix/store"));
    assert_eq!(executable.file_name().unwrap(), "aios-execd");
    // The observer cannot inspect another UID's /proc/PID/exe. Bind the native
    // root bus PID to the actual installed systemd service's public MainPID.
    let service = std::process::Command::new("systemctl").args(["show", "--value", "--property=MainPID", "aios-execd.service"]).output().unwrap();
    assert!(service.status.success());
    assert_eq!(std::str::from_utf8(&service.stdout).unwrap().trim().parse::<u32>().unwrap(), owner_pid);
    for name in ["org.aios.Executor1.Test", "org.aios.System1", "org.aios.Unknown1"] {
        assert!(connection.request_name_with_flags(name, zbus::fdo::RequestNameFlags::DoNotQueue.into()).is_err());
    }
    let introspection = Proxy::new(&connection, aios_exec::bus::NAME, aios_exec::bus::PATH, "org.freedesktop.DBus.Introspectable").unwrap();
    let xml: String = introspection.call("Introspect", &()).unwrap();
    let interface = xml.split("<interface name=\"org.aios.Executor1\">").nth(1).unwrap().split("</interface>").next().unwrap();
    for method in ["GetCapabilities", "Prepare", "GetPlan", "Authorize", "Execute", "GetTransaction", "CancelTransaction", "RequestRollback"] {
        assert!(interface.contains(&format!("<method name=\"{method}\">")));
    }
    assert!(!interface.contains("<signal"));
    let proxy = Proxy::new(&connection, aios_exec::bus::NAME, aios_exec::bus::PATH, aios_exec::bus::NAME).unwrap();
    let capabilities = value(&proxy, "GetCapabilities", ());
    assert_eq!(capabilities["data"]["native_caller_verified"], true);
    assert_eq!(capabilities["data"]["execute"], false);
    let raw = json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":"prepare","mode":"act",
        "intent_text":"Install Kate (disposable IPC qualification)","intent":{"action":"install_package","package_id":"kate"}});
    for field in ["uid", "approved", "target", "template_path", "database_data", "grants"] {
        let mut forged = raw.clone(); forged[field] = json!(true);
        denied(proxy.call::<_, _, String>("Prepare", &(forged.to_string(),)), "INVALID_ARGUMENT");
    }
    denied(proxy.call::<_, _, String>("Prepare", &(" ".repeat(aios_protocol::MAX_TASK_BYTES+1),)), "RESOURCE_EXHAUSTED");
    let prepared = value(&proxy, "Prepare", (raw.to_string(),));
    let data = &prepared["data"];
    let id = data["plan_id"].as_str().unwrap();
    let hash = data["plan_sha256"].as_str().unwrap();
    assert_eq!(data["plan"]["requester"]["uid"], uid);
    assert_eq!(data["plan"]["requester"]["bus_sender"], connection.unique_name().unwrap().as_str());
    assert!(!data["plan"]["requester"]["logind_session"].as_str().unwrap().is_empty());
    assert_eq!(data["plan"]["target"]["boot_id"], std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim());
    assert_eq!(data["plan"]["target"]["installation_uuid"], std::fs::read_to_string("/etc/aios/installation-uuid").unwrap().trim());
    for field in ["running_closure", "profile_closure", "boot_selected_closure"] {
        assert!(data["plan"]["baseline"][field].as_str().unwrap().starts_with("/nix/store/"));
    }
    assert_eq!(data["build_started"], false);
    assert_eq!(data["system_effects_performed"], false);
    assert_eq!(value(&proxy, "Prepare", (raw.to_string(),))["data"]["plan_sha256"], hash);
    let mut changed = raw.clone(); changed["intent_text"] = json!("Changed request with old nonce");
    denied(proxy.call::<_, _, String>("Prepare", &(changed.to_string(),)), "CONFLICT");
    assert_eq!(value(&proxy, "GetPlan", (id,))["data"]["plan_sha256"], hash);
    denied(proxy.call::<_, _, String>("Authorize", &(id, "f".repeat(64))), "PLAN_CHANGED");
    denied(proxy.call::<_, _, String>("Authorize", &(id, hash)), "AUTH_REQUIRED");
    denied(proxy.call::<_, _, String>("Execute", &(id, hash)), "AUTH_REQUIRED");
    let other = Connection::system().unwrap();
    let reconnected = Proxy::new(&other, aios_exec::bus::NAME, aios_exec::bus::PATH, aios_exec::bus::NAME).unwrap();
    for method in ["GetPlan", "GetTransaction", "CancelTransaction", "RequestRollback"] {
        denied(reconnected.call::<_, _, String>(method, &(id,)), "PERMISSION_DENIED");
    }
    let before = value(&proxy, "GetTransaction", (id,));
    assert_eq!(before["data"]["status"]["state"], "PLANNED");
    let cancelled = value(&proxy, "CancelTransaction", (id,));
    assert_eq!(cancelled["data"]["complete"], true);
    assert_eq!(cancelled["data"]["status"]["state"], "CANCELLED");
    assert_eq!(value(&proxy, "CancelTransaction", (id,))["data"]["complete"], true);
    denied(proxy.call::<_, _, String>("Execute", &(id, hash)), "CANCELLED");
    let after = value(&proxy, "GetTransaction", (id,));
    assert!(after["data"]["history"].as_array().unwrap().len() > before["data"]["history"].as_array().unwrap().len());
    println!("AIOS_INSTALLED_EXECUTOR {}", json!({"uid":uid,"root_bus_owner_uid":owner_uid,"root_bus_owner_pid":owner_pid,
        "plan_id":id,"native_caller_and_baseline_verified":true,"typed_denials_verified":true,
        "reconnect_denials_verified":true,"durable_pre_effect_cancellation_verified":true,
        "bus_ownership_policy_verified":true,"trusted_confirmation_verified":false,"activation_performed":false}));
}
