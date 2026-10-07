//! Native read-only block-disk properties. No caller-selected sysfs/device path,
//! raw block access, property write, command, or storage effect is accepted.
use aios_protocol::contracts::ErrorCode;
use serde::Serialize;
use libc::{c_char,c_int,c_void,dev_t};
use std::{collections::BTreeSet,time::{Duration,Instant}};
type Result<T>=std::result::Result<T,ErrorCode>;
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct BlockDevice{
 pub syspath:String,pub devpath:String,pub sysname:String,pub major:u32,pub minor:u32,
 pub initialized:bool,pub serial:Option<String>,pub serial_short:Option<String>,pub wwn:Option<String>,
 pub bus:Option<String>,pub model:Option<String>,pub vendor:Option<String>,pub capacity_bytes:Option<u64>,
 pub removable:Option<bool>,pub read_only:Option<bool>,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize)]
pub struct BlockDevices{pub devices:Vec<BlockDevice>,pub complete:bool}
struct Context(*mut c_void);impl Drop for Context{fn drop(&mut self){unsafe{udev_unref(self.0);}}}
struct Enumeration(*mut c_void);impl Drop for Enumeration{fn drop(&mut self){unsafe{udev_enumerate_unref(self.0);}}}
struct Device(*mut c_void);impl Drop for Device{fn drop(&mut self){unsafe{udev_device_unref(self.0);}}}
fn check(value:c_int)->Result<()>{if value>=0{Ok(())}else{Err(ErrorCode::PartialResult)}}
unsafe fn text(value:*const c_char,limit:usize)->Result<Option<String>>{
 if value.is_null(){return Ok(None);}let n=unsafe{libc::strnlen(value,limit+1)};if n>limit{return Err(ErrorCode::ResourceExhausted);}
 let value=std::str::from_utf8(unsafe{std::slice::from_raw_parts(value.cast(),n)}).map_err(|_|ErrorCode::PartialResult)?;
 if value.chars().any(char::is_control){return Err(ErrorCode::PartialResult);}Ok(if value.is_empty(){None}else{Some(value.into())})
}
fn property(device:&Device,name:&std::ffi::CStr)->Result<Option<String>>{unsafe{text(udev_device_get_property_value(device.0,name.as_ptr()),256)}}
fn sysattr(device:&Device,name:&std::ffi::CStr)->Result<Option<String>>{unsafe{text(udev_device_get_sysattr_value(device.0,name.as_ptr()),64)}}
fn decimal(value:Option<String>)->Result<Option<u64>>{value.map(|value|value.parse().map_err(|_|ErrorCode::PartialResult)).transpose()}
fn boolean(value:Option<String>)->Result<Option<bool>>{match value.as_deref(){None=>Ok(None),Some("0")=>Ok(Some(false)),Some("1")=>Ok(Some(true)),_=>Err(ErrorCode::PartialResult)}}
fn required(value:Option<String>)->Result<String>{value.ok_or(ErrorCode::PartialResult)}
/// Fixed enumeration includes only native block disks, not partitions or other
/// hardware classes. An uninitialized/disappearing device makes it Partial.
/// Missing stable properties remain None, never a fabricated serial or WWN.
pub fn read_block_devices()->Result<BlockDevices>{
 let start=Instant::now();let context=Context(unsafe{udev_new()});if context.0.is_null(){return Err(ErrorCode::ResourceExhausted);}
 let enumeration=Enumeration(unsafe{udev_enumerate_new(context.0)});if enumeration.0.is_null(){return Err(ErrorCode::ResourceExhausted);}
 check(unsafe{udev_enumerate_add_match_subsystem(enumeration.0,c"block".as_ptr())})?;
 check(unsafe{udev_enumerate_scan_devices(enumeration.0)})?;
 let mut entry=unsafe{udev_enumerate_get_list_entry(enumeration.0)};let mut count=0;let mut devices=Vec::new();let mut complete=true;let mut seen=BTreeSet::new();
 while !entry.is_null(){
  count+=1;if count>4096 || start.elapsed()>=Duration::from_secs(2){return Err(ErrorCode::ResourceExhausted);}
  let native=unsafe{udev_list_entry_get_name(entry)};
  let path=required(unsafe{text(native,1024)}?)?;
  if !path.starts_with("/sys/devices/") || path.contains("/../"){return Err(ErrorCode::TargetChanged);}
  let device=Device(unsafe{udev_device_new_from_syspath(context.0,native)});
  entry=unsafe{udev_list_entry_get_next(entry)};
  if device.0.is_null(){complete=false;continue;}
  let kind=unsafe{text(udev_device_get_devtype(device.0),32)}?;
  if kind.as_deref()!=Some("disk"){if kind.as_deref()!=Some("partition"){complete=false;}continue;}
  if devices.len()>=256{return Err(ErrorCode::ResourceExhausted);}
  let syspath=required(unsafe{text(udev_device_get_syspath(device.0),1024)}?)?;
  let devpath=required(unsafe{text(udev_device_get_devpath(device.0),1024)}?)?;
  let sysname=required(unsafe{text(udev_device_get_sysname(device.0),255)}?)?;
  let number=unsafe{udev_device_get_devnum(device.0)};
  let major=libc::major(number);let minor=libc::minor(number);
  if syspath!=path || devpath!=syspath.strip_prefix("/sys").unwrap_or("") || major==0 || !seen.insert((major,minor)) || sysname.contains('/'){
   return Err(ErrorCode::TargetChanged);
  }
  let initialized=unsafe{udev_device_get_is_initialized(device.0)}>0;if !initialized{complete=false;}
  let sectors=decimal(sysattr(&device,c"size")?)?;
  let capacity_bytes=sectors.map(|value|value.checked_mul(512).ok_or(ErrorCode::ResourceExhausted)).transpose()?;
  devices.push(BlockDevice{syspath,devpath,sysname,major,minor,initialized,
   serial:property(&device,c"ID_SERIAL")?,serial_short:property(&device,c"ID_SERIAL_SHORT")?,wwn:property(&device,c"ID_WWN")?,
   bus:property(&device,c"ID_BUS")?,model:property(&device,c"ID_MODEL")?,vendor:property(&device,c"ID_VENDOR")?,capacity_bytes,
   removable:boolean(sysattr(&device,c"removable")?)?,read_only:boolean(sysattr(&device,c"ro")?)?});
 }
 devices.sort_by(|a,b|a.syspath.cmp(&b.syspath));Ok(BlockDevices{devices,complete})
}
#[link(name="udev")]
unsafe extern "C"{
 fn udev_new()->*mut c_void;fn udev_unref(context:*mut c_void)->*mut c_void;
 fn udev_enumerate_new(context:*mut c_void)->*mut c_void;fn udev_enumerate_unref(enumeration:*mut c_void)->*mut c_void;
 fn udev_enumerate_add_match_subsystem(enumeration:*mut c_void,subsystem:*const c_char)->c_int;
 fn udev_enumerate_scan_devices(enumeration:*mut c_void)->c_int;fn udev_enumerate_get_list_entry(enumeration:*mut c_void)->*mut c_void;
 fn udev_list_entry_get_name(entry:*mut c_void)->*const c_char;fn udev_list_entry_get_next(entry:*mut c_void)->*mut c_void;
 fn udev_device_new_from_syspath(context:*mut c_void,path:*const c_char)->*mut c_void;fn udev_device_unref(device:*mut c_void)->*mut c_void;
 fn udev_device_get_devtype(device:*mut c_void)->*const c_char;fn udev_device_get_syspath(device:*mut c_void)->*const c_char;
 fn udev_device_get_devpath(device:*mut c_void)->*const c_char;fn udev_device_get_sysname(device:*mut c_void)->*const c_char;
 fn udev_device_get_devnum(device:*mut c_void)->dev_t;fn udev_device_get_is_initialized(device:*mut c_void)->c_int;
 fn udev_device_get_property_value(device:*mut c_void,name:*const c_char)->*const c_char;
 fn udev_device_get_sysattr_value(device:*mut c_void,name:*const c_char)->*const c_char;
}
