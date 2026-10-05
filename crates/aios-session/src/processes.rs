//! Private own-user handles. This module cannot authorize or send signals.
use crate::{State, ReadResources, Mode, identity::{self, Peer}};
use aios_protocol::contracts::{Action, ErrorCode};
use aios_system::processes::{OwnProcess, Observation};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

type Result<T> = std::result::Result<T, ErrorCode>;
const LIFETIME_MS: u64 = 30_000;
struct Process { owner: Peer, expires: u64, native: OwnProcess, digest: String }
struct Cursor { owner: Peer, expires: u64, query: String, ids: Vec<String>, offset: usize, access_denied: bool }
#[derive(Default)]
pub(super) struct Handles { processes: HashMap<String, Process>, cursors: HashMap<String, Cursor> }
fn expiry() -> Result<u64> { aios_policy::boottime_ms()?.checked_add(LIFETIME_MS).ok_or(ErrorCode::ResourceExhausted) }
fn current(owner: &Peer, peer: &Peer, expires: u64) -> Result<()> {
    if owner != peer { return Err(ErrorCode::PermissionDenied); }
    if aios_policy::boottime_ms()? >= expires { return Err(ErrorCode::TargetNotFound); }
    Ok(())
}
fn resource(field: &str, kind: &str, id: &str, digest: &str) -> aios_policy::Resource {
    aios_policy::Resource { field: field.into(), kind: kind.into(), handle: id.into(), identity_sha256: digest.into() }
}
impl Handles {
    pub(super) fn prune(&mut self) {
        let Ok(now) = aios_policy::boottime_ms() else { self.processes.clear(); self.cursors.clear(); return; };
        self.processes.retain(|_, p| p.expires > now);
        self.cursors.retain(|_, c| c.expires > now);
    }
    pub(super) fn disconnect(&mut self, peer: &Peer) {
        self.processes.retain(|_, p| p.owner != *peer);
        self.cursors.retain(|_, c| c.owner != *peer);
    }
    fn process(&self, peer: &Peer, id: &str) -> Result<&Process> {
        let p = self.processes.get(id).ok_or(ErrorCode::TargetNotFound)?;
        current(&p.owner, peer, p.expires)?;
        Ok(p)
    }
    fn cursor(&self, peer: &Peer, id: &str, query: &str) -> Result<&Cursor> {
        let c = self.cursors.get(id).ok_or(ErrorCode::TargetNotFound)?;
        current(&c.owner, peer, c.expires)?;
        if c.query != query { return Err(ErrorCode::InvalidArgument); }
        Ok(c)
    }
    fn quota(&self, peer: &Peer, count: usize) -> Result<()> {
        if self.processes.len() + count > 1024 || self.processes.values().filter(|p| p.owner == *peer).count() + count > 256 {
            return Err(ErrorCode::ResourceExhausted);
        }
        Ok(())
    }
    fn snapshot(&mut self, peer: &Peer) -> Result<(Vec<String>, bool)> {
        let native = aios_system::processes::inventory()?;
        self.quota(peer, native.processes.len())?;
        let access_denied = native.access_denied;
        let expires = expiry()?;
        let mut pending = Vec::new();
        for process in native.processes {
            let observation = process.inspect()?;
            if observation.identity.uid != peer.uid || observation.identity.boot_id != peer.boot_id { return Err(ErrorCode::TargetChanged); }
            let digest = aios_policy::digest(&observation.identity)?;
            pending.push((Uuid::new_v4().to_string(), Process { owner: peer.clone(), expires, native: process, digest }));
        }
        identity::verify_peer(peer)?;
        let ids = pending.iter().map(|(id, _)| id.clone()).collect();
        self.processes.extend(pending);
        Ok((ids, access_denied))
    }
    fn observe(&self, peer: &Peer, id: &str) -> Result<Observation> {
        let p = self.process(peer, id)?;
        let value = p.native.inspect()?;
        if aios_policy::digest(&value.identity)? != p.digest { return Err(ErrorCode::TargetChanged); }
        current(&p.owner, peer, p.expires)?;
        Ok(value)
    }
    fn page(&mut self, peer: &Peer, ids: Vec<String>, offset: usize, limit: usize, query: String, expires: u64, access_denied: bool) -> Result<Value> {
        let end = (offset + limit).min(ids.len());
        if offset > end { return Err(ErrorCode::TargetChanged); }
        let mut rows = Vec::new();
        for id in &ids[offset..end] {
            let observation = self.observe(peer, id)?;
            rows.push(json!({"process_id": id, "pid": observation.identity.pid,
                "start_time_ticks": observation.identity.start_time_ticks, "app_id": null}));
        }
        let next = if end < ids.len() {
            if self.cursors.len() >= 64 || self.cursors.values().filter(|c| c.owner == *peer).count() >= 8 { return Err(ErrorCode::ResourceExhausted); }
            let id = Uuid::new_v4().to_string();
            self.cursors.insert(id.clone(), Cursor { owner: peer.clone(), expires, query, ids, offset: end, access_denied });
            Some(id)
        } else { None };
        Ok(result(json!({"processes": rows}), next, access_denied))
    }
}
fn result(data: Value, next_cursor: Option<String>, access_denied: bool) -> Value {
    let complete = next_cursor.is_none() && !access_denied;
    let error = if access_denied {
        json!({"code":"PERMISSION_DENIED","message":"Some own-user processes cannot be inspected; this inventory is incomplete","retryable":false})
    } else if complete { Value::Null } else {
        json!({"code":"PARTIAL_RESULT","message":"More process records remain in this snapshot; follow the bound cursor","retryable":false})
    };
    json!({"schema_version":1,"status":if complete {"ok"} else {"partial"},"observed_at":crate::now(),
        "source":{"provider":"linux-own-user-processes","provider_version":"1"},
        "evidence_ids":[],"complete":complete,"next_cursor":next_cursor,"data":data,"error":error})
}
impl State {
    pub(super) fn process_read(&mut self, peer: &Peer, action: &Action) -> Result<Value> {
        identity::verify_peer(peer)?;
        let args = action.arguments_value();
        let query = aios_policy::digest(&json!({"limit":args["limit"].as_u64().unwrap_or(100),"app_id":args["app_id"]}))?;
        // Application ownership associations require the application provider.
        // Do not silently ignore a supplied application filter.
        if args.get("app_id").is_some() { return Err(ErrorCode::UnsupportedCapability); }
        let cursor = args["cursor"].as_str();
        let process = args["process_id"].as_str();
        let resources = if let Some(id) = process {
            let p = self.processes.process(peer, id)?;
            ReadResources(vec![resource("process_id", "scope-owner-expiry", id, &p.digest)])
        } else if let Some(id) = cursor {
            let c = self.processes.cursor(peer, id, &query)?;
            ReadResources(vec![resource("cursor", "query-bound-cursor", id, &aios_policy::digest(&(id, &c.query, &c.ids, c.offset, c.access_denied))?)])
        } else { ReadResources::default() };
        let request = Uuid::new_v4().to_string();
        let scope = aios_policy::Scope { actions:[action.action_id().into()].into(), resources:resources.0.iter().cloned().collect(), ..Default::default() };
        let grant = self.read_grant(peer, request.clone(), &serde_json::to_string(&args).map_err(|_|ErrorCode::InvalidArgument)?, Mode::Ask, scope, 10_000)?;
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)?.check_read(&grant, &peer.policy_subject()?, &request, action, &resources, aios_policy::boottime_ms()?)?;
        let output = match action.action_id() {
            "process.inspect" => {
                let id = process.ok_or(ErrorCode::InvalidArgument)?;
                let value = self.processes.observe(peer, id)?;
                result(json!({"process_id":id,"pid":value.identity.pid,"start_time_ticks":value.identity.start_time_ticks,
                    "executable_identity":value.identity.executable_identity,"metrics":value.metrics}), None, false)
            },
            "process.list" => {
                let limit = args["limit"].as_u64().unwrap_or(100) as usize;
                let (ids, offset, expires, access_denied) = if let Some(id) = cursor {
                    let c = self.processes.cursor(peer, id, &query)?;
                    (c.ids.clone(), c.offset, c.expires, c.access_denied)
                } else {
                    let (ids, denied) = self.processes.snapshot(peer)?;
                    (ids, 0, expiry()?, denied)
                };
                self.processes.page(peer, ids, offset, limit, query, expires, access_denied)?
            },
            _ => return Err(ErrorCode::UnsupportedCapability),
        };
        identity::verify_peer(peer)?;
        if let Some(id) = process { self.processes.process(peer, id)?; }
        if let Some(id) = cursor {
            // Recheck the original cursor lifetime after potentially blocking
            // caller reauthentication; a new page never extends its snapshot.
            let c = self.processes.cursors.get(id).ok_or(ErrorCode::TargetNotFound)?;
            current(&c.owner, peer, c.expires)?;
        }
        self.policy.as_ref().ok_or(ErrorCode::PolicyChanged)?.check_read(&grant, &peer.policy_subject()?, &request, action, &resources, aios_policy::boottime_ms()?)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aios_protocol::contracts::parse_tool_call;

    fn native_peer() -> Peer {
        let mut peer = identity::authenticate_process(nix::unistd::geteuid().as_raw(), std::process::id()).unwrap();
        peer.connection_id = Some(Uuid::new_v4().to_string());
        peer
    }
    fn action(id: &str, args: Value) -> Action {
        parse_tool_call(json!({"kind":"tool_call","action_id":id,"arguments":args}).to_string().as_bytes()).unwrap()
    }
    fn retained(state: &mut State, peer: &Peer, pid: u32) -> String {
        let native = OwnProcess::open(pid).unwrap();
        let digest = aios_policy::digest(&native.inspect().unwrap().identity).unwrap();
        let id = Uuid::new_v4().to_string();
        state.processes.processes.insert(id.clone(), Process { owner:peer.clone(), expires:expiry().unwrap(), native, digest });
        id
    }
    #[test]
    fn native_process_handle_inspect_refuses_reconnect_expiry_disconnect_and_exit() {
        let peer = native_peer();
        let mut state = State::default();
        let id = retained(&mut state, &peer, std::process::id());
        let inspect = action("process.inspect", json!({"process_id":id}));
        let result = state.process_read(&peer, &inspect).unwrap();
        aios_protocol::validation::validate_result("process.inspect", result.to_string().as_bytes()).unwrap();
        assert_eq!(result["data"]["pid"], std::process::id());
        assert!(!result.to_string().contains("cmdline"));
        let mut reconnect = peer.clone(); reconnect.connection_id = Some(Uuid::new_v4().to_string());
        assert_eq!(state.process_read(&reconnect, &inspect), Err(ErrorCode::PermissionDenied));
        state.processes.processes.get_mut(&id).unwrap().expires = 0;
        assert_eq!(state.process_read(&peer, &inspect), Err(ErrorCode::TargetNotFound));
        state.processes.processes.get_mut(&id).unwrap().expires = expiry().unwrap();
        state.disconnect(&peer);
        assert_eq!(state.process_read(&peer, &inspect), Err(ErrorCode::TargetNotFound));
        let mut child = std::process::Command::new("/run/current-system/sw/bin/sleep").arg("1").spawn().unwrap();
        let child_id = retained(&mut state, &peer, child.id());
        child.wait().unwrap();
        assert_eq!(state.process_read(&peer, &action("process.inspect", json!({"process_id":child_id}))), Err(ErrorCode::TargetNotFound));
        let raw = serde_json::value::RawValue::from_string(json!({"kind":"tool_call","action_id":"process.terminate","arguments":{"process_id":child_id}}).to_string()).unwrap();
        assert_eq!(state.dispatch(&peer, crate::Operation::Invoke { tool_call:raw }), Err(ErrorCode::AuthRequired));
    }
    #[test]
    fn native_process_pages_bind_full_peer_query_snapshot_and_expiry() {
        let peer = native_peer();
        let mut state = State::default();
        // Two real observations of the same process make a deterministic page
        // corpus without fabricating native identity or depending on churn.
        let ids = vec![retained(&mut state, &peer, std::process::id()), retained(&mut state, &peer, std::process::id())];
        let query = aios_policy::digest(&json!({"limit":1,"app_id":null})).unwrap();
        let first = state.processes.page(&peer, ids, 0, 1, query, expiry().unwrap(), false).unwrap();
        aios_protocol::validation::validate_result("process.list", first.to_string().as_bytes()).unwrap();
        assert_eq!(first["complete"], false);
        let cursor = first["next_cursor"].as_str().unwrap();
        let continuation = action("process.list", json!({"limit":1,"cursor":cursor}));
        let second = state.process_read(&peer, &continuation).unwrap();
        aios_protocol::validation::validate_result("process.list", second.to_string().as_bytes()).unwrap();
        assert_eq!(second["complete"], true);
        assert_ne!(first["data"]["processes"][0]["process_id"], second["data"]["processes"][0]["process_id"]);
        assert_eq!(state.process_read(&peer, &action("process.list",json!({"limit":2,"cursor":cursor}))), Err(ErrorCode::InvalidArgument));
        let mut reconnect = peer.clone(); reconnect.connection_id = Some(Uuid::new_v4().to_string());
        assert_eq!(state.process_read(&reconnect, &continuation), Err(ErrorCode::PermissionDenied));
        state.processes.cursors.get_mut(cursor).unwrap().expires = 0;
        assert_eq!(state.process_read(&peer, &continuation), Err(ErrorCode::TargetNotFound));
        assert_eq!(state.process_read(&peer, &action("process.list",json!({"app_id":"unqualified"}))), Err(ErrorCode::UnsupportedCapability));
        let real = aios_system::processes::inventory().unwrap();
        assert!(real.processes.iter().any(|p| p.inspect().is_ok_and(|v|v.identity.pid == std::process::id())));
        let partial = result(json!({"processes":[]}), None, true);
        aios_protocol::validation::validate_result("process.list",partial.to_string().as_bytes()).unwrap();
        assert_eq!(partial["status"],"partial");
        assert_eq!(partial["complete"],false);
        assert_eq!(partial["error"]["code"],"PERMISSION_DENIED");
        assert!(real.processes.iter().all(|p|p.inspect().map_or(true, |v|v.identity.uid == peer.uid)));
    }
}
