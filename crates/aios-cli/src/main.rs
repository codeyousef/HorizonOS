mod native;

use aios_protocol::contracts::{Action, parse_tool_call};

fn main() {
    run(std::env::args().skip(1).collect());
}

pub fn run_as_ask() {
    let supplied = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(args) = standalone_ask_arguments(&supplied) else {
        eprintln!("Usage: ask TEXT [--mode read-only] [--json]");
        std::process::exit(2);
    };
    run(args);
}

fn standalone_ask_arguments(supplied: &[String]) -> Option<Vec<String>> {
    match supplied {
        [text] => Some(vec!["ask".into(), text.clone()]),
        [text, flag] if flag == "--json" => Some(vec!["ask".into(), text.clone(), "--json".into()]),
        [text, mode, value] if mode == "--mode" && value == "read-only" =>
            Some(vec!["ask".into(), text.clone()]),
        [text, mode, value, flag] if mode == "--mode" && value == "read-only" && flag == "--json" =>
            Some(vec!["ask".into(), text.clone(), "--json".into()]),
        _ => None,
    }
}

fn run(args: Vec<String>) {
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
    if let Some((text, json)) = ask_arguments(&args) {
        match run_ask(text) {
            Ok(value) => {
                let failed = value["state"] != "completed";
                if json {
                    println!("{value}");
                } else if failed {
                    eprintln!("aiosctl: request failed: {}", value["error"].as_str().unwrap_or("UNKNOWN"));
                } else {
                    match human_ask_output(&value) {
                        Ok(output) => println!("{output}"),
                        Err(code) => api_error(code),
                    }
                }
                std::process::exit(if failed { 1 } else { 0 });
            },
            Err(code) => api_error(code),
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
    let session_observation = match args.as_slice() {
        [a, b] if a == "privacy" && b == "scopes" => Some(("privacy", false)),
        [a, b, flag] if a == "privacy" && b == "scopes" && flag == "--json" => Some(("privacy", true)),
        [a, b] if a == "history" && b == "list" => Some(("history", false)),
        [a, b, flag] if a == "history" && b == "list" && flag == "--json" => Some(("history", true)),
        _ => None,
    };
    if let Some((operation, json)) = session_observation {
        let result=aios_session::bus::Client::connect_user_bus().and_then(|client|
            if operation=="privacy" {client.privacy_scopes()} else {client.list_history()});
        match result {
            Ok(value)=>{
                if json {println!("{value}");}
                else if operation=="privacy" {
                    println!("Privacy scopes: {} file roots, {} retained history entries, {} active tasks",
                        value["data"]["file_roots"].as_array().map_or(0,Vec::len),
                        value["data"]["retained_history"].as_u64().unwrap_or(0),
                        value["data"]["active_tasks"].as_u64().unwrap_or(0));
                } else if let Some(entries)=value["data"]["entries"].as_array() {
                    if entries.is_empty(){println!("No retained session history.");}
                    else {for entry in entries {println!("{}\t{}",entry["submitted_at"].as_str().unwrap_or(""),entry["task_id"].as_str().unwrap_or(""));}}
                }
                return;
            },
            Err(code)=>api_error(code),
        }
    }
    let package = match args.as_slice() {
        [a, b, value] if a == "package" && matches!(b.as_str(), "search" | "info") =>
            Some((b.as_str(), value.as_str(), false)),
        [a, b, value, flag] if a == "package" && matches!(b.as_str(), "search" | "info") && flag == "--json" =>
            Some((b.as_str(), value.as_str(), true)),
        _ => None,
    };
    if let Some((operation, value, json)) = package {
        let result = native::Client::connect().and_then(|client|
            if operation == "search" { client.package_search(value) } else { client.package_info(value) });
        match result {
            Ok(result) => {
                if json { println!("{result}"); }
                else if operation == "search" {
                    if let Some(matches) = result["data"]["matches"].as_array() {
                        for package in matches {
                            println!("{}\t{}\t{}", package["package_id"].as_str().unwrap_or(""),
                                package["version"].as_str().unwrap_or(""), package["name"].as_str().unwrap_or(""));
                        }
                    }
                } else {
                    println!("{} {} — {}", result["data"]["package_id"].as_str().unwrap_or(""),
                        result["data"]["version"].as_str().unwrap_or(""), result["data"]["name"].as_str().unwrap_or(""));
                }
                return;
            },
            Err(code) => api_error(code),
        }
    }
    let graph_json = match args.as_slice() {
        [a, b] if a == "graph" && b == "status" => Some(false),
        [a, b, flag] if a == "graph" && b == "status" && flag == "--json" => Some(true),
        _ => None,
    };
    if let Some(json) = graph_json {
        match native::Client::connect().and_then(|client| client.graph_status()) {
            Ok(value) => {
                if json { println!("{value}"); }
                else {
                    println!("Graph: {} ({} loaded services, {} complete reconciliations)",
                        value["provider"]["status"].as_str().unwrap_or("unknown"),
                        value["observed_loaded_services"].as_u64().unwrap_or(0),
                        value["complete_reconciliations"].as_u64().unwrap_or(0));
                }
                return;
            },
            Err(code) => api_error(code),
        }
    }
    let model = match args.as_slice() {
        [a, b] if a == "model" && matches!(b.as_str(), "status" | "unload") =>
            Some((b.as_str(), false)),
        [a, b, flag] if a == "model" && matches!(b.as_str(), "status" | "unload") && flag == "--json" =>
            Some((b.as_str(), true)),
        _ => None,
    };
    if let Some((operation, json)) = model {
        let result = aios_session::bus::Client::connect_user_bus().and_then(|client|
            if operation == "status" { client.model_status() } else { client.unload_model() });
        match result {
            Ok(value) => {
                if json {
                    println!("{value}");
                } else if operation == "status" {
                    match human_model_status(&value["data"]) {
                        Ok(output) => println!("{output}"),
                        Err(code) => api_error(code),
                    }
                } else if value["data"]["unload_requested"] == true {
                    println!("Model unload requested.");
                } else {
                    api_error(aios_protocol::contracts::ErrorCode::ModelOutputInvalid);
                }
                return;
            },
            Err(code) => api_error(code),
        }
    }
    let service = match args.as_slice() {
        [a, b, unit] if a == "inspect" && b == "service" =>
            Some((unit.as_str(), None, false)),
        [a, b, unit, flag] if a == "inspect" && b == "service" && flag == "--json" =>
            Some((unit.as_str(), None, true)),
        [a, b, unit, socket_flag, socket] if a == "inspect" && b == "service" && socket_flag == "--socket" =>
            Some((unit.as_str(), Some(socket.as_str()), false)),
        [a, b, unit, flag, socket_flag, socket]
            if a == "inspect" && b == "service" && flag == "--json" && socket_flag == "--socket" =>
            Some((unit.as_str(), Some(socket.as_str()), true)),
        _ => None,
    };
    if let Some((unit, socket, json)) = service {
        let path = socket.map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(format!("/run/user/{}/aios/session.sock", std::fs::metadata("/proc/self").map(|m| { use std::os::unix::fs::MetadataExt; m.uid() }).unwrap_or(u32::MAX))));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut client = aios_session::Client::connect(&path)?;
            let resolved = client.call(serde_json::json!({"kind":"resolve_service","unit_name":unit}))?;
            if resolved.error.is_some() {
                if json { println!("{}", serde_json::to_string(&resolved)?); }
                return Err("service could not be resolved".into());
            }
            let id = resolved.data.and_then(|v| v["service_id"].as_str().map(str::to_owned)).ok_or("missing service handle")?;
            let response = client.call(serde_json::json!({"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":id}}}))?;
            if response.error.is_some() {
                if json { println!("{}", serde_json::to_string(&response)?); }
                return Err("service scope denied".into());
            }
            let value = response.data.ok_or("missing provider result")?;
            let failed = value["status"] == "error";
            if json { println!("{}", serde_json::to_string(&value)?); }
            else if !failed { println!("{}", human_service_output(&value).map_err(|_| "malformed service observation")?); }
            if failed { return Err("service observation failed".into()); }
            Ok(())
        })();
        if let Err(error) = result { eprintln!("aiosctl: {error}"); std::process::exit(1); }
        return;
    }
    eprintln!("Usage: aiosctl status --json | ask [--mode read-only] TEXT [--json] | ask TEXT --json --service UNIT | package search QUERY [--json] | package info ID [--json] | graph status [--json] | privacy scopes [--json] | history list [--json] | model status [--json] | model unload [--json] | ui select-session SESSION --json | ui read-window SESSION EXACT_TITLE GOAL --json | process list --json | process terminate SESSION BOOT_UUID PID START_TICKS GOAL --json | system info --json | inspect service UNIT [--json] [--socket PRIVATE_PATH]");
    std::process::exit(2);
}

fn ask_arguments(args: &[String]) -> Option<(&str, bool)> {
    match args {
        [command, text] if command == "ask" => Some((text, false)),
        [command, text, flag] if command == "ask" && flag == "--json" => Some((text, true)),
        [command, mode, value, text] if command == "ask" && mode == "--mode" && value == "read-only" =>
            Some((text, false)),
        [command, mode, value, text, flag]
            if command == "ask" && mode == "--mode" && value == "read-only" && flag == "--json" =>
            Some((text, true)),
        _ => None,
    }
}

fn run_ask(text: &str) -> Result<serde_json::Value, aios_protocol::contracts::ErrorCode> {
    let client = aios_session::bus::Client::connect_user_bus()?;
    let task = client.submit(&aios_session::Submit {
        mode: aios_session::Mode::Ask,
        text: text.to_owned(),
        client_nonce: new_nonce(),
        context_handles: vec![],
        retain_for_history: false,
        history_handles: vec![],
        selected_app_handle: None,
        selected_session_handle: None,
    })?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(95);
    loop {
        let status = client.status(&task)?;
        if matches!(status["state"].as_str(), Some("completed" | "failed" | "cancelled")) {
            return Ok(status);
        }
        if std::time::Instant::now() >= deadline {
            let _ = client.cancel(&task);
            return Err(aios_protocol::contracts::ErrorCode::DeadlineExceeded);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn human_ask_output(status: &serde_json::Value) -> Result<String, aios_protocol::contracts::ErrorCode> {
    use aios_protocol::contracts::ErrorCode;
    let response = status["output"]["response"].as_object().ok_or(ErrorCode::InvalidArgument)?;
    let text = match response.get("kind").and_then(serde_json::Value::as_str) {
        Some("answer") => response.get("text"),
        Some("clarification") => response.get("question"),
        Some("abstain") => response.get("reason"),
        _ => None,
    }.and_then(serde_json::Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
    let mut rendered = text.to_owned();
    if let Some(ids) = response.get("evidence_ids").and_then(serde_json::Value::as_array).filter(|ids| !ids.is_empty()) {
        rendered.push_str("\n\nEvidence:");
        for id in ids {
            let id = id.as_str().ok_or(ErrorCode::InvalidArgument)?;
            rendered.push_str("\n- ");
            rendered.push_str(id);
        }
    }
    Ok(rendered)
}

fn human_service_output(value: &serde_json::Value) -> Result<String, aios_protocol::contracts::ErrorCode> {
    use aios_protocol::contracts::ErrorCode;
    use std::fmt::Write;
    let data = value["data"].as_object().ok_or(ErrorCode::InvalidArgument)?;
    let text = |field| data.get(field).and_then(serde_json::Value::as_str).ok_or(ErrorCode::InvalidArgument);
    let number = |field| data.get(field).and_then(serde_json::Value::as_u64).ok_or(ErrorCode::InvalidArgument);
    let mut output = format!(
        "Unit: {}\nLoad: {}\nState: {} ({})\nResult: {}\nMain PID: {}\nRestarts: {}",
        text("unit_name")?,
        text("load_state")?,
        text("active_state")?,
        text("sub_state")?,
        text("result")?,
        number("main_pid")?,
        number("restart_count")?,
    );
    if let Some(ids) = value["evidence_ids"].as_array().filter(|ids| !ids.is_empty()) {
        output.push_str("\nEvidence:");
        for id in ids {
            write!(output, "\n- {}", id.as_str().ok_or(ErrorCode::InvalidArgument)?)
                .map_err(|_| ErrorCode::ResourceExhausted)?;
        }
    }
    Ok(output)
}

fn human_model_status(value: &serde_json::Value) -> Result<String, aios_protocol::contracts::ErrorCode> {
    use aios_protocol::contracts::ErrorCode;
    let loaded = value["loaded"].as_bool().ok_or(ErrorCode::ModelOutputInvalid)?;
    let busy = value["busy"].as_bool().ok_or(ErrorCode::ModelOutputInvalid)?;
    let queued = value["own_queued"].as_u64().ok_or(ErrorCode::ModelOutputInvalid)?;
    let limit = value["queue_limit"].as_u64().ok_or(ErrorCode::ModelOutputInvalid)?;
    let threads = value["threads"].as_u64().ok_or(ErrorCode::ModelOutputInvalid)?;
    let context = value["context_tokens"].as_u64().ok_or(ErrorCode::ModelOutputInvalid)?;
    Ok(format!(
        "Loaded: {}\nBusy: {}\nOwn queue: {queued}/{limit}\nThreads: {threads}\nContext tokens: {context}",
        if loaded { "yes" } else { "no" },
        if busy { "yes" } else { "no" },
    ))
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
    #[test]fn ask_surfaces_accept_only_the_fixed_read_only_mode(){
        let values=|args:&[&str]|args.iter().map(|value|(*value).to_owned()).collect::<Vec<_>>();
        assert_eq!(standalone_ask_arguments(&values(&["what failed?"])),Some(values(&["ask","what failed?"])));
        assert_eq!(standalone_ask_arguments(&values(&["what failed?","--json"])),Some(values(&["ask","what failed?","--json"])));
        assert_eq!(standalone_ask_arguments(&values(&["what failed?","--mode","read-only"])),Some(values(&["ask","what failed?"])));
        assert_eq!(standalone_ask_arguments(&values(&["what failed?","--mode","read-only","--json"])),Some(values(&["ask","what failed?","--json"])));
        assert_eq!(ask_arguments(&values(&["ask","what failed?"])),Some(("what failed?",false)));
        assert_eq!(ask_arguments(&values(&["ask","--mode","read-only","what failed?","--json"])),Some(("what failed?",true)));
        for denied in [
            values(&["id","--mode","shell","--json"]),
            values(&["id","--shell","--json"]),
            values(&["id","--mode","read-only","--json","--approved"]),
        ]{assert_eq!(standalone_ask_arguments(&denied),None);}
        assert_eq!(ask_arguments(&values(&["ask","--mode","shell","id"])),None);
    }
    #[test]fn human_ask_output_preserves_answer_and_citations(){
        let status=json!({"state":"completed","output":{"response":{"kind":"answer","text":"The unit failed.","evidence_ids":["ev-1","ev-2"]}}});
        assert_eq!(human_ask_output(&status).unwrap(),"The unit failed.\n\nEvidence:\n- ev-1\n- ev-2");
        let clarification=json!({"state":"completed","output":{"response":{"kind":"clarification","question":"Which unit?","evidence_ids":[]}}});
        assert_eq!(human_ask_output(&clarification).unwrap(),"Which unit?");
        assert_eq!(human_ask_output(&json!({"state":"completed","output":{}})),Err(ErrorCode::InvalidArgument));
    }
    #[test]fn human_service_output_reports_native_state_and_evidence(){
        let value=json!({"status":"ok","evidence_ids":["ev-service"],"data":{
            "unit_name":"sshd.service","load_state":"loaded","active_state":"active",
            "sub_state":"running","result":"success","main_pid":42,"restart_count":1
        }});
        assert_eq!(human_service_output(&value).unwrap(),
            "Unit: sshd.service\nLoad: loaded\nState: active (running)\nResult: success\nMain PID: 42\nRestarts: 1\nEvidence:\n- ev-service");
        assert_eq!(human_service_output(&json!({"status":"ok","data":{}})),Err(ErrorCode::InvalidArgument));
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
