//! Public session control. The bus, never request JSON, supplies the subject.
use crate::{Operation, Request, SharedState, identity::{self, Peer}, parse_operation};
use aios_protocol::{MAX_TASK_BYTES, contracts::ErrorCode};
use serde_json::Value;
use std::{collections::HashMap,fmt,sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}},time::{Duration,Instant}};
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
pub struct Agent { state:SharedState,active:Arc<AtomicUsize>,ui:Arc<Mutex<HashMap<(String,String),UiConnection>>>,processes:Arc<Mutex<HashMap<(String,String),ProcessConnection>>> }
struct UiConnection { peer:Peer,expires:Instant,client:Arc<crate::graphical::Connection> }
struct ProcessConnection { peer:Peer,expires:u64,client:crate::process_bridge::Client }
impl Agent {
    pub fn new(state: SharedState) -> Self { Self { state,active:Arc::new(AtomicUsize::new(0)),ui:Arc::new(Mutex::new(HashMap::new())),processes:Arc::new(Mutex::new(HashMap::new())) } }
    fn admit(&self) -> Result<Admission> {
        if self.active.fetch_add(1, Ordering::AcqRel) >= 16 {
            self.active.fetch_sub(1, Ordering::AcqRel);
            return Err(ErrorCode::ResourceExhausted.into());
        }
        Ok(Admission(self.active.clone()))
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
        let _admission = self.admit()?;
        let peer = Self::peer(connection, &header).await?;
        let outcome=if let Some(action)=crate::process_bridge::action(&operation)?{
            let agent=self.clone();let original=peer.clone();
            blocking::unblock(move||agent.process_read(&original,&action)).await
        }else{match operation {
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
        let mut clients=self.processes.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
        clients.retain(|_,client|client.expires>now);
        if let Some(client)=clients.get(&key){if client.peer!=*peer{return Err(ErrorCode::TargetChanged);}}
        else{
            if clients.len()>=4{return Err(ErrorCode::ResourceExhausted);}
            clients.insert(key.clone(),ProcessConnection{peer:peer.clone(),expires:now.checked_add(34_000).ok_or(ErrorCode::ResourceExhausted)?,client:crate::process_bridge::Client::connect(None,peer)?});
        }
        let client=clients.get_mut(&key).ok_or(ErrorCode::TargetChanged)?;
        let result=client.client.call(action);
        client.expires=aios_policy::boottime_ms()?.checked_add(34_000).ok_or(ErrorCode::ResourceExhausted)?;
        identity::verify_peer(peer)?;
        // Domain permission errors (e.g. another caller's process handle) do
        // not invalidate this caller's other retained handles or cursor.
        if matches!(result,Err(ErrorCode::TargetChanged)){clients.remove(&key);}
        result
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
        // Authenticate through the same control path even when the capability
        // set is empty. A registered contract never implies provider readiness.
        self.dispatch(connection, header, Operation::GetCapabilities).await?;
        let contracts = actions.iter().map(|id| aios_protocol::registry::capability(id)
            .map(|contract| serde_json::json!({"action_id":id,"input_schema":contract.input_schema,
                "output_schema":contract.output_schema,"availability":"unavailable"})))
            .collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(serde_json::json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
            "operation":"capabilities","interface":interface,"available_actions":[],"contracts":contracts}).to_string())
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
surface!(Files,"org.aios.Files1",[(search,"files.search"),(metadata,"files.metadata"),(read,"files.read"),
    (summarize,"files.summarize"),(copy,"files.copy"),(move_file,"files.move"),(trash,"files.trash"),(restore,"files.restore")]);
surface!(Applications,"org.aios.Applications1",[(list,"apps.list"),(launch,"apps.launch"),(actions,"apps.actions"),(invoke,"apps.invoke")]);
surface!(Settings,"org.aios.Settings1",[(get,"settings.get"),(set,"settings.set")]);

pub fn export(address: &str, state: SharedState) -> zbus::Result<zbus::blocking::Connection> {
    let agent = Agent::new(state);
    zbus::blocking::connection::Builder::address(address)?.method_timeout(Duration::from_secs(2))
        .max_queued(64).allow_name_replacements(false).replace_existing_names(false)
        .serve_at("/org/aios/Files1",Files{agent:agent.clone()})?
        .serve_at("/org/aios/Applications1",Applications{agent:agent.clone()})?
        .serve_at("/org/aios/Settings1",Settings{agent:agent.clone()})?
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
