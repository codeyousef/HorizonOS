use aios_protocol::contracts::{Action, parse_tool_call};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args==["process","list","--json"]{
        match process_inventory(){Ok(value)=>{println!("{value}");return;},Err(code)=>api_error(code)}
    }
    if let [command,operation,session,boot,pid,start,goal,flag]=args.as_slice(){
        if command=="process" && operation=="terminate" && flag=="--json"{
            match terminate_process(session,boot,pid,start,goal){
                Ok(value)=>{let success=value["state"]=="completed";println!("{value}");std::process::exit(if success{0}else{1});},
                Err(code)=>api_error(code),
            }
        }
    }
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
                    client_nonce: new_nonce(), context_handles: vec![],retain_for_history:false,history_handles:vec![], selected_app_handle: None, selected_session_handle: None })?;
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
    eprintln!("Usage: aiosctl status --json | ask TEXT --json [--service UNIT] | ui select-session SESSION --json | ui read-window SESSION EXACT_TITLE GOAL --json | process list --json | process terminate SESSION BOOT_UUID PID START_TICKS GOAL --json | system info --json | inspect service UNIT --json [--socket PRIVATE_PATH]");
    std::process::exit(2);
}

fn native_boot()->Result<String,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    let boot=std::fs::read_to_string("/proc/sys/kernel/random/boot_id").map_err(|_|ErrorCode::TargetChanged)?.trim().to_owned();
    uuid::Uuid::parse_str(&boot).map_err(|_|ErrorCode::TargetChanged)?;Ok(boot)
}
fn process_inventory()->Result<serde_json::Value,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    let boot=native_boot()?;let client=aios_session::bus::Client::connect_user_bus()?;
    let mut pages=Vec::new();let mut cursor:Option<String>=None;let mut seen=std::collections::HashSet::new();
    for _ in 0..16{
        let value=client.list_processes(cursor.as_deref())?;cursor=value["next_cursor"].as_str().map(str::to_owned);
        pages.push(value);
        if cursor.is_none(){
            if native_boot()?!=boot{return Err(ErrorCode::TargetChanged);}
            return Ok(serde_json::json!({"schema_version":1,"operation":"process_inventory","boot_id":boot,"pages":pages}));
        }
        if !seen.insert(cursor.clone()){return Err(ErrorCode::TargetChanged);}
    }
    Err(ErrorCode::ResourceExhausted)
}
fn selected_process(value:&serde_json::Value,pid:u32,start:u64)->Result<Option<String>,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    if pid<=1 || start==0 || start>9_007_199_254_740_991{return Err(ErrorCode::InvalidArgument);}
    let rows=value["data"]["processes"].as_array().ok_or(ErrorCode::InvalidArgument)?;
    let candidates=rows.iter().filter(|row|row["pid"].as_u64()==Some(u64::from(pid))).collect::<Vec<_>>();
    if candidates.len()>1{return Err(ErrorCode::Conflict);}
    let Some(row)=candidates.first() else{return Ok(None);};
    if row["start_time_ticks"].as_u64()!=Some(start){return Err(ErrorCode::TargetChanged);}
    let id=row["process_id"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    uuid::Uuid::parse_str(id).map_err(|_|ErrorCode::InvalidArgument)?;Ok(Some(id.into()))
}
fn terminate_process(session:&str,boot:&str,pid:&str,start:&str,goal:&str)->Result<serde_json::Value,aios_protocol::contracts::ErrorCode>{
    use aios_protocol::contracts::ErrorCode;
    let pid:u32=pid.parse().map_err(|_|ErrorCode::InvalidArgument)?;
    let start:u64=start.parse().map_err(|_|ErrorCode::InvalidArgument)?;
    if pid<=1 || start==0 || start>9_007_199_254_740_991 || goal.trim().is_empty() || goal.len()>4096{return Err(ErrorCode::InvalidArgument);}
    if native_boot()?!=boot{return Err(ErrorCode::TargetChanged);}
    let client=aios_session::bus::Client::connect_user_bus()?;
    let selected=client.select_ui_session(session)?;
    let session=selected["candidate_handle"].as_str().ok_or(ErrorCode::InvalidArgument)?;
    let mut cursor:Option<String>=None;let mut seen=std::collections::HashSet::new();let mut process=None;
    for _ in 0..16{
        let value=client.list_processes(cursor.as_deref())?;
        if let Some(id)=selected_process(&value,pid,start)?{process=Some(id);break;}
        cursor=value["next_cursor"].as_str().map(str::to_owned);
        if cursor.is_none(){return Err(if value["complete"]==true{ErrorCode::TargetNotFound}else{ErrorCode::PermissionDenied});}
        if !seen.insert(cursor.clone()){return Err(ErrorCode::TargetChanged);}
    }
    let process=process.ok_or(ErrorCode::ResourceExhausted)?;
    if native_boot()?!=boot{return Err(ErrorCode::TargetChanged);}
    let id=new_nonce();
    if let Err(code)=client.start_process_termination(&id,&process,session,goal){
        let cancelled=client.cancel_process_termination(&id).is_ok();return Ok(unverified_termination(&id,code,cancelled));
    }
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(95);
    loop{
        let value=match client.process_termination_status(&id){Ok(value)=>value,Err(code)=>{
            let cancelled=client.cancel_process_termination(&id).is_ok();return Ok(unverified_termination(&id,code,cancelled));
        }};
        if matches!(value["state"].as_str(),Some("completed"|"partial"|"failed"|"cancelled")){
            // Preserve the truthful terminal receipt across best-effort
            // private record cleanup. Never retry an effect after an error.
            let _=client.forget_process_termination(&id);return Ok(value);
        }
        if std::time::Instant::now()>=deadline{
            let cancelled=client.cancel_process_termination(&id).is_ok();return Ok(unverified_termination(&id,ErrorCode::DeadlineExceeded,cancelled));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
fn unverified_termination(id:&str,code:aios_protocol::contracts::ErrorCode,cancel_requested:bool)->serde_json::Value{
    serde_json::json!({"schema_version":1,"operation":"process_termination_unverified","task_id":id,"state":"unverified",
        "error":code,"receipt":null,"effect_outcome_unknown":true,"cancel_requested":cancel_requested})
}

#[cfg(test)]mod tests{
    use super::*;use serde_json::json;use aios_protocol::contracts::ErrorCode;
    #[test]fn process_selection_refuses_pid_reuse_and_ambiguous_inventory(){
        let id=uuid::Uuid::new_v4().to_string();let row=json!({"process_id":id,"pid":42,"start_time_ticks":100});
        let value=json!({"data":{"processes":[row]}});
        assert_eq!(selected_process(&value,42,100).unwrap(),Some(id));
        assert_eq!(selected_process(&value,42,99),Err(ErrorCode::TargetChanged));
        assert_eq!(selected_process(&value,43,100).unwrap(),None);
        let duplicated=json!({"data":{"processes":[row,row]}});
        assert_eq!(selected_process(&duplicated,42,100),Err(ErrorCode::Conflict));
        assert_eq!(selected_process(&value,0,100),Err(ErrorCode::InvalidArgument));
    }
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
