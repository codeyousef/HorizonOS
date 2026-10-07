//! Fixed authenticated System1/Packages1 surfaces. Observations grant no effects.
use super::{Executor, Operation, Result};
use crate::{approval::boottime_ms, caller::{CallerIdentity, VerifiedCaller}, candidate::InstalledTemplate};
use aios_protocol::{MAX_TASK_BYTES, contracts::{Action, ErrorCode}};
use aios_state::{Catalog, CatalogEntry};
use serde::Deserialize;
use serde_json::{Value, json, value::RawValue};
use std::{collections::BTreeMap, io::{Read, Write}, os::unix::net::UnixStream, time::Duration};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zbus::message::Header;

pub(super) const NAME: &str = "org.aios.System1";
pub(super) const SYSTEM_PATH: &str = "/org/aios/System1";
pub(super) const PACKAGES_PATH: &str = "/org/aios/Packages1";

#[derive(Clone, Copy)]
pub(super) enum Scope { System, Packages }
impl Scope {
    fn interface(self) -> &'static str {
        match self { Self::System => NAME, Self::Packages => "org.aios.Packages1" }
    }
    fn actions(self) -> &'static [&'static str] {
        match self {
            Self::System => &["system.info", "system.services", "system.service_status", "system.service_restart",
                "system.hardware", "storage.status", "network.status", "network.set_wifi_enabled",
                "bluetooth.status", "bluetooth.set_enabled", "system.boots", "system.boot_diagnostics", "system.logs"],
            Self::Packages => &["packages.search", "packages.info", "packages.installed", "packages.install",
                "packages.remove", "packages.upgrade_plan"],
        }
    }
}
fn available(id: &str) -> bool {
    matches!(id, "system.info" | "system.hardware" | "storage.status" | "network.status" |
        "bluetooth.status" | "packages.info" | "packages.search")
}
fn caller_scope(caller: &VerifiedCaller) -> String {
    let identity = caller.identity();
    format!("{}:{}:{}:{}:{}:{}", identity.uid, identity.pid, identity.start_ticks,
        identity.boot_id, identity.bus_id, identity.sender)
}
pub(super) fn capabilities(scope: Scope) -> Result<Value> {
    let contracts = scope.actions().iter().map(|id| {
        let contract = aios_protocol::registry::capability(id)?;
        Ok(json!({"action_id":id,"input_schema":contract.input_schema,"output_schema":contract.output_schema,
            "availability":if available(id) {"available"} else {"unavailable"}}))
    }).collect::<std::result::Result<Vec<_>, ErrorCode>>()?;
    Ok(super::envelope("capabilities", json!({"interface":scope.interface(),
        "native_caller_verified":true,"available_actions":scope.actions().iter().filter(|id|available(id)).collect::<Vec<_>>(),
        "contracts":contracts,"max_request_bytes":MAX_TASK_BYTES,"private_results":true,
        "max_live_cursors_per_uid":256,"cursor_ttl_ms":30000})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request { schema_version: u32, request_id: String, operation: Invoke }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invoke { kind: InvokeKind, tool_call: Box<RawValue> }
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum InvokeKind { Invoke }
fn parse(request: &str, expected: &str, scope: Scope) -> Result<Action> {
    if request.len() > MAX_TASK_BYTES { return Err(ErrorCode::ResourceExhausted.into()); }
    let value = aios_protocol::validation::strict_json(request.as_bytes())?;
    let request: Request = serde_json::from_str(request).map_err(|_| ErrorCode::InvalidArgument)?;
    if request.schema_version != 1 { return Err(ErrorCode::UnsupportedSchema.into()); }
    if !crate::uuid(&request.request_id) || !scope.actions().contains(&expected) {
        return Err(ErrorCode::InvalidArgument.into());
    }
    aios_protocol::validation::validate(include_str!("../../../../schemas/api/session-control-request.json"), &value)?;
    let Invoke { kind:InvokeKind::Invoke, tool_call } = request.operation;
    let action = aios_protocol::contracts::parse_tool_call(tool_call.get().as_bytes())?;
    if action.action_id() != expected { return Err(ErrorCode::InvalidArgument.into()); }
    Ok(action)
}

struct Cursor { owner: CallerIdentity, query: String, limit: usize, revision: String, offset: usize, expires_ms: u64 }
#[derive(Default)]
pub(super) struct ReadState { pub(super) journal:super::journal::JournalState, cursors: BTreeMap<String, Cursor> }
impl ReadState {
    pub(super) fn cleanup(&mut self) -> crate::Result<()> {
        let now = boottime_ms()?;
        self.cursors.retain(|_, c| c.expires_ms > now);
        Ok(())
    }
    fn offset(&self, reference: Option<&str>, caller: &CallerIdentity, query: &str, limit: usize, revision: &str) -> Result<usize> {
        let Some(reference) = reference else { return Ok(0) };
        let cursor = self.cursors.get(reference).ok_or(ErrorCode::TargetNotFound)?;
        if &cursor.owner != caller { return Err(ErrorCode::PermissionDenied.into()); }
        if cursor.expires_ms <= boottime_ms()? { return Err(ErrorCode::StaleEvidence.into()); }
        if cursor.query != query || cursor.limit != limit || cursor.revision != revision {
            return Err(ErrorCode::StaleEvidence.into());
        }
        Ok(cursor.offset)
    }
    fn issue(&mut self, owner: CallerIdentity, query: String, limit: usize, revision: String, offset: usize) -> Result<String> {
        self.cleanup()?;
        // Re-reading a page returns its existing next handle. A catalogue has
        // at most 256 entries, so limit=1 can traverse the whole reviewed set.
        if let Some((id,_)) = self.cursors.iter().find(|(_,c)| c.owner==owner && c.query==query &&
            c.limit==limit && c.revision==revision && c.offset==offset) { return Ok(id.clone()); }
        if self.cursors.len() >= 4096 || self.cursors.values().filter(|c|c.owner.uid==owner.uid).count() >= 256 {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.cursors.insert(id.clone(), Cursor { owner, query, limit, revision, offset,
            expires_ms:boottime_ms()?.checked_add(30000).ok_or(ErrorCode::ResourceExhausted)? });
        Ok(id)
    }
    pub(super) fn dispatch(&mut self, caller: &VerifiedCaller, scope: Scope, expected: &str, request: &str) -> Result<Value> {
        self.cleanup()?;
        let action = parse(request, expected, scope)?;
        if !aios_protocol::registry::capability(expected)?.read_only { return Err(ErrorCode::AuthRequired.into()); }
        let value = match action {
            Action::SystemLogs(_) => self.journal.logs(caller,&action)?,
            Action::SystemInfo => serde_json::to_value(aios_system::observe_system_info_native()).map_err(|_|ErrorCode::InvalidArgument)?,
            Action::SystemHardware(args) => {
                let arguments = serde_json::to_value(args).map_err(|_| ErrorCode::InvalidArgument)?;
                let class = arguments["device_class"].as_str().unwrap_or("all");
                let scope = caller_scope(caller);
                serde_json::to_value(aios_system::hardware::observe(class, scope.as_bytes()))
                    .map_err(|_| ErrorCode::InvalidArgument)?
            },
            Action::StorageStatus(_) => {
                let scope = caller_scope(caller);
                serde_json::to_value(aios_system::storage::observe(caller.identity().uid, scope.as_bytes()))
                    .map_err(|_| ErrorCode::InvalidArgument)?
            },
            Action::NetworkStatus(args) => {
                let arguments = serde_json::to_value(args).map_err(|_| ErrorCode::InvalidArgument)?;
                let scope = caller_scope(caller);
                serde_json::to_value(aios_system::network::observe(scope.as_bytes(), arguments["interface_id"].as_str()))
                    .map_err(|_| ErrorCode::InvalidArgument)?
            },
            Action::BluetoothStatus(args) => {
                let arguments = serde_json::to_value(args).map_err(|_| ErrorCode::InvalidArgument)?;
                let scope = caller_scope(caller);
                serde_json::to_value(aios_system::bluetooth::observe(scope.as_bytes(), arguments["adapter_id"].as_str()))
                    .map_err(|_| ErrorCode::InvalidArgument)?
            },
            Action::PackagesInfo(args) => {
                let template = InstalledTemplate::from_installed()?;
                let catalog = template.catalog();
                let entry = catalog.entry(&args.package_id).map_err(|_|ErrorCode::TargetNotFound)?;
                let value = package_result(package(entry, catalog), None);
                template.recheck()?;
                value
            },
            Action::PackagesSearch(args) => {
                let arguments = serde_json::to_value(args).map_err(|_|ErrorCode::InvalidArgument)?;
                let query = arguments["query"].as_str().ok_or(ErrorCode::InvalidArgument)?;
                if query.trim().is_empty() { return Err(ErrorCode::InvalidArgument.into()); }
                let limit = arguments["limit"].as_u64().unwrap_or(20) as usize;
                let template = InstalledTemplate::from_installed()?;
                let catalog = template.catalog();
                let offset = self.offset(arguments["cursor"].as_str(), caller.identity(), query, limit, catalog.revision())?;
                let entries = search(catalog, query);
                if offset > entries.len() { return Err(ErrorCode::StaleEvidence.into()); }
                let end = (offset + limit).min(entries.len());
                let next = if end < entries.len() {
                    Some(self.issue(caller.identity().clone(), query.into(), limit, catalog.revision().into(), end)?)
                } else { None };
                let value = package_result(json!({"matches":entries[offset..end]}), next);
                template.recheck()?;
                value
            },
            _ => return Err(ErrorCode::UnsupportedCapability.into()),
        };
        // Provider output is constrained by the same normative action contract
        // as its input. Data never turns a read request into authorization.
        let bytes = serde_json::to_vec(&value).map_err(|_|ErrorCode::InvalidArgument)?;
        aios_protocol::validation::validate_result(expected, &bytes)?;
        Ok(value)
    }
}
fn package(entry: &CatalogEntry, catalog: &Catalog) -> Value {
    json!({"package_id":entry.id,"name":entry.display_name,"version":entry.version,"licenses":entry.licenses,
        "unfree":entry.unfree,"capabilities":[entry.capability],"catalog_revision":catalog.revision()})
}
fn search(catalog: &Catalog, query: &str) -> Vec<Value> {
    let query = query.to_lowercase();
    catalog.content().packages.iter().filter(|p| p.id.to_lowercase().contains(&query) ||
        p.display_name.to_lowercase().contains(&query) || p.binaries.iter().any(|b|b.to_lowercase().contains(&query)))
        .map(|entry|package(entry, catalog)).collect()
}
fn package_result(data: Value, cursor: Option<String>) -> Value {
    json!({"schema_version":1,"status":"ok","observed_at":OffsetDateTime::now_utc().format(&Rfc3339).expect("valid UTC timestamp"),
        "source":{"provider":"aios-installed-catalog","provider_version":env!("CARGO_PKG_VERSION")},
        "evidence_ids":[],"complete":true,"next_cursor":cursor,"data":data,"error":null})
}
pub(super) fn graph_status() -> Result<Value> {
    let mut stream = UnixStream::connect("/run/aios-state/owner.sock")
        .map_err(|_| ErrorCode::UnsupportedCapability)?;
    stream.set_read_timeout(Some(Duration::from_millis(500))).map_err(|_| ErrorCode::PartialResult)?;
    stream.set_write_timeout(Some(Duration::from_millis(500))).map_err(|_| ErrorCode::PartialResult)?;
    let request = br#"{"kind":"status"}"#;
    stream.write_all(&(request.len() as u32).to_be_bytes()).map_err(|_| ErrorCode::PartialResult)?;
    stream.write_all(request).map_err(|_| ErrorCode::PartialResult)?;
    let mut length = [0; 4];
    stream.read_exact(&mut length).map_err(|_| ErrorCode::PartialResult)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > aios_protocol::MAX_FRAME_BYTES {
        return Err(ErrorCode::ResourceExhausted.into());
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).map_err(|_| ErrorCode::PartialResult)?;
    let reply:Value = serde_json::from_slice(&bytes).map_err(|_| ErrorCode::InvalidArgument)?;
    let value = reply.get("data").filter(|_| reply["ok"] == true).cloned().ok_or(ErrorCode::PartialResult)?;
    if value["schema_version"] != 1 || value["scope"] != "system" || value["model_invoked"] != false
        || value["execution_authority"] != false || !value["boot"].is_string() {
        return Err(ErrorCode::TargetChanged.into());
    }
    Ok(value)
}


// Each exported member fixes one reviewed action. No generic bus method,
// path, command, administrator assertion or caller-provided UID is accepted.
macro_rules! surface {
    ($name:ident,$scope:expr,$interface:literal,[$(($method:ident,$action:literal)),+ $(,)?], $($extra:item)*) => {
        pub(super) struct $name { pub(super) executor: Executor }
        #[zbus::interface(name=$interface)]
        impl $name {
            async fn get_capabilities(&self, #[zbus(header)] header:Header<'_>) -> Result<String> {
                self.executor.call(header,Operation::SystemCapabilities($scope)).await
            }
            $(async fn $method(&self,request_json:&str,#[zbus(header)] header:Header<'_>) -> Result<String> {
                if request_json.len()>MAX_TASK_BYTES {return Err(ErrorCode::ResourceExhausted.into());}
                self.executor.call(header,Operation::SystemAction($scope,$action,request_json.into())).await
            })+
            $($extra)*
        }
    }
}
surface!(System,Scope::System,"org.aios.System1",[(info,"system.info"),(services,"system.services"),
    (service_status,"system.service_status"),(service_restart,"system.service_restart"),(hardware,"system.hardware"),
    (storage_status,"storage.status"),(network_status,"network.status"),(network_set_wifi_enabled,"network.set_wifi_enabled"),
    (bluetooth_status,"bluetooth.status"),(bluetooth_set_enabled,"bluetooth.set_enabled"),
    (boots,"system.boots"),(boot_diagnostics,"system.boot_diagnostics"),(logs,"system.logs")],
    async fn resolve_log_service(&self,unit_name:&str,#[zbus(header)]header:Header<'_>)->Result<String> {
        if unit_name.len()>255 {return Err(ErrorCode::InvalidArgument.into());}
        self.executor.call(header,Operation::JournalResolveService(unit_name.into())).await
    }
    async fn get_journal_evidence(&self,evidence_id:&str,#[zbus(header)]header:Header<'_>)->Result<String> {
        if !crate::uuid(evidence_id) {return Err(ErrorCode::InvalidArgument.into());}
        self.executor.call(header,Operation::JournalEvidence(evidence_id.into())).await
    }
    async fn resolve_user_log_service(&self,unit_name:&str,#[zbus(header)]header:Header<'_>)->Result<String> {
        if unit_name.len()>255 {return Err(ErrorCode::InvalidArgument.into());}
        self.executor.call(header,Operation::JournalResolveUserService(unit_name.into())).await
    }
    async fn graph_status(&self,#[zbus(header)]header:Header<'_>)->Result<String> {
        self.executor.call(header,Operation::GraphStatus).await
    }
);
surface!(Packages,Scope::Packages,"org.aios.Packages1",[(search,"packages.search"),(info,"packages.info"),
    (installed,"packages.installed"),(install,"packages.install"),(remove,"packages.remove"),(upgrade_plan,"packages.upgrade_plan")],);

#[cfg(test)]
mod tests {
    use super::*;
    fn request(id:&str,args:Value)->String {
        json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),
            "operation":{"kind":"invoke","tool_call":{"kind":"tool_call","action_id":id,"arguments":args}}}).to_string()
    }
    #[test]
    fn public_methods_fix_the_action_and_reject_identity_or_authority_assertions() {
        let raw=request("system.info",json!({}));
        assert!(parse(&raw,"system.info",Scope::System).is_ok());
        assert!(parse(&raw,"packages.info",Scope::Packages).is_err());
        assert!(parse(&raw,"system.info",Scope::Packages).is_err());
        for field in ["uid","approved","admin","session","grants","bus_name","method"] {
            let mut value:Value=serde_json::from_str(&raw).unwrap();value[field]=json!(true);
            assert_eq!(parse(&value.to_string(),"system.info",Scope::System).unwrap_err().code,ErrorCode::InvalidArgument);
        }
        assert_eq!(parse(&raw.replace("\"schema_version\":1","\"schema_version\":2"),"system.info",Scope::System).unwrap_err().code,ErrorCode::UnsupportedSchema);
        assert!(parse(&raw.replace("\"schema_version\":1","\"schema_version\":1,\"schema_version\":1"),"system.info",Scope::System).is_err());
        assert_eq!(parse(&" ".repeat(MAX_TASK_BYTES+1),"system.info",Scope::System).unwrap_err().code,ErrorCode::ResourceExhausted);
    }
    fn owner()->CallerIdentity {
        CallerIdentity{uid:1000,pid:44,start_ticks:1,boot_id:uuid::Uuid::new_v4().to_string(),bus_id:"a".repeat(32),sender:":1.8".into(),session:None}
    }
    #[test]
    fn pagination_handles_bind_native_owner_query_limit_revision_and_expiry() {
        let mut state=ReadState::default();let original=owner();
        let handle=state.issue(original.clone(),"editor".into(),2,"a".repeat(64),2).unwrap();
        assert_eq!(state.offset(Some(&handle),&original,"editor",2,&"a".repeat(64)).unwrap(),2);
        let mut changed=original.clone();changed.uid=1001;
        assert_eq!(state.offset(Some(&handle),&changed,"editor",2,&"a".repeat(64)).unwrap_err().code,ErrorCode::PermissionDenied);
        changed=original.clone();changed.sender=":1.9".into();
        assert_eq!(state.offset(Some(&handle),&changed,"editor",2,&"a".repeat(64)).unwrap_err().code,ErrorCode::PermissionDenied);
        for (query,limit,revision) in [("browser",2,"a".repeat(64)),("editor",3,"a".repeat(64)),("editor",2,"b".repeat(64))] {
            assert_eq!(state.offset(Some(&handle),&original,query,limit,&revision).unwrap_err().code,ErrorCode::StaleEvidence);
        }
        state.cursors.get_mut(&handle).unwrap().expires_ms=0;
        assert_eq!(state.offset(Some(&handle),&original,"editor",2,&"a".repeat(64)).unwrap_err().code,ErrorCode::StaleEvidence);
    }
    #[test]
    fn capability_registration_does_not_claim_unimplemented_effects() {
        for scope in [Scope::System,Scope::Packages] {
            let caps=capabilities(scope).unwrap();
            assert_eq!(caps["data"]["native_caller_verified"],true);
            for contract in caps["data"]["contracts"].as_array().unwrap() {
                let id=contract["action_id"].as_str().unwrap();
                assert_eq!(contract["availability"]=="available",available(id));
            }
        }
        assert!(!available("packages.install"));assert!(!available("system.service_restart"));
        assert!(available("network.status"));assert!(available("bluetooth.status"));
        assert!(!available("network.set_wifi_enabled"));assert!(!available("bluetooth.set_enabled"));
        let network=aios_protocol::registry::capability("network.set_wifi_enabled").unwrap();
        assert_eq!(network.risk_class,"R3");assert!(network.preconditions.iter().any(|p|p=="transport-guard"));
        let bluetooth=aios_protocol::registry::capability("bluetooth.set_enabled").unwrap();
        assert_eq!(bluetooth.risk_class,"R2");assert!(!bluetooth.read_only);
    }
    #[test]
    fn cursor_budget_counts_uid_across_connections_and_reclaims_expired_resources() {
        let mut state=ReadState::default();let original=owner();
        for i in 0..256 {
            let mut caller=original.clone();caller.sender=format!(":1.{}",i+20);
            state.issue(caller,"editor".into(),1,"a".repeat(64),1).unwrap();
        }
        assert_eq!(state.issue(original.clone(),"editor".into(),1,"a".repeat(64),1).unwrap_err().code,ErrorCode::ResourceExhausted);
        state.cursors.values_mut().for_each(|c|c.expires_ms=0);
        state.issue(original,"editor".into(),1,"a".repeat(64),1).unwrap();
        assert_eq!(state.cursors.len(),1);
    }
    fn fixture_catalog()->Catalog {
        let mut entries=vec![];
        for (id,attribute,capability,binary) in [("kate",json!(["kdePackages","kate"]),"desktop_application","kate"),
            ("postgresql-17",json!(["postgresql_17"]),"postgresql17","pg_isready")] {
            let mut entry=json!({"id":id,"attribute":attribute,"display_name":id,"version":if id=="postgresql-17" {"17.6-fixture"} else {"fixture-1"},
                "licenses":["MIT"],"unfree":false,"platform":"x86_64-linux","binaries":[binary],
                "desktop_ids":[],"capability":capability});
            entry["metadata_revision"]=json!(crate::sha256(&crate::canonical(&entry).unwrap()));
            entries.push(entry);
        }
        let options=[
            ("power_policy.profile_on_ac","power_profile"),
            ("power_policy.profile_on_battery","power_profile"),
            ("services.openssh.enabled","boolean"),
            ("services.openssh.open_firewall","boolean"),
            ("services.postgresql.enabled","boolean"),
            ("services.postgresql.listen_mode","postgresql_listen_mode"),
            ("services.postgresql.package_id","package_id"),
        ].into_iter().map(|(id,value_kind)| {
            let mut option=json!({"id":id,"value_kind":value_kind});
            option["metadata_revision"]=json!(crate::sha256(&crate::canonical(&option).unwrap()));
            option
        }).collect::<Vec<_>>();
        let content=json!({"schema_version":1,"base_template_revision":"1".repeat(64),"lock_sha256":"2".repeat(64),
            "nixpkgs_revision":"774debe7a0d1b496e35677ad955a1011c6ff74f3","installation_state_version":"26.05",
            "platform":"x86_64-linux","packages":entries,"options":options});
        Catalog::from_installed(&serde_json::to_vec(&json!({"catalog_revision":crate::sha256(&crate::canonical(&content).unwrap()),"content":content})).unwrap()).unwrap()
    }
    #[test]
    fn catalog_results_obey_normative_contract_and_search_only_reviewed_entries() {
        // Catalog metadata is explicitly fixture data. Native installed origin
        // and provider readiness require the separate installed image probe.
        let catalog=fixture_catalog();
        let kate=package(catalog.entry("kate").unwrap(),&catalog);
        let result=package_result(kate.clone(),None);
        aios_protocol::validation::validate_result("packages.info",&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(search(&catalog,"KaTe"),vec![kate]);
        assert_eq!(search(&catalog,"pg_isready")[0]["package_id"],"postgresql-17");
        assert!(search(&catalog,"unreviewed-package").is_empty());
        let result=package_result(json!({"matches":search(&catalog,"kate")}),None);
        aios_protocol::validation::validate_result("packages.search",&serde_json::to_vec(&result).unwrap()).unwrap();
    }
}
