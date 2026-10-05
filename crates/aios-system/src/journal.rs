//! Bounded native journal snapshots. No arbitrary paths, fields or expressions.
//!
//! The session broker must check its caller's read grant before and after this
//! reader, and keep Continuation behind an expiring, peer/query-bound handle.
//! This library does not mint policy authority or advertise a runtime capability.
pub mod redaction;
pub mod user;

use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use std::{ffi::{c_char, c_int, c_void, CStr, CString}, fs::{self, File, OpenOptions},
    os::{fd::{AsRawFd, IntoRawFd}, unix::fs::{MetadataExt, OpenOptionsExt}},
    path::{Path, PathBuf}, ptr, time::{Duration, Instant}};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const MAX_FILES: usize = 128;
const MAX_SCAN: usize = 10_000;
const MAX_FIELD: usize = 16_384;

#[repr(C)] struct NativeJournal { _opaque: [u8; 0] }
#[link(name="systemd")]
unsafe extern "C" {
    fn sd_journal_open_files_fd(ret: *mut *mut NativeJournal, fds: *mut c_int, count: u32, flags: c_int) -> c_int;
    fn sd_journal_close(j: *mut NativeJournal);
    fn sd_journal_set_data_threshold(j: *mut NativeJournal, size: usize) -> c_int;
    fn sd_journal_add_match(j: *mut NativeJournal, data: *const c_void, size: usize) -> c_int;
    fn sd_journal_seek_realtime_usec(j: *mut NativeJournal, timestamp: u64) -> c_int;
    fn sd_journal_seek_cursor(j: *mut NativeJournal, cursor: *const c_char) -> c_int;
    fn sd_journal_test_cursor(j: *mut NativeJournal, cursor: *const c_char) -> c_int;
    fn sd_journal_get_cursor(j: *mut NativeJournal, cursor: *mut *mut c_char) -> c_int;
    fn sd_journal_next(j: *mut NativeJournal) -> c_int;
    fn sd_journal_get_realtime_usec(j: *mut NativeJournal, timestamp: *mut u64) -> c_int;
    fn sd_journal_get_data(j: *mut NativeJournal, field: *const c_char, data: *mut *const c_void, size: *mut usize) -> c_int;
    fn sd_journal_query_unique(j: *mut NativeJournal, field: *const c_char) -> c_int;
    fn sd_journal_enumerate_unique(j: *mut NativeJournal, data: *mut *const c_void, size: *mut usize) -> c_int;
}
unsafe extern "C" {
    fn geteuid() -> u32;
    fn free(ptr: *mut c_void);
    fn close(fd: c_int) -> c_int;
    fn faccessat(fd: c_int, path: *const c_char, mode: c_int, flags: c_int) -> c_int;
}

fn native(value: c_int) -> Result<c_int, ErrorCode> {
    match value {
        0.. => Ok(value), -13 | -1 => Err(ErrorCode::PermissionDenied),
        -2 => Err(ErrorCode::TargetNotFound), -12 | -7 => Err(ErrorCode::ResourceExhausted),
        _ => Err(ErrorCode::PartialResult),
    }
}
fn io_error(error: std::io::Error) -> ErrorCode {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => ErrorCode::PermissionDenied,
        std::io::ErrorKind::NotFound => ErrorCode::TargetChanged,
        _ => ErrorCode::PartialResult,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all="snake_case")]
pub enum Source { Kernel, System, User }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unit { System(String), User(String) }

#[derive(Clone, Debug, PartialEq, Eq)]
struct Target { machine: String, boot: String, uid: u32 }
impl Target {
    fn current() -> Result<Self, ErrorCode> {
        let machine = super::bounded(Path::new("/etc/machine-id"), 128)?.trim().to_owned();
        if machine.len()!=32 || !machine.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(Self { machine, boot: super::boot_id(&super::bounded(Path::new("/proc/sys/kernel/random/boot_id"),128)?)?,
            uid: unsafe { geteuid() } })
    }
    fn check(&self) -> Result<(), ErrorCode> {
        if *self != Self::current()? { return Err(ErrorCode::TargetChanged); } Ok(())
    }
}

/// Normalized frozen time window. UID is captured from the kernel, not input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    target: Target, source: Source, unit: Option<Unit>, boot: String,
    since: u64, until: u64, priority: u8, limit: usize, user: Option<user::NativeUser>,
}
impl Query {
    pub fn new(source: Source, unit: Option<Unit>, boot: &str, since_usec: u64, until_usec: u64,
               priority_max: u8, max_entries: usize) -> Result<Self, ErrorCode> {
        Self::with_target(Target::current()?, source, unit, boot, since_usec, until_usec, priority_max, max_entries)
    }
    pub fn for_native_user(user: &user::NativeUser, source: Source, unit: Option<Unit>, boot: &str,
        since: u64, until: u64, priority: u8, limit: usize) -> Result<Self, ErrorCode> {
        user.verify()?;
        let mut query=Self::new(source,unit,boot,since,until,priority,limit)?;
        query.user=Some(user.clone()); Ok(query)
    }
    pub fn boot_id(&self) -> &str { &self.boot }
    fn user_uid(&self) -> u32 { self.user.as_ref().map_or(self.target.uid, user::NativeUser::uid) }
    fn verify(&self) -> Result<(),ErrorCode> {
        self.target.check()?;
        if let Some(user)=&self.user { user.verify()?; } Ok(())
    }
    fn with_target(target: Target, source: Source, unit: Option<Unit>, boot: &str, since: u64, until: u64,
                   priority: u8, limit: usize) -> Result<Self, ErrorCode> {
        if priority>7 || !(1..=200).contains(&limit) || since>until
            || OffsetDateTime::from_unix_timestamp_nanos(i128::from(until)*1000).is_err() {
            return Err(ErrorCode::InvalidArgument);
        }
        match (&unit, source) {
            (Some(_), Source::Kernel) | (Some(Unit::User(_)), Source::System) => return Err(ErrorCode::InvalidArgument),
            (Some(Unit::User(name) | Unit::System(name)), _) => super::services::validate_service_name(name)?,
            _ => (),
        }
        let boot = if boot=="current" { target.boot.clone() } else { super::boot_id(boot)? };
        Ok(Self { target, source, unit, boot, since, until, priority, limit, user:None })
    }
}

/// Never serialized. A wire cursor must refer to broker-owned state, and never
/// accept a caller-supplied native cursor or replay one under a different query.
pub struct Continuation { query: Query, native_cursor: CString }
#[derive(Serialize)]
pub struct Entry {
    pub timestamp: String, pub priority: u8, pub source: Source,
    pub message: redaction::Message,
    #[serde(skip)] locator: CString,
}
impl Entry {
    /// Journal locator is for the scoped evidence store, never a public cursor.
    pub fn locator(&self) -> &CStr { &self.locator }
}
pub struct Read { pub entries: Vec<Entry>, pub continuation: Option<Continuation> }

struct Input { file: File, dev: u64, inode: u64, mode: u32 }
impl Input {
    fn check(&self) -> Result<(), ErrorCode> {
        let m=self.file.metadata().map_err(io_error)?;
        if m.uid()!=0 || m.dev()!=self.dev || m.ino()!=self.inode || m.mode()!=self.mode {
            return Err(ErrorCode::TargetChanged);
        }
        // Check current DAC/ACL access on the retained object, even after rename.
        // AT_EMPTY_PATH | AT_EACCESS; unsupported kernels fail closed.
        if unsafe { faccessat(self.file.as_raw_fd(), c"".as_ptr(), 4, 0x1000|0x200) } != 0 {
            return Err(ErrorCode::PermissionDenied);
        }
        Ok(())
    }
}

fn selected_file(name: &str, source: Source, uid: u32) -> bool {
    let system = name=="system.journal" || name.starts_with("system@") && name.ends_with(".journal");
    let user = format!("user-{uid}");
    system || source==Source::User && (name==format!("{user}.journal")
        || name.starts_with(&(user+"@")) && name.ends_with(".journal"))
}
fn directory(path: &Path) -> Result<(), ErrorCode> {
    let m=fs::symlink_metadata(path).map_err(io_error)?;
    if !m.is_dir() || m.uid()!=0 || m.mode()&0o022!=0 { return Err(ErrorCode::PermissionDenied); }
    Ok(())
}
fn inputs(query: &Query) -> Result<Vec<Input>, ErrorCode> {
    let mut result=Vec::new();
    for root in ["/run/log/journal", "/var/log/journal"] {
        let root=Path::new(root);
        // Missing journal storage is distinct from denied or substituted storage.
        if fs::symlink_metadata(root).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound) { continue; }
        for path in root.ancestors().collect::<Vec<_>>().into_iter().rev() { directory(path)?; }
        let path=root.join(&query.target.machine);
        if fs::symlink_metadata(&path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound) { continue; }
        directory(&path)?;
        let mut paths:Vec<PathBuf>=Vec::new();let mut scanned=0;
        for entry in fs::read_dir(&path).map_err(io_error)? {
            scanned+=1;if scanned>1024 { return Err(ErrorCode::ResourceExhausted); }
            let entry=entry.map_err(io_error)?;
            let name=entry.file_name();let name=name.to_str().ok_or(ErrorCode::PartialResult)?;
            if selected_file(name,query.source,query.user_uid()) { paths.push(entry.path()); }
        }
        paths.sort();
        for path in paths {
            if result.len()>=MAX_FILES { return Err(ErrorCode::ResourceExhausted); }
            // Linux O_NOFOLLOW | O_NONBLOCK. Reject symlinks/devices/FIFOs before
            // supplying descriptors to libsystemd. No caller path is accepted.
            let file=OpenOptions::new().read(true).custom_flags(0x20000|0x800).open(path).map_err(io_error)?;
            let m=file.metadata().map_err(io_error)?;
            if !m.is_file() || m.uid()!=0 || m.mode()&0o022!=0 { return Err(ErrorCode::PermissionDenied); }
            if m.len()>4*1024*1024*1024 { return Err(ErrorCode::ResourceExhausted); }
            let input=Input { file,dev:m.dev(),inode:m.ino(),mode:m.mode() };input.check()?;result.push(input);
        }
    }
    if result.is_empty() { return Err(ErrorCode::PermissionDenied); }
    Ok(result)
}

struct Journal { ptr: *mut NativeJournal, inputs: Vec<Input>, deadline: Instant }
impl Drop for Journal { fn drop(&mut self) { unsafe { sd_journal_close(self.ptr) }; } }
impl Journal {
    fn open(query: &Query) -> Result<Self, ErrorCode> {
        let deadline=Instant::now()+Duration::from_secs(5);
        let inputs=inputs(query)?;
        // libsystemd takes ownership only on success. Duplicate the checked FDs
        // so the original descriptors remain available for access/identity checks.
        let duplicates:Result<Vec<_>,_>=inputs.iter().map(|i|i.file.try_clone()).collect();
        let mut fds:Vec<_>=duplicates.map_err(io_error)?.into_iter().map(IntoRawFd::into_raw_fd).collect();
        let mut ptr=ptr::null_mut();
        let result=unsafe { sd_journal_open_files_fd(&mut ptr,fds.as_mut_ptr(),fds.len() as u32,0) };
        if result<0 { for fd in fds { unsafe { close(fd) }; } return Err(native(result).unwrap_err()); }
        if ptr.is_null() { for fd in fds { unsafe { close(fd) }; } return Err(ErrorCode::PartialResult); }
        let value=Self { ptr,inputs,deadline };
        native(unsafe { sd_journal_set_data_threshold(value.ptr,MAX_FIELD) })?;
        value.budget()?;Ok(value)
    }
    fn budget(&self) -> Result<(), ErrorCode> {
        if Instant::now()>=self.deadline { return Err(ErrorCode::DeadlineExceeded); } Ok(())
    }
    fn add(&self, field: &str, value: &str) -> Result<(), ErrorCode> {
        self.budget()?;
        let data=format!("{field}={value}");
        native(unsafe { sd_journal_add_match(self.ptr,data.as_ptr().cast(),data.len()) })?;self.budget()
    }
    fn next(&self) -> Result<bool, ErrorCode> {
        self.budget()?;let value=native(unsafe { sd_journal_next(self.ptr) })?;self.budget()?;Ok(value>0)
    }
    fn field(&self, field: &'static CStr) -> Result<Option<Vec<u8>>, ErrorCode> {
        self.budget()?;let mut data=ptr::null();let mut size=0;
        let result=unsafe { sd_journal_get_data(self.ptr,field.as_ptr(),&mut data,&mut size) };
        if result == -2 { return Ok(None); } native(result)?;self.budget()?;
        if data.is_null() || size>MAX_FIELD { return Err(ErrorCode::ResourceExhausted); }
        let data=unsafe { std::slice::from_raw_parts(data.cast::<u8>(),size) };
        let prefix=field.to_bytes();
        if data.len()<=prefix.len() || !data.starts_with(prefix) || data[prefix.len()]!=b'=' { return Err(ErrorCode::PartialResult); }
        Ok(Some(data[prefix.len()+1..].to_vec()))
    }
    fn text(&self, field: &'static CStr) -> Result<Option<String>, ErrorCode> {
        self.field(field)?.map(|bytes|String::from_utf8(bytes).map_err(|_|ErrorCode::PartialResult)).transpose()
    }
    fn cursor(&self) -> Result<CString, ErrorCode> {
        let mut p=ptr::null_mut();native(unsafe { sd_journal_get_cursor(self.ptr,&mut p) })?;
        if p.is_null() { return Err(ErrorCode::PartialResult); }
        let value=unsafe { CStr::from_ptr(p).to_owned() };unsafe { free(p.cast()) };
        if value.as_bytes().len()>512 || value.as_bytes().is_empty() { return Err(ErrorCode::ResourceExhausted); }
        Ok(value)
    }
    fn boot_exists(&self, boot: &str) -> Result<(), ErrorCode> {
        native(unsafe { sd_journal_query_unique(self.ptr,c"_BOOT_ID".as_ptr()) })?;
        for _ in 0..4096 {
            self.budget()?;let mut data=ptr::null();let mut size=0;
            let result=native(unsafe { sd_journal_enumerate_unique(self.ptr,&mut data,&mut size) })?;
            if result==0 { return Err(ErrorCode::TargetNotFound); }
            if data.is_null() || size!=9+32 { return Err(ErrorCode::PartialResult); }
            let value=unsafe { std::slice::from_raw_parts(data.cast::<u8>(),size) };
            if value==format!("_BOOT_ID={boot}").as_bytes() { return Ok(()); }
        }
        Err(ErrorCode::ResourceExhausted)
    }
}

fn is_system(unit: Option<&str>, user_unit: Option<&str>, owner: Option<&str>, uid: Option<&str>) -> bool {
    user_unit.is_none() && owner.is_none() && match unit {
        Some(name) => !name.starts_with("user@") && !name.starts_with("session-"),
        None => uid==Some("0"),
    }
}

pub fn read(query: &Query, resume: Option<&Continuation>) -> Result<Read, ErrorCode> {
    query.verify()?;
    if resume.is_some_and(|c|c.query!=*query) { return Err(ErrorCode::PermissionDenied); }
    let journal=Journal::open(query)?;
    let boot=query.boot.replace('-',"");journal.boot_exists(&boot)?;
    journal.add("_MACHINE_ID",&query.target.machine)?;journal.add("_BOOT_ID",&boot)?;
    for priority in 0..=query.priority { journal.add("PRIORITY",&priority.to_string())?; }
    if query.source==Source::Kernel { journal.add("_TRANSPORT","kernel")?; }
    if query.source==Source::User { journal.add("_UID",&query.user_uid().to_string())?; }
    if let Some(unit)=&query.unit { match unit {
        Unit::System(name) => journal.add("_SYSTEMD_UNIT",name)?, Unit::User(name) => journal.add("_SYSTEMD_USER_UNIT",name)?,
    } }
    if let Some(cursor)=resume {
        native(unsafe { sd_journal_seek_cursor(journal.ptr,cursor.native_cursor.as_ptr()) })?;
        if !journal.next()? || native(unsafe { sd_journal_test_cursor(journal.ptr,cursor.native_cursor.as_ptr()) })?!=1 {
            return Err(ErrorCode::TargetChanged);
        }
    } else { native(unsafe { sd_journal_seek_realtime_usec(journal.ptr,query.since) })?; }
    let mut entries=Vec::new();let mut last=None;let mut more=false;let mut finished=false;
    for _ in 0..MAX_SCAN {
        if !journal.next()? { finished=true;break; }
        let mut timestamp=0;native(unsafe { sd_journal_get_realtime_usec(journal.ptr,&mut timestamp) })?;
        if timestamp<query.since { continue; } if timestamp>query.until { finished=true;break; }
        if journal.text(c"_MACHINE_ID")?.as_deref()!=Some(&query.target.machine)
            || journal.text(c"_BOOT_ID")?.as_deref()!=Some(&boot) { return Err(ErrorCode::TargetChanged); }
        let transport=journal.text(c"_TRANSPORT")?;let uid=journal.text(c"_UID")?;
        let unit=journal.text(c"_SYSTEMD_UNIT")?;let user_unit=journal.text(c"_SYSTEMD_USER_UNIT")?;
        if query.source==Source::System && (transport.as_deref()==Some("kernel")
            || !is_system(unit.as_deref(),user_unit.as_deref(),journal.text(c"_SYSTEMD_OWNER_UID")?.as_deref(),uid.as_deref())) { continue; }
        if query.source==Source::User && uid.as_deref()!=Some(query.user_uid().to_string().as_str()) { return Err(ErrorCode::PermissionDenied); }
        if query.source==Source::Kernel && transport.as_deref()!=Some("kernel") { return Err(ErrorCode::TargetChanged); }
        match &query.unit {
            Some(Unit::System(name)) if unit.as_ref()!=Some(name) => return Err(ErrorCode::TargetChanged),
            Some(Unit::User(name)) if user_unit.as_ref()!=Some(name) => return Err(ErrorCode::TargetChanged), _ => (),
        }
        let priority:u8=journal.text(c"PRIORITY")?.ok_or(ErrorCode::PartialResult)?.parse().map_err(|_|ErrorCode::PartialResult)?;
        if priority>query.priority { return Err(ErrorCode::TargetChanged); }
        if entries.len()==query.limit { more=true;break; }
        let message=journal.field(c"MESSAGE")?.ok_or(ErrorCode::PartialResult)?;
        let message=redaction::redact(&message); // Before serialization, evidence or hashing.
        let locator=journal.cursor()?;last=Some(locator.clone());
        entries.push(Entry { timestamp:OffsetDateTime::from_unix_timestamp_nanos(i128::from(timestamp)*1000)
            .map_err(|_|ErrorCode::PartialResult)?.format(&Rfc3339).map_err(|_|ErrorCode::PartialResult)?,
            priority,source:query.source,message,locator });
    }
    if !finished && !more { return Err(ErrorCode::ResourceExhausted); }
    for file in &journal.inputs { file.check()?; }query.verify()?;journal.budget()?;
    Ok(Read { entries,continuation:if more { Some(Continuation { query:query.clone(),native_cursor:last.ok_or(ErrorCode::PartialResult)? }) } else { None } })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target() -> Target { Target { machine:"1".repeat(32),boot:"11111111-1111-4111-8111-111111111111".into(),uid:1000 } }
    #[test]
    fn invalid_filters_fail_before_any_journal_access() {
        for (source,unit,start,end,priority,limit) in [
            (Source::Kernel,Some(Unit::System("sshd.service".into())),0,1,7,1),
            (Source::System,Some(Unit::User("foo.service".into())),0,1,7,1),
            (Source::User,None,2,1,7,1),(Source::User,None,0,1,8,1),
            (Source::User,None,0,1,7,0),(Source::User,None,0,1,7,201)] {
            assert_eq!(Query::with_target(target(),source,unit,"current",start,end,priority,limit).unwrap_err(),ErrorCode::InvalidArgument);
        }
    }
    #[test]
    fn source_classification_does_not_include_other_users_in_system_data() {
        assert!(is_system(Some("sshd.service"),None,None,Some("0")));
        assert!(is_system(Some("foo.service"),None,None,Some("65534")));
        assert!(!is_system(Some("user@1001.service"),None,None,Some("1001")));
        assert!(!is_system(Some("foo.service"),Some("user.service"),None,Some("1001")));
        assert!(!is_system(None,None,None,Some("1001")));
        assert!(!is_system(Some("foo.service"),None,Some("1001"),Some("1001")));
    }
    #[test]
    fn file_selection_never_reads_another_user_journal() {
        assert!(selected_file("system@abcd.journal",Source::User,1000));
        assert!(selected_file("user-1000@abcd.journal",Source::User,1000));
        assert!(!selected_file("user-1001.journal",Source::User,1000));
        assert!(!selected_file("user-1000.journal",Source::System,1000));
        assert!(!selected_file("system.journal~",Source::System,1000));
        assert!(!selected_file("../../../secrets",Source::System,1000));
    }
}
