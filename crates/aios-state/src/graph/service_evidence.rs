//! Native, read-only service property citations. A typed citation selects the
//! fixed systemd viewer; neither text nor a URI can select an executable.
use super::{native::{Error, NativeTime, Result}, store::{GraphStore, Scope, SourceTruth, Node, ProviderSnapshot,
    ObservationInput, EvidenceInput, SourceLocator, Sensitivity, ReadPurpose, ViewerTarget}, ObservationTime, SourceRevision, FreshnessClass};
use aios_protocol::contracts::ErrorCode;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const BINDING: &str = "native-system-service-read";
const LIFETIME: u64 = 5_000_000_000;
fn changed() -> Error { Error::Native(ErrorCode::TargetChanged) }
fn invalid() -> Error { Error::Native(ErrorCode::InvalidArgument) }
fn hash<T: serde::Serialize>(value: &T) -> Result<String> {
    let value=serde_json::to_value(value).map_err(|_|invalid())?;
    let bytes=aios_protocol::contracts::canonical_json(&value).map_err(|_|invalid())?;
    Ok(format!("{:x}",Sha256::digest(bytes)))
}
fn identities(unit: &str) -> Result<(String,String)> {
    aios_system::services::validate_service_name(unit)?;
    let digest=hash(&("native-selected-loaded-system-service",unit))?;
    Ok((format!("service-view:{digest}"),format!("native-service-view:{}",&digest[..32])))
}
fn scope(store:&GraphStore)->Result<()> {
    if store.native_scope()!=Scope::System { return Err(Error::WrongScope); } Ok(())
}
fn timely(captured:&ObservationTime,now:&ObservationTime)->Result<()> {
    if captured.boot!=now.boot || now.monotonic_ns.checked_sub(captured.monotonic_ns).is_none_or(|n|n>=LIFETIME) { return Err(changed()); } Ok(())
}
fn observe(unit:&str,node:&str)->Result<(ObservationTime,Value)> {
    let before=NativeTime::observe()?.observation().clone();
    // GetUnit and uncached, root-manager-pinned properties only. This cannot
    // load/start a unit or choose a D-Bus method from citation contents.
    let native=aios_system::services::read_service_status(unit,node)?;
    let now=NativeTime::observe()?.observation().clone();timely(&before,&now)?;
    if native.boot_id!=serde_json::to_value(&before.boot).map_err(|_|invalid())?.as_str().ok_or_else(invalid)? { return Err(changed()); }
    Ok((before,serde_json::to_value(native).map_err(|_|invalid())?))
}

/// Constructed solely by a fresh native read, with a pre-read CAS checkpoint.
/// No deserializer accepts a status, clock, provider or observation from IPC.
pub struct NativeServiceEvidence { captured:ObservationTime, unit:String, node:String, provider:String, data:Value, token:Option<String> }
impl NativeServiceEvidence {
    pub fn collect(store:&GraphStore,unit:&str)->Result<Self> {
        scope(store)?;let (node,provider)=identities(unit)?;
        let before=NativeTime::observe()?.observation().clone();
        let token=store.reconciliation_plan(provider.clone(),before,SourceRevision::default())?.state.map(|s|s.token);
        let (captured,data)=observe(unit,&node)?;
        Ok(Self{captured,unit:unit.into(),node,provider,data,token})
    }
    pub fn record(self,store:&GraphStore)->Result<String> {
        scope(store)?;timely(&self.captured,NativeTime::observe()?.observation())?;
        let (_,live)=observe(&self.unit,&self.node)?;
        timely(&self.captured,NativeTime::observe()?.observation())?;
        if live!=self.data { return Err(changed()); }
        // This provider's complete scope is exactly ONE selected loaded unit,
        // not the machine's service inventory or any runtime cause/effect.
        store.apply_provider_snapshot(ProviderSnapshot{provider:self.provider.clone(),expected_token:self.token,source_truth:SourceTruth::Running,
            time:self.captured.clone(),source_revision:SourceRevision::default(),complete:true,verified_absent_ids:vec![],
            nodes:vec![Node{id:self.node.clone(),kind:"service".into(),scope:Scope::System,provider:self.provider.clone(),stable_key:self.node.clone(),
                properties:json!({"observation":self.data,"captured":self.captured,"inventory_scope":"one-selected-loaded-system-service","execution_authority":false}),
                source_truth:SourceTruth::Running,realtime_ns:self.captured.realtime_ns}]})?;
        let node=store.nodes(vec![self.node.clone()])?.into_iter().next().ok_or_else(changed)?;
        let digest=hash(&(&self.captured,&self.data))?;
        let id=format!("service-evidence:{digest}");
        store.append_evidence(ObservationInput{id:format!("service-observation:{digest}"),provider:self.provider,entity_id:self.node,
            entity_revision:node.revision,time:self.captured.clone(),payload:self.data,sensitivity:Sensitivity::Public},
            EvidenceInput{id:id.clone(),scope:Scope::System,locator:SourceLocator::Service{unit:self.unit,snapshot_id:id.clone()},
                excerpt:"Native systemd service properties; ordering is not causation".into(),authenticated_binding:BINDING.into(),
                access_lifetime_ns:LIFETIME,freshness_class:FreshnessClass::Service,source_revision:SourceRevision::default()})?;
        // Return only a descriptor actually resolvable at the end of capture.
        store.resolve_evidence(id.clone(),BINDING.into(),NativeTime::observe()?.observation().clone(),SourceRevision::default(),ReadPurpose::Current)?;
        Ok(id)
    }
}

/// Root-private diagnostic viewer, not a model tool or a permission grant.
/// Reject expired/tampered/foreign-kind descriptors BEFORE querying systemd.
/// After the live read, recheck the exact stored seal, revision and lifetime.
pub fn view(store:&GraphStore,id:&str)->Result<Value> {
    scope(store)?;
    let evidence=store.resolve_evidence(id.into(),BINDING.into(),NativeTime::observe()?.observation().clone(),SourceRevision::default(),ReadPurpose::Current)?;
    let (unit,snapshot)=match &evidence.viewer_target {
        ViewerTarget::Service{unit,snapshot_id}=>(unit,snapshot_id),_=>return Err(invalid()),
    };
    let (node,_)=identities(unit)?;
    if snapshot!=id || evidence.payload["unit_name"]!=unit.as_str() || evidence.payload["service_id"]!=node
        || evidence.payload["scope"]!="system" || evidence.payload["ordering_is_not_causation"]!=true { return Err(invalid()); }
    let (captured,live)=observe(unit,&node)?;
    if live!=evidence.payload { return Err(changed()); }
    let after=store.resolve_evidence(id.into(),BINDING.into(),NativeTime::observe()?.observation().clone(),SourceRevision::default(),ReadPurpose::Current)?;
    if after.evidence_hash!=evidence.evidence_hash || after.observation_hash!=evidence.observation_hash { return Err(changed()); }
    Ok(json!({"schema_version":1,"viewer":"native-systemd-service-properties","evidence_id":evidence.id,
        "observation_sha256":evidence.observation_hash,"evidence_sha256":evidence.evidence_hash,"captured":evidence.captured,
        "live_compared_at":captured,"freshness":"Current","source_truth":"running","data":live,
        "executable_uri":false,"execution_authority":false,"complete_service_inventory_verified":false}))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn names_cannot_be_commands_paths_uris_or_nonservice_units() {
        for s in ["file:///etc/shadow","https://example.invalid","../sshd.service","sshd.service;reboot","x.socket","-x.service"] { assert!(identities(s).is_err()); }
        let (node,provider)=identities("sshd.service").unwrap();assert!(node.len()<128 && provider.len()<64);
        assert_ne!(identities("other.service").unwrap(),(node,provider));
    }
}
