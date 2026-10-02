//! CPU inference only. No tool, subprocess, download or authorization API.
pub mod protocol;
pub mod service;
use aios_protocol::contracts::ErrorCode;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{ffi::{CString,CStr,c_char,c_void}, fs::{File,OpenOptions}, io::{Read,Seek,SeekFrom},
    os::{fd::AsRawFd,unix::fs::{MetadataExt,OpenOptionsExt}}, path::Path,ptr::NonNull,sync::Arc,time::{Duration,Instant}};

mod ffi {
    use super::*;
    unsafe extern "C" {
        pub fn aios_abi_version() -> u32;
        pub fn aios_runtime_revision() -> *const c_char;
        pub fn aios_cpu_backend_check() -> i32;
        pub fn aios_cancel_new() -> *mut c_void;
        pub fn aios_cancel_set(token:*mut c_void);
        pub fn aios_cancelled(token:*mut c_void) -> u32;
        pub fn aios_cancel_free(token:*mut c_void);
        pub fn aios_model_open(path:*const c_char,out:*mut *mut c_void) -> i32;
        pub fn aios_model_open_cancelable(path:*const c_char,cancel:*mut c_void,out:*mut *mut c_void) -> i32;
        pub fn aios_model_free(model:*mut c_void);
        pub fn aios_model_template(model:*mut c_void,out:*mut c_char,capacity:usize,written:*mut usize) -> i32;
        pub fn aios_chat_format(model:*mut c_void,system:*const c_char,user:*const c_char,out:*mut c_char,capacity:usize,written:*mut usize) -> i32;
        pub fn aios_context_new(model:*mut c_void,tokens:u32,threads:u32,cancel:*mut c_void,out:*mut *mut c_void) -> i32;
        pub fn aios_context_free(context:*mut c_void);
        pub fn aios_context_prompt(context:*mut c_void,prompt:*const c_char,grammar:*const c_char,maximum:u32,tokens:*mut u32) -> i32;
        pub fn aios_context_next(context:*mut c_void,out:*mut c_char,capacity:usize,written:*mut usize) -> i32;
    }
}
fn result(code:i32) -> Result<(),ErrorCode> {
    match code { 0=>Ok(()),1|8=>Err(ErrorCode::InvalidArgument),2=>Err(ErrorCode::ResourceExhausted),
        3|7=>Err(ErrorCode::ModelUnavailable),5=>Err(ErrorCode::Cancelled),_=>Err(ErrorCode::PartialResult) }
}
fn hash(bytes:&[u8]) -> String { format!("{:x}",Sha256::digest(bytes)) }
fn valid_hash(value:&str)->bool { value.len()==64 && value.bytes().all(|b|b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact { pub filename:String,pub bytes:u64,pub sha256:String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lock {
    schema_version:u32,profile:String,availability:String,source_lock_sha256:Option<String>,
    runtime_revision:String,artifact:Option<Artifact>,chat_template_sha256:String,
    converter_sha256:Option<String>,quantizer_sha256:Option<String>,quality_qualified:bool,performance_qualified:bool,
}
fn lock() -> Result<Lock,ErrorCode> {
    let lock:Lock=serde_json::from_str(include_str!("../../../models/lock.json")).map_err(|_|ErrorCode::UnsupportedSchema)?;
    if lock.schema_version!=1 || lock.profile!="normal" { return Err(ErrorCode::UnsupportedSchema); }
    if lock.availability!="available" { return Err(ErrorCode::ModelUnavailable); }
    if lock.source_lock_sha256.as_deref()!=Some(hash(include_bytes!("../../../models/source-lock.json")).as_str()) ||
        !valid_hash(&lock.chat_template_sha256) || !lock.converter_sha256.as_deref().is_some_and(valid_hash) ||
        !lock.quantizer_sha256.as_deref().is_some_and(valid_hash) { return Err(ErrorCode::TargetChanged); }
    // Quality/performance gates are explicit metadata, never authorization.
    let _=(lock.quality_qualified,lock.performance_qualified);
    Ok(lock)
}
struct NativeCancel(NonNull<c_void>);
// The native token exposes only a shared atomic flag; it does not access a
// context. Contexts retain the flag even if the original C token is released.
unsafe impl Send for NativeCancel {}
unsafe impl Sync for NativeCancel {}
impl Drop for NativeCancel { fn drop(&mut self) { unsafe { ffi::aios_cancel_free(self.0.as_ptr()); } } }
#[derive(Clone)]
pub struct Cancellation(Arc<NativeCancel>);
impl Cancellation {
    pub fn new()->Result<Self,ErrorCode> { NonNull::new(unsafe { ffi::aios_cancel_new() }).map(|v|Self(Arc::new(NativeCancel(v)))).ok_or(ErrorCode::ResourceExhausted) }
    pub fn cancel(&self) { unsafe { ffi::aios_cancel_set(self.0.0.as_ptr()); } }
    pub fn is_cancelled(&self) -> bool { unsafe { ffi::aios_cancelled(self.0.0.as_ptr()) != 0 } }
}
/// Production accepts only root-owned immutable store files. Qualification
/// permits an owned read-only converted artifact in the development guest.
pub enum ArtifactTrust { Production, Qualification }
pub struct Model { value:NonNull<c_void>,_descriptor:File }
impl Drop for Model { fn drop(&mut self) { unsafe { ffi::aios_model_free(self.value.as_ptr()); } } }
impl Model {
    pub fn load(directory:&Path,trust:ArtifactTrust) -> Result<Self,ErrorCode> {
        Self::load_cancelable(directory,trust,None)
    }
    pub fn load_cancelable(directory:&Path,trust:ArtifactTrust,cancel:Option<&Cancellation>) -> Result<Self,ErrorCode> {
        if cancel.is_some_and(Cancellation::is_cancelled) { return Err(ErrorCode::Cancelled); }
        let lock=lock()?;let artifact=lock.artifact.ok_or(ErrorCode::ModelUnavailable)?;
        if !valid_hash(&artifact.sha256) || artifact.bytes<1024 || artifact.bytes>4*1024*1024*1024 ||
            Path::new(&artifact.filename).file_name().and_then(|v|v.to_str())!=Some(artifact.filename.as_str()) ||
            !directory.is_absolute() || directory.canonicalize().map_err(|_|ErrorCode::TargetNotFound)?!=directory {
            return Err(ErrorCode::InvalidArgument);
        }
        let path=directory.join(&artifact.filename);
        let mut file=OpenOptions::new().read(true).custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK).open(&path).map_err(|_|ErrorCode::TargetNotFound)?;
        let before=file.metadata().map_err(|_|ErrorCode::TargetChanged)?;
        if !before.is_file() || before.len()!=artifact.bytes || before.mode()&0o222!=0 || before.nlink()==0 { return Err(ErrorCode::PermissionDenied); }
        match trust {
            ArtifactTrust::Production if before.uid()!=0 || !path.starts_with("/nix/store") => return Err(ErrorCode::PermissionDenied),
            ArtifactTrust::Qualification if before.uid()!=nix::unistd::geteuid().as_raw() || before.nlink()!=1 => return Err(ErrorCode::PermissionDenied),
            _=>{},
        }
        let mut digest=Sha256::new();let mut buffer=[0_u8;65536];
        loop {
            if cancel.is_some_and(Cancellation::is_cancelled) {buffer.fill(0);return Err(ErrorCode::Cancelled);}
            let read=file.read(&mut buffer).map_err(|_|ErrorCode::TargetChanged)?;if read==0 {break;}digest.update(&buffer[..read]);
        }
        buffer.fill(0);
        if format!("{:x}",digest.finalize())!=artifact.sha256 { return Err(ErrorCode::TargetChanged); }
        file.seek(SeekFrom::Start(0)).map_err(|_|ErrorCode::TargetChanged)?;
        let revision=unsafe { CStr::from_ptr(ffi::aios_runtime_revision()) }.to_str().map_err(|_|ErrorCode::TargetChanged)?;
        if unsafe { ffi::aios_abi_version() }!=1 || revision!=lock.runtime_revision { return Err(ErrorCode::TargetChanged); }
        result(unsafe { ffi::aios_cpu_backend_check() })?;
        let descriptor=CString::new(format!("/proc/self/fd/{}",file.as_raw_fd())).unwrap();let mut value=std::ptr::null_mut();
        let code=unsafe {
            match cancel {
                Some(token)=>ffi::aios_model_open_cancelable(descriptor.as_ptr(),token.0.0.as_ptr(),&mut value),
                None=>ffi::aios_model_open(descriptor.as_ptr(),&mut value),
            }
        };
        result(code)?;
        let model=Self { value:NonNull::new(value).ok_or(ErrorCode::ModelUnavailable)?,_descriptor:file };
        let after=model._descriptor.metadata().map_err(|_|ErrorCode::TargetChanged)?;
        if (before.dev(),before.ino(),before.len(),before.ctime(),before.ctime_nsec())!=
            (after.dev(),after.ino(),after.len(),after.ctime(),after.ctime_nsec()) { return Err(ErrorCode::TargetChanged); }
        let template=model.bytes(32768,|out,capacity,written|unsafe { ffi::aios_model_template(model.value.as_ptr(),out,capacity,written) })?;
        if hash(&template)!=lock.chat_template_sha256 { return Err(ErrorCode::TargetChanged); }
        Ok(model)
    }
    fn bytes(&self,maximum:usize,call:impl FnOnce(*mut c_char,usize,*mut usize)->i32)->Result<Vec<u8>,ErrorCode> {
        let mut output=vec![0;maximum];let mut count=0;
        if let Err(code)=result(call(output.as_mut_ptr().cast(),maximum,&mut count)) {service::wipe(&mut output);return Err(code);}
        if count>maximum { service::wipe(&mut output);return Err(ErrorCode::ResourceExhausted); }output.truncate(count);Ok(output)
    }
    pub fn prompt(&self,system:&str,user:&str)->Result<Vec<u8>,ErrorCode> {
        if system.len()>16384 || user.len()>65536 { return Err(ErrorCode::ResourceExhausted); }
        let system=CString::new(system).map_err(|_|ErrorCode::InvalidArgument)?;
        let user=CString::new(user).map_err(|_|ErrorCode::InvalidArgument)?;
        let value=self.bytes(131072,|out,capacity,written|unsafe { ffi::aios_chat_format(self.value.as_ptr(),system.as_ptr(),user.as_ptr(),out,capacity,written) });
        let mut system=system.into_bytes_with_nul();let mut user=user.into_bytes_with_nul();service::wipe(&mut system);service::wipe(&mut user);value
    }
    pub fn context(&self,cancel:Cancellation)->Result<Context<'_>,ErrorCode> {
        let cpus=std::thread::available_parallelism().map(|n|n.get()).unwrap_or(1);
        let threads=cpus.saturating_sub(1).clamp(1,4) as u32;let mut context=std::ptr::null_mut();
        result(unsafe { ffi::aios_context_new(self.value.as_ptr(),8192,threads,cancel.0.0.as_ptr(),&mut context) })?;
        Ok(Context {value:NonNull::new(context).ok_or(ErrorCode::ModelUnavailable)?,_model:self,_cancel:cancel})
    }
}
pub struct Context<'a> { value:NonNull<c_void>,_model:&'a Model,_cancel:Cancellation }
impl Drop for Context<'_> { fn drop(&mut self) { unsafe { ffi::aios_context_free(self.value.as_ptr()); } } }
impl Context<'_> {
    pub fn evaluate(&mut self,prompt:Vec<u8>,grammar:&str)->Result<u32,ErrorCode> {
        if prompt.len()>131072 || grammar.len()>32768 { return Err(ErrorCode::ResourceExhausted); }
        let prompt=CString::new(prompt).map_err(|_|ErrorCode::InvalidArgument)?;
        let grammar=CString::new(grammar).map_err(|_|ErrorCode::InvalidArgument)?;
        let mut tokens=0;let code=unsafe { ffi::aios_context_prompt(self.value.as_ptr(),prompt.as_ptr(),grammar.as_ptr(),6144,&mut tokens) };
        let mut bytes=prompt.into_bytes_with_nul();service::wipe(&mut bytes);
        if code==2 {return Err(ErrorCode::ContextBudgetExceeded);}result(code)?;Ok(tokens)
    }
    pub fn next(&mut self)->Result<Option<Vec<u8>>,ErrorCode> {
        let mut buffer=vec![0_u8;16384];let mut count=0;
        let code=unsafe { ffi::aios_context_next(self.value.as_ptr(),buffer.as_mut_ptr().cast(),buffer.len(),&mut count) };
        if code==6 { return Ok(None); }
        if let Err(error)=result(code) {buffer.fill(0);return Err(error);}
        if count>buffer.len() {buffer.fill(0);return Err(ErrorCode::ResourceExhausted); }buffer.truncate(count);Ok(Some(buffer))
    }
    pub fn generate(&mut self,maximum:u32,deadline:Instant,mut token:impl FnMut(&[u8]))->Result<String,ErrorCode> {
        if maximum<1 || maximum>768 || deadline.saturating_duration_since(Instant::now())>Duration::from_secs(90) { return Err(ErrorCode::InvalidArgument); }
        let mut output=Vec::new();
        for _ in 0..maximum {
            if Instant::now()>=deadline { self._cancel.cancel();service::wipe(&mut output);return Err(ErrorCode::DeadlineExceeded); }
            match self.next() {
                Ok(Some(mut bytes))=>{if output.len()+bytes.len()>65536 {service::wipe(&mut bytes);service::wipe(&mut output);return Err(ErrorCode::ResourceExhausted);}token(&bytes);output.extend_from_slice(&bytes);service::wipe(&mut bytes);},
                Ok(None)=>return String::from_utf8(output).map_err(|error|{let mut bytes=error.into_bytes();service::wipe(&mut bytes);ErrorCode::InvalidArgument}),
                Err(code)=>{service::wipe(&mut output);return Err(code);},
            }
        }
        service::wipe(&mut output);Err(ErrorCode::ResourceExhausted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn pinned_manifest_is_complete_and_hashes_are_strict() {
        let manifest=lock().expect("the qualification artifact must have a complete pinned manifest");
        assert!(manifest.artifact.is_some());
        assert!(!valid_hash("not-a-hash"));assert!(!valid_hash(&"A".repeat(64)));
    }
    #[test] fn unsafe_and_corrupted_artifacts_are_rejected_before_native_loading() {
        use std::os::unix::fs::{symlink,PermissionsExt};
        struct Directory(std::path::PathBuf);
        impl Drop for Directory { fn drop(&mut self) { let _=std::fs::remove_dir_all(&self.0); } }
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let directory=Directory(std::env::temp_dir().join(format!("horizon-model-contract-{}-{nonce}",std::process::id())));
        std::fs::create_dir(&directory.0).unwrap();
        let artifact=lock().unwrap().artifact.unwrap();let path=directory.0.join(artifact.filename);
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::TargetNotFound)));
        let fifo=CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe {nix::libc::mkfifo(fifo.as_ptr(),0o400)},0);
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::PermissionDenied)));
        std::fs::remove_file(&path).unwrap();
        symlink("/etc/os-release",&path).unwrap();
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::TargetNotFound)));
        std::fs::remove_file(&path).unwrap();
        let file=OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
        file.set_len(artifact.bytes).unwrap();drop(file);
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::PermissionDenied)));
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o444)).unwrap();
        std::fs::hard_link(&path,directory.0.join("alias.gguf")).unwrap();
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::PermissionDenied)));
        std::fs::remove_file(directory.0.join("alias.gguf")).unwrap();
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Production),Err(ErrorCode::PermissionDenied)));
        assert!(matches!(Model::load(&directory.0,ArtifactTrust::Qualification),Err(ErrorCode::TargetChanged)));
    }
    #[test] fn atomic_cancellation_handle_can_outlive_its_requester() {
        let cancel=Cancellation::new().unwrap();let other=cancel.clone();
        std::thread::spawn(move||other.cancel()).join().unwrap();drop(cancel);
    }
}
