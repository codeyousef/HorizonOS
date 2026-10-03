//! Private, peer-authenticated read-only control and task lifecycle.
pub mod identity;
pub mod bus;
use aios_protocol::{MAX_TASK_BYTES, read_frame_with_limit, write_frame, contracts::{Action, ErrorCode, ProviderError, parse_tool_call, canonical_json}};
use aios_system::services::{service_result, validate_service_name};
use identity::Peer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, value::RawValue};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io, os::unix::net::UnixStream, sync::{Arc, Mutex}, time::{Duration, Instant}};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Ask, Act, Diagnose, Automate }

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Submit {
    pub mode: Mode, pub text: String, pub client_nonce: String,
    #[serde(default)] pub context_handles: Vec<String>,
    pub selected_app_handle: Option<String>,
    pub selected_session_handle: Option<String>,
}

#[derive(Debug)]
pub enum Operation {
    GetCapabilities,
    GetSystemInfo,
    ResolveService { unit_name: String },
    Invoke { tool_call: Box<RawValue> },
    Submit { request: Submit },
    GetStatus { task_id: String },
    GetEvents { task_id: String, after_sequence: u64, limit: u32 },
    Cancel { task_id: String },
    Forget { task_id: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request { pub schema_version: u32, pub request_id: String, pub operation: Box<RawValue> }

fn parse_operation(raw: &str) -> Result<Operation, ErrorCode> {
    #[derive(Deserialize)]
    struct Kind { kind: String }
    let kind: Kind = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
    // Parse each variant directly from the original bytes. Internally tagged
    // enums buffer content and their unit variants can ignore unknown fields.
    // Direct structs reject duplicate/unknown fields and preserve raw calls.
    macro_rules! fields {
        ($variant:ident { $($name:ident: $ty:ty),* }) => {{
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Fields { kind: String, $($name: $ty),* }
            let fields: Fields = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
            if fields.kind != kind.kind { return Err(ErrorCode::InvalidArgument); }
            Ok(Operation::$variant { $($name: fields.$name),* })
        }};
    }
    match kind.kind.as_str() {
        "get_capabilities" | "get_system_info" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Empty { kind: String }
            let fields: Empty = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
            match fields.kind.as_str() {
                "get_capabilities" => Ok(Operation::GetCapabilities),
                "get_system_info" => Ok(Operation::GetSystemInfo),
                _ => Err(ErrorCode::InvalidArgument),
            }
        },
        "resolve_service" => fields!(ResolveService { unit_name: String }),
        "invoke" => fields!(Invoke { tool_call: Box<RawValue> }),
        "submit" => fields!(Submit { request: Submit }),
        "get_status" => fields!(GetStatus { task_id: String }),
        "get_events" => fields!(GetEvents { task_id: String, after_sequence: u64, limit: u32 }),
        "cancel" => fields!(Cancel { task_id: String }),
        "forget" => fields!(Forget { task_id: String }),
        _ => Err(ErrorCode::InvalidArgument),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema_version: u32, pub request_id: String, pub operation: String,
    pub data: Option<Value>, pub error: Option<ProviderError>,
}

#[derive(Clone, Serialize)]
pub struct TaskStatus {
    pub schema_version: u32, pub operation: String,
    pub request_id: String, pub mode: Mode, pub state: String,
    pub submitted_at: String, pub mutation_performed: bool, pub error: ErrorCode,
}

struct Task { owner: Peer, expires: Instant, nonce: String, digest: [u8; 32], status: TaskStatus }
struct Handle { owner: Peer, expires: Instant, unit: String }
#[derive(Default)]
pub struct State { tasks: HashMap<String, Task>, handles: HashMap<String, Handle> }
pub type SharedState = Arc<Mutex<State>>;

fn now() -> String { OffsetDateTime::now_utc().format(&Rfc3339).expect("valid timestamp") }
fn uuid(value: &str) -> bool { Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value) }
fn provider<T: Serialize>(value: T) -> Result<Value, ErrorCode> { serde_json::to_value(value).map_err(|_| ErrorCode::InvalidArgument) }

impl State {
    fn prune(&mut self) {
        let time = Instant::now();
        self.tasks.retain(|_, task| task.expires > time);
        self.handles.retain(|_, handle| handle.expires > time);
    }
    fn task(&self, id: &str, peer: &Peer) -> Result<&Task, ErrorCode> {
        if id.is_empty() || id.chars().count() > 128 { return Err(ErrorCode::InvalidArgument); }
        let task = self.tasks.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if task.owner != *peer { return Err(ErrorCode::PermissionDenied); }
        Ok(task)
    }
    pub fn dispatch(&mut self, peer: &Peer, operation: Operation) -> Result<Value, ErrorCode> {
        self.prune();
        match operation {
            Operation::GetCapabilities => Ok(json!({"schema_version":1,"request_id":Uuid::new_v4().to_string(),"operation":"capabilities","actions":["system.info","system.service_status"],
                "read_only":true,"inference_available":false,"ui_enabled":false,"transport":"private-unix",
                "task_request_max_bytes":MAX_TASK_BYTES,"session_associated":peer.logind_session.is_some()})),
            Operation::GetSystemInfo => provider(aios_system::observe_system_info()),
            Operation::ResolveService { unit_name } => {
                validate_service_name(&unit_name)?;
                if self.handles.len() >= 256 || self.handles.values().filter(|h| h.owner == *peer).count() >= 32 { return Err(ErrorCode::ResourceExhausted); }
                // Resolve a known loaded unit before issuing a scope-bound handle.
                let id = Uuid::new_v4().to_string();
                aios_system::services::read_service_status(&unit_name, &id)?;
                self.handles.insert(id.clone(), Handle { owner: peer.clone(), expires: Instant::now() + Duration::from_secs(30), unit: unit_name });
                Ok(json!({"service_id":id,"expires_after_ms":30000}))
            },
            Operation::Invoke { tool_call } => match parse_tool_call(tool_call.get().as_bytes())? {
                Action::SystemInfo => provider(aios_system::observe_system_info()),
                Action::SystemServiceStatus(args) => {
                    let handle = self.handles.get(&args.service_id).ok_or(ErrorCode::TargetNotFound)?;
                    if handle.owner != *peer { return Err(ErrorCode::PermissionDenied); }
                    provider(service_result(&handle.unit, &args.service_id))
                },
                _ => Err(ErrorCode::UnsupportedCapability),
            },
            Operation::Submit { request } => {
                if request.text.trim().is_empty() || request.text.len() > 60000 || request.client_nonce.is_empty() || request.client_nonce.len() > 128 {
                    return Err(ErrorCode::InvalidArgument);
                }
                if !request.context_handles.is_empty() || request.selected_app_handle.is_some() || request.selected_session_handle.is_some() {
                    return Err(ErrorCode::AuthRequired);
                }
                let digest: [u8; 32] = Sha256::digest(canonical_json(&provider(&request)?)?).into();
                for (id, task) in &self.tasks {
                    if task.owner == *peer && task.nonce == request.client_nonce {
                        if task.digest != digest { return Err(ErrorCode::Conflict); }
                        return Ok(json!({"request_id":id}));
                    }
                }
                if self.tasks.len() >= 64 || self.tasks.values().filter(|t| t.owner == *peer).count() >= 8 { return Err(ErrorCode::ResourceExhausted); }
                let id = Uuid::new_v4().to_string();
                // Never guess an action from text, execute a mutation or fake a
                // model answer. Lifecycle remains available while model is absent.
                let status = TaskStatus { schema_version: 1, operation: "task_status".into(), request_id: id.clone(), mode: request.mode, state: "failed".into(), submitted_at: now(),
                    mutation_performed: false, error: ErrorCode::ModelUnavailable };
                self.tasks.insert(id.clone(), Task { owner: peer.clone(), expires: Instant::now() + Duration::from_secs(300),
                    nonce: request.client_nonce, digest, status });
                Ok(json!({"request_id":id}))
            },
            Operation::GetStatus { task_id } => provider(&self.task(&task_id, peer)?.status),
            Operation::GetEvents { task_id, after_sequence, limit } => {
                let task = self.task(&task_id, peer)?;
                if !(1..=100).contains(&limit) { return Err(ErrorCode::InvalidArgument); }
                let all = [json!({"sequence":1,"kind":"accepted","request_id":task_id,"observed_at":task.status.submitted_at}),
                    json!({"sequence":2,"kind":"failed","request_id":task_id,"code":"MODEL_UNAVAILABLE","mutation_performed":false})];
                let events = all.into_iter().filter(|event| event["sequence"].as_u64().unwrap() > after_sequence).take(limit as usize).collect::<Vec<_>>();
                let last = events.last().and_then(|e| e["sequence"].as_u64()).unwrap_or(after_sequence);
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"task_events","events":events,"complete":last>=2,"next_sequence":last}))
            },
            Operation::Cancel { task_id } => {
                self.task(&task_id, peer)?;
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"cancellation","cancelled":false,"already_terminal":true,"mutation_performed":false}))
            },
            Operation::Forget { task_id } => {
                self.task(&task_id, peer)?;
                self.tasks.remove(&task_id);
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"deletion","deleted":true}))
            },
        }
    }
}

pub fn serve_connection(mut stream: UnixStream, state: SharedState) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let peer = identity::authenticate(&stream).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "untrusted peer"))?;
    for _ in 0..128 {
        let Some(frame) = read_frame_with_limit(&mut stream, MAX_TASK_BYTES)? else { return Ok(()); };
        identity::verify(&stream, &peer).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "peer changed"))?;
        let request: Request = serde_json::from_str(&frame).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid request"))?;
        if request.schema_version != 1 || !uuid(&request.request_id) { return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid version or correlation id")); }
        let operation = parse_operation(request.operation.get()).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid operation"))?;
        let outcome = state.lock().map_err(|_| io::Error::other("state unavailable"))?.dispatch(&peer, operation);
        let (data, error) = match outcome {
            Ok(value) => (Some(value), None),
            Err(code) => (None, Some(ProviderError { code, message: "Request could not be fulfilled within authenticated scope".into(), retryable: false })),
        };
        identity::verify(&stream, &peer).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "peer changed"))?;
        let response = Response { schema_version: 1, request_id: request.request_id, operation: "response".into(), data, error };
        write_frame(&mut stream, &serde_json::to_string(&response).map_err(io::Error::other)?)?;
    }
    Ok(())
}

pub struct Client { stream: UnixStream, peer: Peer }
impl Client {
    pub fn connect(path: &std::path::Path) -> io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        let peer = identity::authenticate(&stream).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied,"server identity unavailable"))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self { stream, peer })
    }
    pub fn call(&mut self, operation: Value) -> io::Result<Response> {
        identity::verify(&self.stream, &self.peer).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied,"server identity changed"))?;
        let id = Uuid::new_v4().to_string();
        let request = json!({"schema_version":1,"request_id":id,"operation":operation}).to_string();
        if request.len() > MAX_TASK_BYTES { return Err(io::Error::new(io::ErrorKind::InvalidInput,"request too large")); }
        write_frame(&mut self.stream, &request)?;
        let raw = read_frame_with_limit(&mut self.stream, aios_protocol::MAX_FRAME_BYTES)?.ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof,"missing reply"))?;
        let response: Response = serde_json::from_str(&raw).map_err(io::Error::other)?;
        identity::verify(&self.stream, &self.peer).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied,"server identity changed"))?;
        if response.schema_version != 1 || response.request_id != id || response.operation != "response" {
            return Err(io::Error::new(io::ErrorKind::InvalidData,"reply correlation mismatch"));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn peer_fixture() -> Peer {
        Peer { uid: 1000, pid: 200, start_ticks: 3, boot_id: "fixture-boot".into(),
            logind_session: None, remote: true, session_type: None, ui_enabled: false, bus_sender: None, bus_id: None }
    }
    fn submit_fixture(mode: Mode, nonce: &str, text: &str) -> Operation {
        Operation::Submit { request: Submit { mode, text: text.into(), client_nonce: nonce.into(),
            context_handles: vec![], selected_app_handle: None, selected_session_handle: None } }
    }
    #[test]
    fn raw_calls_preserve_duplicates_for_strict_action_parser() {
        let raw=r#"{"kind":"invoke","tool_call":{"kind":"tool_call","action_id":"system.info","action_id":"system.info","arguments":{}}}"#;
        let Operation::Invoke{tool_call}=parse_operation(raw).unwrap() else {panic!("wrong operation")};
        assert_eq!(parse_tool_call(tool_call.get().as_bytes()),Err(ErrorCode::InvalidArgument));
        assert!(parse_operation(r#"{"kind":"get_capabilities","uid":0}"#).is_err());
        assert!(parse_operation(r#"{"kind":"invoke","kind":"invoke","tool_call":{}}"#).is_err());
        for raw in [
            r#"{"kind":"get_system_info","approved":true}"#,
            r#"{"kind":"get_status","task_id":"a","task_id":"b"}"#,
            r#"{"kind":"cancel","task_id":"a","uid":0}"#,
            r#"{"kind":"get_events","task_id":"a","after_sequence":0,"limit":1,"trust":"admin"}"#,
            r#"{"kind":"submit","request":{"mode":"ask","text":"test","client_nonce":"a","mode":"act"}}"#,
        ] { assert_eq!(parse_operation(raw).unwrap_err(), ErrorCode::InvalidArgument); }
    }
    #[test]
    fn unavailable_model_modes_nonce_conflict_and_quota_fixture() {
        let peer = peer_fixture();
        let mut state = State::default();
        for (index, mode) in [Mode::Ask, Mode::Diagnose, Mode::Act, Mode::Automate].into_iter().enumerate() {
            let nonce = index.to_string();
            let task = state.dispatch(&peer, submit_fixture(mode, &nonce, "restart the service")).unwrap()["request_id"].as_str().unwrap().to_owned();
            let status = state.dispatch(&peer, Operation::GetStatus { task_id: task }).unwrap();
            assert_eq!(status["error"], "MODEL_UNAVAILABLE");
            assert_eq!(status["mutation_performed"], false);
            assert_eq!(state.dispatch(&peer, submit_fixture(mode, &nonce, "different request")).unwrap_err(), ErrorCode::Conflict);
        }
        for index in 4..8 { state.dispatch(&peer, submit_fixture(Mode::Ask, &index.to_string(), "test")).unwrap(); }
        assert_eq!(state.dispatch(&peer, submit_fixture(Mode::Ask, "ninth", "test")).unwrap_err(), ErrorCode::ResourceExhausted);
    }
    #[test]
    fn expired_tasks_and_handles_are_denied_before_provider_access_fixture() {
        let peer = peer_fixture();
        let mut state = State::default();
        let task = state.dispatch(&peer, submit_fixture(Mode::Ask, "expire", "test")).unwrap()["request_id"].as_str().unwrap().to_owned();
        state.tasks.get_mut(&task).unwrap().expires = Instant::now() - Duration::from_secs(1);
        state.handles.insert("handle".into(), Handle { owner: peer.clone(), expires: Instant::now() - Duration::from_secs(1), unit: "must-not-be-accessed.service".into() });
        assert_eq!(state.dispatch(&peer, Operation::GetStatus { task_id: task }).unwrap_err(), ErrorCode::TargetNotFound);
        let call = RawValue::from_string(r#"{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":"handle"}}"#.into()).unwrap();
        assert_eq!(state.dispatch(&peer, Operation::Invoke { tool_call: call }).unwrap_err(), ErrorCode::TargetNotFound);
    }
}
