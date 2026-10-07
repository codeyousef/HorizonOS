//! Fixed, user-scoped Nix profile inventory. The effective UID selects the
//! account and the provider accepts no caller path, user name or profile name.
use super::{ObservationTime,SourceRevision,native::{Error,NativeTime,Result},store::{GraphStore,Node,ProviderSnapshot,ProviderState,Scope,SourceTruth}};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{ffi::CStr,fs,os::unix::fs::MetadataExt,path::{Path,PathBuf}};

pub const PROVIDER:&str="native-user-nix-profiles";
const MAX_DIRECTORY_ENTRIES:usize=1024;
const MAX_GENERATIONS:usize=256;

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserProfileGeneration { pub generation:u64,pub closure:String,pub selected:bool,pub ownership:String,pub management_attribution:Option<String> }
const IDENTITY:Error=Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged);
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserProfile { pub profile_kind:String,pub selected_closure:String,pub selected_generation:u64,pub generations:Vec<UserProfileGeneration>,pub history_complete:bool }
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserProfiles { pub uid:u32,pub profiles:Vec<UserProfile>,pub unmanaged_inventory_complete:bool,pub unmanaged_inventory_reason:String }

fn account(uid:u32)->Result<(String,PathBuf)> {
    let entry=unsafe{libc::getpwuid(uid)};
    if entry.is_null(){return Err(IDENTITY);}
    let entry=unsafe{&*entry};
    let name=unsafe{CStr::from_ptr(entry.pw_name)}.to_str().map_err(|_|IDENTITY)?;
    let home=unsafe{CStr::from_ptr(entry.pw_dir)}.to_str().map_err(|_|IDENTITY)?;
    if name.is_empty() || name.len()>64 || !name.bytes().all(|b|b.is_ascii_alphanumeric()||b"_-".contains(&b)){return Err(IDENTITY);}
    let home=PathBuf::from(home);
    if !home.is_absolute() || home==Path::new("/"){return Err(IDENTITY);}
    let metadata=fs::symlink_metadata(&home).map_err(|_|IDENTITY)?;
    if !metadata.is_dir() || metadata.uid()!=uid || metadata.mode()&0o002!=0{return Err(IDENTITY);}
    Ok((name.into(),home))
}
fn store_closure(path:&Path)->Result<String>{
    let resolved=fs::canonicalize(path).map_err(|_|Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))?;
    let value=resolved.to_str().ok_or(IDENTITY)?;
    let name=value.strip_prefix("/nix/store/").ok_or(IDENTITY)?;
    let (hash,label)=name.split_once('-').ok_or(IDENTITY)?;
    if hash.len()!=32 || !hash.bytes().all(|b|b"0123456789abcdfghijklmnpqrsvwxyz".contains(&b)) || label.is_empty() || label.len()>200
        || !label.bytes().all(|b|b.is_ascii_alphanumeric()||b"+._-".contains(&b)){return Err(IDENTITY);}
    let metadata=fs::symlink_metadata(&resolved).map_err(|_|IDENTITY)?;
    if !metadata.is_dir() || metadata.uid()!=0 || metadata.mode()&0o022!=0{return Err(IDENTITY);}
    Ok(value.into())
}
fn generation(name:&str,base:&str)->Option<u64>{
    let number=name.strip_prefix(base)?.strip_prefix('-')?.strip_suffix("-link")?;
    if number.is_empty()||number.starts_with('0')||!number.bytes().all(|b|b.is_ascii_digit()){return None;}
    number.parse().ok()
}
fn profile(uid:u32,kind:&str,path:&Path)->Result<Option<UserProfile>>{
    let before=match fs::symlink_metadata(path){Ok(value)=>value,Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(_)=>return Err(IDENTITY)};
    if !before.file_type().is_symlink()||before.uid()!=uid{return Err(IDENTITY);}
    let raw=fs::read_link(path).map_err(|_|IDENTITY)?;
    let base=path.file_name().and_then(|v|v.to_str()).ok_or(IDENTITY)?;
    let selected_generation=generation(raw.file_name().and_then(|v|v.to_str()).ok_or(IDENTITY)?,base).ok_or(IDENTITY)?;
    let selected_closure=store_closure(path)?;
    let directory=path.parent().ok_or(IDENTITY)?;
    let directory_before=fs::symlink_metadata(directory).map_err(|_|IDENTITY)?;
    if !directory_before.is_dir() || directory_before.uid()!=uid || directory_before.mode()&0o022!=0{return Err(IDENTITY);}
    let mut links=Vec::new();
    for (count,entry) in fs::read_dir(directory).map_err(|_|IDENTITY)?.enumerate(){
        if count>=MAX_DIRECTORY_ENTRIES{return Err(Error::Native(aios_protocol::contracts::ErrorCode::ResourceExhausted));}
        let entry=entry.map_err(|_|IDENTITY)?;let name=entry.file_name();let Some(name)=name.to_str() else{return Err(IDENTITY)};
        if let Some(number)=generation(name,base){links.push((number,entry.path()));}
    }
    if links.len()>MAX_GENERATIONS{return Err(Error::Native(aios_protocol::contracts::ErrorCode::ResourceExhausted));}
    links.sort_by_key(|value|value.0);
    if links.windows(2).any(|pair|pair[0].0==pair[1].0){return Err(IDENTITY);}
    let mut generations=Vec::with_capacity(links.len());
    for (number,path) in links{
        let metadata=fs::symlink_metadata(&path).map_err(|_|IDENTITY)?;
        if !metadata.file_type().is_symlink()||metadata.uid()!=uid{return Err(IDENTITY);}
        let closure=store_closure(&path)?;
        generations.push(UserProfileGeneration{generation:number,selected:number==selected_generation&&closure==selected_closure,closure,ownership:"user_profile".into(),management_attribution:None});
    }
    let after=fs::symlink_metadata(path).map_err(|_|IDENTITY)?;
    let directory_after=fs::symlink_metadata(directory).map_err(|_|IDENTITY)?;
    if (before.dev(),before.ino(),before.ctime(),before.ctime_nsec())!=(after.dev(),after.ino(),after.ctime(),after.ctime_nsec())
        || fs::read_link(path).ok().as_ref()!=Some(&raw)
        || (directory_before.dev(),directory_before.ino(),directory_before.ctime(),directory_before.ctime_nsec())!=(directory_after.dev(),directory_after.ino(),directory_after.ctime(),directory_after.ctime_nsec())
        || generations.iter().filter(|value|value.selected).count()!=1{return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));}
    Ok(Some(UserProfile{profile_kind:kind.into(),selected_closure,selected_generation,generations,history_complete:true}))
}
pub fn observe()->Result<(ObservationTime,UserProfiles)>{
    let uid=unsafe{libc::geteuid()};if uid<1000{return Err(Error::WrongScope);}
    let captured=NativeTime::observe()?.observation().clone();let (name,home)=account(uid)?;
    let candidates=[("xdg",home.join(".local/state/nix/profiles/profile")),("per_user",PathBuf::from("/nix/var/nix/profiles/per-user").join(name).join("profile"))];
    let mut profiles=Vec::new();for (kind,path) in &candidates{if let Some(value)=profile(uid,kind,path)?{profiles.push(value);}}
    let now=NativeTime::observe()?.observation().clone();if captured.boot!=now.boot||now.monotonic_ns.checked_sub(captured.monotonic_ns).is_none_or(|age|age>2_000_000_000){return Err(Error::Expired);}
    Ok((captured,UserProfiles{uid,profiles,unmanaged_inventory_complete:false,unmanaged_inventory_reason:"ephemeral_and_non_profile_store_references_require_separate_scoped_providers".into()}))
}
pub struct NativeUserProfileSnapshot{captured:ObservationTime,value:UserProfiles,token:Option<String>}
impl NativeUserProfileSnapshot{
    pub fn collect(store:&GraphStore)->Result<Self>{
        let uid=unsafe{libc::geteuid()};if store.native_scope()!=Scope::User(uid){return Err(Error::WrongScope);}
        let (captured,value)=observe()?;let revision=Self::revision(&value);
        let token=store.reconciliation_plan(PROVIDER.into(),captured.clone(),revision)?.state.map(|state|state.token);
        Ok(Self{captured,value,token})
    }
    fn revision(value:&UserProfiles)->SourceRevision{let bytes=serde_json::to_vec(value).expect("bounded native profiles serialize");SourceRevision{generation:Some(format!("{:x}",Sha256::digest(bytes))),profile:Some(value.uid.to_string()),closure_hash:None,document_hash:None}}
    pub fn value(&self)->&UserProfiles{&self.value}
    pub fn apply(&self,store:&GraphStore)->Result<ProviderState>{
        if store.native_scope()!=Scope::User(self.value.uid){return Err(Error::WrongScope);}
        let (_,current)=observe()?;if current!=self.value{return Err(Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged));}
        let node=Node{id:"generation:user-profiles".into(),kind:"generation".into(),scope:Scope::User(self.value.uid),provider:PROVIDER.into(),stable_key:"generation:user-profiles".into(),properties:serde_json::to_value(&self.value).map_err(|_|IDENTITY)?,source_truth:SourceTruth::UserApp,realtime_ns:self.captured.realtime_ns};
        store.apply_provider_snapshot(ProviderSnapshot{provider:PROVIDER.into(),expected_token:self.token.clone(),source_truth:SourceTruth::UserApp,time:self.captured.clone(),source_revision:Self::revision(&self.value),complete:true,verified_absent_ids:vec![],nodes:vec![node]}).map_err(Error::Graph)
    }
}

#[cfg(test)]mod tests{use super::*;#[test]fn generation_names_are_strict(){assert_eq!(generation("profile-12-link","profile"),Some(12));for value in ["profile-0-link","profile-01-link","profile-+1-link","other-1-link","../profile-1-link"]{assert_eq!(generation(value,"profile"),None);}}}
