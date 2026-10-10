//! Public session control. The bus, never request JSON, supplies the subject.
use crate::{Operation, Request, SharedState, identity::{self, Peer}, parse_operation};
use aios_protocol::{MAX_TASK_BYTES, contracts::ErrorCode};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap,fmt,path::PathBuf,sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}},time::{Duration,Instant}};
use zbus::{Connection, DBusError, Message, message::Header, names::ErrorName};

pub const NAME: &str = "org.aios.Session1";
pub const PATH: &str = "/org/aios/Session1";
pub const INTERFACE: &str = "org.aios.Agent1";

#[derive(Debug)]
pub struct BusError { code: ErrorCode, name: String }
impl From<ErrorCode> for BusError {
    fn from(code: ErrorCode) -> Self {
        let value = serde_json::to_value(code).expect("finite error enum serializes");
        Self { code, name: format!("org.aios.Error.{}", value.as_str().expect("error code is a string")) }
    }
}
impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:?}", self.code) }
}
impl std::error::Error for BusError {}
impl DBusError for BusError {
    fn name(&self) -> ErrorName<'_> { ErrorName::try_from(self.name.as_str()).expect("finite error enum yields a valid name") }
    fn description(&self) -> Option<&str> { Some("Request could not be fulfilled within authenticated scope") }
    fn create_reply(&self, header: &Header<'_>) -> zbus::Result<Message> {
        Message::error(header, self.name())?.build(&(self.description().unwrap(),))
    }
}
type Result<T> = std::result::Result<T, BusError>;

struct Admission(Arc<AtomicUsize>);
impl Drop for Admission { fn drop(&mut self) { self.0.fetch_sub(1, Ordering::AcqRel); } }

#[derive(Clone)]
pub struct Agent { state:SharedState,active:Arc<AtomicUsize>,control_active:Arc<AtomicUsize>,ui:Arc<Mutex<HashMap<(String,String),UiConnection>>>,processes:Arc<Mutex<HashMap<(String,String),ProcessConnection>>>,files:Arc<Mutex<aios_files::Manager>> }
struct UiConnection { peer:Peer,expires:Instant,client:Arc<crate::graphical::Connection> }
struct ProcessConnection { peer:Peer,expires:u64,client:crate::process_selection::Connection,control:Arc<crate::process_control::Connection> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnrollRootsRequest { schema_version:u32,proposal_id:String,approved_root_ids:Vec<String>,allowed_access:Vec<aios_files::Access>,confirmed:bool }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenScopedRequest { schema_version:u32,root_id:String,relative_path:String,access:aios_files::Access }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HandleRequest { schema_version:u32,file_handle:String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadScopedRequest { schema_version:u32,file_handle:String,max_bytes:usize }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevokeRootRequest { schema_version:u32,root_id:String,confirmed:bool }
fn file_owner(peer:&Peer)->std::result::Result<aios_files::Owner,ErrorCode>{
    let subject=peer.policy_subject()?;
    Ok(aios_files::Owner{uid:peer.uid,boot_id:peer.boot_id.clone(),session_id:peer.logind_session.clone(),
        client_binding_sha256:aios_policy::digest(&subject)?})
}
fn file_json(value:Value)->Result<String>{
    let text=serde_json::to_string(&value).map_err(|_|ErrorCode::InvalidArgument)?;
    if text.len()>aios_protocol::MAX_FRAME_BYTES{return Err(ErrorCode::ResourceExhausted.into());}Ok(text)
}
fn user_home(uid:u32)->std::result::Result<PathBuf,ErrorCode>{
    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid)).map_err(|_|ErrorCode::TargetChanged)?
        .map(|user|user.dir).ok_or(ErrorCode::PermissionDenied)
}
impl Agent {
    pub fn new(state: SharedState) -> Self { Self { state,active:Arc::new(AtomicUsize::new(0)),control_active:Arc::new(AtomicUsize::new(0)),ui:Arc::new(Mutex::new(HashMap::new())),processes:Arc::new(Mutex::new(HashMap::new())),files:Arc::new(Mutex::new(aios_files::Manager::default())) } }
    fn admit(&self) -> Result<Admission> {
        if self.active.fetch_add(1, Ordering::AcqRel) >= 16 {
            self.active.fetch_sub(1, Ordering::AcqRel);
            return Err(ErrorCode::ResourceExhausted.into());
        }
        Ok(Admission(self.active.clone()))
    }
    fn admit_control(&self)->Result<Admission>{
        if self.control_active.fetch_add(1,Ordering::AcqRel)>=4{self.control_active.fetch_sub(1,Ordering::AcqRel);return Err(ErrorCode::ResourceExhausted.into());}
        Ok(Admission(self.control_active.clone()))
    }
    async fn peer(connection: &Connection, header: &Header<'_>) -> Result<Peer> {
        let sender = header.sender().ok_or(ErrorCode::PermissionDenied)?.to_string();
        if !sender.starts_with(':') { return Err(ErrorCode::PermissionDenied.into()); }
        let bus = zbus::Proxy::new(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus")
            .await.map_err(|_| ErrorCode::PermissionDenied)?;
        let credentials: zbus::fdo::ConnectionCredentials = bus.call("GetConnectionCredentials", &(sender.as_str(),))
            .await.map_err(|_| ErrorCode::PermissionDenied)?;
        let uid = credentials.unix_user_id().ok_or(ErrorCode::PermissionDenied)?;
        let pid = credentials.process_id().ok_or(ErrorCode::PermissionDenied)?;
        let bus_id: String = bus.call("GetId", &()).await.map_err(|_| ErrorCode::PermissionDenied)?;
        let mut peer = blocking::unblock(move || identity::authenticate_process(uid, pid)).await?;
        peer.bus_sender = Some(sender); peer.bus_id = Some(bus_id);
        Ok(peer)
    }
    async fn dispatch(&self, connection: &Connection, header: Header<'_>, operation: Operation) -> Result<Value> {
        let _admission = if matches!(operation,Operation::CancelProcessTermination{..}|Operation::ForgetProcessTermination{..}){self.admit_control()?}else{self.admit()?};
        let peer = Self::peer(connection, &header).await?;
        let outcome=if let Some(action)=crate::process_bridge::action(&operation)?{
            let agent=self.clone();let original=peer.clone();
            blocking::unblock(move||agent.process_read(&original,&action)).await
        }else{match operation {
            operation if crate::process_control::is_operation(&operation)=>{
                let agent=self.clone();let original=peer.clone();blocking::unblock(move||agent.process_control(&original,operation)).await
            },
            Operation::Submit{request} if request.selected_session_handle.is_none() && request.selected_app_handle.is_none() && !request.context_handles.is_empty()=>{
                let agent=self.clone();let original=peer.clone();blocking::unblock(move||agent.submit_process(&original,request)).await
            },
            Operation::ListUiWindows{session_handle}=>{
            let agent=self.clone();let original=peer.clone();
            blocking::unblock(move||agent.list_windows(&original,&session_handle)).await
            },
            Operation::Submit{request} if request.selected_session_handle.is_some() || request.selected_app_handle.is_some()=>{
                let agent=self.clone();let original=peer.clone();blocking::unblock(move||agent.submit_graphical(&original,request)).await
            },
            operation=>self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.dispatch(&peer,operation),
        }};
        if Self::peer(connection, &header).await? != peer { return Err(ErrorCode::TargetChanged.into()); }
        outcome.map_err(Into::into)
    }
    fn process_read(&self,peer:&Peer,action:&aios_protocol::contracts::Action)->std::result::Result<Value,ErrorCode>{
        identity::verify_peer(peer)?;
        let now=aios_policy::boottime_ms()?;
        let key=(peer.bus_id.clone().ok_or(ErrorCode::PermissionDenied)?,peer.bus_sender.clone().ok_or(ErrorCode::PermissionDenied)?);
        let cached={let mut clients=self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            clients.retain(|_,client|client.expires>now);
            if let Some(client)=clients.get_mut(&key){
                if client.peer!=*peer{return Err(ErrorCode::TargetChanged);}
                client.expires=now.checked_add(95_000).ok_or(ErrorCode::ResourceExhausted)?;Some(client.client.clone())
            }else{if clients.len()>=4{return Err(ErrorCode::ResourceExhausted);}None}};
        let client=if let Some(client)=cached{client}else{
            // Managed handshakes can perform native I/O. Keep them outside the
            // connection table so another caller can still latch Stop.
            let client=Arc::new(Mutex::new(crate::process_bridge::Client::connect(None,peer)?));
            let control=Arc::new(crate::process_control::Connection::new(peer.clone(),client.clone()));
            let mut clients=self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            if let Some(existing)=clients.get(&key){if existing.peer!=*peer{return Err(ErrorCode::TargetChanged);}existing.client.clone()}
            else{
                if clients.len()>=4{return Err(ErrorCode::ResourceExhausted);}
                clients.insert(key.clone(),ProcessConnection{peer:peer.clone(),expires:now.checked_add(95_000).ok_or(ErrorCode::ResourceExhausted)?,client:client.clone(),control});client
            }
        };
        let result=client.lock().map_err(|_|ErrorCode::ResourceExhausted)?.call(action);
        identity::verify_peer(peer)?;
        // Domain permission errors (e.g. another caller's process handle) do
        // not invalidate this caller's other retained handles or cursor.
        if matches!(result,Err(ErrorCode::TargetChanged)){self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?.remove(&key);}
        result
    }
    fn process_control(&self,peer:&Peer,operation:Operation)->std::result::Result<Value,ErrorCode>{
        let now=aios_policy::boottime_ms()?;
        let key=(peer.bus_id.clone().ok_or(ErrorCode::PermissionDenied)?,peer.bus_sender.clone().ok_or(ErrorCode::PermissionDenied)?);
        let control={let mut clients=self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
            clients.retain(|_,client|client.expires>now);
            let client=clients.get_mut(&key).ok_or(ErrorCode::TargetNotFound)?;
            if client.peer!=*peer{return Err(ErrorCode::TargetChanged);}
            client.expires=now.checked_add(95_000).ok_or(ErrorCode::ResourceExhausted)?;client.control.clone()};
        control.execute(&self.state,peer,operation)
    }
    fn submit_process(&self,peer:&Peer,request:crate::Submit)->std::result::Result<Value,ErrorCode>{
        if let Some(value)=self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.existing_submission(peer,&request)?{return Ok(value);}
        let ids=crate::process_context_ids(&self.state,peer,&request)?;
        let selected=if ids.is_empty(){None}else{
            let first=aios_protocol::contracts::parse_tool_call(serde_json::json!({"kind":"tool_call","action_id":"process.inspect","arguments":{"process_id":ids[0]}}).to_string().as_bytes())?;
            self.process_read(peer,&first)?;
            let key=(peer.bus_id.clone().ok_or(ErrorCode::PermissionDenied)?,peer.bus_sender.clone().ok_or(ErrorCode::PermissionDenied)?);
            let connection=self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?.get(&key).ok_or(ErrorCode::TargetChanged)?.client.clone();
            Some(Arc::new(crate::process_selection::Selection::select(peer,connection,&ids)?))
        };
        identity::verify_peer(peer)?;
        self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.submit_scoped(peer,request,None,selected)
    }
    fn list_windows(&self,peer:&Peer,handle:&str)->std::result::Result<Value,ErrorCode>{
        let session=crate::selected_ui_session(&self.state,peer,handle)?;
        identity::verify_peer(peer)?;
        let key=(peer.bus_id.clone().ok_or(ErrorCode::PermissionDenied)?,peer.bus_sender.clone().ok_or(ErrorCode::PermissionDenied)?);
        // Do not hold the shared task lock during provider/native bus queries.
        // A contending metadata request receives a bounded error, not a queue
        // behind desktop work.
        let mut contexts=self.ui.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        contexts.retain(|_,value|value.expires>Instant::now());
        if let Some(value)=contexts.get(&key){if value.peer!=*peer{contexts.remove(&key);return Err(ErrorCode::TargetChanged);}}
        let client=if let Some(value)=contexts.get_mut(&key){
            value.expires=Instant::now()+Duration::from_secs(30);value.client.clone()
        } else {
            if contexts.len()>=8{return Err(ErrorCode::ResourceExhausted);}
            let client=Arc::new(crate::graphical::Connection::new(crate::ui_bridge::Client::connect_bus(peer)?,peer.clone()));
            contexts.insert(key.clone(),UiConnection{peer:peer.clone(),expires:Instant::now()+Duration::from_secs(30),client:client.clone()});client
        };
        drop(contexts);
        let result=client.discover(&session);
        identity::verify_peer(peer)?;
        if matches!(result,Err(ErrorCode::TargetChanged|ErrorCode::PermissionDenied)){
            self.ui.lock().map_err(|_|ErrorCode::ResourceExhausted)?.remove(&key);
        }
        result
    }
    fn submit_graphical(&self,peer:&Peer,request:crate::Submit)->std::result::Result<Value,ErrorCode>{
        if let Some(value)=self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.existing_submission(peer,&request)?{return Ok(value);}
        if !request.context_handles.is_empty() || request.retain_for_history || !request.history_handles.is_empty() || request.text.len()>4096{return Err(ErrorCode::InvalidArgument);}
        let session=crate::selected_ui_session(&self.state,peer,request.selected_session_handle.as_deref().ok_or(ErrorCode::AuthRequired)?)?;
        let key=(peer.bus_id.clone().ok_or(ErrorCode::PermissionDenied)?,peer.bus_sender.clone().ok_or(ErrorCode::PermissionDenied)?);
        let connection=self.ui.try_lock().map_err(|_|ErrorCode::ResourceExhausted)?.get(&key).ok_or(ErrorCode::AuthRequired)?.client.clone();
        let selection=crate::graphical::Connection::select(connection,peer,&session,request.selected_app_handle.as_deref().ok_or(ErrorCode::AuthRequired)?)?;
        identity::verify_peer(peer)?;
        self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.submit_owned(peer,request,Some(selection))
    }
    async fn json(&self, connection: &Connection, header: Header<'_>, operation: Operation) -> Result<String> {
        let result = self.dispatch(connection, header, operation).await?;
        let json = serde_json::to_string(&result).map_err(|_| ErrorCode::InvalidArgument)?;
        if json.len() > aios_protocol::MAX_FRAME_BYTES { return Err(ErrorCode::ResourceExhausted.into()); }
        Ok(json)
    }
    async fn capabilities_for(&self, connection: &Connection, header: Header<'_>, interface: &str, actions: &[&str]) -> Result<String> {
        // Authenticate through the same control path before probing providers.
        self.dispatch(connection, header, Operation::GetCapabilities).await?;
        let ids=actions.iter().map(|id|(*id).to_owned()).collect::<Vec<_>>();
        let available=blocking::unblock(move||crate::native_settings::available_actions(&ids)).await;
        let contracts = actions.iter().map(|id| aios_protocol::registry::capability(id)
            .map(|contract| serde_json::json!({"action_id":id,"input_schema":contract.input_schema,
                "output_schema":contract.output_schema,"availability":if available.iter().any(|value|value.as_str()==*id){"available"}else{"unavailable"}})))
            .collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(serde_json::json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
            "operation":"capabilities","interface":interface,"available_actions":available,"contracts":contracts}).to_string())
    }
    async fn action(&self, connection: &Connection, header: Header<'_>, request_json: &str, expected: &str) -> Result<String> {
        let _admission = self.admit()?;
        let peer = Self::peer(connection, &header).await?;
        let prepared = (|| -> std::result::Result<(_,aios_protocol::contracts::Action), ErrorCode> {
            if request_json.len() > MAX_TASK_BYTES { return Err(ErrorCode::ResourceExhausted); }
            let request: Request = serde_json::from_str(request_json).map_err(|_| ErrorCode::InvalidArgument)?;
            if request.schema_version != 1 { return Err(ErrorCode::UnsupportedSchema); }
            if !crate::uuid(&request.request_id) { return Err(ErrorCode::InvalidArgument); }
            let Operation::Invoke { tool_call } = parse_operation(request.operation.get())? else { return Err(ErrorCode::InvalidArgument); };
            let action = aios_protocol::contracts::parse_tool_call(tool_call.get().as_bytes())?;
            if action.action_id() != expected { return Err(ErrorCode::InvalidArgument); }
            Ok((tool_call,action))
        })();
        let outcome=match prepared{
            Ok((_tool_call,action)) if matches!(action.action_id(),"process.list"|"process.inspect")=>{
                let agent=self.clone();let original=peer.clone();blocking::unblock(move||agent.process_read(&original,&action)).await
            },
            Ok((tool_call,_))=>self.state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.dispatch(&peer,Operation::Invoke{tool_call}),
            Err(error)=>Err(error),
        };
        if Self::peer(connection, &header).await? != peer { return Err(ErrorCode::TargetChanged.into()); }
        let value = outcome?;
        let json = serde_json::to_string(&value).map_err(|_| ErrorCode::InvalidArgument)?;
        if json.len() > aios_protocol::MAX_FRAME_BYTES { return Err(ErrorCode::ResourceExhausted.into()); }
        Ok(json)
    }
}

#[zbus::interface(name = "org.aios.Agent1")]
impl Agent {
    async fn start_process_termination(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:Request=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1{return Err(ErrorCode::UnsupportedSchema.into());}
        if !crate::uuid(&request.request_id){return Err(ErrorCode::InvalidArgument.into());}
        let operation=parse_operation(request.operation.get())?;
        if !matches!(operation,Operation::StartProcessTermination{..}){return Err(ErrorCode::InvalidArgument.into());}
        self.json(connection,header,operation).await
    }
    async fn get_process_termination(&self,task_id:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        if task_id.len()>128 || !crate::uuid(task_id){return Err(ErrorCode::InvalidArgument.into());}
        self.json(connection,header,Operation::GetProcessTermination{task_id:task_id.into()}).await
    }
    async fn cancel_process_termination(&self,task_id:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        if task_id.len()>128 || !crate::uuid(task_id){return Err(ErrorCode::InvalidArgument.into());}
        self.json(connection,header,Operation::CancelProcessTermination{task_id:task_id.into()}).await
    }
    async fn forget_process_termination(&self,task_id:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        if task_id.len()>128 || !crate::uuid(task_id){return Err(ErrorCode::InvalidArgument.into());}
        self.json(connection,header,Operation::ForgetProcessTermination{task_id:task_id.into()}).await
    }
    async fn execute_task_action(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:Request=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!crate::uuid(&request.request_id){return Err(ErrorCode::InvalidArgument.into());}
        let operation=parse_operation(request.operation.get())?;
        if !matches!(&operation,Operation::ExecuteTaskAction{task_id,..} if task_id==&request.request_id){return Err(ErrorCode::InvalidArgument.into());}
        self.json(connection,header,operation).await
    }
    async fn list_processes(&self, request_json: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.action(connection, header, request_json, "process.list").await
    }
    async fn inspect_process(&self, request_json: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.action(connection, header, request_json, "process.inspect").await
    }
    async fn get_capabilities(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        let value = self.dispatch(connection, header, Operation::GetCapabilities).await?;
        let mut value = value;
        value["transport"] = Value::String("session-dbus".into());
        serde_json::to_string(&value).map_err(|_| ErrorCode::InvalidArgument.into())
    }
    async fn privacy_scopes(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        let mut value=self.dispatch(connection,header.clone(),Operation::PrivacyScopes).await?;
        let peer=Self::peer(connection,&header).await?;let original=peer.clone();let agent=self.clone();
        let roots=blocking::unblock(move||{identity::verify_peer(&original)?;let now=aios_policy::boottime_ms()?;
            Ok::<_,ErrorCode>(agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.active_roots(&file_owner(&original)?,now))}).await?;
        if Self::peer(connection,&header).await?!=peer{return Err(ErrorCode::TargetChanged.into());}
        value["data"]["file_roots"]=serde_json::to_value(roots).map_err(|_|ErrorCode::InvalidArgument)?;
        file_json(value)
    }
    async fn list_history(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.json(connection,header,Operation::ListHistory).await
    }
    async fn list_automations(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.json(connection,header,Operation::ListAutomations).await
    }
    async fn model_status(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.dispatch(connection, header, Operation::GetCapabilities).await?;
        let value = blocking::unblock(|| crate::inference::Endpoint::installed().status()).await?;
        serde_json::to_string(&serde_json::json!({"schema_version":1,"operation":"model_status","data":value,"mutation_performed":false}))
            .map_err(|_| ErrorCode::InvalidArgument.into())
    }
    async fn unload_model(&self, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        self.dispatch(connection, header, Operation::GetCapabilities).await?;
        let value = blocking::unblock(|| crate::inference::Endpoint::installed().unload()).await?;
        let mutation_performed=value["unload_requested"]==true;
        serde_json::to_string(&serde_json::json!({"schema_version":1,"operation":"model_unload","data":value,"mutation_performed":mutation_performed}))
            .map_err(|_| ErrorCode::InvalidArgument.into())
    }
    async fn submit(&self, request_json: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if request_json.len() > MAX_TASK_BYTES { return Err(ErrorCode::ResourceExhausted.into()); }
        let request: Request = serde_json::from_str(request_json).map_err(|_| ErrorCode::InvalidArgument)?;
        if request.schema_version != 1 { return Err(ErrorCode::UnsupportedSchema.into()); }
        if !crate::uuid(&request.request_id) { return Err(ErrorCode::InvalidArgument.into()); }
        let operation = parse_operation(request.operation.get())?;
        if !matches!(operation, Operation::Submit { .. }) { return Err(ErrorCode::InvalidArgument.into()); }
        let value = self.dispatch(connection, header, operation).await?;
        value["request_id"].as_str().map(str::to_owned).ok_or_else(|| ErrorCode::InvalidArgument.into())
    }
    async fn get_status(&self, request_id: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if request_id.len() > 128 || !crate::uuid(request_id) { return Err(ErrorCode::InvalidArgument.into()); }
        self.json(connection, header, Operation::GetStatus { task_id: request_id.into() }).await
    }
    async fn get_events(&self, request_id: &str, after_sequence: u64, limit: u32, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if request_id.len() > 128 || !crate::uuid(request_id) { return Err(ErrorCode::InvalidArgument.into()); }
        self.json(connection, header, Operation::GetEvents { task_id: request_id.into(), after_sequence, limit }).await
    }
    async fn cancel(&self, request_id: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if request_id.len() > 128 || !crate::uuid(request_id) { return Err(ErrorCode::InvalidArgument.into()); }
        self.json(connection, header, Operation::Cancel { task_id: request_id.into() }).await
    }
    async fn forget(&self, request_id: &str, #[zbus(connection)] connection: &Connection, #[zbus(header)] header: Header<'_>) -> Result<String> {
        if request_id.len() > 128 || !crate::uuid(request_id) { return Err(ErrorCode::InvalidArgument.into()); }
        self.json(connection, header, Operation::Forget { task_id: request_id.into() }).await
    }
}

pub struct Ui { agent: Agent }
#[zbus::interface(name = "org.aios.UI1")]
impl Ui {
    async fn get_capabilities(&self, #[zbus(connection)] connection:&Connection, #[zbus(header)] header:Header<'_>) -> Result<String> {
        self.agent.capabilities_for(connection,header,"org.aios.UI1",&["ui.snapshot","ui.find","ui.activate","ui.set_value","ui.select_text","ui.visual_step"]).await
    }
    async fn select_session(&self, session_id:&str, #[zbus(connection)] connection:&Connection, #[zbus(header)] header:Header<'_>) -> Result<String> {
        self.agent.json(connection,header,Operation::SelectUiSession {session_id:session_id.into()}).await
    }
    async fn list_windows(&self, session_handle:&str, #[zbus(connection)] connection:&Connection, #[zbus(header)] header:Header<'_>) -> Result<String> {
        if !crate::uuid(session_handle){return Err(ErrorCode::InvalidArgument.into());}
        self.agent.json(connection,header,Operation::ListUiWindows{session_handle:session_handle.into()}).await
    }
}

// Each method fixes its action in server code. No method accepts a bus name,
// object path, privileged command or caller-selected dispatch namespace.
macro_rules! surface {
    ($name:ident, $interface:literal, [$(($method:ident,$action:literal)),+ $(,)?]) => {
        pub struct $name { agent: Agent }
        #[zbus::interface(name = $interface)]
        impl $name {
            async fn get_capabilities(&self, #[zbus(connection)] connection:&Connection, #[zbus(header)] header:Header<'_>) -> Result<String> {
                self.agent.capabilities_for(connection,header,$interface,&[$($action),+]).await
            }
            $(async fn $method(&self, request_json:&str, #[zbus(connection)] connection:&Connection, #[zbus(header)] header:Header<'_>) -> Result<String> {
                self.agent.action(connection,header,request_json,$action).await
            })+
        }
    }
}
// The method dispatch authenticates the unique bus sender and its live kernel
// process before handing bounded work to the blocking pool. The file manager
// then rechecks the owner, grant and descriptor identity for each operation.
// Do not resample the sender after an accepted request: disconnect can race the
// reply, but cannot turn the already authenticated request into another caller.
pub struct Files{agent:Agent}
#[zbus::interface(name="org.aios.Files1")]
impl Files{
    async fn get_capabilities(&self,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        self.agent.capabilities_for(connection,header,"org.aios.Files1",&["files.search","files.metadata","files.read","files.summarize","files.copy","files.move","files.trash","files.restore"]).await
    }
    async fn propose_roots(&self,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let proposal=blocking::unblock(move||{let home=user_home(original.uid)?;let now=aios_policy::boottime_ms()?;
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.propose_xdg_roots(file_owner(&original)?,&home,now)}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_roots_proposed","data":proposal,"mutation_performed":false}))
    }
    async fn enroll_roots(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:EnrollRootsRequest=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!request.confirmed||!crate::uuid(&request.proposal_id){return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let roots=blocking::unblock(move||{let now=aios_policy::boottime_ms()?;
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.enroll(&file_owner(&original)?,&request.proposal_id,&request.approved_root_ids,&request.allowed_access,now)}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_roots_enrolled","data":{"roots":roots},"mutation_performed":true}))
    }
    async fn list_roots(&self,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let roots=blocking::unblock(move||{let now=aios_policy::boottime_ms()?;
            Ok::<_,ErrorCode>(agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.active_roots(&file_owner(&original)?,now))}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_roots","data":{"roots":roots},"mutation_performed":false}))
    }
    async fn open_scoped(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:OpenScopedRequest=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!crate::uuid(&request.root_id){return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let metadata=blocking::unblock(move||{let now=aios_policy::boottime_ms()?;
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.issue_handle(&file_owner(&original)?,&request.root_id,PathBuf::from(request.relative_path).as_path(),request.access,now)}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_handle_issued","data":metadata,"mutation_performed":false}))
    }
    async fn scoped_metadata(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:HandleRequest=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!crate::uuid(&request.file_handle){return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let metadata=blocking::unblock(move||{let now=aios_policy::boottime_ms()?;
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.metadata(&file_owner(&original)?,&request.file_handle,now)}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_metadata","data":metadata,"mutation_performed":false}))
    }
    async fn read_scoped(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:ReadScopedRequest=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!crate::uuid(&request.file_handle)||request.max_bytes>262_144{return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();let handle=request.file_handle.clone();
        let bytes=blocking::unblock(move||{let now=aios_policy::boottime_ms()?;
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.read(&file_owner(&original)?,&request.file_handle,request.max_bytes,now)}).await?;
        let content=String::from_utf8(bytes).map_err(|_|ErrorCode::UnsupportedCapability)?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_read","data":{"file_handle":handle,"bytes_read":content.len(),"content":content},"mutation_performed":false}))
    }
    async fn revoke_root(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:RevokeRootRequest=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!request.confirmed||!crate::uuid(&request.root_id){return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;let original=peer.clone();let agent=self.agent.clone();
        let receipt=blocking::unblock(move||{
            agent.files.lock().map_err(|_|ErrorCode::ResourceExhausted)?.revoke(&file_owner(&original)?,&request.root_id)}).await?;
        file_json(serde_json::json!({"schema_version":1,"operation":"file_root_revoked","data":receipt,"mutation_performed":true}))
    }
    async fn search(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.search").await}
    async fn metadata(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.metadata").await}
    async fn read(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.read").await}
    async fn summarize(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.summarize").await}
    async fn copy(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.copy").await}
    async fn move_file(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.move").await}
    async fn trash(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.trash").await}
    async fn restore(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{self.agent.action(connection,header,request_json,"files.restore").await}
}
surface!(Applications,"org.aios.Applications1",[(list,"apps.list"),(launch,"apps.launch"),(actions,"apps.actions"),(invoke,"apps.invoke")]);
surface!(Settings,"org.aios.Settings1",[(get,"settings.get"),(set,"settings.set")]);
surface!(Audio,"org.aios.Audio1",[(outputs,"audio.outputs"),(inputs,"audio.inputs"),(default_get,"audio.default_get"),
    (default_set,"audio.default_set"),(mute_set,"audio.mute_set")]);
pub struct Power {agent:Agent}
#[zbus::interface(name = "org.aios.Power1")]
impl Power {
    async fn get_capabilities(&self,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        self.agent.capabilities_for(connection,header,"org.aios.Power1",&["power.status","power.profile_set"]).await
    }
    async fn status(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        self.agent.action(connection,header,request_json,"power.status").await
    }
    async fn profile_set(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        self.agent.action(connection,header,request_json,"power.profile_set").await
    }
    async fn confirm_profile_set(&self,request_json:&str,#[zbus(connection)] connection:&Connection,#[zbus(header)] header:Header<'_>)->Result<String>{
        let _admission=self.agent.admit()?;
        if request_json.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted.into());}
        let request:Request=serde_json::from_str(request_json).map_err(|_|ErrorCode::InvalidArgument)?;
        if request.schema_version!=1||!crate::uuid(&request.request_id){return Err(ErrorCode::InvalidArgument.into());}
        let operation=parse_operation(request.operation.get())?;
        let crate::Operation::ConfirmPowerProfile{task_id,session_handle,goal,mode,tool_call}=operation else{return Err(ErrorCode::InvalidArgument.into());};
        if request.request_id!=task_id||mode!=crate::Mode::Act||!crate::uuid(&task_id)||!crate::uuid(&session_handle){return Err(ErrorCode::InvalidArgument.into());}
        let action=aios_protocol::contracts::parse_tool_call(tool_call.get().as_bytes())?;
        if action.action_id()!="power.profile_set"{return Err(ErrorCode::InvalidArgument.into());}
        let peer=Agent::peer(connection,&header).await?;
        let session=crate::selected_ui_session(&self.agent.state,&peer,&session_handle)?;
        let original=peer.clone();let value=blocking::unblock(move||crate::power_profile_change::execute(original,session,task_id,goal,action)).await?;
        if Agent::peer(connection,&header).await?!=peer{return Err(ErrorCode::TargetChanged.into());}
        let result=serde_json::to_string(&value).map_err(|_|ErrorCode::InvalidArgument)?;
        if result.len()>aios_protocol::MAX_FRAME_BYTES{return Err(ErrorCode::ResourceExhausted.into());}Ok(result)
    }
}

pub fn export(address: &str, state: SharedState) -> zbus::Result<zbus::blocking::Connection> {
    let agent = Agent::new(state);
    zbus::blocking::connection::Builder::address(address)?.method_timeout(Duration::from_secs(2))
        .max_queued(64).allow_name_replacements(false).replace_existing_names(false)
        .serve_at("/org/aios/Files1",Files{agent:agent.clone()})?
        .serve_at("/org/aios/Applications1",Applications{agent:agent.clone()})?
        .serve_at("/org/aios/Settings1",Settings{agent:agent.clone()})?
        .serve_at("/org/aios/Audio1",Audio{agent:agent.clone()})?
        .serve_at("/org/aios/Power1",Power{agent:agent.clone()})?
        .serve_at("/org/aios/UI1",Ui{agent:agent.clone()})?
        .serve_at(PATH, agent)?.name(NAME)?.build()
}

pub fn export_user_bus(state: SharedState) -> zbus::Result<zbus::blocking::Connection> {
    // Ignore a caller-controlled DBUS_SESSION_BUS_ADDRESS. Default deployment
    // always selects this user's standard systemd runtime bus.
    let uid = nix::unistd::geteuid().as_raw();
    export(&format!("unix:path=/run/user/{uid}/bus"), state)
}

/// Headless client with a fixed bus endpoint and pinned live daemon owner.
pub struct Client { connection: zbus::blocking::Connection, owner: String, peer: Peer }
impl Client {
    pub fn list_processes(&self,cursor:Option<&str>)->std::result::Result<Value,ErrorCode>{
        let arguments=if let Some(cursor)=cursor{serde_json::json!({"limit":100,"cursor":cursor})}else{serde_json::json!({"limit":100})};
        self.process_read("ListProcesses","process.list",arguments)
    }
    pub fn inspect_process(&self,id:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        self.process_read("InspectProcess","process.inspect",serde_json::json!({"process_id":id}))
    }
    fn process_read(&self,method:&str,action:&str,arguments:Value)->std::result::Result<Value,ErrorCode>{
        let raw=serde_json::json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":{"kind":"invoke","tool_call":{"kind":"tool_call","action_id":action,"arguments":arguments}}}).to_string();
        let result=self.call(method,&(raw.as_str(),))?;
        aios_protocol::validation::validate_result(action,result.as_bytes())
    }
    pub fn start_process_termination(&self,id:&str,process:&str,session:&str,goal:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(id) || !crate::uuid(process) || !crate::uuid(session){return Err(ErrorCode::InvalidArgument);}
        let raw=serde_json::json!({"schema_version":1,"request_id":id,"operation":{"kind":"start_process_termination","task_id":id,"process_id":process,"session_handle":session,"goal":goal,"mode":"act"}}).to_string();
        if raw.len()>MAX_TASK_BYTES{return Err(ErrorCode::ResourceExhausted);}
        let value=serde_json::from_str(&self.call("StartProcessTermination",&(raw.as_str(),))?).map_err(|_|ErrorCode::InvalidArgument)?;
        crate::process_control::validate_status(value,id)
    }
    pub fn process_termination_status(&self,id:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        let value=serde_json::from_str(&self.call("GetProcessTermination",&(id,))?).map_err(|_|ErrorCode::InvalidArgument)?;
        crate::process_control::validate_status(value,id)
    }
    pub fn cancel_process_termination(&self,id:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        let value:Value=serde_json::from_str(&self.call("CancelProcessTermination",&(id,))?).map_err(|_|ErrorCode::InvalidArgument)?;
        #[derive(serde::Deserialize)]#[serde(deny_unknown_fields)]struct Cancel{schema_version:u32,task_id:String,cancel_requested:bool}
        let receipt:Cancel=serde_json::from_value(value.clone()).map_err(|_|ErrorCode::InvalidArgument)?;
        if receipt.schema_version!=1 || receipt.task_id!=id || !receipt.cancel_requested{return Err(ErrorCode::TargetChanged);}Ok(value)
    }
    pub fn forget_process_termination(&self,id:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(id){return Err(ErrorCode::InvalidArgument);}
        let value=serde_json::from_str(&self.call("ForgetProcessTermination",&(id,))?).map_err(|_|ErrorCode::InvalidArgument)?;
        crate::process_control::validate_deleted(value,id)
    }
    fn subject(connection: &zbus::blocking::Connection) -> std::result::Result<(String, Peer), ErrorCode> {
        let bus = zbus::blocking::Proxy::new(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus").map_err(|_| ErrorCode::AuthRequired)?;
        let owner: String = bus.call("GetNameOwner", &(NAME,)).map_err(|_| ErrorCode::UnsupportedCapability)?;
        let credentials: zbus::fdo::ConnectionCredentials = bus.call("GetConnectionCredentials", &(owner.as_str(),)).map_err(|_| ErrorCode::PermissionDenied)?;
        let uid = credentials.unix_user_id().ok_or(ErrorCode::PermissionDenied)?;
        let pid = credentials.process_id().ok_or(ErrorCode::PermissionDenied)?;
        let mut peer = identity::authenticate_process(uid, pid)?;
        peer.bus_sender = Some(owner.clone());
        peer.bus_id = Some(bus.call("GetId", &()).map_err(|_| ErrorCode::PermissionDenied)?);
        Ok((owner, peer))
    }
    pub fn connect_user_bus() -> std::result::Result<Self, ErrorCode> {
        let address = format!("unix:path=/run/user/{}/bus", nix::unistd::geteuid());
        let connection = zbus::blocking::connection::Builder::address(address.as_str()).map_err(|_| ErrorCode::UnsupportedCapability)?
            .method_timeout(Duration::from_secs(5)).build().map_err(|_| ErrorCode::UnsupportedCapability)?;
        let (owner, peer) = Self::subject(&connection)?;
        Ok(Self { connection, owner, peer })
    }
    fn verify(&self) -> std::result::Result<(), ErrorCode> {
        let (owner, peer) = Self::subject(&self.connection)?;
        if owner != self.owner || peer != self.peer { return Err(ErrorCode::TargetChanged); }
        Ok(())
    }
    fn call<B: serde::Serialize + zbus::zvariant::DynamicType>(&self, method: &str, body: &B) -> std::result::Result<String, ErrorCode> {
        self.call_at(PATH, INTERFACE, method, body)
    }
    fn call_at<B: serde::Serialize + zbus::zvariant::DynamicType>(&self, path: &str, interface: &str, method: &str, body: &B) -> std::result::Result<String, ErrorCode> {
        self.verify()?;
        let api = zbus::blocking::Proxy::new(&self.connection, self.owner.as_str(), path, interface).map_err(|_| ErrorCode::UnsupportedCapability)?;
        let result: String = api.call(method, body).map_err(client_error)?;
        self.verify()?;
        if result.len() > aios_protocol::MAX_FRAME_BYTES { return Err(ErrorCode::ResourceExhausted); }
        Ok(result)
    }
    pub fn select_ui_session(&self, session_id: &str) -> std::result::Result<Value, ErrorCode> {
        let value: Value = serde_json::from_str(&self.call_at("/org/aios/UI1", "org.aios.UI1", "SelectSession", &(session_id,))?).map_err(|_| ErrorCode::InvalidArgument)?;
        if value["schema_version"] != 1 || value["operation"] != "ui_session_candidate"
            || value["session"]["id"] != session_id || value["confirmation_required"] != true
            || value["ui_authorized"] != false || !value["candidate_handle"].as_str().is_some_and(crate::uuid) {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn list_ui_windows(&self,session_handle:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(session_handle){return Err(ErrorCode::InvalidArgument);}
        let value:Value=serde_json::from_str(&self.call_at("/org/aios/UI1","org.aios.UI1","ListWindows",&(session_handle,))?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="ui_window_candidates" || value["confirmation_required"]!=true
            || value["ui_authorized"]!=false || !value["windows"].is_array(){return Err(ErrorCode::TargetChanged);}Ok(value)
    }
    pub fn capabilities(&self) -> std::result::Result<Value, ErrorCode> {
        serde_json::from_str(&self.call("GetCapabilities", &())?).map_err(|_| ErrorCode::InvalidArgument)
    }
    pub fn privacy_scopes(&self) -> std::result::Result<Value, ErrorCode> {
        let value:Value=serde_json::from_str(&self.call("PrivacyScopes",&())?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="privacy_scopes" || value["mutation_performed"]!=false
            || value["data"]["owner"]!="authenticated_client" || !value["data"]["file_roots"].is_array()
            || !value["data"]["retained_history"].is_u64() || !value["data"]["active_tasks"].is_u64() {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn list_history(&self) -> std::result::Result<Value, ErrorCode> {
        let value:Value=serde_json::from_str(&self.call("ListHistory",&())?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="history_list" || value["mutation_performed"]!=false
            || value["data"]["owner"]!="authenticated_client" || value["data"]["persistent"]!=false
            || !value["data"]["entries"].is_array() {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn list_automations(&self) -> std::result::Result<Value, ErrorCode> {
        let value:Value=serde_json::from_str(&self.call("ListAutomations",&())?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="automation_list" || value["mutation_performed"]!=false
            || value["data"]["owner"]!="authenticated_client" || value["data"]["persistent"]!=false
            || value["data"]["scheduling_available"]!=false || !value["data"]["definitions"].is_array() {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn model_status(&self) -> std::result::Result<Value, ErrorCode> {
        let value:Value=serde_json::from_str(&self.call("ModelStatus",&())?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="model_status" || value["mutation_performed"]!=false
            || !value["data"]["loaded"].is_boolean() || !value["data"]["busy"].is_boolean()
            || !value["data"]["own_queued"].is_u64() || !value["data"]["queue_limit"].is_u64()
            || !value["data"]["threads"].is_u64() || !value["data"]["context_tokens"].is_u64() {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn unload_model(&self) -> std::result::Result<Value, ErrorCode> {
        let value:Value=serde_json::from_str(&self.call("UnloadModel",&())?).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["operation"]!="model_unload"
            || !value["data"]["unload_requested"].is_boolean()
            || value["mutation_performed"]!=value["data"]["unload_requested"] {
            return Err(ErrorCode::TargetChanged);
        }
        Ok(value)
    }
    pub fn submit(&self, request: &crate::Submit) -> std::result::Result<String, ErrorCode> {
        let value = serde_json::json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":{"kind":"submit","request":request}}).to_string();
        if value.len() > MAX_TASK_BYTES { return Err(ErrorCode::ResourceExhausted); }
        let task = self.call("Submit", &(value.as_str(),))?;
        if !crate::uuid(&task) { return Err(ErrorCode::InvalidArgument); }
        Ok(task)
    }
    pub fn status(&self, task: &str) -> std::result::Result<Value, ErrorCode> {
        if task.len() > 128 || !crate::uuid(task) { return Err(ErrorCode::InvalidArgument); }
        let value: Value = serde_json::from_str(&self.call("GetStatus", &(task,))?).map_err(|_| ErrorCode::InvalidArgument)?;
        if value["schema_version"] != 1 || value["request_id"] != task || value["operation"] != "task_status" { return Err(ErrorCode::TargetChanged); }
        Ok(value)
    }
    pub fn events(&self,task:&str,after_sequence:u64,limit:u32)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(task) || !(1..=100).contains(&limit){return Err(ErrorCode::InvalidArgument);}
        self.task_reply(task,"task_events",self.call("GetEvents",&(task,after_sequence,limit))?)
    }
    pub fn cancel(&self,task:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(task){return Err(ErrorCode::InvalidArgument);}
        self.task_reply(task,"cancellation",self.call("Cancel",&(task,))?)
    }
    pub fn forget(&self,task:&str)->std::result::Result<Value,ErrorCode>{
        if !crate::uuid(task){return Err(ErrorCode::InvalidArgument);}
        self.task_reply(task,"deletion",self.call("Forget",&(task,))?)
    }
    fn task_reply(&self,task:&str,operation:&str,raw:String)->std::result::Result<Value,ErrorCode>{
        let value:Value=serde_json::from_str(&raw).map_err(|_|ErrorCode::InvalidArgument)?;
        if value["schema_version"]!=1 || value["request_id"]!=task || value["operation"]!=operation{return Err(ErrorCode::TargetChanged);}Ok(value)
    }
}

fn client_error(error: zbus::Error) -> ErrorCode {
    if let zbus::Error::MethodError(name, _, _) = &error {
        if let Some(code) = name.as_str().strip_prefix("org.aios.Error.") {
            return serde_json::from_value(Value::String(code.into())).unwrap_or(ErrorCode::PartialResult);
        }
    }
    match error {
        zbus::Error::InputOutput(error) if error.kind() == std::io::ErrorKind::TimedOut => ErrorCode::DeadlineExceeded,
        _ => ErrorCode::PartialResult,
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn stop_has_reserved_bounded_admission_when_observations_are_saturated(){
        let agent=Agent::new(Arc::new(Mutex::new(crate::State::default())));
        let normal=(0..16).map(|_|agent.admit().unwrap()).collect::<Vec<_>>();
        assert!(agent.admit().is_err());
        let stops=(0..4).map(|_|agent.admit_control().unwrap()).collect::<Vec<_>>();
        assert!(agent.admit_control().is_err());
        drop(stops);assert_eq!(agent.control_active.load(Ordering::Acquire),0);
        assert!(agent.admit().is_err());drop(normal);assert_eq!(agent.active.load(Ordering::Acquire),0);
    }
}
