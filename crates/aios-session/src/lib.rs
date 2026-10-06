//! Private, peer-authenticated read-only control and task lifecycle.
pub mod identity;
pub mod display;
pub mod accessibility;
pub mod ui_read;
mod user_bus;
mod graphical;
pub mod managed_service;
pub mod ui_bridge;
pub mod process_bridge;
pub(crate) mod process_termination;
mod process_tasks;
mod process_control;
pub mod native_startup;
pub mod bus;
pub mod inference;
mod processes;
mod process_selection;
use aios_protocol::{MAX_TASK_BYTES, read_frame_with_limit, write_frame, contracts::{Action, ErrorCode, ProviderError, parse_tool_call, canonical_json}};
use aios_system::services::{service_result, validate_service_name};
use identity::Peer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, value::RawValue};
use sha2::{Digest, Sha256};
use std::{collections::{HashMap, VecDeque}, io, os::unix::net::UnixStream, sync::{Arc, Mutex, atomic::{AtomicU8, Ordering}}, time::{Duration, Instant}};
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
    #[serde(default)] pub retain_for_history: bool,
    #[serde(default)] pub history_handles: Vec<String>,
    pub selected_app_handle: Option<String>,
    pub selected_session_handle: Option<String>,
}

#[derive(Debug)]
pub enum Operation {
    StartProcessTermination { task_id:String,process_id:String,session_handle:String,goal:String,mode:Mode },
    GetProcessTermination { task_id:String },CancelProcessTermination { task_id:String },ForgetProcessTermination { task_id:String },
    GetCapabilities,
    GetSystemInfo,
    SelectUiSession { session_id: String },
    ListUiWindows { session_handle: String },
    StartUiRead { window_handle: String, goal: String, mode: Mode },
    GetUiReadStatus { task_id: String },
    TakeUiSnapshot { task_id: String },
    CancelUiRead { task_id: String },
    ForgetUiRead { task_id: String },
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
        "start_process_termination"=>fields!(StartProcessTermination{task_id:String,process_id:String,session_handle:String,goal:String,mode:Mode}),
        "get_process_termination"=>fields!(GetProcessTermination{task_id:String}),
        "cancel_process_termination"=>fields!(CancelProcessTermination{task_id:String}),
        "forget_process_termination"=>fields!(ForgetProcessTermination{task_id:String}),
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
        "select_ui_session" => fields!(SelectUiSession { session_id: String }),
        "list_ui_windows" => fields!(ListUiWindows { session_handle: String }),
        "start_ui_read" => fields!(StartUiRead { window_handle: String, goal: String, mode: Mode }),
        "get_ui_read_status" => fields!(GetUiReadStatus { task_id: String }),
        "take_ui_snapshot" => fields!(TakeUiSnapshot { task_id: String }),
        "cancel_ui_read" => fields!(CancelUiRead { task_id: String }),
        "forget_ui_read" => fields!(ForgetUiRead { task_id: String }),
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
    pub submitted_at: String, pub mutation_performed: bool, pub error: Option<ErrorCode>,
    pub output: Option<Value>,
}

fn history_response_text(response:&Value)->Result<&str,ErrorCode>{
    let field=match response["kind"].as_str(){Some("answer")=>"text",Some("clarification")=>"question",Some("abstain")=>"reason",_=>return Err(ErrorCode::PartialResult)};
    response[field].as_str().ok_or(ErrorCode::PartialResult)
}
struct HistoryRef { id: String, digest: String }
struct Task {
    owner: Peer, expires: Instant, nonce: String, digest: [u8; 32], status: TaskStatus,
    deadline: Instant, boottime_deadline: u64, control: Arc<AtomicU8>, text: Option<String>, events: Vec<Value>, grant: Option<aios_policy::ReadGrant>, service_handles: Vec<String>,
    retained_question:Option<inference::Secret>, history_refs:Vec<HistoryRef>,
    process_selection:Option<Arc<process_selection::Selection>>,
    graphical:Option<graphical::Selection>,native_cancel:Option<Arc<ui_bridge::Cancellation>>,native_receipt:Option<graphical::Receipt>,
}
impl Task {
    fn terminal(&self) -> bool { matches!(self.status.state.as_str(), "completed" | "failed" | "cancelled") }
    fn event(&mut self, kind: &str) {
        self.events.push(json!({"sequence":self.events.len()+1,"kind":kind,"request_id":self.status.request_id,"observed_at":now()}));
    }
    fn finish(&mut self, result: Result<Value, ErrorCode>) {
        if self.terminal() { return; }
        if let Some(grant) = &self.grant { grant.revoke(); }
        self.process_selection.take();
        if let Some(cancel)=self.native_cancel.take(){cancel.cancel();}self.native_receipt.take();
        let cause = self.control.load(Ordering::Acquire);
        let result = match result { Ok(_) if cause == 1 => Err(ErrorCode::Cancelled), Ok(_) if cause == 2 => Err(ErrorCode::DeadlineExceeded), other => other };
        match result {
            Ok(output) => { self.status.output = Some(output); self.status.state = "completed".into(); }
            Err(error) => { self.retained_question.take(); self.status.error = Some(error); self.status.state = if error == ErrorCode::Cancelled { "cancelled" } else { "failed" }.into(); }
        }
        if let Some(mut text) = self.text.take() { inference::wipe(&mut text); }
        self.expires = Instant::now() + Duration::from_secs(300);
        let kind = self.status.state.clone(); self.event(&kind);
        let last = self.events.last_mut().expect("terminal event");
        last["code"] = json!(self.status.error); last["mutation_performed"] = json!(false);
    }
}
impl Drop for Task {
    fn drop(&mut self) { self.control.store(1,Ordering::Release);if let Some(cancel)=&self.native_cancel{cancel.cancel();}
        if let Some(grant)=&self.grant{grant.revoke();}if let Some(text)=&mut self.text{inference::wipe(text);} }
}
struct UiCandidate { owner: Peer, expires: Instant, session: identity::GraphicalSession }
struct Handle { owner: Peer, expires: Instant, unit: String }
#[derive(Default)]
struct ReadResources(Vec<aios_policy::Resource>);
impl aios_policy::CurrentResources for ReadResources {
    fn resolve(&self, field: &str, kind: &str, handle: &str) -> Result<String, ErrorCode> {
        self.0.iter().find(|r| r.field == field && r.kind == kind && r.handle == handle)
            .map(|r| r.identity_sha256.clone()).ok_or(ErrorCode::PermissionDenied)
    }
    fn dynamic_arguments(&self, _: &str, _: &Value, _: &aios_policy::Scope) -> Result<(), ErrorCode> { Err(ErrorCode::UnsupportedCapability) }
}
#[derive(Default)]
pub struct State { tasks: HashMap<String, Task>, handles: HashMap<String, Handle>, ui_candidates: HashMap<String, UiCandidate>, queue: VecDeque<String>, inference_configured: bool, inference_available: bool, policy: Option<aios_policy::Policy>, processes: processes::Handles }
pub type SharedState = Arc<Mutex<State>>;

fn now() -> String { OffsetDateTime::now_utc().format(&Rfc3339).expect("valid timestamp") }
fn uuid(value: &str) -> bool { Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value) }
fn provider<T: Serialize>(value: T) -> Result<Value, ErrorCode> { serde_json::to_value(value).map_err(|_| ErrorCode::InvalidArgument) }

impl State {
    pub fn with_inference() -> Self { Self { inference_configured: true, ..Self::default() } }
    fn policy(&mut self, peer: &Peer) -> Result<&aios_policy::Policy, ErrorCode> {
        if self.policy.is_none() { self.policy = Some(aios_policy::Policy::new(peer.boot_id.clone(), aios_policy::registry_revision())?); }
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)
    }
    fn read_grant(&mut self, peer: &Peer, id: String, text: &str, mode: Mode, scope: aios_policy::Scope, expiry_ms: u64) -> Result<aios_policy::ReadGrant, ErrorCode> {
        let policy = self.policy(peer)?;
        let mode = match mode { Mode::Ask => aios_policy::Mode::Ask, Mode::Diagnose => aios_policy::Mode::Diagnose,
            Mode::Act => aios_policy::Mode::Act, Mode::Automate => aios_policy::Mode::Automate };
        let intent = policy.authenticated_user_intent(peer.policy_subject()?, id, text, mode)?;
        policy.grant_reads(intent, scope, aios_policy::boottime_ms()?, expiry_ms)
    }
    fn check_task_read(&self, id: &str, peer: &Peer) -> Result<(), ErrorCode> {
        let task = self.task(id, peer)?;
        for reference in &task.history_refs {
            if self.history_digest(&reference.id,peer)? != reference.digest { return Err(ErrorCode::TargetChanged); }
        }
        if let Some(selection)=&task.graphical{return task.native_receipt.as_ref().ok_or(ErrorCode::AuthRequired)?.check(selection,peer,id,&task.digest);}
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)?.check_read(task.grant.as_ref().ok_or(ErrorCode::AuthRequired)?,
            &peer.policy_subject()?, id, &Action::SystemInfo, &ReadResources::default(), aios_policy::boottime_ms()?)
    }
    fn history_source(&self,id:&str,peer:&Peer)->Result<&Task,ErrorCode>{
        let source=self.task(id,peer)?;
        if source.expires<=Instant::now(){return Err(ErrorCode::ApprovalExpired);}
        if source.retained_question.is_none(){return Err(ErrorCode::AuthRequired);}
        if source.status.state!="completed" || source.graphical.is_some() || source.status.mutation_performed {
            return Err(ErrorCode::AuthRequired);
        }
        let response=&source.status.output.as_ref().ok_or(ErrorCode::PartialResult)?["response"];
        history_response_text(response)?;
        Ok(source)
    }
    fn history_digest(&self,id:&str,peer:&Peer)->Result<String,ErrorCode>{
        let source=self.history_source(id,peer)?;
        let response=&source.status.output.as_ref().ok_or(ErrorCode::PartialResult)?["response"];
        aios_policy::digest(&json!({"request_digest":source.digest,"submitted_at":source.status.submitted_at,
            "question":source.retained_question.as_ref().ok_or(ErrorCode::AuthRequired)?.0,
            "kind":response["kind"],"text":history_response_text(response)?}))
    }
    fn task_history(&self,id:&str,peer:&Peer)->Result<Vec<inference::HistoryEntry>,ErrorCode>{
        self.check_task_read(id,peer)?;
        let mut entries=Vec::new();
        for reference in &self.task(id,peer)?.history_refs {
            let source=self.history_source(&reference.id,peer)?;
            let response=&source.status.output.as_ref().ok_or(ErrorCode::PartialResult)?["response"];
            entries.push(inference::HistoryEntry { id:reference.id.clone(),submitted_at:source.status.submitted_at.clone(),
                question:inference::Secret(source.retained_question.as_ref().ok_or(ErrorCode::AuthRequired)?.0.clone()),
                kind:response["kind"].as_str().ok_or(ErrorCode::PartialResult)?.into(),
                text:inference::Secret(history_response_text(response)?.into()) });
        }
        entries.sort_by(|a,b|b.submitted_at.cmp(&a.submitted_at).then_with(||a.id.cmp(&b.id)));
        Ok(entries)
    }
    fn service_resource(&self,peer:&Peer,id:&str)->Result<(String,aios_policy::Resource),ErrorCode>{
        let handle=self.handles.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if handle.owner!=*peer{return Err(ErrorCode::PermissionDenied);}
        if handle.expires<=Instant::now(){return Err(ErrorCode::ApprovalExpired);}
        let resource=aios_policy::Resource{field:"service_id".into(),kind:"scope-owner-expiry".into(),
            handle:id.into(),identity_sha256:aios_policy::digest(&handle.unit)?};
        Ok((handle.unit.clone(),resource))
    }
    fn task_process_selection(&self,id:&str,peer:&Peer)->Result<Option<Arc<process_selection::Selection>>,ErrorCode>{
        self.check_task_read(id,peer)?;Ok(self.task(id,peer)?.process_selection.clone())
    }
    fn task_service_handles(&self,id:&str,peer:&Peer)->Result<Vec<String>,ErrorCode>{
        let task=self.task(id,peer)?;
        Ok(task.service_handles.clone())
    }
    fn task_tools(&self,id:&str,peer:&Peer)->Result<Vec<aios_protocol::inference::ReadTool>,ErrorCode>{
        self.check_task_read(id,peer)?;
        let mut tools=vec![aios_protocol::inference::ReadTool::SystemInfo];
        if !self.task(id,peer)?.service_handles.is_empty(){tools.push(aios_protocol::inference::ReadTool::SystemServiceStatus);}
        if self.task(id,peer)?.process_selection.is_some(){tools.push(aios_protocol::inference::ReadTool::ProcessInspect);}
        Ok(tools)
    }
    fn task_read_target(&self,id:&str,peer:&Peer,action:&Action)->Result<Option<String>,ErrorCode>{
        self.check_task_read(id,peer)?;
        let task=self.task(id,peer)?;
        let (unit,resources)=match action {
            Action::SystemInfo=>(None,ReadResources::default()),
            Action::SystemServiceStatus(args)=>{
                if !task.service_handles.contains(&args.service_id){return Err(ErrorCode::PermissionDenied);}
                let (unit,resource)=self.service_resource(peer,&args.service_id)?;
                (Some(unit),ReadResources(vec![resource]))
            },
            Action::ProcessInspect(args)=>(None,task.process_selection.as_ref().ok_or(ErrorCode::PermissionDenied)?.current(peer,&args.process_id)?),
            _=>return Err(ErrorCode::UnsupportedCapability),
        };
        // Check the original authenticated task grant, not a direct-read grant
        // synthesized from the model's proposal. Resource ownership/expiry is
        // rechecked on every call before entering the real provider.
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)?.check_read(task.grant.as_ref().ok_or(ErrorCode::AuthRequired)?,
            &peer.policy_subject()?,id,action,&resources,aios_policy::boottime_ms()?)?;
        Ok(unit)
    }

    fn check_direct_read(&mut self, peer: &Peer, action: &Action, resources: ReadResources) -> Result<(), ErrorCode> {
        let id = Uuid::new_v4().to_string();
        let scope = aios_policy::Scope { actions: [action.action_id().into()].into(),
            resources: resources.0.iter().cloned().collect(), ..Default::default() };
        let grant = self.read_grant(peer, id.clone(), &serde_json::to_string(&action.arguments_value()).map_err(|_| ErrorCode::InvalidArgument)?, Mode::Ask, scope, 10_000)?;
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)?.check_read(&grant, &peer.policy_subject()?, &id, action, &resources, aios_policy::boottime_ms()?)
    }
    fn prune(&mut self) {
        let time = Instant::now();
        let boottime=aios_policy::boottime_ms().ok();
        for task in self.tasks.values_mut() {
            if !task.terminal() && (task.deadline <= time || boottime.is_none_or(|now|now>=task.boottime_deadline)) {
                if let Some(grant) = &task.grant { grant.revoke(); }
                if let Some(cancel)=&task.native_cancel{cancel.cancel();}
                let _=task.control.compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
                if task.status.state == "queued" { task.finish(Err(if task.control.load(Ordering::Acquire)==1 {ErrorCode::Cancelled}else{ErrorCode::DeadlineExceeded})); }
                else if task.status.state != "cancelling" { task.status.state="cancelling".into(); task.event("deadline_requested"); }
            }
        }
        self.tasks.retain(|_, task| !task.terminal() || task.expires > time);
        self.queue.retain(|id| self.tasks.get(id).is_some_and(|task| !task.terminal()));
        self.handles.retain(|_, handle| handle.expires > time);
        self.ui_candidates.retain(|_, candidate| candidate.expires > time);
        self.processes.prune();
    }
    fn disconnect(&mut self, peer: &Peer) {
        self.processes.disconnect(peer);
        for task in self.tasks.values_mut().filter(|task| task.owner == *peer) {
            task.retained_question.take();
            if task.terminal(){continue;}
            if let Some(grant) = &task.grant { grant.revoke(); }
            if let Some(cancel)=&task.native_cancel{cancel.cancel();}
            let _=task.control.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
            if task.status.state == "queued" { task.finish(Err(ErrorCode::Cancelled)); }
            else if task.status.state != "cancelling" { task.status.state="cancelling".into(); task.event("requester_disconnected"); }
        }
    }
    fn task(&self, id: &str, peer: &Peer) -> Result<&Task, ErrorCode> {
        if id.is_empty() || id.chars().count() > 128 { return Err(ErrorCode::InvalidArgument); }
        let task = self.tasks.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if task.owner != *peer { return Err(ErrorCode::PermissionDenied); }
        Ok(task)
    }
    fn existing_submission(&mut self,peer:&Peer,request:&Submit)->Result<Option<Value>,ErrorCode>{
        self.prune();
        if request.text.trim().is_empty() || request.text.len()>60000 || request.client_nonce.is_empty() || request.client_nonce.len()>128{return Err(ErrorCode::InvalidArgument);}
        let digest:[u8;32]=Sha256::digest(canonical_json(&provider(request)?)?).into();
        for (id,task) in &self.tasks {
            if task.owner==*peer && task.nonce==request.client_nonce {
                if task.digest!=digest{return Err(ErrorCode::Conflict);}return Ok(Some(json!({"request_id":id})));
            }
        }
        Ok(None)
    }
    fn submit_owned(&mut self,peer:&Peer,request:Submit,selection:Option<graphical::Selection>)->Result<Value,ErrorCode>{
        self.submit_scoped(peer,request,selection,None)
    }
    fn submit_scoped(&mut self,peer:&Peer,request:Submit,selection:Option<graphical::Selection>,process_selection:Option<Arc<process_selection::Selection>>)->Result<Value,ErrorCode>{
        self.prune();
        if request.text.trim().is_empty() || request.text.len() > 60000 || request.client_nonce.is_empty() || request.client_nonce.len() > 128 {
            return Err(ErrorCode::InvalidArgument);
        }
        if selection.is_none() {
        if let Some(handle)=&request.selected_session_handle {
            let candidate=self.ui_candidates.get(handle).ok_or(ErrorCode::TargetNotFound)?;
            if candidate.owner!=*peer {return Err(ErrorCode::PermissionDenied);}
            if identity::observe_graphical_session(&candidate.session.id,peer.uid)?!=candidate.session {return Err(ErrorCode::TargetChanged);}
            // Selection is an observation, never a consent receipt.
            return Err(ErrorCode::AuthRequired);
        }
        if request.selected_app_handle.is_some() {
            return Err(ErrorCode::AuthRequired);
        }
        }
        if request.context_handles.len()>8 || request.history_handles.len()>4 || selection.is_some() && (!request.context_handles.is_empty() || request.retain_for_history || !request.history_handles.is_empty()){return Err(ErrorCode::InvalidArgument);}
        let mut history_refs=Vec::new();
        for id in &request.history_handles {
            if !uuid(id) || history_refs.iter().any(|r:&HistoryRef|r.id==*id){return Err(ErrorCode::InvalidArgument);}
            history_refs.push(HistoryRef{id:id.clone(),digest:self.history_digest(id,peer)?});
        }
        let selected_processes=process_selection.as_ref().map(|s|s.ids()).unwrap_or_default();
        let mut resources=process_selection.as_ref().map(|s|s.resources()).unwrap_or_default();
        let mut service_handles=Vec::new();
        for id in &request.context_handles {
            if selected_processes.contains(id){continue;}
            service_handles.push(id.clone());
            if resources.iter().any(|r:&aios_policy::Resource|&r.handle==id){return Err(ErrorCode::InvalidArgument);}
            resources.push(self.service_resource(peer,id)?.1);
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
        let grant = if selection.is_none() && self.inference_configured && matches!(request.mode, Mode::Ask | Mode::Diagnose) {
            let mut actions=std::collections::BTreeSet::from(["system.info".into()]);
            if !service_handles.is_empty(){actions.insert("system.service_status".into());}
            if process_selection.is_some(){actions.insert("process.inspect".into());}
            resources.extend(history_refs.iter().map(|r|aios_policy::Resource{field:"history_id".into(),kind:"session-history".into(),handle:r.id.clone(),identity_sha256:r.digest.clone()}));
            let scope = aios_policy::Scope { actions, resources:resources.into_iter().collect(), ..Default::default() };
            Some(self.read_grant(peer, id.clone(), &request.text, request.mode, scope, 90_000)?)
        } else { None };
        let submitted_at = now();
        let deadline = Instant::now() + Duration::from_secs(90);
        let boottime_deadline=aios_policy::boottime_ms()?.checked_add(90_000).ok_or(ErrorCode::DeadlineExceeded)?;
        let status = TaskStatus { schema_version: 1, operation: "task_status".into(), request_id: id.clone(), mode: request.mode,
            state: "queued".into(), submitted_at, mutation_performed: false, error: None, output: None };
        let mut task = Task { owner: peer.clone(), expires: deadline + Duration::from_secs(300),
            nonce: request.client_nonce, digest, status, deadline, boottime_deadline, control: Arc::new(AtomicU8::new(0)), retained_question:request.retain_for_history.then(||inference::Secret(request.text.clone())),history_refs, text: Some(request.text), events: vec![], grant,service_handles,process_selection,graphical:selection,native_cancel:None,native_receipt:None };
        task.event("accepted");
        if !self.inference_configured { task.finish(Err(ErrorCode::ModelUnavailable)); }
        else if matches!(request.mode, Mode::Act | Mode::Automate) { task.finish(Err(ErrorCode::UnsupportedCapability)); }
        else { self.queue.push_back(id.clone()); }
        self.tasks.insert(id.clone(), task);
        Ok(json!({"request_id":id}))
    }
    pub fn dispatch(&mut self, peer: &Peer, operation: Operation) -> Result<Value, ErrorCode> {
        self.prune();
        match operation {
            Operation::StartProcessTermination{..}|Operation::GetProcessTermination{..}|Operation::CancelProcessTermination{..}|Operation::ForgetProcessTermination{..}=>Err(ErrorCode::AuthRequired),
            Operation::GetCapabilities => Ok(json!({"schema_version":1,"request_id":Uuid::new_v4().to_string(),"operation":"capabilities","actions":["system.info","system.service_status"],
                "read_only":true,"inference_available":self.inference_available,"inference_configured":self.inference_configured,"ui_enabled":false,"ui_session_selection_available":true,"transport":"private-unix",
                "session_history":{"opt_in_required":true,"max_selected":4,"retention_ms":300000,"owner":"authenticated_client","persistent":false},"task_request_max_bytes":MAX_TASK_BYTES,"session_associated":peer.logind_session.is_some()})),
            Operation::SelectUiSession { session_id } => {
                if self.ui_candidates.len()>=64 || self.ui_candidates.values().filter(|c|c.owner==*peer).count()>=8 {return Err(ErrorCode::ResourceExhausted);}
                let session=identity::observe_graphical_session(&session_id,peer.uid)?;
                let id=Uuid::new_v4().to_string();let view=provider(&session)?;
                self.ui_candidates.insert(id.clone(),UiCandidate{owner:peer.clone(),expires:Instant::now()+Duration::from_secs(30),session});
                Ok(json!({"schema_version":1,"operation":"ui_session_candidate","candidate_handle":id,"session":view,
                    "expires_after_ms":30000,"confirmation_required":true,"ui_authorized":false}))
            },
            Operation::ListUiWindows { .. } | Operation::StartUiRead { .. } | Operation::GetUiReadStatus { .. }
                | Operation::TakeUiSnapshot { .. } | Operation::CancelUiRead { .. } | Operation::ForgetUiRead { .. } => {
                // Graphical forwarding requires the actual original Unix FD.
                // A claimed subject or plain State dispatch is insufficient.
                Err(ErrorCode::AuthRequired)
            },
            Operation::GetSystemInfo => {
                self.check_direct_read(peer, &Action::SystemInfo, ReadResources::default())?;
                provider(aios_system::observe_system_info())
            },
            Operation::ResolveService { unit_name } => {
                validate_service_name(&unit_name)?;
                if self.handles.len() >= 256 || self.handles.values().filter(|h| h.owner == *peer).count() >= 32 { return Err(ErrorCode::ResourceExhausted); }
                // Resolve a known loaded unit before issuing a scope-bound handle.
                let id = Uuid::new_v4().to_string();
                let action = parse_tool_call(json!({"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":id}}).to_string().as_bytes())?;
                let resource = aios_policy::Resource { field: "service_id".into(), kind: "scope-owner-expiry".into(),
                    handle: id.clone(), identity_sha256: aios_policy::digest(&unit_name)? };
                self.check_direct_read(peer, &action, ReadResources(vec![resource]))?;
                aios_system::services::read_service_status(&unit_name, &id)?;
                self.handles.insert(id.clone(), Handle { owner: peer.clone(), expires: Instant::now() + Duration::from_secs(30), unit: unit_name });
                Ok(json!({"service_id":id,"expires_after_ms":30000}))
            },
            Operation::Invoke { tool_call } => {
                let action = parse_tool_call(tool_call.get().as_bytes())?;
                // Direct control requests carry no task grant or immutable
                // approved plan. Never turn future provider registration into
                // write authority on this low-level observation route.
                if !aios_protocol::registry::capability(action.action_id())?.read_only {
                    return Err(ErrorCode::AuthRequired);
                }
                match action {
                Action::SystemInfo => {
                    self.check_direct_read(peer, &action, ReadResources::default())?;
                    provider(aios_system::observe_system_info())
                },
                Action::SystemServiceStatus(ref args) => {
                    let handle = self.handles.get(&args.service_id).ok_or(ErrorCode::TargetNotFound)?;
                    if handle.owner != *peer { return Err(ErrorCode::PermissionDenied); }
                    let unit = handle.unit.clone();
                    let resource = aios_policy::Resource { field: "service_id".into(), kind: "scope-owner-expiry".into(),
                        handle: args.service_id.clone(), identity_sha256: aios_policy::digest(&unit)? };
                    self.check_direct_read(peer, &action, ReadResources(vec![resource]))?;
                    provider(service_result(&unit, &args.service_id))
                },
                _ if matches!(action.action_id(), "process.list" | "process.inspect") => self.process_read(peer, &action),
                _ => Err(ErrorCode::UnsupportedCapability),
                }
            },
            Operation::Submit { request } => self.submit_owned(peer,request,None),
            Operation::GetStatus { task_id } => provider(&self.task(&task_id, peer)?.status),
            Operation::GetEvents { task_id, after_sequence, limit } => {
                let task = self.task(&task_id, peer)?;
                if !(1..=100).contains(&limit) { return Err(ErrorCode::InvalidArgument); }
                let events = task.events.iter().filter(|event| event["sequence"].as_u64().unwrap() > after_sequence).take(limit as usize).cloned().collect::<Vec<_>>();
                let last = events.last().and_then(|e| e["sequence"].as_u64()).unwrap_or(after_sequence);
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"task_events","events":events,
                    "complete":task.terminal() && last>=task.events.len() as u64,"next_sequence":last}))
            },
            Operation::Cancel { task_id } => {
                self.task(&task_id, peer)?;
                let task = self.tasks.get_mut(&task_id).expect("authenticated task");
                let terminal = task.terminal();
                if !terminal && task.control.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    if let Some(grant) = &task.grant { grant.revoke(); }
                    if let Some(cancel)=&task.native_cancel{cancel.cancel();}
                    if task.status.state == "queued" { task.finish(Err(ErrorCode::Cancelled)); }
                    else { task.status.state = "cancelling".into(); task.event("cancellation_requested"); }
                }
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"cancellation","cancelled":!terminal,
                    "already_terminal":terminal,"mutation_performed":false,"boundary":"no_side_effects"}))
            },
            Operation::Forget { task_id } => {
                self.task(&task_id, peer)?;
                self.tasks.remove(&task_id);
                Ok(json!({"schema_version":1,"request_id":task_id,"operation":"deletion","deleted":true}))
            },
        }
    }
}

struct ConnectionOwner { state: SharedState, peer: Peer }
impl Drop for ConnectionOwner { fn drop(&mut self) { if let Ok(mut state)=self.state.lock() { state.disconnect(&self.peer); } } }

pub fn serve_connection(mut stream: UnixStream, state: SharedState) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut peer = identity::authenticate(&stream).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "untrusted peer"))?;
    peer.connection_id = Some(Uuid::new_v4().to_string());
    let _owner = ConnectionOwner { state: state.clone(), peer: peer.clone() };
    let mut ui:Option<Arc<graphical::Connection>>=None;
    let mut processes:Option<process_selection::Connection>=None;
    let mut process_control:Option<Arc<process_control::Connection>>=None;
    // A 90-second task must remain inspectable/cancellable on its original
    // authenticated connection; short polling cannot force a reconnect.
    for _ in 0..4096 {
        let Some(frame) = read_frame_with_limit(&mut stream, MAX_TASK_BYTES)? else { return Ok(()); };
        identity::verify(&stream, &peer).map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "peer changed"))?;
        let request: Request = serde_json::from_str(&frame).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid request"))?;
        if !uuid(&request.request_id) { return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid correlation id")); }
        // A bounded, authenticated envelope with a valid correlation ID receives
        // stable errors. Invalid framing/JSON/envelopes still close the stream.
        let outcome = if request.schema_version != 1 {
            Err(ErrorCode::UnsupportedSchema)
        } else {
            match parse_operation(request.operation.get()) {
                Ok(operation) => {
                    match process_bridge::action(&operation){
                        Ok(Some(action))=>process_dispatch(&stream,&peer,&mut processes,&action),
                        Ok(None)=>match operation{
                            operation if process_control::is_operation(&operation)=>{
                                (||->Result<Value,ErrorCode>{
                                    if process_control.is_none(){
                                        if !matches!(operation,Operation::StartProcessTermination{..}){return Err(ErrorCode::TargetNotFound);}
                                        let client=processes.as_ref().ok_or(ErrorCode::AuthRequired)?.clone();
                                        process_control=Some(Arc::new(process_control::Connection::new(peer.clone(),client)));
                                    }
                                    process_control.as_ref().ok_or(ErrorCode::TargetNotFound)?.execute(&state,&peer,operation)
                                })()
                            },
                            Operation::Submit{request} if request.selected_session_handle.is_none() && request.selected_app_handle.is_none() && !request.context_handles.is_empty()=>{
                                (||->Result<Value,ErrorCode>{
                                    if let Some(value)=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.existing_submission(&peer,&request)?{return Ok(value);}
                                    let ids=process_context_ids(&state,&peer,&request)?;
                                    let selected=if ids.is_empty(){None}else{
                                        if processes.is_none(){processes=Some(Arc::new(Mutex::new(process_bridge::Client::connect(Some(&stream),&peer)?)));}
                                        Some(Arc::new(process_selection::Selection::select(&peer,processes.as_ref().ok_or(ErrorCode::TargetChanged)?.clone(),&ids)?))
                                    };
                                    state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.submit_scoped(&peer,request,None,selected)
                                })()
                            },
                            operation=>graphical_dispatch(&stream,&state,&peer,&mut ui,operation),
                        },
                        Err(error)=>Err(error),
                    }
                },
                Err(error) => Err(error),
            }
        };
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

fn process_context_ids(state:&SharedState,peer:&Peer,request:&Submit)->Result<Vec<String>,ErrorCode>{
    if request.context_handles.len()>8 || !matches!(request.mode,Mode::Ask|Mode::Diagnose){return Err(ErrorCode::InvalidArgument);}
    let state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;
    let mut processes=Vec::new();let mut seen=std::collections::BTreeSet::new();
    for id in &request.context_handles{
        if !uuid(id) || !seen.insert(id){return Err(ErrorCode::InvalidArgument);}
        if state.handles.contains_key(id){state.service_resource(peer,id)?;}else{processes.push(id.clone());}
    }
    Ok(processes)
}

fn process_dispatch(stream:&UnixStream,peer:&Peer,client:&mut Option<process_selection::Connection>,action:&Action)->Result<Value,ErrorCode>{
    identity::verify(stream,peer)?;
    if client.is_none(){*client=Some(Arc::new(Mutex::new(process_bridge::Client::connect(Some(stream),peer)?)));}
    let result=client.as_ref().ok_or(ErrorCode::UnsupportedCapability)?.lock().map_err(|_|ErrorCode::ResourceExhausted)?.call(action);
    identity::verify(stream,peer)?;
    if matches!(result,Err(ErrorCode::TargetChanged)){client.take();}
    result
}
fn graphical_dispatch(stream:&UnixStream,state:&SharedState,peer:&Peer,ui:&mut Option<Arc<graphical::Connection>>,operation:Operation)->Result<Value,ErrorCode>{
    let native=match operation {
        Operation::ListUiWindows{session_handle}=>{
            let candidate=selected_ui_session(state,peer,&session_handle)?;
            if ui.is_none(){*ui=Some(Arc::new(graphical::Connection::new(ui_bridge::Client::connect(stream)?,peer.clone())));}
            let result=ui.as_ref().ok_or(ErrorCode::AuthRequired)?.discover(&candidate);
            identity::verify(stream,peer)?;return result;
        },
        Operation::StartUiRead{window_handle,goal,mode}=>json!({"kind":"start_read","window_handle":window_handle,"goal":goal,"mode":mode}),
        Operation::GetUiReadStatus{task_id}=>json!({"kind":"get_read_status","task_id":task_id}),
        Operation::TakeUiSnapshot{task_id}=>json!({"kind":"take_snapshot","task_id":task_id}),
        Operation::CancelUiRead{task_id}=>json!({"kind":"cancel","task_id":task_id}),
        Operation::ForgetUiRead{task_id}=>json!({"kind":"forget","task_id":task_id}),
        Operation::Submit{request} if request.selected_session_handle.is_some() || request.selected_app_handle.is_some()=>{
            if let Some(value)=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.existing_submission(peer,&request)?{return Ok(value);}
            if !request.context_handles.is_empty() || request.retain_for_history || !request.history_handles.is_empty() || request.text.len()>4096{return Err(ErrorCode::InvalidArgument);}
            let session=selected_ui_session(state,peer,request.selected_session_handle.as_deref().ok_or(ErrorCode::AuthRequired)?)?;
            let selection=graphical::Connection::select(ui.as_ref().ok_or(ErrorCode::AuthRequired)?.clone(),peer,&session,
                request.selected_app_handle.as_deref().ok_or(ErrorCode::AuthRequired)?)?;
            identity::verify(stream,peer)?;
            return state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.submit_owned(peer,request,Some(selection));
        },
        operation=>return state.lock().map_err(|_|ErrorCode::ResourceExhausted)?.dispatch(peer,operation),
    };
    identity::verify(stream,peer)?;
    let result=ui.as_ref().ok_or(ErrorCode::AuthRequired)?.client.lock().map_err(|_|ErrorCode::ResourceExhausted)?.call(native);
    identity::verify(stream,peer)?;
    if matches!(result,Err(ErrorCode::TargetChanged|ErrorCode::PermissionDenied)){ui.take();}
    result
}
fn selected_ui_session(state:&SharedState,peer:&Peer,handle:&str)->Result<identity::GraphicalSession,ErrorCode>{
    let candidate={let mut state=state.lock().map_err(|_|ErrorCode::ResourceExhausted)?;state.prune();
        let candidate=state.ui_candidates.get(handle).ok_or(ErrorCode::TargetNotFound)?;
        if candidate.owner!=*peer{return Err(ErrorCode::PermissionDenied);}candidate.session.clone()};
    // Native lookup happens outside the task-state lock.
    if identity::observe_graphical_session(&candidate.id,peer.uid)?!=candidate{return Err(ErrorCode::TargetChanged);}Ok(candidate)
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
        Peer { uid: 1000, pid: 200, start_ticks: 3, boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
            logind_session: None, remote: true, session_type: None, ui_enabled: false, bus_sender: None, bus_id: None, connection_id: Some(Uuid::new_v4().to_string()) }
    }
    fn submit_fixture(mode: Mode, nonce: &str, text: &str) -> Operation {
        Operation::Submit { request: Submit { mode, text: text.into(), client_nonce: nonce.into(),
            context_handles: vec![],retain_for_history:false,history_handles:vec![], selected_app_handle: None, selected_session_handle: None } }
    }
    pub(super) fn history_request(retain:bool,history:Vec<String>)->Submit{
        Submit{mode:Mode::Ask,text:"Original private question".into(),client_nonce:Uuid::new_v4().to_string(),
            context_handles:vec![],retain_for_history:retain,history_handles:history,selected_app_handle:None,selected_session_handle:None}
    }
    fn retained_fixture(state:&mut State,peer:&Peer,retain:bool)->String{
        let id=state.submit_owned(peer,history_request(retain,vec![]),None).unwrap()["request_id"].as_str().unwrap().to_string();
        state.tasks.get_mut(&id).unwrap().finish(Ok(json!({"response":{"kind":"answer","text":"Old answer: not current evidence","evidence_ids":["old-citation"]},"mutation_performed":false})));
        id
    }
    #[test]
    fn history_requires_two_explicit_choices_and_same_entire_peer(){
        let peer=peer_fixture();let mut state=State::with_inference();
        let omitted=retained_fixture(&mut state,&peer,false);
        assert!(state.tasks[&omitted].retained_question.is_none());
        assert_eq!(state.history_digest(&omitted,&peer),Err(ErrorCode::AuthRequired));
        let retained=retained_fixture(&mut state,&peer,true);
        for changed in ["uid","pid","start","boot","connection","bus","session"]{
            let mut foreign=peer.clone();match changed{
                "uid"=>foreign.uid+=1,"pid"=>foreign.pid+=1,"start"=>foreign.start_ticks+=1,
                "boot"=>foreign.boot_id=Uuid::new_v4().to_string(),"connection"=>foreign.connection_id=Some(Uuid::new_v4().to_string()),
                "bus"=>foreign.bus_sender=Some(":1.9".into()),_=>foreign.logind_session=Some("other".into())}
            assert_eq!(state.history_digest(&retained,&foreign),Err(ErrorCode::PermissionDenied));
        }
        let follow=state.submit_owned(&peer,history_request(false,vec![retained.clone()]),None).unwrap()["request_id"].as_str().unwrap().to_string();
        let history=state.task_history(&follow,&peer).unwrap();assert_eq!(history.len(),1);
        assert_eq!(history[0].question.0,"Original private question");
        assert_eq!(state.task_tools(&follow,&peer).unwrap(),vec![aios_protocol::inference::ReadTool::SystemInfo]);
        assert!(state.tasks[&follow].retained_question.is_none());
        state.tasks.get_mut(&retained).unwrap().status.output.as_mut().unwrap()["response"]["text"]=json!("changed");
        assert_eq!(state.check_task_read(&follow,&peer),Err(ErrorCode::TargetChanged));
    }
    #[test]
    fn forget_expiry_and_disconnect_revoke_selected_history(){
        for operation in ["forget","expiry","disconnect"]{
            let peer=peer_fixture();let mut state=State::with_inference();let retained=retained_fixture(&mut state,&peer,true);
            let follow=state.submit_owned(&peer,history_request(false,vec![retained.clone()]),None).unwrap()["request_id"].as_str().unwrap().to_string();
            assert!(state.check_task_read(&follow,&peer).is_ok());
            match operation{
                "forget"=>{state.dispatch(&peer,Operation::Forget{task_id:retained.clone()}).unwrap();},
                "expiry"=>state.tasks.get_mut(&retained).unwrap().expires=Instant::now()-Duration::from_millis(1),
                _=>state.disconnect(&peer)}
            assert!(state.check_task_read(&follow,&peer).is_err());
            if operation=="disconnect"{assert!(state.tasks[&retained].retained_question.is_none());}
        }
    }
    #[test]
    fn history_rejects_uncompleted_duplicate_overfull_and_cancelled_sources(){
        let peer=peer_fixture();let mut state=State::with_inference();let source=state.submit_owned(&peer,history_request(true,vec![]),None).unwrap()["request_id"].as_str().unwrap().to_string();
        assert_eq!(state.history_digest(&source,&peer),Err(ErrorCode::AuthRequired));
        state.tasks.get_mut(&source).unwrap().finish(Ok(json!({"response":{"kind":"clarification","question":"Which one?"}})));
        assert!(state.history_digest(&source,&peer).is_ok());
        assert_eq!(state.submit_owned(&peer,history_request(false,vec![source.clone(),source.clone()]),None),Err(ErrorCode::InvalidArgument));
        assert_eq!(state.submit_owned(&peer,history_request(false,vec![source.clone();5]),None),Err(ErrorCode::InvalidArgument));
        assert_eq!(state.submit_owned(&peer,history_request(false,vec!["malformed".into()]),None),Err(ErrorCode::InvalidArgument));
        let failed=state.submit_owned(&peer,history_request(true,vec![]),None).unwrap()["request_id"].as_str().unwrap().to_string();
        state.tasks.get_mut(&failed).unwrap().finish(Err(ErrorCode::Cancelled));
        assert!(state.tasks[&failed].retained_question.is_none());
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
    #[test]
    fn task_service_scope_cannot_be_expanded_by_model_or_foreign_handles_fixture(){
        let peer=peer_fixture();let mut foreign=peer.clone();foreign.connection_id=Some(Uuid::new_v4().to_string());
        let mut state=State::with_inference();
        let own=Uuid::new_v4().to_string();let other=Uuid::new_v4().to_string();
        state.handles.insert(own.clone(),Handle{owner:peer.clone(),expires:Instant::now()+Duration::from_secs(30),unit:"sshd.service".into()});
        state.handles.insert(other.clone(),Handle{owner:foreign,expires:Instant::now()+Duration::from_secs(30),unit:"must-not-be-accessed.service".into()});
        let request=|handles:Vec<String>|Submit{mode:Mode::Ask,text:"Inspect selected service".into(),client_nonce:Uuid::new_v4().to_string(),context_handles:handles,retain_for_history:false,history_handles:vec![],selected_app_handle:None,selected_session_handle:None};
        assert_eq!(state.submit_owned(&peer,request(vec![other.clone()]),None).unwrap_err(),ErrorCode::PermissionDenied);
        let id=state.submit_owned(&peer,request(vec![own.clone()]),None).unwrap()["request_id"].as_str().unwrap().to_string();
        let action=|handle:&str|parse_tool_call(json!({"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":handle}}).to_string().as_bytes()).unwrap();
        assert_eq!(state.task_read_target(&id,&peer,&action(&own)).unwrap(),Some("sshd.service".into()));
        assert_eq!(state.task_read_target(&id,&peer,&action(&other)).unwrap_err(),ErrorCode::PermissionDenied);
        let unselected=Uuid::new_v4().to_string();state.handles.insert(unselected.clone(),Handle{owner:peer.clone(),expires:Instant::now()+Duration::from_secs(30),unit:"must-not-be-accessed.service".into()});
        assert_eq!(state.task_read_target(&id,&peer,&action(&unselected)).unwrap_err(),ErrorCode::PermissionDenied);
        state.handles.get_mut(&own).unwrap().expires=Instant::now();
        assert_eq!(state.task_read_target(&id,&peer,&action(&own)).unwrap_err(),ErrorCode::ApprovalExpired);
        state.dispatch(&peer,Operation::Cancel{task_id:id.clone()}).unwrap();
        assert_eq!(state.task_read_target(&id,&peer,&Action::SystemInfo).unwrap_err(),ErrorCode::ApprovalExpired);
    }
    /// Kernel sockets and task controls are real; desktop work/subjects are
    /// fixtures. This proves lock independence, not native permission issuance.
    #[test]
    fn public_stop_forget_and_disconnect_revoke_only_owned_native_channel_fixture() {
        use std::io::Read;
        let peer=peer_fixture();let mut foreign=peer.clone();foreign.connection_id=Some(Uuid::new_v4().to_string());
        for operation in ["cancel","forget","disconnect","deadline"] {
            let mut state=State::with_inference();
            let id=state.dispatch(&peer,submit_fixture(Mode::Ask,"native","fixture question")).unwrap()["request_id"].as_str().unwrap().to_owned();
            let other=state.dispatch(&peer,submit_fixture(Mode::Ask,"other","other question")).unwrap()["request_id"].as_str().unwrap().to_owned();
            let (cancel,mut receiver)=ui_bridge::Cancellation::pair().unwrap();
            let (other_cancel,mut other_receiver)=ui_bridge::Cancellation::pair().unwrap();
            receiver.set_read_timeout(Some(Duration::from_millis(20))).unwrap();other_receiver.set_read_timeout(Some(Duration::from_millis(20))).unwrap();
            let task=state.tasks.get_mut(&id).unwrap();task.native_cancel=Some(cancel);task.status.state="inspecting".into();
            state.tasks.get_mut(&other).unwrap().native_cancel=Some(other_cancel);
            assert_eq!(state.dispatch(&foreign,Operation::Cancel{task_id:id.clone()}).unwrap_err(),ErrorCode::PermissionDenied);
            let mut byte=[0];assert!(receiver.read(&mut byte).is_err());
            // The desktop query's mutex stays held throughout public control.
            let query=Mutex::new(());let _busy=query.lock().unwrap();let start=Instant::now();
            match operation {
                "cancel"=>{state.dispatch(&peer,Operation::Cancel{task_id:id.clone()}).unwrap();},
                "forget"=>{state.dispatch(&peer,Operation::Forget{task_id:id.clone()}).unwrap();},
                "disconnect"=>state.disconnect(&peer),
                "deadline"=>{state.tasks.get_mut(&id).unwrap().boottime_deadline=0;state.prune();},
                _=>unreachable!(),
            }
            assert!(start.elapsed()<Duration::from_millis(100));assert_eq!(receiver.read(&mut byte).unwrap(),0);
            if operation!="disconnect"{assert!(other_receiver.read(&mut byte).is_err(),"unrelated task was revoked");}
            else{assert_eq!(other_receiver.read(&mut byte).unwrap(),0);}
            if operation=="forget"{assert_eq!(state.task(&id,&peer).err(),Some(ErrorCode::TargetNotFound));}
            else{assert_ne!(state.tasks[&id].control.load(Ordering::Acquire),0);}
        }
    }
}
