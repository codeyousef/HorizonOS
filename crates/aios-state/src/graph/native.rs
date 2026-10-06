//! Native graph adapters. No deserialized identity, arbitrary path, execution
//! API or cached observation is accepted as authorization. Retained descriptors
//! belong to the enclosing broker's authenticated task/read-scope lifetime.
use super::{BootId,ObservationTime,ProcessIdentity,SourceRevision,Freshness,FreshnessClass,freshness};
use super::store::{GraphStore,Node,Scope,SourceTruth,ProviderSnapshot,ProviderState};
use aios_protocol::contracts::ErrorCode;
use aios_system::processes::{self,OwnProcess,Observation};
use std::{fs::OpenOptions,io::Read,os::{fd::AsRawFd,unix::fs::OpenOptionsExt}};
pub const PROCESS_PROVIDER:&str="native-own-processes";
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error {Graph(super::store::Error),Native(ErrorCode),Clock,WrongScope,Expired}
pub type Result<T>=std::result::Result<T,Error>;
impl From<super::store::Error> for Error{fn from(value:super::store::Error)->Self{Self::Graph(value)}}
impl From<ErrorCode> for Error{fn from(value:ErrorCode)->Self{Self::Native(value)}}
fn native_boot()->Result<BootId>{
    let file=OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC).open("/proc/sys/kernel/random/boot_id").map_err(|_|Error::Clock)?;
    let mut filesystem=std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe{libc::fstatfs(file.as_raw_fd(),filesystem.as_mut_ptr())}!=0 || unsafe{filesystem.assume_init()}.f_type!=libc::PROC_SUPER_MAGIC{return Err(Error::Clock);}
    let mut bytes=String::new();file.take(65).read_to_string(&mut bytes).map_err(|_|Error::Clock)?;
    if bytes.len()>64{return Err(Error::Clock);}BootId::parse(bytes.trim()).ok_or(Error::Clock)
}
fn clock(id:libc::clockid_t)->Result<u64>{
    let mut value=std::mem::MaybeUninit::<libc::timespec>::uninit();
    if unsafe{libc::clock_gettime(id,value.as_mut_ptr())}!=0{return Err(Error::Clock);}let value=unsafe{value.assume_init()};
    if value.tv_sec<0 || !(0..1_000_000_000).contains(&value.tv_nsec){return Err(Error::Clock);}
    u64::try_from(value.tv_sec).ok().and_then(|s|s.checked_mul(1_000_000_000)).and_then(|s|s.checked_add(value.tv_nsec as u64))
        .filter(|&v|v<=i64::MAX as u64).ok_or(Error::Clock)
}
/// Captured through fixed procfs and kernel clocks. No Deserialize or public
/// constructor accepts caller/model clock or boot claims.
#[derive(Clone,Debug)]
pub struct NativeTime(ObservationTime);
impl NativeTime{
    pub fn observe()->Result<Self>{
        let boot=native_boot()?;let monotonic_ns=clock(libc::CLOCK_MONOTONIC)?;let realtime_ns=clock(libc::CLOCK_REALTIME)?;
        if native_boot()?!=boot{return Err(Error::Clock);}Ok(Self(ObservationTime{boot,monotonic_ns,realtime_ns}))
    }
    pub fn observation(&self)->&ObservationTime{&self.0}
}
struct Entry {key:String,process:OwnProcess}
/// A live native read. The payload remains an observation, never a task grant.
#[derive(Debug)]
pub struct LiveProcessObservation {pub captured:ObservationTime,pub observation:Observation}
/// A native own-UID census and its pre-enumeration checkpoint. Retained pidfds
/// prevent PID reuse from rebinding a cached node to another process. This
/// adapter does not claim visibility into root/other users' process inventories.
pub struct NativeProcessSnapshot {uid:u32,captured:NativeTime,expected_token:Option<String>,complete:bool,access_denied:bool,entries:Vec<Entry>}
impl NativeProcessSnapshot{
    fn scope(store:&GraphStore)->Result<u32>{
        let uid=unsafe{libc::geteuid()};
        if uid==0 || store.native_scope()!=Scope::User(uid){return Err(Error::WrongScope);}Ok(uid)
    }
    pub fn collect(store:&GraphStore)->Result<Self>{
        let uid=Self::scope(store)?;
        let result=(||{
            let captured=NativeTime::observe()?;
            // Capture CAS before enumeration, not after a potentially missed
            // event. A notification racing collection makes apply refuse.
            let expected_token=store.reconciliation_plan(PROCESS_PROVIDER.into(),captured.0.clone(),SourceRevision::default())?.state.map(|s|s.token);
            let inventory=processes::inventory()?;let mut complete=!inventory.access_denied;let mut entries=Vec::new();
            for process in inventory.processes{
                match process.inspect(){
                    Ok(observation)=>{
                        let identity=&observation.identity;
                        if identity.uid!=uid || BootId::parse(&identity.boot_id)!=Some(captured.0.boot.clone()){return Err(Error::Clock);}
                        let key=ProcessIdentity::observed(uid,captured.0.boot.clone(),identity.pid,identity.start_time_ticks).ok_or(Error::Clock)?.stable_key();
                        entries.push(Entry{key,process});
                    },
                    Err(ErrorCode::TargetNotFound|ErrorCode::TargetChanged|ErrorCode::PermissionDenied)=>complete=false,
                    Err(error)=>return Err(Error::Native(error)),
                }
            }
            let snapshot=Self{uid,captured,expected_token,complete,access_denied:inventory.access_denied,entries};snapshot.fresh()?;Ok(snapshot)
        })();
        if result.is_err(){store.report_event_loss();}result
    }
    fn fresh(&self)->Result<NativeTime>{
        if unsafe{libc::geteuid()}!=self.uid{return Err(Error::WrongScope);}
        let now=NativeTime::observe()?;
        match freshness(FreshnessClass::Process,&self.captured.0,&now.0,&SourceRevision::default(),&SourceRevision::default()){
            Freshness::Current=>Ok(now),Freshness::Stale=>Err(Error::Expired),Freshness::Unknown=>Err(Error::Clock),
        }
    }
    pub fn captured(&self)->&ObservationTime{&self.captured.0}
    pub fn is_complete(&self)->bool{self.complete}
    pub fn has_access_denials(&self)->bool{self.access_denied}
    pub fn ids(&self)->impl Iterator<Item=&str>{self.entries.iter().map(|e|e.key.as_str())}
    /// Re-inspect a retained native object. No value read from graph properties
    /// is used to choose a PID or acquire another handle. Caller read scope
    /// remains the enclosing broker's responsibility, including revocation.
    pub fn read_live(&self,id:&str)->Result<LiveProcessObservation>{
        if id.len()>128{return Err(Error::Native(ErrorCode::TargetNotFound));}
        let entry=self.entries.iter().find(|e|e.key==id).ok_or(Error::Native(ErrorCode::TargetNotFound))?;
        let before=NativeTime::observe()?;let observation=entry.process.inspect()?;let after=NativeTime::observe()?;
        if before.0.boot!=after.0.boot || BootId::parse(&observation.identity.boot_id)!=Some(after.0.boot.clone())
            || after.0.monotonic_ns<before.0.monotonic_ns{return Err(Error::Clock);}
        Ok(LiveProcessObservation{captured:before.0,observation})
    }
    pub fn apply(&self,store:&GraphStore)->Result<ProviderState>{
        if Self::scope(store)?!=self.uid{return Err(Error::WrongScope);}
        let result=(||{
            self.fresh()?;let mut complete=self.complete;let mut nodes=Vec::new();let mut verified_absent_ids=Vec::new();
            for entry in &self.entries{
                match entry.process.inspect(){
                    Ok(observation)=>nodes.push(Node{id:entry.key.clone(),kind:"process".into(),scope:Scope::User(self.uid),provider:PROCESS_PROVIDER.into(),
                        stable_key:entry.key.clone(),properties:serde_json::json!({"observation":observation,"captured":self.captured.0}),
                        source_truth:SourceTruth::Running,realtime_ns:self.captured.0.realtime_ns}),
                    Err(ErrorCode::TargetNotFound)=>{complete=false;if entry.process.exited()?{verified_absent_ids.push(entry.key.clone());}},
                    Err(ErrorCode::TargetChanged|ErrorCode::PermissionDenied)=>complete=false,
                    Err(error)=>return Err(Error::Native(error)),
                }
            }
            self.fresh()?;
            Ok(store.apply_provider_snapshot(ProviderSnapshot{provider:PROCESS_PROVIDER.into(),expected_token:self.expected_token.clone(),source_truth:SourceTruth::Running,
                time:self.captured.0.clone(),source_revision:SourceRevision::default(),complete,nodes,verified_absent_ids})?)
        })();
        if result.is_err(){store.report_event_loss();}result
    }
}
#[cfg(test)] mod tests{
    use super::*;
    #[test] fn native_clock_fields_and_boot_are_bounded(){
        let before=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let first=NativeTime::observe().unwrap();let second=NativeTime::observe().unwrap();
        let after=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        assert_eq!(first.0.boot,second.0.boot);assert!(second.0.monotonic_ns>=first.0.monotonic_ns);
        assert!((before..=after).contains(&(first.0.realtime_ns as u128)));assert_eq!(first.0.boot,native_boot().unwrap());
    }
}


pub const SYSTEMD_PROVIDER:&str="native-systemd-loaded-services";
/// Identity of a system-manager unit, independent of an individual activation.
/// Source truth and provider scope remain pinned by the graph store.
pub fn system_service_key(name:&str)->Result<String>{
    aios_system::services::validate_service_name(name)?;
    use sha2::{Digest,Sha256};
    let bytes=serde_json::to_vec(&("systemd-system-service",name)).map_err(|_|Error::Clock)?;
    Ok(format!("service:{:x}",Sha256::digest(bytes)))
}
pub struct NativeSystemdSnapshot {captured:NativeTime,expected_token:Option<String>,inventory:aios_system::services::LoadedServices}
impl NativeSystemdSnapshot{
    pub fn collect(store:&GraphStore)->Result<Self>{
        if store.native_scope()!=Scope::System{return Err(Error::WrongScope);}
        let result=(||{
            let captured=NativeTime::observe()?;
            let expected_token=store.reconciliation_plan(SYSTEMD_PROVIDER.into(),captured.0.clone(),SourceRevision::default())?.state.map(|s|s.token);
            let inventory=aios_system::services::read_loaded_services()?;
            if BootId::parse(&inventory.boot_id)!=Some(captured.0.boot.clone()){return Err(Error::Clock);}
            let snapshot=Self{captured,expected_token,inventory};snapshot.fresh()?;Ok(snapshot)
        })();if result.is_err(){store.report_event_loss();}result
    }
    fn fresh(&self)->Result<()>{
        let now=NativeTime::observe()?;
        match freshness(FreshnessClass::Service,&self.captured.0,&now.0,&SourceRevision::default(),&SourceRevision::default()){
            Freshness::Current=>Ok(()),Freshness::Stale=>Err(Error::Expired),Freshness::Unknown=>Err(Error::Clock),
        }
    }
    pub fn ids(&self)->Result<Vec<String>>{self.inventory.services.iter().map(|s|system_service_key(&s.unit_name)).collect()}
    pub fn captured(&self)->&ObservationTime{&self.captured.0}
    pub fn apply(&self,store:&GraphStore)->Result<ProviderState>{
        if store.native_scope()!=Scope::System{return Err(Error::WrongScope);}
        let result=(||{
            self.fresh()?;
            let mut nodes=Vec::new();for service in &self.inventory.services{
                let id=system_service_key(&service.unit_name)?;
                nodes.push(Node{id:id.clone(),kind:"service".into(),scope:Scope::System,provider:SYSTEMD_PROVIDER.into(),stable_key:id,
                    properties:serde_json::json!({"observation":service,"captured":self.captured.0,"manager_owner":self.inventory.manager_owner,
                        "manager_version":self.inventory.manager_version,"main_pid":null,"result":null,"ordering_after":null}),
                    source_truth:SourceTruth::Running,realtime_ns:self.captured.0.realtime_ns});
            }
            self.fresh()?;Ok(store.apply_provider_snapshot(ProviderSnapshot{provider:SYSTEMD_PROVIDER.into(),expected_token:self.expected_token.clone(),source_truth:SourceTruth::Running,
                time:self.captured.0.clone(),source_revision:SourceRevision::default(),complete:self.inventory.complete,nodes,verified_absent_ids:vec![]})?)
        })();if result.is_err(){store.report_event_loss();}result
    }
}
