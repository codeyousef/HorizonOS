//! Fixed system-manager notification channel. Signals invalidate observations;
//! their bodies never become graph facts, permissions, or execution requests.
use super::ErrorCode;
use libc::{c_char,c_int,c_void};
use std::{ffi::CString,ptr,time::{Duration,Instant}};
const MANAGER:&str="/org/freedesktop/systemd1";
const INTERFACE:&str="org.freedesktop.systemd1.Manager";
type Result<T>=std::result::Result<T,ErrorCode>;
#[repr(C)]struct BusError{name:*const c_char,message:*const c_char,need_free:c_int}
impl BusError{fn new()->Self{Self{name:ptr::null(),message:ptr::null(),need_free:0}}}
impl Drop for BusError{fn drop(&mut self){unsafe{sd_bus_error_free(self);}}}
struct Bus(*mut c_void);
impl Drop for Bus{fn drop(&mut self){unsafe{sd_bus_close(self.0);sd_bus_unref(self.0);}}}
struct Message(*mut c_void);
impl Drop for Message{fn drop(&mut self){unsafe{sd_bus_message_unref(self.0);}}}
fn check(value:c_int)->Result<()>{if value>=0{Ok(())}else{Err(match value.checked_neg(){
 Some(libc::ETIMEDOUT)=>ErrorCode::DeadlineExceeded,Some(libc::EACCES|libc::EPERM)=>ErrorCode::PermissionDenied,
 Some(libc::ENOMEM|libc::ENOBUFS|libc::E2BIG)=>ErrorCode::ResourceExhausted,_=>ErrorCode::PartialResult})}}
// Only library-owned strings are copied; never expose native error text.
unsafe fn text(value:*const c_char)->Option<String>{
 if value.is_null(){return None;}let size=unsafe{libc::strnlen(value,1025)};if size>1024{return None;}
 std::str::from_utf8(unsafe{std::slice::from_raw_parts(value.cast(),size)}).ok().map(str::to_owned)
}
fn unique(value:&str)->bool{value.starts_with(':') && value.len()<=255 && value.len()>1 && value.bytes().skip(1).all(|b|b.is_ascii_digit()||b==b'.')}
fn accepted(owner:&str,sender:&str,path:&str,interface:&str,member:&str)->bool{
 sender==owner && ((path==MANAGER && interface==INTERFACE && matches!(member,"UnitNew"|"UnitRemoved"|"JobNew"|"JobRemoved"|"Reloading"|"UnitFilesChanged"))
 || (path.starts_with("/org/freedesktop/systemd1/unit/") && path.len()>MANAGER.len()+6 && interface=="org.freedesktop.DBus.Properties" && member=="PropertiesChanged"))
}
struct Pending{owner:String,notifications:u64}
unsafe extern "C" fn notification(m:*mut c_void,data:*mut c_void,_error:*mut BusError)->c_int{
 if m.is_null()||data.is_null(){return 0;}
 let mut kind=0;if unsafe{sd_bus_message_get_type(m,&mut kind)}<0||kind!=4{return 0;}
 let pending=unsafe{&mut *data.cast::<Pending>()};
 let fields=unsafe{(text(sd_bus_message_get_sender(m)),text(sd_bus_message_get_path(m)),text(sd_bus_message_get_interface(m)),text(sd_bus_message_get_member(m)))};
 if let (Some(sender),Some(path),Some(interface),Some(member))=fields{
  if accepted(&pending.owner,&sender,&path,&interface,&member){pending.notifications=pending.notifications.saturating_add(1);return 1;}
 }0
}
impl Bus{
 fn owner(&self)->Result<String>{
  let mut reply=Message(ptr::null_mut());let mut error=BusError::new();
  check(unsafe{sd_bus_call_method(self.0,c"org.freedesktop.DBus".as_ptr(),c"/org/freedesktop/DBus".as_ptr(),c"org.freedesktop.DBus".as_ptr(),c"GetNameOwner".as_ptr(),&mut error,&mut reply.0,c"s".as_ptr(),c"org.freedesktop.systemd1".as_ptr())})?;
  let mut value:*const c_char=ptr::null();check(unsafe{sd_bus_message_read(reply.0,c"s".as_ptr(),&mut value)})?;
  let owner=unsafe{text(value)}.filter(|v|unique(v)).ok_or(ErrorCode::PermissionDenied)?;
  let destination=CString::new(owner.as_str()).map_err(|_|ErrorCode::PermissionDenied)?;
  let mut reply=Message(ptr::null_mut());let mut uid=u32::MAX;
  check(unsafe{sd_bus_call_method(self.0,c"org.freedesktop.DBus".as_ptr(),c"/org/freedesktop/DBus".as_ptr(),c"org.freedesktop.DBus".as_ptr(),c"GetConnectionUnixUser".as_ptr(),&mut error,&mut reply.0,c"s".as_ptr(),destination.as_ptr())})?;
  check(unsafe{sd_bus_message_read(reply.0,c"u".as_ptr(),&mut uid)})?;if uid!=0{return Err(ErrorCode::PermissionDenied);}Ok(owner)
 }
}
/// A live, single-thread-owned subscription to the authenticated UID-0 system
/// manager. The fixed bus address ignores environment-provided bus addresses.
/// No unit name, object path, destination, or method is accepted from callers.
pub struct SystemdEvents{bus:Bus,pending:Box<Pending>}
#[derive(Debug,PartialEq,Eq)]pub struct EventBatch{pub notifications:u64,pub loss:bool}
impl SystemdEvents{
 pub fn connect()->Result<Self>{
  let mut bus=Bus(ptr::null_mut());check(unsafe{sd_bus_new(&mut bus.0)})?;
  check(unsafe{sd_bus_set_address(bus.0,c"unix:path=/run/dbus/system_bus_socket".as_ptr())})?;
  check(unsafe{sd_bus_set_bus_client(bus.0,1)})?;
  check(unsafe{sd_bus_set_method_call_timeout(bus.0,250_000)})?;
  check(unsafe{sd_bus_set_allow_interactive_authorization(bus.0,0)})?;check(unsafe{sd_bus_start(bus.0)})?;
  let owner=bus.owner()?;let mut watcher=Self{bus,pending:Box::new(Pending{owner:owner.clone(),notifications:0})};
  for rule in [format!("type='signal',sender='{owner}',path='{MANAGER}',interface='{INTERFACE}'"),
   format!("type='signal',sender='{owner}',path_namespace='/org/freedesktop/systemd1/unit',interface='org.freedesktop.DBus.Properties',member='PropertiesChanged'")]{
   let rule=CString::new(rule).map_err(|_|ErrorCode::PermissionDenied)?;
   check(unsafe{sd_bus_add_match(watcher.bus.0,ptr::null_mut(),rule.as_ptr(),Some(notification),(&mut *watcher.pending as *mut Pending).cast())})?;
  }
  let destination=CString::new(owner.as_str()).map_err(|_|ErrorCode::PermissionDenied)?;
  let mut error=BusError::new();let mut reply=Message(ptr::null_mut());
  check(unsafe{sd_bus_call_method(watcher.bus.0,destination.as_ptr(),c"/org/freedesktop/systemd1".as_ptr(),c"org.freedesktop.systemd1.Manager".as_ptr(),c"Subscribe".as_ptr(),&mut error,&mut reply.0,c"".as_ptr())})?;
  if watcher.bus.owner()?!=owner{return Err(ErrorCode::TargetChanged);}Ok(watcher)
 }
 pub fn manager_owner(&self)->&str{&self.pending.owner}
 /// Bound each drain to 64 operations/20ms. Reaching either bound explicitly
 /// reports possible loss and discards the subscription: callers reconnect and
 /// reconcile a native snapshot instead of treating an incomplete drain as full.
 pub fn poll(&mut self)->Result<EventBatch>{
  if self.bus.owner()?!=self.pending.owner{return Err(ErrorCode::TargetChanged);}
  let start=Instant::now();let mut drained=false;
  for _ in 0..64{
   let r=unsafe{sd_bus_process(self.bus.0,ptr::null_mut())};check(r)?;
   if r==0{drained=true;break;}if start.elapsed()>=Duration::from_millis(20){break;}
  }
  if self.bus.owner()?!=self.pending.owner{return Err(ErrorCode::TargetChanged);}
  let notifications=std::mem::take(&mut self.pending.notifications);
  Ok(EventBatch{notifications,loss:!drained})
 }
}
#[link(name="systemd")]
unsafe extern "C"{
 fn sd_bus_new(ret:*mut *mut c_void)->c_int;fn sd_bus_set_address(bus:*mut c_void,address:*const c_char)->c_int;
 fn sd_bus_set_bus_client(bus:*mut c_void,value:c_int)->c_int;fn sd_bus_set_method_call_timeout(bus:*mut c_void,usec:u64)->c_int;
 fn sd_bus_set_allow_interactive_authorization(bus:*mut c_void,value:c_int)->c_int;fn sd_bus_start(bus:*mut c_void)->c_int;
 fn sd_bus_close(bus:*mut c_void);fn sd_bus_unref(bus:*mut c_void)->*mut c_void;fn sd_bus_error_free(error:*mut BusError);
 fn sd_bus_message_unref(message:*mut c_void)->*mut c_void;
 fn sd_bus_call_method(bus:*mut c_void,destination:*const c_char,path:*const c_char,interface:*const c_char,member:*const c_char,error:*mut BusError,reply:*mut *mut c_void,types:*const c_char,...)->c_int;
 fn sd_bus_message_read(message:*mut c_void,types:*const c_char,...)->c_int;
 fn sd_bus_add_match(bus:*mut c_void,slot:*mut *mut c_void,rule:*const c_char,callback:Option<unsafe extern "C" fn(*mut c_void,*mut c_void,*mut BusError)->c_int>,data:*mut c_void)->c_int;
 fn sd_bus_process(bus:*mut c_void,ret:*mut *mut c_void)->c_int;
 fn sd_bus_message_get_type(message:*mut c_void,ret:*mut u8)->c_int;
 fn sd_bus_message_get_sender(message:*mut c_void)->*const c_char;fn sd_bus_message_get_path(message:*mut c_void)->*const c_char;
 fn sd_bus_message_get_interface(message:*mut c_void)->*const c_char;fn sd_bus_message_get_member(message:*mut c_void)->*const c_char;
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn signals_only_invalidate_from_pinned_manager_and_fixed_paths(){
  assert!(accepted(":1.0",":1.0",MANAGER,INTERFACE,"UnitNew"));
  assert!(accepted(":1.0",":1.0","/org/freedesktop/systemd1/unit/sshd_2eservice","org.freedesktop.DBus.Properties","PropertiesChanged"));
  for (sender,path,interface,member) in [(":1.2",MANAGER,INTERFACE,"UnitNew"),(":1.0","/org/freedesktop/systemd10",INTERFACE,"UnitNew"),(":1.0",MANAGER,INTERFACE,"StartUnit"),(":1.0",MANAGER,"arbitrary","UnitNew"),(":1.0","/org/freedesktop/systemd1/unit/","org.freedesktop.DBus.Properties","PropertiesChanged")]{assert!(!accepted(":1.0",sender,path,interface,member));}
  for owner in ["org.freedesktop.systemd1",":1.0','sender='attacker",":",":abc"]{assert!(!unique(owner));}
 }
 #[test]#[ignore="requires an enrolled NixOS guest, actual system bus; does not mutate a unit"]
 fn native_systemd_subscription_matches_independent_manager_owner(){
  let expected=super::super::read_loaded_services().unwrap().manager_owner;
  let mut events=SystemdEvents::connect().unwrap();assert_eq!(events.manager_owner(),expected);
  let batch=events.poll().unwrap();assert!(!batch.loss);
  println!("AIOS_NATIVE_SYSTEMD_SUBSCRIPTION owner={} notifications={} loss={} unit_mutated=false",events.manager_owner(),batch.notifications,batch.loss);
 }
}
