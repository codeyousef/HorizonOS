//! Fixed native component startup privilege refusal.
use std::{fs,io};
pub fn drop_capabilities()->io::Result<()>{
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
    Ok(())
}
