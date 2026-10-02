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
            Err(_) => std::process::exit(2),
        }
    }
    eprintln!("Usage: aiosctl system info --json");
    std::process::exit(2);
}
