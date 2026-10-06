//! Fixed read-only sd-bus calls on the already kernel-verified manager socket.
//! systemd's direct replies use a well-known sender, unlike message-bus replies.
use super::{Result, ErrorCode};
use std::{ffi::{CStr, CString}, os::{fd::IntoRawFd, unix::net::UnixStream}, ptr};
use libc::{c_char, c_int, c_void};

#[repr(C)] struct NativeError { name: *const c_char, message: *const c_char, need_free: c_int }
impl NativeError {
    fn new() -> Self { Self { name: ptr::null(), message: ptr::null(), need_free: 0 } }
    fn check(&self, result: c_int) -> Result<()> {
        if result >= 0 { return Ok(()); }
        let named = |name: &CStr| unsafe { sd_bus_error_has_name(self, name.as_ptr()) != 0 };
        Err(if named(c"org.freedesktop.systemd1.NoSuchUnit") { ErrorCode::TargetNotFound }
            else if named(c"org.freedesktop.DBus.Error.AccessDenied") || named(c"org.freedesktop.DBus.Error.AuthFailed") { ErrorCode::PermissionDenied }
            else if named(c"org.freedesktop.DBus.Error.NoReply") || named(c"org.freedesktop.DBus.Error.Timeout") { ErrorCode::DeadlineExceeded }
            else { errno(result) })
    }
}
impl Drop for NativeError { fn drop(&mut self) { unsafe { sd_bus_error_free(self); } } }
fn errno(result: c_int) -> ErrorCode {
    match result.checked_neg() {
        Some(libc::ETIMEDOUT) => ErrorCode::DeadlineExceeded,
        Some(libc::EACCES | libc::EPERM) => ErrorCode::PermissionDenied,
        Some(libc::ENOMEM | libc::ENOBUFS | libc::E2BIG) => ErrorCode::ResourceExhausted,
        Some(libc::ENOENT) => ErrorCode::TargetNotFound,
        _ => ErrorCode::PartialResult,
    }
}
fn check(result: c_int) -> Result<()> { if result < 0 { Err(errno(result)) } else { Ok(()) } }
// The pointers here come only from libsystemd-owned results. Copy bounded UTF-8
// before releasing the result; do not print their contents or error messages.
unsafe fn text(value: *const c_char, limit: usize) -> Result<String> {
    if value.is_null() { return Err(ErrorCode::PartialResult); }
    let size = unsafe { libc::strnlen(value, limit + 1) };
    if size > limit { return Err(ErrorCode::ResourceExhausted); }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(value.cast::<u8>(), size) })
        .map(str::to_owned).map_err(|_| ErrorCode::PartialResult)
}
struct Message(*mut c_void);
impl Drop for Message { fn drop(&mut self) { unsafe { sd_bus_message_unref(self.0); } } }
pub(super) struct Bus(*mut c_void);
impl Drop for Bus { fn drop(&mut self) { unsafe { sd_bus_close(self.0); sd_bus_unref(self.0); } } }
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Unit { pub path: String, pub id: String, pub load: String, pub invocation: Vec<u8> }
impl Bus {
    pub(super) fn connect(socket: UnixStream) -> Result<Self> {
        let mut bus = Self(ptr::null_mut());
        check(unsafe { sd_bus_new(&mut bus.0) })?;
        check(unsafe { sd_bus_set_bus_client(bus.0, 0) })?;
        check(unsafe { sd_bus_set_method_call_timeout(bus.0, 250_000) })?;
        let fd = socket.into_raw_fd();
        let result = unsafe { sd_bus_set_fd(bus.0, fd, fd) };
        if result < 0 {
            // Ownership transfers only on successful sd_bus_set_fd.
            unsafe { libc::close(fd); } return Err(errno(result));
        }
        check(unsafe { sd_bus_start(bus.0) })?;
        Ok(bus)
    }
    pub(super) fn unit(&self, name: &str) -> Result<Unit> {
        crate::services::validate_service_name(name)?;
        let name = CString::new(name).map_err(|_| ErrorCode::InvalidArgument)?;
        let mut error = NativeError::new(); let mut reply = Message(ptr::null_mut());
        let called = unsafe { sd_bus_call_method(self.0, c"org.freedesktop.systemd1".as_ptr(),
            c"/org/freedesktop/systemd1".as_ptr(), c"org.freedesktop.systemd1.Manager".as_ptr(),
            c"GetUnit".as_ptr(), &mut error, &mut reply.0, c"s".as_ptr(), name.as_ptr()) };
        error.check(called)?;
        if reply.0.is_null() { return Err(ErrorCode::PartialResult); }
        let mut path = ptr::null::<c_char>();
        if unsafe { sd_bus_message_read(reply.0, c"o".as_ptr(), &mut path as *mut *const c_char) } <= 0 { return Err(ErrorCode::PartialResult); }
        let path = unsafe { text(path, 1024) }?;
        if !path.starts_with("/org/freedesktop/systemd1/unit/")
            || !path.bytes().all(|v| v.is_ascii_alphanumeric() || b"/_".contains(&v)) { return Err(ErrorCode::TargetChanged); }
        let native_path = CString::new(path.as_str()).map_err(|_| ErrorCode::TargetChanged)?;
        let id = self.string(&native_path, c"Id", 255)?;
        let load = self.string(&native_path, c"LoadState", 64)?;
        let invocation = self.invocation(&native_path)?;
        Ok(Unit { path, id, load, invocation })
    }
    fn string(&self, path: &CStr, property: &CStr, limit: usize) -> Result<String> {
        let mut error = NativeError::new(); let mut value = ptr::null_mut::<c_char>();
        let result = unsafe { sd_bus_get_property_string(self.0, c"org.freedesktop.systemd1".as_ptr(),
            path.as_ptr(), c"org.freedesktop.systemd1.Unit".as_ptr(), property.as_ptr(), &mut error, &mut value) };
        let decoded = error.check(result).and_then(|_| unsafe { text(value, limit) });
        unsafe { libc::free(value.cast()); } decoded
    }
    fn invocation(&self, path: &CStr) -> Result<Vec<u8>> {
        let mut error = NativeError::new(); let mut reply = Message(ptr::null_mut());
        let fetched = unsafe { sd_bus_get_property(self.0, c"org.freedesktop.systemd1".as_ptr(),
            path.as_ptr(), c"org.freedesktop.systemd1.Unit".as_ptr(), c"InvocationID".as_ptr(),
            &mut error, &mut reply.0, c"ay".as_ptr()) };
        error.check(fetched)?;
        if reply.0.is_null() { return Err(ErrorCode::PartialResult); }
        let mut bytes = ptr::null::<c_void>(); let mut size = 0usize;
        let read = unsafe { sd_bus_message_read_array(reply.0, b'y' as c_char, &mut bytes, &mut size) };
        if read <= 0 || size != 16 || bytes.is_null() { return Err(ErrorCode::TargetChanged); }
        Ok(unsafe { std::slice::from_raw_parts(bytes.cast::<u8>(), size) }.to_vec())
    }
}

#[link(name = "systemd")]
unsafe extern "C" {
    fn sd_bus_new(bus: *mut *mut c_void) -> c_int;
    fn sd_bus_unref(bus: *mut c_void) -> *mut c_void;
    fn sd_bus_close(bus: *mut c_void);
    fn sd_bus_set_bus_client(bus: *mut c_void, client: c_int) -> c_int;
    fn sd_bus_set_method_call_timeout(bus: *mut c_void, timeout: u64) -> c_int;
    fn sd_bus_set_fd(bus: *mut c_void, input: c_int, output: c_int) -> c_int;
    fn sd_bus_start(bus: *mut c_void) -> c_int;
    fn sd_bus_message_unref(message: *mut c_void) -> *mut c_void;
    fn sd_bus_error_free(error: *mut NativeError);
    fn sd_bus_error_has_name(error: *const NativeError, name: *const c_char) -> c_int;
    fn sd_bus_call_method(bus: *mut c_void, destination: *const c_char, path: *const c_char,
        interface: *const c_char, member: *const c_char, error: *mut NativeError,
        reply: *mut *mut c_void, types: *const c_char, ...) -> c_int;
    fn sd_bus_message_read(message: *mut c_void, types: *const c_char, ...) -> c_int;
    fn sd_bus_get_property_string(bus: *mut c_void, destination: *const c_char, path: *const c_char,
        interface: *const c_char, member: *const c_char, error: *mut NativeError, value: *mut *mut c_char) -> c_int;
    fn sd_bus_get_property(bus: *mut c_void, destination: *const c_char, path: *const c_char,
        interface: *const c_char, member: *const c_char, error: *mut NativeError,
        reply: *mut *mut c_void, kind: *const c_char) -> c_int;
    fn sd_bus_message_read_array(message: *mut c_void, kind: c_char, bytes: *mut *const c_void, size: *mut usize) -> c_int;
}
