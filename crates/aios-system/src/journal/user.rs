//! Read authority is checked by the broker. This binds only the native user
//! observation, and deliberately has no constructor from a claimed UID/PID.
use aios_protocol::contracts::ErrorCode;
use std::{fs, os::unix::fs::MetadataExt, path::Path};
use zbus::blocking::Proxy;
use super::Target;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeUser {
    sender: String, bus_id: String, uid: u32, pid: u32, start_ticks: u64, boot: String,
}
impl NativeUser {
    /// The adapter passes the injected native unique sender and authenticated
    /// system-bus ID, never strings taken from request JSON. Only the privileged
    /// observer may inspect a different user's process and journal scope.
    pub fn observe(sender: &str, expected_bus_id: &str) -> Result<Self, ErrorCode> {
        if unsafe { super::geteuid() } != 0 { return Err(ErrorCode::PermissionDenied); }
        if !sender.starts_with(':') || sender.len()>128 || !sender[1..].contains('.')
            || !sender[1..].bytes().all(|b|b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            || expected_bus_id.len()!=32 || !expected_bus_id.bytes().all(|b|b.is_ascii_hexdigit()) {
            return Err(ErrorCode::InvalidArgument);
        }
        // Fixed root-protected native system bus. No alternate bus/environment.
        for path in ["/", "/run", "/run/dbus"] {
            let m=fs::symlink_metadata(path).map_err(|_|ErrorCode::TargetChanged)?;
            if !m.is_dir() || m.uid()!=0 || m.mode()&0o022!=0 { return Err(ErrorCode::PermissionDenied); }
        }
        let connection=crate::services::system_connection()?;
        let bus=Proxy::new(&connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus")
            .map_err(crate::services::dbus_error)?;
        let bus_id:String=bus.call("GetId",&()).map_err(crate::services::dbus_error)?;
        if bus_id!=expected_bus_id { return Err(ErrorCode::TargetChanged); }
        let uid:u32=bus.call("GetConnectionUnixUser",&(sender,)).map_err(crate::services::dbus_error)?;
        let pid:u32=bus.call("GetConnectionUnixProcessID",&(sender,)).map_err(crate::services::dbus_error)?;
        if uid==0 || uid==u32::MAX || pid<=1 { return Err(ErrorCode::PermissionDenied); }
        let path=format!("/proc/{pid}");
        if fs::metadata(&path).map_err(|_|ErrorCode::TargetChanged)?.uid()!=uid { return Err(ErrorCode::PermissionDenied); }
        let stat=crate::bounded(Path::new(&format!("{path}/stat")),16384)?;
        let fields=stat.rsplit_once(')').ok_or(ErrorCode::TargetChanged)?.1.split_whitespace().collect::<Vec<_>>();
        let start_ticks=fields.get(19).ok_or(ErrorCode::TargetChanged)?.parse::<u64>().map_err(|_|ErrorCode::TargetChanged)?;
        if start_ticks==0 { return Err(ErrorCode::TargetChanged); }
        let boot=Target::current()?.boot;
        if bus.call::<_,_,u32>("GetConnectionUnixUser",&(sender,)).map_err(crate::services::dbus_error)?!=uid
            || bus.call::<_,_,u32>("GetConnectionUnixProcessID",&(sender,)).map_err(crate::services::dbus_error)?!=pid
            || bus.call::<_,_,String>("GetId",&()).map_err(crate::services::dbus_error)?!=bus_id {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(Self{sender:sender.into(),bus_id,uid,pid,start_ticks,boot})
    }
    pub fn uid(&self) -> u32 { self.uid }
    pub fn verify(&self) -> Result<(),ErrorCode> {
        if *self!=Self::observe(&self.sender,&self.bus_id)? { return Err(ErrorCode::TargetChanged); } Ok(())
    }
}
