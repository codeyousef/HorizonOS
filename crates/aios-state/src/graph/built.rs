//! Read-only configuration metadata embedded in the running realized closure.
//! Embedded managed data is built state, not an approval receipt, a live service
//! observation, or a complete inventory of packages and user profiles.
use super::{generations, native::{Error, NativeTime, Result}, store::{GraphStore, Node, ProviderSnapshot, ProviderState, Scope, SourceTruth}, ObservationTime, SourceRevision};
use crate::{Catalog, CatalogEntry, ManagedState, MAX_CATALOG_BYTES, MAX_MANIFEST_BYTES};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::{self, OpenOptions}, io::Read, os::unix::fs::{MetadataExt, OpenOptionsExt}, path::Path};

pub const PROVIDER: &str = "native-built-managed-configuration";
pub const NODE: &str = "configuration:built-managed";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BuiltConfiguration {
    pub running_closure: String,
    pub selected_profile_closure: Option<String>,
    pub manifest_sha256: String,
    pub catalog_sha256: String,
    pub catalog_revision: String,
    pub template_revision: String,
    pub nixpkgs_revision: String,
    pub lock_sha256: String,
    pub installation_state_version: String,
    pub platform: String,
    pub catalog_packages: Vec<CatalogEntry>,
    pub configuration: ManagedState,
}
fn changed() -> Error { Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged) }
fn invalid() -> Error { Error::Native(aios_protocol::contracts::ErrorCode::InvalidArgument) }
fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn store_file(path: &Path) -> bool {
    let Some(name) = path.strip_prefix("/nix/store").ok().and_then(|p|p.components().next()).and_then(|p|p.as_os_str().to_str()) else { return false; };
    let Some((digest, suffix)) = name.split_once('-') else { return false; };
    digest.len()==32 && digest.bytes().all(|b|b"0123456789abcdfghijklmnpqrsvwxyz".contains(&b)) && !suffix.is_empty()
}
fn fingerprint(m: &fs::Metadata) -> (u64,u64,u64,i64,i64,i64,i64) {
    (m.dev(),m.ino(),m.len(),m.mtime(),m.mtime_nsec(),m.ctime(),m.ctime_nsec())
}
fn store_mode(mode:u32)->bool {
    // Multi-user Nix uses root-owned 01775 at the store boundary. Sticky
    // semantics protect root-owned objects; every object/ancestor below it
    // must still be root-owned and non-writable by other identities.
    mode&0o022==0 || mode&0o7777==0o1775
}
fn immutable(path: &Path, bound: usize) -> Result<Vec<u8>> {
    let store = fs::symlink_metadata("/nix/store").map_err(|_|changed())?;
    if !store.is_dir() || store.uid()!=0 || !store_mode(store.mode()) { return Err(changed()); }
    let resolved = fs::canonicalize(path).map_err(|_|changed())?;
    if !store_file(&resolved) { return Err(changed()); }
    let mut parent=resolved.parent().ok_or_else(changed)?;
    while parent!=Path::new("/nix/store") {
        let info=fs::symlink_metadata(parent).map_err(|_|changed())?;
        if !info.is_dir() || info.uid()!=0 || info.mode()&0o022!=0 { return Err(changed()); }
        parent=parent.parent().ok_or_else(changed)?;
    }
    let mut file = OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC|libc::O_NONBLOCK)
        .open(&resolved).map_err(|_|changed())?;
    let before = file.metadata().map_err(|_|changed())?;
    if !before.is_file() || before.uid()!=0 || before.mode()&0o222!=0 || before.len()==0 || before.len()>bound as u64 { return Err(changed()); }
    let mut bytes=Vec::new();file.by_ref().take(bound as u64+1).read_to_end(&mut bytes).map_err(|_|changed())?;
    let after=file.metadata().map_err(|_|changed())?;
    let located=fs::symlink_metadata(&resolved).map_err(|_|changed())?;
    if bytes.len()!=before.len() as usize || fingerprint(&before)!=fingerprint(&after)
        || !located.is_file() || fingerprint(&located)!=fingerprint(&after)
        || fs::canonicalize(path).ok().as_ref()!=Some(&resolved) { return Err(changed()); }
    Ok(bytes)
}
fn decode(running: String, profile: Option<String>, manifest: &[u8], catalog: &[u8]) -> Result<BuiltConfiguration> {
    let parsed=Catalog::from_installed(catalog).map_err(|_|invalid())?;
    let compiled=parsed.compile(manifest).map_err(|_|invalid())?;
    // Fixed template output is canonical complete data. Duplicate keys, omitted
    // defaults, reordering and unknown fields cannot become different facts.
    if compiled.bytes!=manifest { return Err(invalid()); }
    let content=parsed.content();
    Ok(BuiltConfiguration{running_closure:running,selected_profile_closure:profile,
        manifest_sha256:hash(manifest),catalog_sha256:hash(catalog),catalog_revision:parsed.revision().into(),
        template_revision:content.base_template_revision.clone(),nixpkgs_revision:content.nixpkgs_revision.clone(),
        lock_sha256:content.lock_sha256.clone(),installation_state_version:content.installation_state_version.clone(),
        platform:content.platform.clone(),catalog_packages:content.packages.clone(),configuration:compiled.state})
}
/// No caller path or deserialized observation is accepted as native provenance.
pub fn observe() -> Result<(ObservationTime, BuiltConfiguration)> {
    let (captured,pointers)=generations::observe()?;
    let root=Path::new(&pointers.running_closure);
    let manifest=immutable(&root.join("etc/aios/managed.json"),MAX_MANIFEST_BYTES)?;
    let catalog=immutable(&root.join("etc/aios/catalog.json"),MAX_CATALOG_BYTES)?;
    let data=decode(pointers.running_closure.clone(),pointers.selected_profile_closure.clone(),&manifest,&catalog)?;
    let (now,after)=generations::observe()?;
    if pointers!=after || captured.boot!=now.boot || now.monotonic_ns.checked_sub(captured.monotonic_ns).is_none_or(|n|n>2_000_000_000) { return Err(changed()); }
    Ok((captured,data))
}
pub struct NativeBuiltSnapshot { captured:ObservationTime, data:BuiltConfiguration, token:Option<String> }
impl NativeBuiltSnapshot {
    pub fn collect(store:&GraphStore) -> Result<Self> {
        if store.native_scope()!=Scope::System { return Err(Error::WrongScope); }
        let result=(||{
            let before=NativeTime::observe()?.observation().clone();
            let token=store.reconciliation_plan(PROVIDER.into(),before,SourceRevision::default())?.state.map(|s|s.token);
            let (captured,data)=observe()?;Ok(Self{captured,data,token})
        })();if result.is_err(){store.report_event_loss();}result
    }
    pub fn data(&self)->&BuiltConfiguration { &self.data }
    pub fn captured(&self)->&ObservationTime { &self.captured }
    pub fn revision(&self)->SourceRevision { SourceRevision{generation:Some(self.data.running_closure.clone()),profile:self.data.selected_profile_closure.clone(),closure_hash:None,document_hash:Some(self.data.manifest_sha256.clone())} }
    pub fn apply(&self,store:&GraphStore)->Result<ProviderState> {
        if store.native_scope()!=Scope::System { return Err(Error::WrongScope); }
        let result=(||{
            let (now,current)=observe()?;
            if current!=self.data || now.boot!=self.captured.boot || now.monotonic_ns.checked_sub(self.captured.monotonic_ns).is_none_or(|n|n>2_000_000_000){return Err(changed());}
            let properties=serde_json::json!({"running_closure":self.data.running_closure,"manifest_sha256":self.data.manifest_sha256,
                "catalog_sha256":self.data.catalog_sha256,"catalog_revision":self.data.catalog_revision,"template_revision":self.data.template_revision,
                "nixpkgs_revision":self.data.nixpkgs_revision,"lock_sha256":self.data.lock_sha256,
                "installation_state_version":self.data.installation_state_version,"platform":self.data.platform,
                "services":self.data.configuration.services,"power_policy":self.data.configuration.power_policy,
                "configured_system_package_count":self.data.configuration.system_packages.len(),"captured":self.captured,
                "approved_manifest_verified":false,"managed_transaction":null,"runtime_postconditions_verified":false,
                "complete_package_inventory_verified":false,"execution_authority":false});
            let mut nodes=vec![Node{id:NODE.into(),kind:"configuration".into(),scope:Scope::System,provider:PROVIDER.into(),
                stable_key:NODE.into(),properties,source_truth:SourceTruth::Built,realtime_ns:self.captured.realtime_ns}];
            for package in &self.data.catalog_packages {
                let id=format!("catalog:package:{}",package.id);
                let selected=self.data.configuration.system_packages.contains(&package.id);
                nodes.push(Node{id:id.clone(),kind:"package_catalog_entry".into(),scope:Scope::System,provider:PROVIDER.into(),stable_key:id,
                    properties:serde_json::json!({"metadata":package,"catalog_revision":self.data.catalog_revision,
                        "catalog_sha256":self.data.catalog_sha256,"nixpkgs_revision":self.data.nixpkgs_revision,
                        "lock_sha256":self.data.lock_sha256,"platform":self.data.platform,"declared_selected":selected,
                        "realized_package_closure":null,"binary_paths_verified":false,"desktop_entries_verified":false,
                        "runtime_available":null,"source_permissions":"root-owned-immutable-nix-store",
                        "catalog_complete":true,"execution_authority":false}),
                    source_truth:SourceTruth::Built,realtime_ns:self.captured.realtime_ns});
            }
            for package in &self.data.configuration.system_packages {
                let id=format!("configuration:built-package:{package}");
                nodes.push(Node{id:id.clone(),kind:"configuration".into(),scope:Scope::System,provider:PROVIDER.into(),stable_key:id,
                    properties:serde_json::json!({"catalog_id":package,"catalog_entry_id":format!("catalog:package:{package}"),
                        "running_closure":self.data.running_closure,"manifest_sha256":self.data.manifest_sha256,
                        "catalog_revision":self.data.catalog_revision,"captured":self.captured,"runtime_available":null,
                        "realized_package_closure":null,"binary_paths_verified":false,"desktop_entries_verified":false}),
                    source_truth:SourceTruth::Built,realtime_ns:self.captured.realtime_ns});
            }
            let state=store.apply_provider_snapshot(ProviderSnapshot{provider:PROVIDER.into(),expected_token:self.token.clone(),source_truth:SourceTruth::Built,
                time:self.captured.clone(),source_revision:self.revision(),complete:true,nodes,verified_absent_ids:vec![]})?;
            if observe()?.1!=self.data { return Err(changed()); }Ok(state)
        })();if result.is_err(){store.report_event_loss();}result
    }
}

#[cfg(test)] mod tests {
    use super::*;
    fn catalog() -> Vec<u8> {
        let mut package=serde_json::json!({"id":"postgresql-17","attribute":["postgresql_17"],"display_name":"PostgreSQL",
            "version":"17.7","licenses":["PostgreSQL"],"unfree":false,"platform":"x86_64-linux",
            "binaries":["pg_isready"],"desktop_ids":[],"capability":"postgresql17"});
        package["metadata_revision"]=serde_json::json!(hash(&aios_protocol::contracts::canonical_json(&package).unwrap()));
        let content=serde_json::json!({"schema_version":1,"base_template_revision":"1".repeat(64),"lock_sha256":"2".repeat(64),
            "nixpkgs_revision":"3".repeat(40),"installation_state_version":"26.05","platform":"x86_64-linux","packages":[package]});
        serde_json::to_vec(&serde_json::json!({"catalog_revision":hash(&aios_protocol::contracts::canonical_json(&content).unwrap()),"content":content})).unwrap()
    }
    #[test] fn embedded_data_must_be_complete_canonical_and_bound_to_catalog() {
        let bytes=catalog();let catalog=Catalog::from_installed(&bytes).unwrap();
        let canonical=catalog.compile(&serde_json::to_vec(&catalog.defaults()).unwrap()).unwrap().bytes;
        let result=decode("fixture-realized-closure".into(),None,&canonical,&bytes).unwrap();
        assert_eq!(result.manifest_sha256,hash(&canonical));assert_eq!(result.catalog_sha256,hash(&bytes));
        assert_eq!(result.configuration,catalog.defaults());
        assert_eq!(result.catalog_packages.len(),1);
        assert_eq!(result.catalog_packages[0],*catalog.entry("postgresql-17").unwrap());
        let mut value:serde_json::Value=serde_json::from_slice(&canonical).unwrap();
        value["base_template_revision"]=serde_json::json!("4".repeat(64));
        assert!(decode("fixture".into(),None,&serde_json::to_vec(&value).unwrap(),&bytes).is_err());
        let minimal=serde_json::json!({"schema_version":1,"base_template_revision":catalog.content().base_template_revision,"catalog_revision":catalog.revision()});
        assert!(decode("fixture".into(),None,&serde_json::to_vec(&minimal).unwrap(),&bytes).is_err());
        let mut noncanonical=canonical.clone();noncanonical.push(b' ');
        assert!(decode("fixture".into(),None,&noncanonical,&bytes).is_err());
        let text=String::from_utf8(canonical).unwrap().replace("\"schema_version\":1","\"schema_version\":0,\"schema_version\":1");
        assert!(decode("fixture".into(),None,text.as_bytes(),&bytes).is_err());
        assert!(decode("fixture".into(),None,b"{}",b"{}").is_err());
    }
    #[test] fn metadata_paths_require_actual_store_objects() {
        assert!(store_file(Path::new("/nix/store/0123456789abcdfghijklmnpqrsvwxyz-metadata/data.json")));
        for p in ["/tmp/metadata.json","/nix/storeish/object","/nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-metadata/x","/nix/store/metadata/x"] {
            assert!(!store_file(Path::new(p)));
        }
    }
    #[test] fn writable_store_boundary_requires_nix_sticky_permissions() {
        for mode in [0o555,0o755,0o1775] { assert!(store_mode(mode)); }
        for mode in [0o777,0o775,0o1777,0o2775] { assert!(!store_mode(mode)); }
    }
}
