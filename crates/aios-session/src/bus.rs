//! Public session control. The bus, never request JSON, supplies the subject.
use crate::{Operation, Request, SharedState, identity::{self, Peer}, parse_operation};
use aios_protocol::{MAX_TASK_BYTES, contracts::ErrorCode};
use serde_json::Value;
use std::{fmt, sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
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

pub struct Agent { state: SharedState, active: Arc<AtomicUsize> }
impl Agent {
    pub fn new(state: SharedState) -> Self { Self { state, active: Arc::new(AtomicUsize::new(0)) } }
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
        let outcome = self.state.lock().map_err(|_| ErrorCode::ResourceExhausted)?.dispatch(&peer, operation);
        if Self::peer(connection, &header).await? != peer { return Err(ErrorCode::TargetChanged.into()); }
        outcome.map_err(Into::into)
    }
    async fn json(&self, connection: &Connection, header: Header<'_>, operation: Operation) -> Result<String> {
        let result = self.dispatch(connection, header, operation).await?;
        let json = serde_json::to_string(&result).map_err(|_| ErrorCode::InvalidArgument)?;
        if json.len() > aios_protocol::MAX_FRAME_BYTES { return Err(ErrorCode::ResourceExhausted.into()); }
        Ok(json)
    }
}

#[zbus::interface(name = "org.aios.Agent1")]
impl Agent {
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

pub fn export(address: &str, state: SharedState) -> zbus::Result<zbus::blocking::Connection> {
    zbus::blocking::connection::Builder::address(address)?.method_timeout(Duration::from_secs(2))
        .max_queued(64).allow_name_replacements(false).replace_existing_names(false)
        .serve_at(PATH, Agent::new(state))?.name(NAME)?.build()
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
        self.verify()?;
        let api = zbus::blocking::Proxy::new(&self.connection, self.owner.as_str(), PATH, INTERFACE).map_err(|_| ErrorCode::UnsupportedCapability)?;
        let result: String = api.call(method, body).map_err(client_error)?;
        self.verify()?;
        if result.len() > aios_protocol::MAX_FRAME_BYTES { return Err(ErrorCode::ResourceExhausted); }
        Ok(result)
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
