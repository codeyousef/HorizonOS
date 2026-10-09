//! Explicit installed-image qualification. Ordinary unit runs never invoke RPC.
use serde_json::{Value, json};
use zbus::blocking::{Connection, Proxy};
use std::process::Command;

#[track_caller]
fn value(proxy: &Proxy<'_>, method: &str, arguments: impl serde::Serialize + zbus::zvariant::DynamicType) -> Value {
    let response: String = proxy.call(method, &arguments)
        .unwrap_or_else(|error| panic!("{method} failed: {error:?}"));
    serde_json::from_str(&response).unwrap()
}
fn denied<T: std::fmt::Debug>(result: zbus::Result<T>, code: &str) {
    let zbus::Error::MethodError(name, _, _) = result.unwrap_err() else { panic!("expected stable method denial") };
    assert_eq!(name.as_str(), format!("org.aios.Error.{code}"));
}
fn bus_denied<T: std::fmt::Debug>(result: zbus::Result<T>) {
    let zbus::Error::MethodError(name, _, _) = result.unwrap_err() else { panic!("expected native policy denial") };
    assert_eq!(name.as_str(), "org.freedesktop.DBus.Error.AccessDenied");
}
fn action(id: &str, arguments: Value) -> String {
    json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
        "operation":{"kind":"invoke","tool_call":{"kind":"tool_call","action_id":id,"arguments":arguments}}}).to_string()
}
fn system_and_packages(connection: &Connection, executor_owner: &str) {
    let bus=Proxy::new(connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus").unwrap();
    let owner:String=bus.call("GetNameOwner",&("org.aios.System1",)).unwrap();
    assert_eq!(owner,executor_owner);
    for (path,interface,methods) in [
        ("/org/aios/System1","org.aios.System1",vec!["GetCapabilities","Info","Services","ServiceStatus","ServiceRestart","Hardware","StorageStatus","NetworkStatus","NetworkSetWifiEnabled","BluetoothStatus","BluetoothSetEnabled","Boots","BootDiagnostics","Logs","GraphStatus"]),
        ("/org/aios/Packages1","org.aios.Packages1",vec!["GetCapabilities","Search","Info","Installed","Install","Remove","UpgradePlan"]),
    ] {
        let introspection=Proxy::new(connection,"org.aios.System1",path,"org.freedesktop.DBus.Introspectable").unwrap();
        let xml:String=introspection.call("Introspect",&()).unwrap();
        let surface=xml.split(&format!("<interface name=\"{interface}\">" )).nth(1).unwrap().split("</interface>").next().unwrap();
        assert!(!surface.contains("<signal"));
        for method in methods { assert!(surface.contains(&format!("<method name=\"{method}\">"))); }
        let proxy=Proxy::new(connection,"org.aios.System1",path,interface).unwrap();
        assert_eq!(value(&proxy,"GetCapabilities",())["data"]["native_caller_verified"],true);
        // Policy destinations match the owning connection, not only the
        // addressed name. Exact reviewed paths/members must work through all
        // aliases, while another interface/path/member cannot gain access.
        for destination in ["org.aios.System1", "org.aios.Executor1", executor_owner] {
            let alias=Proxy::new(connection,destination,path,interface).unwrap();
            assert_eq!(value(&alias,"GetCapabilities",())["data"]["native_caller_verified"],true);
            bus_denied(alias.call::<_,_,String>("UnreviewedMethod",&()));
            let wrong_path=Proxy::new(connection,destination,"/org/aios/Unreviewed",interface).unwrap();
            bus_denied(wrong_path.call::<_,_,String>("GetCapabilities",&()));
            let wrong_interface=Proxy::new(connection,destination,path,"org.aios.Unreviewed1").unwrap();
            bus_denied(wrong_interface.call::<_,_,String>("GetCapabilities",&()));
        }
        denied(proxy.call::<_,_,String>("Info",&(" ".repeat(aios_protocol::MAX_TASK_BYTES+1),)),"RESOURCE_EXHAUSTED");
    }
    let system=Proxy::new(connection,"org.aios.System1","/org/aios/System1","org.aios.System1").unwrap();
    let capabilities=value(&system,"GetCapabilities",());
    for id in ["network.status","bluetooth.status"] {
        let contract=capabilities["data"]["contracts"].as_array().unwrap().iter().find(|entry|entry["action_id"]==id).unwrap();
        assert_eq!(contract["availability"],"available");
    }
    for id in ["network.set_wifi_enabled","bluetooth.set_enabled"] {
        let contract=capabilities["data"]["contracts"].as_array().unwrap().iter().find(|entry|entry["action_id"]==id).unwrap();
        assert_eq!(contract["availability"],"unavailable");
    }
    let info=value(&system,"Info",(action("system.info",json!({})),));
    aios_protocol::validation::validate_result("system.info",&serde_json::to_vec(&info).unwrap()).unwrap();
    assert_eq!(info["data"]["os_id"],"nixos");assert_eq!(info["data"]["virtualization"],"kvm");
    assert_eq!(info["data"]["boot_id"],std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim());
    let graph=value(&system,"GraphStatus",());
    assert_eq!(graph["schema_version"],1);assert_eq!(graph["scope"],"system");
    assert_eq!(graph["boot"],info["data"]["boot_id"]);assert_eq!(graph["model_invoked"],false);
    assert_eq!(graph["execution_authority"],false);assert!(graph["reconciliations_attempted"].as_u64().is_some());
    let mut forged:Value=serde_json::from_str(&action("system.info",json!({}))).unwrap();forged["uid"]=json!(0);
    denied(system.call::<_,_,String>("Info",&(forged.to_string(),)),"INVALID_ARGUMENT");
    denied(system.call::<_,_,String>("Info",&(action("system.hardware",json!({})),)),"INVALID_ARGUMENT");
    let hardware=value(&system,"Hardware",(action("system.hardware",json!({})),));
    aios_protocol::validation::validate_result("system.hardware",&serde_json::to_vec(&hardware).unwrap()).unwrap();
    assert!(hardware["data"]["items"].as_array().unwrap().iter().any(|item|item["device_class"]=="cpu"));
    assert!(hardware["data"]["unsupported_fields"].as_array().unwrap().iter().any(|field|field=="battery"));
    let storage=value(&system,"StorageStatus",(action("storage.status",json!({})),));
    aios_protocol::validation::validate_result("storage.status",&serde_json::to_vec(&storage).unwrap()).unwrap();
    let root=storage["data"]["mounts"].as_array().unwrap().iter().find(|mount|mount["mount_path"]=="/").unwrap();
    assert!(root["capacity_bytes"].as_u64().unwrap()>0);
    assert!(root["free_bytes"].as_u64().unwrap()<=root["capacity_bytes"].as_u64().unwrap());
    let full=storage["data"]["mounts"].as_array().unwrap().iter().find(|mount|mount["mount_path"]=="/mnt/aios-storage-fixture").unwrap();
    assert_eq!(full["source_identity"],"filesystem:tmpfs");
    assert!(full["free_bytes"].as_u64().unwrap()<=full["capacity_bytes"].as_u64().unwrap()/4);
    let network=value(&system,"NetworkStatus",(action("network.status",json!({})),));
    aios_protocol::validation::validate_result("network.status",&serde_json::to_vec(&network).unwrap()).unwrap();
    assert!(!network["data"]["interfaces"].as_array().unwrap().is_empty());
    assert!(network["data"]["endpoint_reachability"].is_string());
    assert_eq!(network["data"]["management_transport_protected"],true);
    let interface=&network["data"]["interfaces"][0];
    assert!(interface["link_state"].is_string()&&interface["connectivity"].is_string()&&interface["dns_state"].is_string());
    let bluetooth=value(&system,"BluetoothStatus",(action("bluetooth.status",json!({})),));
    aios_protocol::validation::validate_result("bluetooth.status",&serde_json::to_vec(&bluetooth).unwrap()).unwrap();
    assert_eq!(bluetooth["status"],"error");assert_eq!(bluetooth["complete"],false);
    assert_eq!(bluetooth["data"]["adapters"],json!([]));assert_eq!(bluetooth["error"]["code"],"UNSUPPORTED_CAPABILITY");
    denied(system.call::<_,_,String>("NetworkSetWifiEnabled",&(action("network.set_wifi_enabled",json!({"enabled":false})),)),"AUTH_REQUIRED");
    denied(system.call::<_,_,String>("BluetoothSetEnabled",&(action("bluetooth.set_enabled",json!({"adapter_id":"untrusted-reference","enabled":false})),)),"AUTH_REQUIRED");
    denied(system.call::<_,_,String>("ServiceRestart",&(action("system.service_restart",json!({"service_id":"untrusted-reference"})),)),"AUTH_REQUIRED");
    let packages=Proxy::new(connection,"org.aios.System1","/org/aios/Packages1","org.aios.Packages1").unwrap();
    let info=value(&packages,"Info",(action("packages.info",json!({"package_id":"kate"})),));
    aios_protocol::validation::validate_result("packages.info",&serde_json::to_vec(&info).unwrap()).unwrap();
    assert_eq!(info["data"]["package_id"],"kate");assert_eq!(info["source"]["provider"],"aios-installed-catalog");
    assert!(!info["data"]["version"].as_str().unwrap().starts_with("fixture"));
    let search=value(&packages,"Search",(action("packages.search",json!({"query":"e","limit":1})),));
    aios_protocol::validation::validate_result("packages.search",&serde_json::to_vec(&search).unwrap()).unwrap();
    assert_eq!(search["data"]["matches"].as_array().unwrap().len(),1);
    let cursor=search["next_cursor"].as_str().unwrap();
    let next=value(&packages,"Search",(action("packages.search",json!({"query":"e","limit":1,"cursor":cursor})),));
    assert_eq!(next["data"]["matches"].as_array().unwrap().len(),1);
    assert_ne!(search["data"]["matches"][0]["package_id"],next["data"]["matches"][0]["package_id"]);
    denied(packages.call::<_,_,String>("Search",&(action("packages.search",json!({"query":"kate","limit":1,"cursor":cursor})),)),"STALE_EVIDENCE");
    let other=Connection::system().unwrap();
    let reconnect=Proxy::new(&other,"org.aios.System1","/org/aios/Packages1","org.aios.Packages1").unwrap();
    denied(reconnect.call::<_,_,String>("Search",&(action("packages.search",json!({"query":"e","limit":1,"cursor":cursor})),)),"PERMISSION_DENIED");
    denied(packages.call::<_,_,String>("Install",&(action("packages.install",json!({"package_ids":["kate"]})),)),"UNSUPPORTED_CAPABILITY");
    let cli=std::fs::canonicalize("/run/current-system/sw/bin/aiosctl").unwrap();
    assert!(cli.starts_with("/nix/store"));
    for arguments in [
        vec!["package","info","kate","--json"],
        vec!["package","search","Kate","--json"],
        vec!["graph","status","--json"],
    ] {
        let output=Command::new(&cli).args(arguments).output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        let value:Value=serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["schema_version"],1);
    }
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
    system_and_packages(&connection,&owner);
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
    assert_ne!(connection.unique_name(),other.unique_name());
    assert_eq!(value(&reconnected,"GetPlan",(id,))["data"]["plan_sha256"],hash);
    let before = value(&reconnected, "GetTransaction", (id,));
    assert_eq!(before["data"]["status"]["state"], "PLANNED");
    denied(reconnected.call::<_, _, String>("Authorize", &(id, hash)), "AUTH_REQUIRED");
    denied(reconnected.call::<_, _, String>("RequestRollback", &(id,)), "UNSUPPORTED_CAPABILITY");
    let cancelled = value(&reconnected, "CancelTransaction", (id,));
    assert_eq!(cancelled["data"]["complete"], true);
    assert_eq!(cancelled["data"]["status"]["state"], "CANCELLED");
    assert_eq!(value(&proxy, "CancelTransaction", (id,))["data"]["complete"], true);
    denied(proxy.call::<_, _, String>("Execute", &(id, hash)), "CANCELLED");
    let after = value(&proxy, "GetTransaction", (id,));
    assert!(after["data"]["history"].as_array().unwrap().len() > before["data"]["history"].as_array().unwrap().len());
    println!("AIOS_INSTALLED_EXECUTOR {}", json!({"uid":uid,"root_bus_owner_uid":owner_uid,"root_bus_owner_pid":owner_pid,
        "plan_id":id,"native_caller_and_baseline_verified":true,"typed_denials_verified":true,
        "same_session_reconnect_verified":true,"durable_pre_effect_cancellation_verified":true,
        "bus_ownership_policy_verified":true,"system_and_packages_verified":true,
        "trusted_confirmation_verified":false,"activation_performed":false}));
}
