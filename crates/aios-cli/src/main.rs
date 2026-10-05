use aios_protocol::contracts::{Action, parse_tool_call};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let [command,operation,session,title,goal,flag]=args.as_slice(){
        if command=="ui" && operation=="read-window" && flag=="--json"{
            match read_window(session,title,goal){Ok(value)=>{println!("{value}");return;},Err(code)=>api_error(code)}
        }
    }
    if let [command, operation, session, flag] = args.as_slice() {
        if command == "ui" && operation == "select-session" && flag == "--json" {
            match aios_session::bus::Client::connect_user_bus().and_then(|client| client.select_ui_session(session)) {
                Ok(value) => { println!("{value}"); return; },
                Err(code) => api_error(code),
            }
        }
    }
    if args == ["status", "--json"] {
        let result = aios_session::bus::Client::connect_user_bus().and_then(|client| client.capabilities());
        match result {
            Ok(value) => { println!("{value}"); return; },
            Err(code) => api_error(code),
        }
    }
    if let [command,text,flag,service_flag,unit]=args.as_slice(){
        if command=="ask" && flag=="--json" && service_flag=="--service"{
            match ask_service(text,unit){
                Ok(value)=>{let failed=value["state"]!="completed";println!("{value}");std::process::exit(if failed{1}else{0});},
                Err(code)=>api_error(code),
            }
        }
    }
    if let [command, text, flag] = args.as_slice() {
        if command == "ask" && flag == "--json" {
            let outcome = (|| {
                let client = aios_session::bus::Client::connect_user_bus()?;
                let task = client.submit(&aios_session::Submit { mode: aios_session::Mode::Ask, text: text.clone(),
                    client_nonce: new_nonce(), context_handles: vec![], selected_app_handle: None, selected_session_handle: None })?;
                let deadline=std::time::Instant::now()+std::time::Duration::from_secs(95);
                loop {
                    let status=client.status(&task)?;
                    if matches!(status["state"].as_str(),Some("completed"|"failed"|"cancelled")) {break Ok(status);}
                    if std::time::Instant::now()>=deadline {let _=client.cancel(&task);return Err(aios_protocol::contracts::ErrorCode::DeadlineExceeded);}
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            })();
            match outcome {
                Ok(value) => { let failed = value["state"] != "completed"; println!("{value}"); std::process::exit(if failed { 1 } else { 0 }); },
                Err(code) => api_error(code),
            }
        }
    }
    if args == ["system", "info", "--json"] {
        let call = br#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#;
        match parse_tool_call(call) {
            Ok(Action::SystemInfo) => {
                let result = aios_system::observe_system_info();
                let status = if result.data.is_some() { 0 } else { 1 };
                println!("{}", serde_json::to_string(&result).expect("typed result serializes"));
                std::process::exit(status);
            },
            Ok(_) => std::process::exit(2),
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
    eprintln!("Usage: aiosctl status --json | ask TEXT --json [--service UNIT] | ui select-session SESSION --json | ui read-window SESSION EXACT_TITLE GOAL --json | system info --json | inspect service UNIT --json [--socket PRIVATE_PATH]");
    std::process::exit(2);
}

fn ask_service(text:&str,unit:&str)->Result<serde_json::Value,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    use serde_json::{Value,json};
    use std::os::unix::fs::MetadataExt;
    let uid=std::fs::metadata("/proc/self").map_err(|_|ErrorCode::TargetChanged)?.uid();
    let mut client=aios_session::Client::connect(&std::path::PathBuf::from(format!("/run/user/{uid}/aios/session.sock")))
        .map_err(|_|ErrorCode::UnsupportedCapability)?;
    let call=|client:&mut aios_session::Client,operation:Value|->Result<Value,ErrorCode>{
        let response=client.call(operation).map_err(|_|ErrorCode::TargetChanged)?;
        match response.error{Some(error)=>Err(error.code),None=>response.data.ok_or(ErrorCode::InvalidArgument)}
    };
    let resolved=call(&mut client,json!({"kind":"resolve_service","unit_name":unit}))?;
    let handle=resolved["service_id"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let task=call(&mut client,json!({"kind":"submit","request":{"mode":"ask","text":text,"client_nonce":new_nonce(),"context_handles":[handle]}}))?;
    let id=task["request_id"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(95);
    loop{
        let status=call(&mut client,json!({"kind":"get_status","task_id":id}))?;
        if matches!(status["state"].as_str(),Some("completed"|"failed"|"cancelled")){return Ok(status);}
        if std::time::Instant::now()>=deadline{let _=call(&mut client,json!({"kind":"cancel","task_id":id}));return Err(ErrorCode::DeadlineExceeded);}
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn read_window(session:&str,title:&str,goal:&str)->Result<serde_json::Value,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    use serde_json::{Value,json};
    let uid=std::fs::metadata("/proc/self").map_err(|_|ErrorCode::TargetChanged)?;
    use std::os::unix::fs::MetadataExt;
    let path=std::path::PathBuf::from(format!("/run/user/{}/aios/session.sock",uid.uid()));
    let mut client=aios_session::Client::connect(&path).map_err(|_|ErrorCode::UnsupportedCapability)?;
    let call=|client:&mut aios_session::Client,operation:Value|->Result<Value,ErrorCode>{
        let response=client.call(operation).map_err(|_|ErrorCode::TargetChanged)?;
        match response.error{Some(error)=>Err(error.code),None=>response.data.ok_or(ErrorCode::InvalidArgument)}
    };
    let selected=call(&mut client,json!({"kind":"select_ui_session","session_id":session}))?;
    let handle=selected["candidate_handle"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let windows=call(&mut client,json!({"kind":"list_ui_windows","session_handle":handle}))?;
    let candidates=windows["windows"].as_array().ok_or(ErrorCode::InvalidArgument)?.iter().filter(|w|w["title"].as_str()==Some(title)).collect::<Vec<_>>();
    if candidates.len()!=1{return Err(if candidates.is_empty(){ErrorCode::TargetNotFound}else{ErrorCode::Conflict});}
    let handle=candidates[0]["window_handle"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let started=call(&mut client,json!({"kind":"start_ui_read","window_handle":handle,"goal":goal,"mode":"ask"}))?;
    let id=started["task_id"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(95);
    loop{
        let status=call(&mut client,json!({"kind":"get_ui_read_status","task_id":id}))?;
        match status["state"].as_str(){
            Some("completed")=>return call(&mut client,json!({"kind":"take_ui_snapshot","task_id":id})),
            Some("failed"|"cancelled")=>return Err(serde_json::from_value(status["error"].clone()).map_err(|_|ErrorCode::InvalidArgument)?),
            Some("queued"|"needs_permission"|"inspecting")=>{},_=>return Err(ErrorCode::InvalidArgument),
        }
        if std::time::Instant::now()>=deadline{
            let _=call(&mut client,json!({"kind":"cancel_ui_read","task_id":id}));return Err(ErrorCode::DeadlineExceeded);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn new_nonce() -> String { uuid::Uuid::new_v4().to_string() }
fn api_error(code: aios_protocol::contracts::ErrorCode) -> ! {
    println!("{}",serde_json::json!({"schema_version":1,"request_id":new_nonce(),"operation":"client_error",
        "error":{"code":code,"message":"Authenticated session API unavailable or request denied","retryable":false}}));
    std::process::exit(1);
}
