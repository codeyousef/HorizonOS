//! Reviewed contract metadata. Registration does not enable execution.
use crate::contracts::{Action,ErrorCode,REGISTRY_SOURCE,schema_source};
use serde::{Deserialize,Serialize};
use serde_json::Value;
use std::sync::OnceLock;
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
 pub action_id:String,pub api_version:u32,pub executor_kind:String,pub implementation_id:String,
 pub input_schema:String,pub output_schema:String,pub required_scopes:Vec<String>,pub risk_class:String,
 pub read_only:bool,pub idempotency_class:String,pub timeout_ms:u64,pub preconditions:Vec<String>,
 pub verification_id:String,pub recovery_class:String,pub recovery_adapter_id:Option<String>,
 pub requires_interactive_session:bool,pub supports_automation:bool,pub source_revision:String,
 pub implementation_hash:String,pub availability:String,
}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Registry {schema_version:u32,actions:Vec<Capability>}
static REGISTRY:OnceLock<Registry>=OnceLock::new();
pub fn capabilities()->&'static [Capability]{&REGISTRY.get_or_init(||{
 let r:Registry=serde_json::from_str(REGISTRY_SOURCE).expect("reviewed generated registry");assert_eq!(r.schema_version,1);r
}).actions}
pub fn capability(id:&str)->Result<&'static Capability,ErrorCode>{capabilities().iter().find(|c|c.action_id==id).ok_or(ErrorCode::UnknownCapability)}
pub const PROTOCOL_VERSION:u32=1;
pub fn negotiate_version(schema_version:u32,protocol_version:u32)->Result<(),ErrorCode>{
 if schema_version==crate::SCHEMA_VERSION && protocol_version==PROTOCOL_VERSION {Ok(())}else{Err(ErrorCode::UnsupportedSchema)}
}
/// Supplied by trusted code with the authenticated requester and enrolled grants.
/// Implementations MUST check owner, kind, query/app binding, expiry and revocation.
/// Resolving a string is not evidence of authorization to perform a mutation.
pub trait ResourceResolver {
 fn resolve(&self,field:&str,kind:&str,reference:&str)->Result<(),ErrorCode>;
 fn dynamic_arguments(&self,action_id:&str,arguments:&Value)->Result<(),ErrorCode>;
}
pub fn validate_references(action:&Action,resolver:&impl ResourceResolver)->Result<(),ErrorCode>{
 fn walk(schema:&Value,value:&Value,field:&str,id:&str,r:&impl ResourceResolver)->Result<(),ErrorCode>{
  if let Some(kind)=schema.get("x-aios-resource").and_then(Value::as_str){
   let reference=value.as_str().ok_or(ErrorCode::InvalidArgument)?;
   if !((field=="boot_id")&&(reference=="current"||(reference=="previous"&&id=="system.boot_diagnostics"))){r.resolve(field,kind,reference)?;}
  }
  if let Some(props)=schema.get("properties").and_then(Value::as_object){for(k,s)in props{if let Some(v)=value.get(k){walk(s,v,k,id,r)?;}}}
  if let Some(items)=schema.get("items"){if let Some(values)=value.as_array(){for v in values{walk(items,v,field,id,r)?;}}}
  if let Some(variants)=schema.get("oneOf").and_then(Value::as_array){
   let mut selected=None;
   for s in variants {let validator=jsonschema::draft202012::options().should_validate_formats(true).build(s).map_err(|_|ErrorCode::UnsupportedSchema)?;if validator.is_valid(value){if selected.is_some(){return Err(ErrorCode::InvalidArgument);}selected=Some(s);}}
   walk(selected.ok_or(ErrorCode::InvalidArgument)?,value,field,id,r)?;
  }
  Ok(())
 }
 let id=action.action_id();let value=action.arguments_value();let schema:Value=serde_json::from_str(schema_source(id,"arguments").ok_or(ErrorCode::UnknownCapability)?).map_err(|_|ErrorCode::UnsupportedSchema)?;
 walk(&schema,&value,"arguments",id,resolver)?;
 // Registered app actions, setting/option/preference catalog IDs, accessible
 // roles/states and node values require deterministic contextual validation.
 if matches!(id,"apps.invoke"|"ui.find"|"ui.activate"|"ui.set_value"|"config.get"|"config.propose"|"settings.get"|"settings.set"|"memory.remember"|"power.profile_set"|"automation.propose") {resolver.dynamic_arguments(id,&value)?;}
 Ok(())
}
