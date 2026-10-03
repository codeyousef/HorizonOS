use std::{fs,io,os::unix::{fs::{MetadataExt,PermissionsExt,FileTypeExt},net::UnixListener},path::PathBuf,
    sync::{Arc,atomic::{AtomicUsize,Ordering}}};
fn run()->io::Result<()>{
    let uid=nix::unistd::geteuid().as_raw();
    if uid==0 || std::env::args().count()!=1{return Err(io::Error::other("native provider requires a non-root managed graphical session"));}
    // A user manager may inherit CAP_WAKE_ALARM. Drop our own usable and
    // inheritable sets before threads, child processes or client admission.
    // Clearing these sets needs no CAP_SETPCAP; changing the bounding set does.
    #[repr(C)]struct Header{version:u32,pid:i32}
    #[repr(C)]#[derive(Clone,Copy)]struct Caps{effective:u32,permitted:u32,inheritable:u32}
    let header=Header{version:0x20080522,pid:0};let caps=[Caps{effective:0,permitted:0,inheritable:0};2];
    if unsafe{nix::libc::syscall(nix::libc::SYS_capset,&header as *const Header,caps.as_ptr())}!=0{return Err(io::Error::other("provider capability drop failed"));}
    let status=fs::read_to_string("/proc/self/status")?;
    for key in ["CapEff","CapPrm","CapInh","CapAmb"]{
        let values=status.lines().filter_map(|line|line.strip_prefix(&format!("{key}:"))).collect::<Vec<_>>();
        if values.len()!=1 || u64::from_str_radix(values[0].trim(),16).ok()!=Some(0){return Err(io::Error::other("provider capabilities refused"));}
    }
    if unsafe{nix::libc::prctl(nix::libc::PR_GET_NO_NEW_PRIVS,0 as nix::libc::c_ulong,0 as nix::libc::c_ulong,0 as nix::libc::c_ulong,0 as nix::libc::c_ulong)}!=1{return Err(io::Error::other("provider privilege restriction absent"));}
    // The exact graphical-only unit owns this runtime directory. No arbitrary
    // path argument, inherited display selection or fallback daemon mode.
    let directory=PathBuf::from(format!("/run/user/{uid}/aios-ui"));
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
    for stream in listener.incoming(){
        let stream=stream?;
        if active.fetch_add(1,Ordering::AcqRel)>=8{active.fetch_sub(1,Ordering::AcqRel);continue;}
        let active=active.clone();std::thread::spawn(move||{
            let _=aios_session::ui_bridge::serve(stream);active.fetch_sub(1,Ordering::AcqRel);
        });
    }
    Ok(())
}
fn main(){if let Err(error)=run(){eprintln!("aios-ui-agent: native provider startup refused ({error})");std::process::exit(1);}}
