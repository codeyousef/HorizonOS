//! Original-client native selection and task observation. Only authenticated
//! provider replies construct these types; no client/model receipt is accepted.
use crate::{identity::{self,Peer,GraphicalSession},ui_bridge,SharedState,Mode};
use aios_protocol::contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{Value,json};
use std::{collections::HashMap,sync::{Arc,Mutex,atomic::{AtomicU8,Ordering}},time::{Duration,Instant}};
type Result<T> = std::result::Result<T,ErrorCode>;

#[derive(Clone,Deserialize)]#[serde(deny_unknown_fields)]
struct Window { window_handle:String,name:String,title:String,identity_sha256:String }
pub(crate) struct Connection { pub(crate) client:ui_bridge::Client,peer:Peer,session:Option<GraphicalSession>,windows:HashMap<String,Window>,expires:Instant }
impl Connection {
    pub(crate) fn new(client:ui_bridge::Client,peer:Peer)->Self{
        Self{client,peer,session:None,windows:HashMap::new(),expires:Instant::now()}
    }
    pub(crate) fn discover(&mut self,session:&GraphicalSession)->Result<Value>{
        let value=self.client.call(json!({"kind":"discover","session_id":session.id}))?;
        #[derive(Deserialize)]#[serde(deny_unknown_fields)]
        struct Discovery {schema_version:u32,operation:String,session_id:String,windows:Vec<Window>,expires_after_ms:u64,confirmation_required:bool,ui_authorized:bool}
        let view:Discovery=serde_json::from_value(value.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
        if view.schema_version!=1 || view.operation!="ui_window_candidates" || view.session_id!=session.id || view.windows.len()>16
            || view.expires_after_ms!=30_000 || !view.confirmation_required || view.ui_authorized{return Err(ErrorCode::TargetChanged);}
        let mut windows=HashMap::new();
        for window in view.windows {
            if !crate::uuid(&window.window_handle) || window.name.len()>256 || window.title.len()>512 || window.identity_sha256.len()!=64
                || !window.identity_sha256.bytes().all(|b|b.is_ascii_hexdigit()) || windows.insert(window.window_handle.clone(),window).is_some(){return Err(ErrorCode::InvalidArgument);}
        }
        self.windows=windows;self.session=Some(session.clone());self.expires=Instant::now()+Duration::from_secs(30);Ok(value)
    }
    pub(crate) fn select(connection:Arc<Mutex<Self>>,peer:&Peer,session:&GraphicalSession,window_handle:&str)->Result<Selection>{
        let value=connection.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        if value.peer!=*peer{return Err(ErrorCode::PermissionDenied);}
        if value.expires<=Instant::now() || value.session.as_ref()!=Some(session){return Err(ErrorCode::TargetChanged);}
        let window=value.windows.get(window_handle).ok_or(ErrorCode::TargetNotFound)?.clone();drop(value);
        Ok(Selection{connection,peer:peer.clone(),session:session.clone(),window})
    }
}
#[derive(Clone)]
pub(crate) struct Selection { connection:Arc<Mutex<Connection>>,peer:Peer,session:GraphicalSession,window:Window }
// This receipt permits inference over the already consumed observation only.
// It is not a capability for another native query or any input operation.
pub(crate) struct Receipt {task_id:String,peer:Peer,request_digest:[u8;32],window_handle:String,window_sha256:String,revision:String,expires:u64}
impl Receipt {
    pub(crate) fn check(&self,selection:&Selection,peer:&Peer,id:&str,digest:&[u8;32])->Result<()>{
        if self.peer!=*peer || selection.peer!=*peer || self.task_id!=id || self.window_handle!=selection.window.window_handle
            || self.window_sha256!=selection.window.identity_sha256 || self.request_digest!=*digest{return Err(ErrorCode::PermissionDenied);}
        if self.revision!=aios_policy::registry_revision(){return Err(ErrorCode::PolicyChanged);}
        if aios_policy::boottime_ms()?>=self.expires{return Err(ErrorCode::ApprovalExpired);}Ok(())
    }
}
impl Selection {
    pub(crate) fn observe(&self,state:&SharedState,id:&str,goal:&str,mode:Mode,deadline:Instant,control:&AtomicU8)->Result<Value>{
        check(control,deadline)?;identity::verify_peer(&self.peer)?;
        if identity::observe_graphical_session(&self.session.id,self.peer.uid)?!=self.session{return Err(ErrorCode::TargetChanged);}
        let (cancellation,receiver)=ui_bridge::Cancellation::pair()?;
        let (request_digest,expires)={
            let mut state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            let task=state.tasks.get_mut(id).ok_or(ErrorCode::Cancelled)?;
            if task.owner!=self.peer || task.terminal(){return Err(ErrorCode::Cancelled);}
            check(control,deadline)?;task.native_cancel=Some(cancellation.clone());task.status.state="inspecting".into();task.event("inspecting");
            (task.digest,task.boottime_deadline)
        };
        let mode=match mode{Mode::Ask=>"ask",Mode::Diagnose=>"diagnose",_=>return Err(ErrorCode::PermissionDenied)};
        let result=(||{
            let started=self.connection.lock().map_err(|_|ErrorCode::ResourceExhausted)?.client.start_task(
                json!({"kind":"start_task_read","task_id":id,"window_handle":self.window.window_handle,"goal":goal,"mode":mode}),&receiver)?;
            status(&started,id)?;
            loop {
                check(control,deadline)?;identity::verify_peer(&self.peer)?;
                if aios_policy::boottime_ms()?>=expires{return Err(ErrorCode::DeadlineExceeded);}
                let value=self.connection.lock().map_err(|_|ErrorCode::ResourceExhausted)?.client.call(json!({"kind":"get_read_status","task_id":id}))?;
                let current=status(&value,id)?;
                match current.state.as_str(){
                    "queued"|"needs_permission"|"inspecting" if current.error.is_none()=>{
                        let mut state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
                        let task=state.tasks.get_mut(id).ok_or(ErrorCode::Cancelled)?;check(control,deadline)?;
                        if task.status.state!=current.state{task.status.state=current.state.clone();task.event(&current.state);}
                    },
                    "completed" if current.error.is_none() && current.snapshot_ready=>break,
                    "failed"|"cancelled"=>return Err(current.error.ok_or(ErrorCode::InvalidArgument)?),
                    _=>return Err(ErrorCode::InvalidArgument),
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            check(control,deadline)?;
            let snapshot=self.connection.lock().map_err(|_|ErrorCode::ResourceExhausted)?.client.call(json!({"kind":"take_snapshot","task_id":id}))?;
            aios_protocol::validation::validate(aios_protocol::contracts::schema_source("ui.snapshot","data").ok_or(ErrorCode::UnknownCapability)?,&snapshot).map_err(|_|ErrorCode::InvalidArgument)?;
            if snapshot["window_handle"]!=self.window.window_handle || !snapshot["snapshot_id"].as_str().is_some_and(crate::uuid){return Err(ErrorCode::TargetChanged);}
            check(control,deadline)?;identity::verify_peer(&self.peer)?;
            let receipt=Receipt{task_id:id.into(),peer:self.peer.clone(),request_digest,window_handle:self.window.window_handle.clone(),
                window_sha256:self.window.identity_sha256.clone(),revision:aios_policy::registry_revision(),expires};
            receipt.check(self,&self.peer,id,&request_digest)?;
            let evidence_id=snapshot["snapshot_id"].as_str().ok_or(ErrorCode::InvalidArgument)?.to_owned();
            let complete=snapshot["truncated"]==false;
            let observed_at=crate::now();
            let observation=serde_json::to_value(aios_protocol::contracts::ProviderResult {
                schema_version:1,status:if complete{aios_protocol::contracts::ResultStatus::Ok}else{aios_protocol::contracts::ResultStatus::Partial},observed_at,
                source:aios_protocol::contracts::Source{provider:"at-spi".into(),provider_version:env!("CARGO_PKG_VERSION").into()},
                evidence_ids:vec![evidence_id],complete,next_cursor:None,data:Some(snapshot),error:None,
            }).map_err(|_|ErrorCode::InvalidArgument)?;
            let mut state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            let task=state.tasks.get_mut(id).ok_or(ErrorCode::Cancelled)?;check(control,deadline)?;
            if task.terminal(){return Err(ErrorCode::Cancelled);}task.native_receipt=Some(receipt);task.event("scoped_observation_ready");
            Ok(observation)
        })();
        cancellation.cancel();
        // Forget is cleanup of the same exact private task, never a retry or
        // alternative read. Cancellation already invalidated its cached data.
        if let Ok(mut connection)=self.connection.try_lock(){let _=connection.client.call(json!({"kind":"forget","task_id":id}));}
        result
    }
}
#[derive(Deserialize)]#[serde(deny_unknown_fields)]
struct Status{schema_version:u32,task_id:String,state:String,error:Option<ErrorCode>,snapshot_ready:bool}
fn status(value:&Value,id:&str)->Result<Status>{
    let status:Status=serde_json::from_value(value.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
    if status.schema_version!=1 || status.task_id!=id{return Err(ErrorCode::TargetChanged);}Ok(status)
}
fn check(control:&AtomicU8,deadline:Instant)->Result<()>{
    match control.load(Ordering::Acquire){1=>Err(ErrorCode::Cancelled),2=>Err(ErrorCode::DeadlineExceeded),_ if Instant::now()>=deadline=>Err(ErrorCode::DeadlineExceeded),_=>Ok(())}
}
