use aios_model::service::{self,Config,RuntimeSettings};
use std::{fs,io,os::{fd::{AsRawFd,FromRawFd},unix::{fs::{MetadataExt,PermissionsExt},net::UnixListener}},path::{Path,PathBuf}};

fn invalid()->io::Error {io::Error::new(io::ErrorKind::PermissionDenied,"model service identity or configuration rejected")}
fn main_result()->io::Result<()> {
    let args:Vec<_>=std::env::args().skip(1).collect();
    if nix::unistd::geteuid().is_root() {return Err(invalid());}
    let (listener,config)=match args.as_slice() {
        [mode,directory,socket] if mode=="--qualification"=>{
            if fs::read_to_string("/etc/aios/guest-role")?.trim()!="development" {return Err(invalid());}
            let directory=PathBuf::from(directory);let socket=PathBuf::from(socket);
            if !directory.is_absolute() || directory.canonicalize()?!=directory || !socket.is_absolute() || socket.exists() {return Err(invalid());}
            let parent=socket.parent().ok_or_else(invalid)?;let info=fs::symlink_metadata(parent)?;
            if parent.canonicalize()?!=parent || !info.is_dir() || info.uid()!=nix::unistd::geteuid().as_raw() || info.mode()&0o077!=0 {return Err(invalid());}
            let listener=UnixListener::bind(&socket)?;fs::set_permissions(&socket,fs::Permissions::from_mode(0o600))?;
            (listener,Config {model_directory:directory,qualification:true,runtime:RuntimeSettings::default()})
        },
        [mode,directory,..] if mode=="--model-directory" && (args.len()==2 || (args.len()==4 && args[2]=="--runtime-config" && args[3]=="/etc/aios/model-runtime.json"))=>{
            let user=nix::unistd::User::from_name("aios-model").map_err(io::Error::other)?.ok_or_else(invalid)?;
            let group=nix::unistd::Group::from_name("aios-inference").map_err(io::Error::other)?.ok_or_else(invalid)?;
            if user.uid!=nix::unistd::geteuid() || std::env::var("LISTEN_PID").ok()!=Some(std::process::id().to_string()) ||
                std::env::var("LISTEN_FDS").ok().as_deref()!=Some("1") {return Err(invalid());}
            let listener=unsafe {UnixListener::from_raw_fd(3)};
            if listener.local_addr()?.as_pathname()!=Some(Path::new("/run/aios/model.sock")) {return Err(invalid());}
            let mut accepts=0_i32;let mut size=std::mem::size_of_val(&accepts) as nix::libc::socklen_t;
            if unsafe {nix::libc::getsockopt(listener.as_raw_fd(),nix::libc::SOL_SOCKET,nix::libc::SO_ACCEPTCONN,
                (&mut accepts as *mut i32).cast(),&mut size)}!=0 || accepts!=1 {return Err(invalid());}
            let info=fs::symlink_metadata("/run/aios/model.sock")?;
            if info.mode()&0o777!=0o660 || info.gid()!=group.gid.as_raw() || (info.uid()!=0 && info.uid()!=user.uid.as_raw()) {return Err(invalid());}
            let directory=PathBuf::from(directory).canonicalize()?;
            if !directory.starts_with("/nix/store") {return Err(invalid());}
            let runtime=if args.len()==4{RuntimeSettings::installed()?}else{RuntimeSettings::default()};
            (listener,Config {model_directory:directory,qualification:false,runtime})
        },
        _=>return Err(io::Error::new(io::ErrorKind::InvalidInput,"usage: aios-modeld --model-directory STORE_DIRECTORY")),
    };
    service::run(listener,config)
}
fn main() {if main_result().is_err() {eprintln!("aios-modeld: startup or IPC failure");std::process::exit(1);}}
