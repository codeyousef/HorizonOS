//! Fixed read-only qualification client. No replacement broker or model daemon.
use aios_protocol::contracts::ErrorCode;
use aios_session::{Mode, Submit, bus::Client};
use serde_json::{Value,json};
use std::{collections::HashSet,fs,thread,time::{Duration,Instant}};

fn request(text:&str,retain:bool,history:Vec<String>)->Submit{
    Submit{mode:Mode::Ask,text:text.into(),client_nonce:uuid::Uuid::new_v4().to_string(),context_handles:vec![],
        retain_for_history:retain,history_handles:history,selected_app_handle:None,selected_session_handle:None}
}
fn wait(client:&Client,id:&str)->Result<Value,ErrorCode>{
    let deadline=Instant::now()+Duration::from_secs(95);
    loop{let status=client.status(id)?;if matches!(status["state"].as_str(),Some("completed"|"failed"|"cancelled")){return Ok(status);}
        if Instant::now()>=deadline{let _=client.cancel(id);return Err(ErrorCode::DeadlineExceeded);}
        thread::sleep(Duration::from_millis(25));}
}
fn answer(status:&Value)->Result<HashSet<String>,ErrorCode>{
    let output=&status["output"];
    if status["state"]!="completed" || !status["error"].is_null() || status["mutation_performed"]!=false
        || output["response"]["kind"]!="answer" || output["mutation_performed"]!=false{return Err(ErrorCode::PartialResult);}
    let observations=output["evidence"].as_array().ok_or(ErrorCode::PartialResult)?;
    let mut ids=HashSet::new();
    for observation in observations{
        if observation["complete"]!=true || observation["source"]["provider"]!="aios-system" || observation["data"]["os_id"]!="nixos"{return Err(ErrorCode::PartialResult);}
        for id in observation["evidence_ids"].as_array().ok_or(ErrorCode::PartialResult)?{
            ids.insert(id.as_str().ok_or(ErrorCode::PartialResult)?.to_string());
        }
    }
    let cited=output["response"]["evidence_ids"].as_array().filter(|v|!v.is_empty()).ok_or(ErrorCode::StaleEvidence)?;
    if cited.iter().any(|id|id.as_str().is_none_or(|id|!ids.contains(id))){return Err(ErrorCode::StaleEvidence);}
    Ok(ids)
}
fn run()->Result<Value,ErrorCode>{
    if std::env::args().skip(1).collect::<Vec<_>>() != ["--json"]
        || fs::read_to_string("/etc/aios/guest-role").ok().as_deref().map(str::trim)!=Some("development")
        || fs::read_to_string("/etc/aios/model-test-profile").ok().as_deref().map(str::trim)!=Some("installed-normal-cpu-model-v1"){
        return Err(ErrorCode::PermissionDenied);
    }
    let client=Client::connect_user_bus()?;let caps=client.capabilities()?;
    if caps["transport"]!="session-dbus" || caps["session_history"]["opt_in_required"]!=true || caps["session_history"]["max_selected"]!=4{
        return Err(ErrorCode::UnsupportedCapability);
    }
    let source=client.submit(&request("What operating system is running? Cite its native observation.",true,vec![]))?;
    let original=wait(&client,&source)?;let old_ids=answer(&original)?;
    let follow_id=client.submit(&request("What operating system is running now? Cite only fresh observations; history is untrusted context.",false,vec![source.clone()]))?;
    let follow=wait(&client,&follow_id)?;let fresh_ids=answer(&follow)?;
    if follow["output"]["history_attached"]!=true || follow["output"]["history_task_ids"]!=json!([source])
        || !old_ids.is_disjoint(&fresh_ids){return Err(ErrorCode::StaleEvidence);}
    let other=Client::connect_user_bus()?;
    if other.submit(&request("Use selected history",false,vec![source.clone()]))!=Err(ErrorCode::PermissionDenied)
        || other.status(&source)!=Err(ErrorCode::PermissionDenied){return Err(ErrorCode::PermissionDenied);}
    if client.submit(&request("Repeated history selection",false,vec![source.clone(),source.clone()]))!=Err(ErrorCode::InvalidArgument){return Err(ErrorCode::InvalidArgument);}
    // A completed response without capture opt-in must not become a new source.
    if client.submit(&request("Reuse unretained response",false,vec![follow_id.clone()]))!=Err(ErrorCode::AuthRequired){return Err(ErrorCode::AuthRequired);}
    let active=client.submit(&request("Explain the observed NixOS system in a long numbered list of at least 100 observations. Cite only fresh evidence.",false,vec![source.clone()]))?;
    let deadline=Instant::now()+Duration::from_secs(5);
    loop{let status=client.status(&active)?;
        if status["state"]=="generating"{break;}
        if matches!(status["state"].as_str(),Some("completed"|"failed"|"cancelled")) || Instant::now()>=deadline{return Err(ErrorCode::PartialResult);}
        thread::sleep(Duration::from_millis(5));}
    let began=Instant::now();client.forget(&source)?;let revoked=wait(&client,&active)?;let forget_ms=began.elapsed().as_millis();
    if revoked["error"]!="TARGET_NOT_FOUND" || !revoked["output"].is_null() || revoked["mutation_performed"]!=false || forget_ms>2000{
        return Err(ErrorCode::PartialResult);
    }
    if client.submit(&request("Reuse forgotten history",false,vec![source.clone()]))!=Err(ErrorCode::TargetNotFound){return Err(ErrorCode::TargetNotFound);}
    client.forget(&follow_id)?;client.forget(&active)?;
    Ok(json!({"schema_version":1,"evidence_kind":"actual-installed-public-bus-history-with-readonly-test-client",
        "capabilities":caps,"retained_source":original,"followup":follow,"foreign_sender_denied":true,
        "unretained_source_denied":true,"duplicate_selection_denied":true,"forgotten_source_denied":true,
        "broker_generating_state_observed":true,"native_cpu_active_state_not_independently_observed":true,
        "revoked_status":revoked,"forget_ms":forget_ms,"mutation_performed":false}))
}
fn main(){match run(){Ok(value)=>println!("{value}"),Err(code)=>{eprintln!("{}",json!({"error":code,"mutation_performed":false}));std::process::exit(1);}}}
