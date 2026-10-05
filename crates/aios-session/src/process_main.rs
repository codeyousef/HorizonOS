use std::{fs,io,os::unix::{fs::{MetadataExt,PermissionsExt,FileTypeExt},net::UnixListener},path::PathBuf,
    sync::{Arc,atomic::{AtomicUsize,Ordering}}};
fn run()->io::Result<()>{
    let uid=nix::unistd::geteuid().as_raw();
    if uid==0 || std::env::args().count()!=1{return Err(io::Error::other("native process provider requires a non-root managed user session"));}
    aios_session::native_startup::drop_capabilities()?;
    // The exact headless process unit owns this runtime directory. No arbitrary
    // path argument, inherited display selection or fallback daemon mode.
    let directory=PathBuf::from(format!("/run/user/{uid}/aios-process"));
    let meta=fs::symlink_metadata(&directory)?;
    if !meta.is_dir() || meta.uid()!=uid || meta.mode()&0o077!=0 || directory.canonicalize()?!=directory{return Err(io::Error::other("unsafe provider runtime"));}
    let path=directory.join("provider.sock");
    // Restart may leave only this unit's fixed owned socket. Never unlink an
    // arbitrary file or a live listener belonging to another process.
    if let Ok(meta)=fs::symlink_metadata(&path){
        if !meta.file_type().is_socket() || meta.uid()!=uid || meta.mode()&0o777!=0o600 || std::os::unix::net::UnixStream::connect(&path).is_ok(){
            return Err(io::Error::other("provider endpoint already owned"));
        }
        fs::remove_file(&path)?;
    }
    let listener=UnixListener::bind(&path)?;fs::set_permissions(&path,fs::Permissions::from_mode(0o600))?;
    let active=Arc::new(AtomicUsize::new(0));
    let state=Arc::new(std::sync::Mutex::new(aios_session::State::default()));
    for stream in listener.incoming(){
        let stream=stream?;
        if active.fetch_add(1,Ordering::AcqRel)>=4{active.fetch_sub(1,Ordering::AcqRel);continue;}
        let active=active.clone();let state=state.clone();std::thread::spawn(move||{
            let _=aios_session::process_bridge::serve(stream,state);active.fetch_sub(1,Ordering::AcqRel);
        });
    }
    Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("aios-processd: native provider startup refused ({error})");std::process::exit(1);}}
