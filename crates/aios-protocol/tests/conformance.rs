//! Synthetic schemas are explicitly labelled. These checks never execute actions.
use aios_protocol::{MAX_TASK_BYTES, contracts::{Action,ErrorCode,parse_tool_call,canonical_json,schema_source},registry::{capabilities,capability,negotiate_version,ResourceResolver,validate_references},validation::{strict_json,validate,validate_result}};
use serde_json::{json,Value};
fn fixtures()->Value{serde_json::from_str(include_str!("../../../schemas/compatibility-v1.json")).unwrap()}
#[test]
fn service_ordering_and_job_evidence_are_strict_and_not_causal() {
 let f=fixtures();let base=f["fixtures"].as_array().unwrap().iter().find(|v|v["action_id"]=="system.service_status").unwrap()["result"].clone();
 for (key,value) in [("ordering_is_not_causation",json!(false)),("ordering_after",json!(["same.target","same.target"])),("invocation_id",json!("not-a-service-invocation"))] {
  let mut v=base.clone();v["data"][key]=value;assert!(validate_result("system.service_status",&serde_json::to_vec(&v).unwrap()).is_err());
 }
 let mut v=base;v["data"]["job"]=json!({"id":1,"object_path":"/org/freedesktop/systemd1/job/1","job_type":"start","state":"waiting"});
 validate_result("system.service_status",&serde_json::to_vec(&v).unwrap()).unwrap();
 v["data"]["job"]["state"]=json!("completed");assert!(validate_result("system.service_status",&serde_json::to_vec(&v).unwrap()).is_err());
}
fn call(id:&str,args:Value)->Vec<u8>{serde_json::to_vec(&json!({"kind":"tool_call","action_id":id,"arguments":args})).unwrap()}
#[test]
fn every_normative_contract_generates_validated_types_and_round_trips(){
 let fs=fixtures();let entries=fs["fixtures"].as_array().unwrap();assert_eq!(entries.len(),59);assert_eq!(capabilities().len(),entries.len());
 for f in entries {
  let id=f["action_id"].as_str().unwrap();let bytes=call(id,f["arguments"].clone());let action=parse_tool_call(&bytes).unwrap_or_else(|e|panic!("{id} input: {e:?}"));assert_eq!(action.action_id(),id);
  let output=action.arguments_value();validate(schema_source(id,"arguments").unwrap(),&output).unwrap_or_else(|e|panic!("{id} generated defaults: {e:?}"));
  assert_eq!(parse_tool_call(&call(id,output)).unwrap(),action);
  validate_result(id,&serde_json::to_vec(&f["result"]).unwrap()).unwrap_or_else(|e|panic!("{id} result: {e:?}"));
  let mut extra=f["arguments"].clone();extra.as_object_mut().unwrap().insert("approved".into(),true.into());assert_eq!(parse_tool_call(&call(id,extra)),Err(ErrorCode::InvalidArgument),"{id} unknown authority");
  let mut result=f["result"].clone();result["data"]["unreviewed"]=true.into();assert!(validate_result(id,&serde_json::to_vec(&result).unwrap()).is_err(),"{id} unknown result data");
 }
}
#[test]
fn metadata_is_closed_fixed_and_does_not_advertise_unimplemented_providers(){
 let mut ids=std::collections::BTreeSet::new();
 for c in capabilities(){assert!(ids.insert(&c.action_id));assert_eq!(c.api_version,1);assert!(c.timeout_ms>0 && c.timeout_ms<=60000);assert!(!c.required_scopes.is_empty());assert!(!c.preconditions.is_empty());assert!(!c.verification_id.is_empty());assert_eq!(c.implementation_hash.len(),64);assert!(!["shell","exec","sql","nix","dbus"].contains(&c.executor_kind.as_str()));assert_eq!(c.availability,if ["system.info","system.service_status"].contains(&c.action_id.as_str()){"implemented"}else{"contract-only"});}
 assert_eq!(negotiate_version(1,1),Ok(()));for(a,b)in [(0,1),(2,1),(1,0),(1,2)]{assert_eq!(negotiate_version(a,b),Err(ErrorCode::UnsupportedSchema));}
 assert!(capability("shell.run").is_err());
}
#[test]
fn malformed_ranges_limits_duplicates_and_wrong_types_fail_before_execution(){
 for (id,args) in [
  ("system.boots",json!({"limit":0})),("system.boots",json!({"limit":11})),("system.boots",json!({"limit":1.0})),
  ("system.logs",json!({})),("system.logs",json!({"source":"system","max_entries":201})),("system.logs",json!({"source":"system","since":"2026-10-03"})),
  ("system.logs",json!({"source":"system","since":"2026-10-03T02:00:00Z","until":"2026-10-03T04:00:00+03:00"})),
  ("files.search",json!({"query":"a","root_handles":["fixture"],"modified_after":"2026-10-03T02:00:00Z","modified_before":"2026-10-03T01:00:00Z"})),
  ("process.terminate",json!({"process_id":"fixture","graceful":false})),
  ("packages.install",json!({"package_ids":["fixture","fixture"]})),("config.propose",json!({"changes":[{"option_id":"boot.kernelParams","value":"bad"}]})),
  ("settings.set",json!({"key":"display.idle_seconds","value":59})),("settings.set",json!({"key":"desktop.theme_mode","value":0})),
  ("files.move",json!({"source_handle":"a","destination_handle":"b","basename":"..","collision_policy":"fail"})),
  ("files.copy",json!({"source_handle":"a","destination_handle":"b","basename":"path/file","collision_policy":"overwrite"})),
  ("files.read",json!({"file_handle":"a","range":{"kind":"lines","start":4,"end":3}})),
  ("files.read",json!({"file_handle":"a","range":{"kind":"cells","sheet":"a","a1":"B4:A1"}})),
  ("files.read",json!({"file_handle":"a","range":{"kind":"cells","sheet":"a","a1":"A1:XFD1048576"}})),
  ("files.read",json!({"file_handle":"a","range":{"kind":"cells","sheet":"a","a1":"SUM(A1:B2)"}})),
  ("ui.select_text",json!({"snapshot_id":"a","node_handle":"b","start_offset":4,"end_offset":4})),
  ("ui.visual_step",json!({"portal_session_handle":"a","frame_id":"b","window_handle":"c","action":{"kind":"key","key":"Ctrl+Alt+Delete"}})),
  ("ui.visual_step",json!({"portal_session_handle":"a","frame_id":"b","window_handle":"c","action":{"kind":"scroll","direction":"up","steps":6}})),
  ("automation.propose",json!({"rule":{"schema_version":1,"rule_id":"model-issued","enabled":true}})),
 ]{assert_eq!(parse_tool_call(&call(id,args)),Err(ErrorCode::InvalidArgument),"{id}");}
 assert!(matches!(parse_tool_call(&call("system.hardware",json!({}))),Ok(Action::SystemHardware(_))));
 for raw in [r#"{"kind":"tool_call","action_id":"ui.find","arguments":{"snapshot_id":"a","selector":{"name":"x","name":"y"}}}"#,r#"{"kind":"tool_call","action_id":"apps.invoke","arguments":{"app_id":"a","action_id":"b","arguments":{"x":1,"x":2}}}"#]{assert_eq!(parse_tool_call(raw.as_bytes()),Err(ErrorCode::InvalidArgument));}
 assert_eq!(parse_tool_call(&call("shell.run",json!({"command":"anything"}))),Err(ErrorCode::UnknownCapability));
 assert_eq!(parse_tool_call(&vec![b' ';MAX_TASK_BYTES+1]),Err(ErrorCode::ResourceExhausted));
 assert!(strict_json(br#"{"x":{"uid":1,"uid":0}}"#).is_err());
}
struct Deny;
impl ResourceResolver for Deny{fn resolve(&self,_:&str,_:&str,_:&str)->Result<(),ErrorCode>{Err(ErrorCode::PermissionDenied)}fn dynamic_arguments(&self,_:&str,_:&Value)->Result<(),ErrorCode>{Err(ErrorCode::UnsupportedCapability)}}
#[test]
fn syntactic_handles_are_not_authority_and_dynamic_schemas_fail_closed(){
 for(id,args)in [("files.metadata",json!({"file_handle":"model-made-up"})),("packages.info",json!({"package_id":"fixture"})),("files.trash",json!({"file_handles":["fixture"]})),("memory.remember",json!({"key":"audio.preferred_output","value":"fixture"}))]{let a=parse_tool_call(&call(id,args)).unwrap();assert_eq!(validate_references(&a,&Deny),Err(ErrorCode::PermissionDenied));}
 let a=parse_tool_call(&call("settings.set",json!({"key":"desktop.theme_mode","value":"dark"}))).unwrap();assert_eq!(validate_references(&a,&Deny),Err(ErrorCode::UnsupportedCapability));
}
#[test]
fn result_completeness_and_mutation_verification_cannot_claim_success(){
 for f in fixtures()["fixtures"].as_array().unwrap(){let id=f["action_id"].as_str().unwrap();let mut r=f["result"].clone();r["status"]="partial".into();r["complete"]=false.into();assert!(validate_result(id,&serde_json::to_vec(&r).unwrap()).is_err(),"{id}: unexplained partial");r["error"]=json!({"code":"PARTIAL_RESULT","message":"Missing fixture scope","retryable":false});assert!(validate_result(id,&serde_json::to_vec(&r).unwrap()).is_ok(),"{id}: explained partial");r["complete"]=true.into();assert!(validate_result(id,&serde_json::to_vec(&r).unwrap()).is_err());}
 let mut r=fixtures()["fixtures"].as_array().unwrap().iter().find(|f|f["action_id"]=="files.copy").unwrap()["result"].clone();r["data"]["verification"]["outcome"]="unknown".into();assert!(validate_result("files.copy",&serde_json::to_vec(&r).unwrap()).is_err());r["status"]="pending".into();r["complete"]=false.into();r["data"]["verification"]["outcome"]="pending".into();assert!(validate_result("files.copy",&serde_json::to_vec(&r).unwrap()).is_ok());
}
#[test]
fn canonical_signed_units_reject_floats_and_bytes_are_stable(){
 let v=strict_json(br#"{"z":"\u0645","a":{"duration_ms":20,"bytes":18446744073709551615}}"#).unwrap();assert_eq!(canonical_json(&v).unwrap(),"{\"a\":{\"bytes\":18446744073709551615,\"duration_ms\":20},\"z\":\"م\"}".as_bytes());assert!(canonical_json(&json!({"duration_ms":20.0})).is_err());assert_eq!(canonical_json(&json!({"a":-1,"b":0})).unwrap(),br#"{"a":-1,"b":0}"#);
}

#[test]
fn omitted_values_use_reviewed_server_defaults() {
 for(id,input,expected)in [
  ("system.hardware",json!({}),json!({"device_class":"all"})),
  ("system.boots",json!({}),json!({"limit":5})),
  ("system.services",json!({}),json!({"scope":"system","state":"all","limit":100})),
  ("process.terminate",json!({"process_id":"fixture"}),json!({"process_id":"fixture","graceful":true})),
  ("system.logs",json!({"source":"kernel"}),json!({"source":"kernel","boot_id":"current","max_entries":20,"priority_max":7})),
  ("packages.search",json!({"query":"fixture"}),json!({"query":"fixture","limit":20})),
  ("files.summarize",json!({"file_handle":"fixture"}),json!({"file_handle":"fixture","detail":"standard"})),
 ] {assert_eq!(parse_tool_call(&call(id,input)).unwrap().arguments_value(),expected,"{id}");}
}

#[test]
fn generated_parsing_functions_enforce_semantics_and_frame_limits() {
 use aios_protocol::contracts::{parse_files_read_arguments,parse_ui_select_text_arguments,parse_system_info_result};
 assert!(parse_files_read_arguments(br#"{"file_handle":"a","range":{"kind":"lines","start":4,"end":3}}"#).is_err());
 assert!(parse_ui_select_text_arguments(br#"{"snapshot_id":"a","node_handle":"b","start_offset":4,"end_offset":4}"#).is_err());
 let f=fixtures();let info=f["fixtures"].as_array().unwrap().iter().find(|f|f["action_id"]=="system.info").unwrap();
 assert!(parse_system_info_result(&serde_json::to_vec(&info["result"]).unwrap()).is_ok());
 let mut oversized=vec![b' ';MAX_TASK_BYTES+1];oversized.extend_from_slice(br#"{"file_handle":"a","range":{"kind":"lines","start":1,"end":1}}"#);assert!(parse_files_read_arguments(&oversized).is_err());
}
