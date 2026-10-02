use aios_protocol::contracts::{Action, parse_tool_call};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args == ["system", "info", "--json"] {
        let call = br#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#;
        match parse_tool_call(call) {
            Ok(Action::SystemInfo) => {
                let result = aios_system::observe_system_info();
                let status = if result.data.is_some() { 0 } else { 1 };
                println!("{}", serde_json::to_string(&result).expect("typed result serializes"));
                std::process::exit(status);
            },
            Ok(Action::SystemServiceStatus(_)) => std::process::exit(2),
            Err(_) => std::process::exit(2),
        }
    }
    let base = match args.as_slice() {
        [a, b, unit, flag] if a == "inspect" && b == "service" && flag == "--json" => Some((unit.as_str(), None)),
        [a, b, unit, flag, socket_flag, socket] if a == "inspect" && b == "service" && flag == "--json" && socket_flag == "--socket" => Some((unit.as_str(), Some(socket.as_str()))),
        _ => None,
    };
    if let Some((unit, socket)) = base {
        let path = socket.map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(format!("/run/user/{}/aios/session.sock", std::fs::metadata("/proc/self").map(|m| { use std::os::unix::fs::MetadataExt; m.uid() }).unwrap_or(u32::MAX))));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut client = aios_session::Client::connect(&path)?;
            let resolved = client.call(serde_json::json!({"kind":"resolve_service","unit_name":unit}))?;
            if resolved.error.is_some() { println!("{}", serde_json::to_string(&resolved)?); return Err("service could not be resolved".into()); }
            let id = resolved.data.and_then(|v| v["service_id"].as_str().map(str::to_owned)).ok_or("missing service handle")?;
            let response = client.call(serde_json::json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":id}}}))?;
            if response.error.is_some() { println!("{}", serde_json::to_string(&response)?); return Err("service scope denied".into()); }
            let value = response.data.ok_or("missing provider result")?;
            let failed = value["status"] == "error";
            println!("{}", serde_json::to_string(&value)?);
            if failed { return Err("service observation failed".into()); }
            Ok(())
        })();
        if let Err(error) = result { eprintln!("aiosctl: {error}"); std::process::exit(1); }
        return;
    }
    eprintln!("Usage: aiosctl system info --json | inspect service UNIT --json [--socket PRIVATE_PATH]");
    std::process::exit(2);
}
