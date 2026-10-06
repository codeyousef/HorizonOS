//! Private system graph owner. This internal control socket is accessible only
//! to the graph identity and root; it is not a model/user task RPC surface.
use super::{native::{NativeTime,NativeSystemdSnapshot,SYSTEMD_PROVIDER,system_service_key},generations::{self,NativeGenerationSnapshot,SystemPointers,RUNNING_PROVIDER,PROFILE_PROVIDER},store::{GraphStore,Scope,ProviderStatus,ProviderEvent,EventKind},SourceRevision,ObservationTime,FreshnessClass,freshness};
use serde::{Deserialize,Serialize};use serde_json::{Value,json};
use std::{fs,io::{self,Read,Write},os::{fd::AsRawFd,unix::{net::{UnixListener,UnixStream},fs::{MetadataExt,PermissionsExt,FileTypeExt}}},path::Path,time::Duration};
const STATE:&str="/var/lib/aios/state";const RUNTIME:&str="/run/aios-state";const SOCKET:&str="/run/aios-state/owner.sock";
const MAX_REPLY:usize=2*1024*1024;const PERIOD_NS:u64=900_000_000_000;
#[derive(Debug)]pub enum Error {Io,Identity,Protocol,Graph(super::store::Error),Native(super::native::Error)}
impl From<io::Error> for Error{fn from(_:io::Error)->Self{Self::Io}}
impl From<super::store::Error> for Error{fn from(e:super::store::Error)->Self{Self::Graph(e)}}
impl From<super::native::Error> for Error{fn from(e:super::native::Error)->Self{Self::Native(e)}}
type Result<T>=std::result::Result<T,Error>;
#[derive(Debug,Deserialize,Serialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
enum Request{Status{},Reconcile{},Generations{},Service{unit_name:String}}
fn identity()->Result<u32>{
    // Fixed account lookup at startup, before spawning any worker threads.
    let account=unsafe{libc::getpwnam(c"aios-state".as_ptr())};
    if account.is_null(){return Err(Error::Identity);}let uid=unsafe{(*account).pw_uid};if uid==0{return Err(Error::Identity);}Ok(uid)
}
fn peer(stream:&UnixStream)->Result<libc::ucred>{
    let mut credentials=std::mem::MaybeUninit::<libc::ucred>::uninit();let mut size=std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    if unsafe{libc::getsockopt(stream.as_raw_fd(),libc::SOL_SOCKET,libc::SO_PEERCRED,credentials.as_mut_ptr().cast(),&mut size)}!=0 || size as usize!=std::mem::size_of::<libc::ucred>(){return Err(Error::Identity);}
    let credentials=unsafe{credentials.assume_init()};if credentials.pid<=0{return Err(Error::Identity);}Ok(credentials)
}
fn directory(path:&str,uid:u32)->Result<()>{
    let meta=fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.uid()!=uid || meta.mode()&0o777!=0o700 || fs::canonicalize(path)?!=Path::new(path){return Err(Error::Identity);}Ok(())
}
fn frame_read(stream:&mut UnixStream,bound:usize)->Result<Vec<u8>>{
    let mut length=[0;4];stream.read_exact(&mut length)?;let length=u32::from_be_bytes(length) as usize;
    if length==0 || length>bound{return Err(Error::Protocol);}let mut bytes=vec![0;length];stream.read_exact(&mut bytes)?;Ok(bytes)
}
fn frame_write(stream:&mut UnixStream,value:&Value)->Result<()>{
    let bytes=serde_json::to_vec(value).map_err(|_|Error::Protocol)?;if bytes.len()>MAX_REPLY{return Err(Error::Protocol);}
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;stream.write_all(&bytes)?;Ok(())
}
struct GenerationCache{captured:ObservationTime,pointers:SystemPointers}
fn generation_refresh_needed(previous:Option<&SystemPointers>,observed:&SystemPointers,failed:bool)->bool{failed || previous!=Some(observed)}
struct Owner{graph:GraphStore,ids:Vec<String>,attempts:u64,successful:u64,last_attempt:ObservationTime,last_error:Option<String>,
    generations:Option<GenerationCache>,generation_error:Option<String>,last_generation_probe:ObservationTime,generation_changes:u64,event_watcher:bool,event_error:Option<String>,systemd_notifications:u64,event_refreshes:u64}
impl Owner{
    fn systemd_event(&mut self,notifications:u64,loss:bool)->Result<()>{
        let captured=NativeTime::observe()?.observation().clone();
        self.systemd_notifications=self.systemd_notifications.saturating_add(notifications);
        self.event_refreshes=self.event_refreshes.checked_add(1).ok_or(Error::Protocol)?;
        self.graph.ingest_provider_events(SYSTEMD_PROVIDER.into(),vec![ProviderEvent{
            id:format!("systemd:{}:{}:{}",captured.boot.0,captured.monotonic_ns,self.event_refreshes),entity_id:None,
            kind:if loss{EventKind::Lost}else{EventKind::Changed},time:captured,origin_transaction_id:None,
            payload:json!({"source":"pinned-root-systemd-signals","coalesced_notifications":notifications,"possible_loss":loss})}])?;
        // Notification content never becomes state. Rebuild through native
        // providers even on loss/disconnect/reconnection, without inference.
        self.reconcile()
    }
    fn reconcile_generations(&mut self)->Result<()>{
        let result=(||{
            let snapshot=NativeGenerationSnapshot::collect(&self.graph)?;snapshot.apply(&self.graph)?;
            self.generations=Some(GenerationCache{captured:snapshot.captured().clone(),pointers:snapshot.pointers().clone()});Ok(())
        })();
        self.generation_error=result.as_ref().err().map(|e:&Error|format!("{e:?}"));result
    }
    fn reconcile(&mut self)->Result<()>{
        self.last_attempt=NativeTime::observe()?.observation().clone();self.attempts=self.attempts.checked_add(1).ok_or(Error::Protocol)?;
        // Each provider owns its source truth/checkpoint. A failed generation
        // read does not turn service facts into generation or managed facts.
        let generations=self.reconcile_generations();
        let result=(||{
            let snapshot=NativeSystemdSnapshot::collect(&self.graph)?;let state=snapshot.apply(&self.graph)?;
            self.ids=snapshot.ids()?;self.last_error=if state.status==ProviderStatus::Ready{None}else{Some("INCOMPLETE_SNAPSHOT".into())};
            if state.status==ProviderStatus::Ready{self.successful=self.successful.checked_add(1).ok_or(Error::Protocol)?;}Ok(())
        })();if let Err(error)=&result{self.graph.report_event_loss();self.last_error=Some(format!("{error:?}"));}result.and(generations)
    }
    fn poll_generations(&mut self)->Result<()>{
        let now=NativeTime::observe()?.observation().clone();
        if now.boot==self.last_generation_probe.boot && now.monotonic_ns.checked_sub(self.last_generation_probe.monotonic_ns).is_some_and(|age|age<1_000_000_000){return Ok(());}
        self.last_generation_probe=now;
        let (captured,observed)=match generations::observe(){
            Ok(value)=>value,
            Err(error)=>{if self.generation_error.is_none(){self.graph.report_event_loss();}self.generation_error=Some(format!("{error:?}"));return Err(error.into());}
        };
        let prior=self.generations.as_ref().map(|g|&g.pointers);
        if generation_refresh_needed(prior,&observed,self.generation_error.is_some()){
            if let Some(prior)=prior.filter(|p|*p!=&observed){
                use sha2::{Digest,Sha256};
                // Native poll observations, not model or command notifications.
                // Stable unchanged samples produce no event or reconciliation.
                for provider in [RUNNING_PROVIDER,PROFILE_PROVIDER,SYSTEMD_PROVIDER]{
                    let payload=json!({"before":prior,"after":observed,"source":"fixed-native-system-pointer-poll"});
                    let encoded=serde_json::to_vec(&(provider,&captured,&payload)).map_err(|_|Error::Protocol)?;
                    self.graph.ingest_provider_events(provider.into(),vec![ProviderEvent{id:format!("generation:{:x}",Sha256::digest(encoded)),
                        entity_id:None,kind:EventKind::GenerationChanged,time:captured.clone(),origin_transaction_id:None,payload}])?;
                }
                self.generation_changes=self.generation_changes.checked_add(1).ok_or(Error::Protocol)?;
            }
            self.reconcile()?;
        }
        Ok(())
    }
    fn generation_view(&self)->Result<Value>{
        let current=generations::observe();
        let data=if let Some(cached)=&self.generations{
            let freshness=match &current{
                Ok((now,pointers)) if now.boot!=cached.captured.boot || pointers!=&cached.pointers=>"Stale",
                Ok((now,_)) if self.generation_error.is_none()=>{
                    let revision=SourceRevision{generation:Some(cached.pointers.running_closure.clone()),profile:cached.pointers.selected_profile_closure.clone(),closure_hash:None,document_hash:None};
                    let mut eligible=true;for provider in [RUNNING_PROVIDER,PROFILE_PROVIDER]{
                        eligible &= self.graph.reconciliation_plan(provider.into(),now.clone(),revision.clone())?.state.is_some_and(|s|s.status==ProviderStatus::Ready && s.error.is_none());
                    }
                    if eligible{"Current"}else{"Unknown"}
                },_=>"Unknown",
            };
            json!({"pointers":cached.pointers,"captured":cached.captured,"freshness":freshness,
                "running_source_truth":"running","profile_source_truth":"boot_selected","bootloader_selection_verified":false,"managed_provenance_verified":false})
        }else{Value::Null};
        Ok(json!({"schema_version":1,"data":data,"error":self.generation_error,"live_compared_at":current.as_ref().ok().map(|(t,_)|t),
            "source":"cached-native-system-pointers","execution_authority":false}))
    }
    fn status(&self)->Result<Value>{
        let now=NativeTime::observe()?;let plan=self.graph.reconciliation_plan(SYSTEMD_PROVIDER.into(),now.observation().clone(),SourceRevision::default())?;
        let provider=plan.state.map(|s|json!({"provider":s.provider,"status":s.status,"last_success":s.last_success,"error":s.error}));
        Ok(json!({"schema_version":1,"scope":"system","boot":now.observation().boot,"provider":provider,"reconciliations_attempted":self.attempts,
            "complete_reconciliations":self.successful,"last_attempt":self.last_attempt,"last_error":self.last_error,"observed_loaded_services":self.ids.len(),
            "period_seconds":900,"model_invoked":false,"execution_authority":false,"event_watcher_installed":self.event_watcher,"event_watcher_error":self.event_error,
            "systemd_notifications":self.systemd_notifications,"event_reconciliations":self.event_refreshes,
            "generation_poll_seconds":1,"observed_generation_changes":self.generation_changes,"generation_error":self.generation_error,
            "quarantine":self.graph.recovery_receipt().map(|r|r.quarantine_directory.file_name().unwrap_or_default().to_string_lossy())}))
    }
    fn request(&mut self,request:Request)->Result<Value>{
        match request{
            Request::Status{}=>self.status(),
            Request::Generations{}=>self.generation_view(),
            Request::Reconcile{}=>{
                let now=NativeTime::observe()?;
                if now.observation().boot!=self.last_attempt.boot || now.observation().monotonic_ns.checked_sub(self.last_attempt.monotonic_ns).is_none_or(|age|age>=1_000_000_000){self.reconcile()?;}
                self.status()
            },
            Request::Service{unit_name}=>{
                let id=system_service_key(&unit_name)?;let rows=self.graph.nodes(vec![id])?;let now=NativeTime::observe()?;
                let state=self.graph.reconciliation_plan(SYSTEMD_PROVIDER.into(),now.observation().clone(),SourceRevision::default())?.state;
                let data=if let Some(row)=rows.into_iter().next(){
                    let captured:ObservationTime=serde_json::from_value(row.properties.get("captured").cloned().ok_or(Error::Protocol)?).map_err(|_|Error::Protocol)?;
                    let eligible=state.as_ref().is_some_and(|s|s.status==ProviderStatus::Ready && s.error.is_none());
                    json!({"id":row.id,"source_truth":row.source_truth,"revision":row.revision,"properties":row.properties,
                        "freshness":if eligible{format!("{:?}",freshness(FreshnessClass::Service,&captured,now.observation(),&SourceRevision::default(),&SourceRevision::default()))}else{"Unknown".into()}})
                }else{Value::Null};
                Ok(json!({"schema_version":1,"data":data,"source":"cached-native-loaded-system-services","execution_authority":false}))
            }
        }
    }
}
fn serve(mut stream:UnixStream,uid:u32,owner:&mut Owner)->Result<()>{
    let credentials=peer(&stream)?;if credentials.uid!=uid && credentials.uid!=0{return Err(Error::Identity);}
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    let request:Request=serde_json::from_slice(&frame_read(&mut stream,4096)?).map_err(|_|Error::Protocol)?;
    let reply=match owner.request(request){Ok(data)=>json!({"ok":true,"data":data}),Err(error)=>json!({"ok":false,"error":format!("{error:?}")})};
    frame_write(&mut stream,&reply)
}
fn run(uid:u32)->Result<()>{
    if unsafe{libc::geteuid()}!=uid || unsafe{libc::getuid()}!=uid{return Err(Error::Identity);}
    directory(STATE,uid)?;directory(RUNTIME,uid)?;
    let graph=GraphStore::open(Path::new(STATE),Scope::System)?;
    if let Ok(meta)=fs::symlink_metadata(SOCKET){
        if !meta.file_type().is_socket() || meta.uid()!=uid || meta.mode()&0o777!=0o600 || UnixStream::connect(SOCKET).is_ok(){return Err(Error::Identity);}fs::remove_file(SOCKET)?;
    }
    let listener=UnixListener::bind(SOCKET)?;fs::set_permissions(SOCKET,fs::Permissions::from_mode(0o600))?;listener.set_nonblocking(true)?;
    let now=NativeTime::observe()?.observation().clone();
    let mut owner=Owner{graph,ids:Vec::new(),attempts:0,successful:0,last_attempt:now.clone(),last_error:None,
        generations:None,generation_error:None,last_generation_probe:now,generation_changes:0,event_watcher:false,event_error:None,systemd_notifications:0,event_refreshes:0};
    let mut watcher=aios_system::services::events::SystemdEvents::connect().ok();
    owner.event_watcher=watcher.is_some();
    if watcher.is_none(){owner.event_error=Some("SUBSCRIPTION_UNAVAILABLE".into());owner.graph.report_event_loss();}
    let mut last_connection=std::time::Instant::now();
    if let Err(error)=owner.reconcile(){eprintln!("aios-stated: startup snapshot unavailable: {error:?}");}
    loop{
        if let Some(events)=watcher.as_mut(){
            match events.poll(){
                Ok(batch)=>{
                    if batch.notifications>0 || batch.loss{
                        if let Err(error)=owner.systemd_event(batch.notifications,batch.loss){eprintln!("aios-stated: event reconciliation unavailable: {error:?}");}
                    }
                    if batch.loss{watcher=None;owner.event_watcher=false;owner.event_error=Some("BOUNDED_DRAIN_LOSS".into());last_connection=std::time::Instant::now();}
                },
                Err(error)=>{
                    watcher=None;owner.event_watcher=false;owner.event_error=Some(format!("{error:?}"));last_connection=std::time::Instant::now();
                    if let Err(error)=owner.systemd_event(0,true){eprintln!("aios-stated: event loss reconciliation unavailable: {error:?}");}
                }
            }
        }else if last_connection.elapsed()>=Duration::from_secs(5){
            last_connection=std::time::Instant::now();
            match aios_system::services::events::SystemdEvents::connect(){
                Ok(events)=>{watcher=Some(events);owner.event_watcher=true;owner.event_error=None;
                    if let Err(error)=owner.systemd_event(0,true){eprintln!("aios-stated: reconnected snapshot unavailable: {error:?}");}},
                Err(error)=>owner.event_error=Some(format!("{error:?}"))
            }
        }
        // Fixed periodic fallback also covers a timer signal that was lost.
        if let Err(error)=owner.poll_generations(){eprintln!("aios-stated: native generation sampling unavailable: {error:?}");}
        let now=NativeTime::observe()?;
        if now.observation().boot!=owner.last_attempt.boot || now.observation().monotonic_ns.checked_sub(owner.last_attempt.monotonic_ns).is_none_or(|age|age>=PERIOD_NS){
            if let Err(error)=owner.reconcile(){eprintln!("aios-stated: periodic snapshot unavailable: {error:?}");}
        }
        let mut poll=libc::pollfd{fd:listener.as_raw_fd(),events:libc::POLLIN,revents:0};
        let result=unsafe{libc::poll(&mut poll,1,1000)};if result<0{if io::Error::last_os_error().kind()==io::ErrorKind::Interrupted{continue;}return Err(Error::Io);}
        if result>0{for _ in 0..4{match listener.accept(){Ok((stream,_))=>{if let Err(error)=serve(stream,uid,&mut owner){eprintln!("aios-stated: control request refused: {error:?}");}},Err(e) if e.kind()==io::ErrorKind::WouldBlock=>break,Err(_)=>return Err(Error::Io)}}}
    }
}
fn client(uid:u32,request:Request)->Result<()>{
    let caller=unsafe{libc::geteuid()};if caller!=0 && caller!=uid{return Err(Error::Identity);}
    directory(RUNTIME,uid)?;let mut stream=UnixStream::connect(SOCKET)?;
    let credentials=peer(&stream)?;if credentials.uid!=uid{return Err(Error::Identity);}
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    let value=serde_json::to_value(request).map_err(|_|Error::Protocol)?;frame_write(&mut stream,&value)?;
    let response:Value=serde_json::from_slice(&frame_read(&mut stream,MAX_REPLY)?).map_err(|_|Error::Protocol)?;
    println!("{response}");if response.get("ok")!=Some(&Value::Bool(true)){return Err(Error::Protocol);}Ok(())
}
pub fn entry()->Result<()>{
    let uid=identity()?;let args=std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice(){[]=>run(uid),[arg] if arg=="--inspect"=>client(uid,Request::Status{}),[arg] if arg=="--reconcile"=>client(uid,Request::Reconcile{}),
        [arg] if arg=="--generations"=>client(uid,Request::Generations{}),
        [arg,name] if arg=="--service"=>client(uid,Request::Service{unit_name:name.clone()}),_=>Err(Error::Protocol)}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn fixed_control_schema_refuses_unknown_fields_and_generic_sql(){
        for text in [r#"{"kind":"status","uid":0}"#,r#"{"kind":"reconcile","uid":0}"#,r#"{"kind":"status","kind":"reconcile"}"#,r#"{"kind":"service","unit_name":"sshd.service","unit_name":"other.service"}"#,r#"{"kind":"sql","sql":"DROP TABLE nodes"}"#,r#"{"kind":"service","unit_name":"sshd.service","path":"/etc/shadow"}"#]{assert!(serde_json::from_str::<Request>(text).is_err());}
        assert!(matches!(serde_json::from_str::<Request>(r#"{"kind":"status"}"#).unwrap(),Request::Status{}));
        assert!(matches!(serde_json::from_str::<Request>(r#"{"kind":"generations"}"#).unwrap(),Request::Generations{}));
        for text in [r#"{"kind":"generations","path":"/tmp/profile"}"#,r#"{"kind":"generations","kind":"status"}"#]{assert!(serde_json::from_str::<Request>(text).is_err());}
    }
    #[test]fn unchanged_generation_samples_coalesce_but_divergence_or_failure_refreshes(){
        let prior=SystemPointers{running_closure:"fixture-running".into(),selected_profile_closure:Some("fixture-running".into()),selected_profile_generation:Some(1),running_profile_divergence:Some(false),bootloader_entry:None,managed_transaction:None};
        assert!(!generation_refresh_needed(Some(&prior),&prior,false));
        assert!(generation_refresh_needed(None,&prior,false));assert!(generation_refresh_needed(Some(&prior),&prior,true));
        let mut changed=prior.clone();changed.selected_profile_generation=Some(2);assert!(generation_refresh_needed(Some(&prior),&changed,false));
        changed.selected_profile_closure=Some("fixture-other".into());changed.running_profile_divergence=Some(true);assert!(generation_refresh_needed(Some(&prior),&changed,false));
    }
    #[test]
    #[ignore="requires an enrolled NixOS guest; graph is an owned library fixture, not the installed service"]
    fn native_owner_reconciles_pointers_and_invalidates_injected_old_cache(){
        use std::os::unix::fs::PermissionsExt;
        let now=NativeTime::observe().unwrap().observation().clone();
        let root=std::env::temp_dir().join(format!("aios-owner-generation-{}-{}",std::process::id(),now.monotonic_ns));
        fs::create_dir(&root).unwrap();fs::set_permissions(&root,fs::Permissions::from_mode(0o700)).unwrap();
        let graph=GraphStore::open(&root,Scope::System).unwrap();
        let mut owner=Owner{graph,ids:vec![],attempts:0,successful:0,last_attempt:now.clone(),last_error:None,
            generations:None,generation_error:None,last_generation_probe:now,generation_changes:0,event_watcher:false,event_error:None,systemd_notifications:0,event_refreshes:0};
        owner.reconcile().unwrap();
        let observed=generations::observe().unwrap().1;
        let view=owner.generation_view().unwrap();assert_eq!(view["data"]["pointers"],serde_json::to_value(&observed).unwrap());
        assert_eq!(view["data"]["freshness"],"Current");assert_eq!(view["execution_authority"],false);
        let before=owner.attempts;
        std::thread::sleep(Duration::from_millis(1100));owner.poll_generations().unwrap();
        assert_eq!(owner.attempts,before);assert_eq!(owner.generation_changes,0);
        // Simulated old cache against actual unchanged native pointers. This
        // proves polling invalidation, not an actual root generation mutation.
        owner.generations.as_mut().unwrap().pointers.selected_profile_generation=Some(u64::MAX);
        assert_eq!(owner.generation_view().unwrap()["data"]["freshness"],"Stale");
        std::thread::sleep(Duration::from_millis(1100));owner.poll_generations().unwrap();
        assert_eq!(owner.generation_changes,1);assert_eq!(owner.attempts,before+1);
        assert_eq!(owner.generation_view().unwrap()["data"]["freshness"],"Current");
        assert_eq!(owner.generations.as_ref().unwrap().pointers,observed);
        let before=owner.attempts;
        // Fixture notifications exercise native rebuild and explicit loss handling,
        // not actual unit transitions or the installed event subscription.
        owner.systemd_event(3,false).unwrap();assert_eq!(owner.attempts,before+1);
        assert_eq!(owner.systemd_notifications,3);assert_eq!(owner.event_refreshes,1);
        owner.systemd_event(0,true).unwrap();assert_eq!(owner.attempts,before+2);
        assert_eq!(owner.event_refreshes,2);
        println!("AIOS_NATIVE_OWNER_GENERATIONS={}",json!({"native_pointers":observed,"unchanged_samples_coalesced":true,
            "injected_old_cache_invalidated":true,"native_reconciliation_restored":true,"system_profile_mutated":false,"installed_owner_verified":false,"fixture_event_batches_reconcile_native_snapshots":true}));
        drop(owner);fs::remove_dir_all(root).unwrap();
    }
    #[test]fn oversized_or_empty_frames_are_refused_before_allocation(){
        for n in [0u32,4097]{let (mut a,mut b)=UnixStream::pair().unwrap();a.write_all(&n.to_be_bytes()).unwrap();assert!(matches!(frame_read(&mut b,4096),Err(Error::Protocol)));}
    }
}
