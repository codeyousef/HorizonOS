//! Closed local schemas and bounded, duplicate-safe JSON. No network/file schema resolution.
use crate::{MAX_FRAME_BYTES, contracts::ErrorCode};
use serde::{Deserialize, de::{self, MapAccess, SeqAccess, Visitor}};
use serde_json::{Map, Value};
use std::{collections::HashMap, fmt, sync::{Arc, Mutex, OnceLock}};

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
 fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error>{
  struct V;
  impl<'de> Visitor<'de> for V {
   type Value=Strict;
   fn expecting(&self,f:&mut fmt::Formatter)->fmt::Result{f.write_str("JSON without duplicate object keys")}
   fn visit_bool<E:de::Error>(self,v:bool)->Result<Strict,E>{Ok(Strict(v.into()))}
   fn visit_i64<E:de::Error>(self,v:i64)->Result<Strict,E>{Ok(Strict(v.into()))}
   fn visit_u64<E:de::Error>(self,v:u64)->Result<Strict,E>{Ok(Strict(v.into()))}
   fn visit_f64<E:de::Error>(self,v:f64)->Result<Strict,E>{serde_json::Number::from_f64(v).map(|n|Strict(Value::Number(n))).ok_or_else(||E::custom("non-finite JSON number"))}
   fn visit_str<E:de::Error>(self,v:&str)->Result<Strict,E>{Ok(Strict(v.into()))}
   fn visit_string<E:de::Error>(self,v:String)->Result<Strict,E>{Ok(Strict(v.into()))}
   fn visit_unit<E:de::Error>(self)->Result<Strict,E>{Ok(Strict(Value::Null))}
   fn visit_none<E:de::Error>(self)->Result<Strict,E>{self.visit_unit()}
   fn visit_seq<A:SeqAccess<'de>>(self,mut a:A)->Result<Strict,A::Error>{let mut v=Vec::new();while let Some(Strict(x))=a.next_element()?{v.push(x);}Ok(Strict(Value::Array(v)))}
   fn visit_map<A:MapAccess<'de>>(self,mut a:A)->Result<Strict,A::Error>{let mut v=Map::new();while let Some(k)=a.next_key::<String>()?{if v.contains_key(&k){return Err(de::Error::custom("duplicate key"));}let Strict(x)=a.next_value()?;v.insert(k,x);}Ok(Strict(Value::Object(v)))}
  }
  d.deserialize_any(V)
 }
}
pub fn strict_json(bytes:&[u8])->Result<Value,ErrorCode>{
 if bytes.is_empty() || bytes.len()>MAX_FRAME_BYTES{return Err(ErrorCode::ResourceExhausted);}
 let Strict(v)=serde_json::from_slice(bytes).map_err(|_|ErrorCode::InvalidArgument)?;Ok(v)
}
// Keys are only static compiled-in contracts, never caller supplied schema text.
static VALIDATORS:OnceLock<Mutex<HashMap<&'static str,Arc<jsonschema::Validator>>>>=OnceLock::new();
pub fn validate(schema:&'static str,value:&Value)->Result<(),ErrorCode>{
 let cache=VALIDATORS.get_or_init(||Mutex::new(HashMap::new()));
 let mut guard=cache.lock().map_err(|_|ErrorCode::UnsupportedSchema)?;
 let validator=if let Some(v)=guard.get(schema){v.clone()}else{
  let document:Value=serde_json::from_str(schema).map_err(|_|ErrorCode::UnsupportedSchema)?;
  let v=Arc::new(jsonschema::draft202012::options().should_validate_formats(true).build(&document).map_err(|_|ErrorCode::UnsupportedSchema)?);
  guard.insert(schema,v.clone());v
 };drop(guard);
 if validator.is_valid(value){Ok(())}else{Err(ErrorCode::InvalidArgument)}
}
fn ordered_indices(v:&Value)->Result<(),ErrorCode>{
 let start=v["start"].as_u64().ok_or(ErrorCode::InvalidArgument)?;let end=v["end"].as_u64().ok_or(ErrorCode::InvalidArgument)?;
 if end<start {return Err(ErrorCode::InvalidArgument);}Ok(())
}
fn cell(s:&str)->Option<(u64,u64)>{
 let split=s.find(|c:char|c.is_ascii_digit())?;let (col,row)=s.split_at(split);let col=col.bytes().try_fold(0u64,|n,c|n.checked_mul(26)?.checked_add(u64::from(c-b'A'+1)))?;let row=row.parse::<u64>().ok()?;
 if col>16384 || row>1048576 {None}else{Some((col,row))}
}
pub(crate) fn semantic_arguments(id:&str,v:&Value)->Result<(),ErrorCode>{
 if let Some(range)=v.get("range") {
  if range["kind"]=="cells" {let (start,end)=range["a1"].as_str().ok_or(ErrorCode::InvalidArgument)?.split_once(':').ok_or(ErrorCode::InvalidArgument)?;let (a,b)=(cell(start).ok_or(ErrorCode::InvalidArgument)?,cell(end).ok_or(ErrorCode::InvalidArgument)?);if b.0<a.0 || b.1<a.1 || (b.0-a.0+1)*(b.1-a.1+1)>100000 {return Err(ErrorCode::InvalidArgument);}}
  else {ordered_indices(range)?;}
 }
 if id=="ui.select_text" && v["end_offset"].as_u64().ok_or(ErrorCode::InvalidArgument)?<=v["start_offset"].as_u64().ok_or(ErrorCode::InvalidArgument)? {return Err(ErrorCode::InvalidArgument);}
 if (id=="files.move" || id=="files.copy") && v["basename"].as_str().ok_or(ErrorCode::InvalidArgument)?.len()>255 {return Err(ErrorCode::InvalidArgument);}
 if id=="config.propose" {let mut seen=std::collections::HashSet::new();for change in v["changes"].as_array().ok_or(ErrorCode::InvalidArgument)?{if !seen.insert(change["option_id"].as_str()){return Err(ErrorCode::InvalidArgument);}}}
 for (start,end) in [("since","until"),("modified_after","modified_before")] {
  if let (Some(a),Some(b))=(v.get(start).and_then(Value::as_str),v.get(end).and_then(Value::as_str)) {
   let parse=|s|time::OffsetDateTime::parse(s,&time::format_description::well_known::Rfc3339).map_err(|_|ErrorCode::InvalidArgument);
   if parse(a)? > parse(b)? {return Err(ErrorCode::InvalidArgument);}
  }
 }
 // Dynamic schemas must be resolved through an authenticated provider context;
 // the syntactic parser never establishes their scope or trust.
 Ok(())
}
pub fn validate_result(id:&str,bytes:&[u8])->Result<Value,ErrorCode>{
 let schema=crate::contracts::schema_source(id,"result").ok_or(ErrorCode::UnknownCapability)?;
 let v=strict_json(bytes)?;validate(schema,&v)?;
 if v["data"].is_object(){semantic_arguments(id,&v["data"])?;}
 if v["error"].is_object() && v["error"]["message"].as_str().is_none_or(|m|m.trim().is_empty()){return Err(ErrorCode::InvalidArgument);}
 if v["status"]=="pending" && v["data"]["verification"]["outcome"]!="pending" {return Err(ErrorCode::InvalidArgument);}
 if v["status"]=="ok" && crate::registry::capability(id)?.idempotency_class=="prepared-operation" && v["data"]["verification"]["outcome"]!="verified" {return Err(ErrorCode::InvalidArgument);}
 Ok(v)
}
