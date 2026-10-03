//! Generate concrete Rust structures from the installed normative schemas.
use serde_json::Value;
use sha2::{Digest,Sha256};
use std::{env, fs, path::PathBuf};
fn name(s: &str) -> String { s.split(|c:char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()).map(|s| { let mut c=s.chars(); c.next().unwrap().to_uppercase().collect::<String>()+c.as_str() }).collect() }
struct Generator { definitions:String, serial:usize }
impl Generator {
 fn ty(&mut self, schema:&Value, hint:&str, root:&Value) -> String {
  if let Some(r)=schema.get("$ref").and_then(Value::as_str) { return self.ty(root.pointer(r.strip_prefix('#').expect("local reference only")).expect("definition"),hint,root); }
  if let Some(union)=schema.get("oneOf").and_then(Value::as_array) {
   if union.len()==2 && union[1].get("type").and_then(Value::as_str)==Some("null") {return format!("Option<{}>",self.ty(&union[0],hint,root));}
   let n=self.unique(hint); let mut variants=String::new();
   for (i,s) in union.iter().enumerate() {variants+=&format!("V{}({}),\n",i,self.ty(s,&format!("{n}V{i}"),root));}
   self.definitions+=&format!("#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,PartialEq)]\n#[serde(untagged)]\npub enum {n} {{ {variants} }}\n");return n;
  }
  if let Some(values)=schema.get("enum").and_then(Value::as_array) {
   if values.iter().all(Value::is_string) {
    let n=self.unique(hint); let mut variants=String::new();
    for (i,v) in values.iter().enumerate() {variants+=&format!("#[serde(rename={})] V{},\n",v,i);}
    self.definitions+=&format!("#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,PartialEq,Eq)]\npub enum {n} {{{variants}}}\n");return n;
   }
  }
  if let Some(v)=schema.get("const") {return match v {Value::Bool(_)=>"bool",Value::Number(_)=>"u64",Value::String(_)=>"String",_=>panic!("unsupported constant")}.to_string();}
  if let Some(types)=schema.get("type").and_then(Value::as_array) {
   assert_eq!(types.len(),2);assert!(types.contains(&Value::String("null".into())));
   let mut base=schema.clone();base["type"]=types.iter().find(|v|v.as_str()!=Some("null")).unwrap().clone();return format!("Option<{}>",self.ty(&base,hint,root));
  }
  match schema.get("type").and_then(Value::as_str).expect("concrete schema type") {
   "string"=>"String".into(),"boolean"=>"bool".into(),"integer"=>if schema.get("minimum").and_then(Value::as_i64).is_some_and(|n|n<0){"i64"}else{"u64"}.into(),"number"=>"f64".into(),"null"=>"()".into(),
   "array"=>format!("Vec<{}>",self.ty(&schema["items"],&format!("{hint}Item"),root)),
   "object"=>{
    if schema.get("x-aios-dynamic-schema").is_some() {return "serde_json::Map<String,serde_json::Value>".into();}
    assert_eq!(schema["additionalProperties"],false,"closed object contract");
    let n=self.unique(hint);let props=schema["properties"].as_object().expect("properties");let required=schema["required"].as_array().expect("required");let mut fields=String::new();
    for (key,s) in props {let t=self.ty(s,&format!("{n}{}",name(key)),root);let is_required=required.contains(&Value::String(key.clone()));
     let attr=if is_required {String::new()}else if let Some(default)=s.get("default") {let f=format!("default_{}",self.serial);self.serial+=1;let encoded=serde_json::to_string(&default.to_string()).unwrap();self.definitions+=&format!("fn {f}()->{t} {{serde_json::from_str({encoded}).expect(\"reviewed schema default\")}}\n");format!("#[serde(default=\"{f}\")]\n")}else{"#[serde(default,skip_serializing_if=\"Option::is_none\")]\n".into()};
     let t=if !is_required && s.get("default").is_none(){format!("Option<{t}>")}else{t};
     let field=if key=="type"{"r#type"}else{key};fields+=&format!("{attr}pub {field}: {t},\n");
    }
    self.definitions+=&format!("#[derive(Debug,Clone,serde::Serialize,serde::Deserialize,PartialEq)]\n#[serde(deny_unknown_fields)]\npub struct {n} {{{fields}}}\n");n
   },t=>panic!("unsupported schema type {t}"),
  }
 }
 fn unique(&mut self,hint:&str)->String {let result=format!("{hint}{}",self.serial);self.serial+=1;result}
}
fn main() {
 let root=PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../schemas");
 println!("cargo:rerun-if-changed={}",root.display());
 for k in ["src/contracts.rs","src/validation.rs","src/registry.rs"]{println!("cargo:rerun-if-changed={}",PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join(k).display());}
 let registry_text=fs::read_to_string(root.join("registry/actions.json")).unwrap();
 let registry:Value=serde_json::from_str(&registry_text).unwrap();
 let mut g=Generator{definitions:String::new(),serial:0};let mut variants=String::new();let mut dispatch=String::new();let mut ids=String::new();let mut values=String::new();let mut schemas=String::new();
 for action in registry["actions"].as_array().unwrap() {
  let id=action["action_id"].as_str().unwrap();let n=name(id);
  let mut hash=Sha256::new();
  for k in ["arguments","result"]{hash.update(fs::read(root.join(format!("actions/{id}.{k}.json"))).unwrap());}
  for k in ["src/contracts.rs","src/validation.rs","src/registry.rs","build.rs"]{hash.update(fs::read(PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join(k)).unwrap());}
  assert_eq!(action["implementation_hash"].as_str().unwrap(),format!("{:x}",hash.finalize()),"reviewed contract hash changed for {id}");
  for kind in ["arguments","data","result"] {let text=fs::read_to_string(root.join(format!("actions/{id}.{kind}.json"))).unwrap();let schema:Value=serde_json::from_str(&text).unwrap();
   let t=g.ty(&schema,&format!("{n}{}",name(kind)),&schema);
   g.definitions+=&format!("pub type {n}{} = {t};\n",name(kind));
   schemas+=&format!("({id:?},{kind:?})=>Some({text:?}),\n");
   let func=format!("parse_{}_{}",id.replace('.',"_"),kind);
   let size_guard=if kind=="arguments"{"if bytes.len()>crate::MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted);}"}else{""};
   let semantic=if kind=="arguments"{format!("crate::validation::semantic_arguments({id:?},&v)?;")}else{String::new()};
   let validation=if kind=="result"{format!("crate::validation::validate_result({id:?},bytes)?")}else{format!("{{let v=crate::validation::strict_json(bytes)?;crate::validation::validate({text:?},&v)?;v}}")};
   g.definitions+=&format!("pub fn {func}(bytes:&[u8])->Result<{t},ErrorCode>{{{size_guard}let v={validation};{semantic}serde_json::from_value(v).map_err(|_|ErrorCode::InvalidArgument)}}\n");
  }
  if id=="system.info" {variants+=&format!("{n},\n");dispatch+=&format!("{id:?}=>{{serde_json::from_value::<{n}Arguments>(value).map_err(|_|ErrorCode::InvalidArgument)?;Ok(Action::{n})}},\n");ids+=&format!("Self::{n}=>{id:?},\n");values+=&format!("Self::{n}=>serde_json::json!({{}}),\n");}
  else {variants+=&format!("{n}({n}Arguments),\n");dispatch+=&format!("{id:?}=>Ok(Action::{n}(serde_json::from_value(value).map_err(|_|ErrorCode::InvalidArgument)?)),\n");ids+=&format!("Self::{n}(_)=>{id:?},\n");values+=&format!("Self::{n}(args)=>serde_json::to_value(args).expect(\"generated arguments\"),\n");}
 }
 let generated=format!("{}\n#[derive(Debug,Clone,PartialEq)]pub enum Action {{{variants}}}\nimpl Action {{ pub fn action_id(&self)->&'static str{{match self{{{ids}}}}} pub fn arguments_value(&self)->serde_json::Value{{match self{{{values}}}}} }}\npub(crate) fn typed_action(id:&str,value:serde_json::Value)->Result<Action,ErrorCode>{{match id{{{dispatch}_=>Err(ErrorCode::UnknownCapability)}}}}\npub fn schema_source(id:&str,kind:&str)->Option<&'static str>{{match(id,kind){{{schemas}_=>None}}}}\npub const REGISTRY_SOURCE:&str={registry_text:?};\n",g.definitions);
 fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("contracts.rs"),generated).unwrap();
}
