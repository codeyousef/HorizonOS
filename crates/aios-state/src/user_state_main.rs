//! Fixed per-user profile graph entry point. No caller-selected account, path,
//! profile, SQL, command or mutation is accepted.
use aios_state::graph::{store::{GraphStore,Scope},user_profiles::{self,NativeUserProfileSnapshot}};
use std::{ffi::CStr,path::PathBuf};
fn directory()->Result<PathBuf,()> {
    let uid=unsafe{libc::geteuid()};if uid<1000{return Err(());}
    let entry=unsafe{libc::getpwuid(uid)};if entry.is_null(){return Err(());}
    let home=unsafe{CStr::from_ptr((*entry).pw_dir)}.to_str().map_err(|_|())?;
    let home=PathBuf::from(home);if !home.is_absolute()||home==std::path::Path::new("/"){return Err(());}
    Ok(home.join(".local/state/aios/user-graph"))
}
fn run()->Result<(),()> {
    let argument=std::env::args().nth(1).ok_or(())?;if std::env::args().nth(2).is_some(){return Err(());}
    match argument.as_str(){
        "--observe"=>{let (_,value)=user_profiles::observe().map_err(|_|())?;println!("{}",serde_json::to_string(&value).map_err(|_|())?);Ok(())},
        "--reconcile"=>{let uid=unsafe{libc::geteuid()};let store=GraphStore::open(&directory()?,Scope::User(uid)).map_err(|_|())?;let snapshot=NativeUserProfileSnapshot::collect(&store).map_err(|_|())?;let value=snapshot.value().clone();let state=snapshot.apply(&store).map_err(|_|())?;println!("{}",serde_json::json!({"schema_version":1,"provider":user_profiles::PROVIDER,"status":format!("{:?}",state.status),"data":value,"execution_authority":false}));Ok(())},
        _=>Err(())
    }
}
fn main(){if run().is_err(){eprintln!("aios-user-stated: refused");std::process::exit(1);}}
