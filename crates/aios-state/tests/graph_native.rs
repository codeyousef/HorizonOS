//! Actual Nix profile generations in an isolated, owned guest fixture. This
//! does not activate the system, edit its profile, or imply installed wiring.
use aios_state::graph::{BootId,ObservationTime,SourceRevision};
use aios_state::graph::store::*;
use std::{fs,os::unix::fs::PermissionsExt,path::{Path,PathBuf},process::Command,time::{SystemTime,UNIX_EPOCH}};
use serde_json::json;
use sha2::{Digest,Sha256};
struct Fixture(PathBuf);
impl Drop for Fixture{fn drop(&mut self){fs::remove_dir_all(&self.0).unwrap();}}
fn time()->ObservationTime {
    let boot=BootId::parse(fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim()).unwrap();
    let mut t=libc::timespec{tv_sec:0,tv_nsec:0};assert_eq!(unsafe{libc::clock_gettime(libc::CLOCK_MONOTONIC,&mut t)},0);
    ObservationTime{boot,realtime_ns:SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().try_into().unwrap(),
        monotonic_ns:u64::try_from(t.tv_sec).unwrap()*1_000_000_000+u64::try_from(t.tv_nsec).unwrap()}
}
fn set(profile:&Path,target:&Path)->Vec<String>{
    assert!(target.starts_with("/nix/store"));let argv=vec!["/run/current-system/sw/bin/nix-env".to_string(),"--profile".into(),profile.to_str().unwrap().into(),"--set".into(),target.to_str().unwrap().into()];
    let result=Command::new(&argv[0]).args(&argv[1..]).output().unwrap();assert!(result.stdout.len()+result.stderr.len()<131072);
    assert!(result.status.success(),"fixed owned profile command failed: {}",String::from_utf8_lossy(&result.stderr));argv
}
fn binding(profile:&Path)->SourceRevision {
    let generation=fs::read_link(profile).unwrap();assert!(generation.file_name().unwrap().to_str().unwrap().starts_with("profile-"));
    let resolved=fs::canonicalize(profile).unwrap();assert!(resolved.starts_with("/nix/store"));
    SourceRevision{generation:Some(generation.to_str().unwrap().into()),profile:Some(resolved.to_str().unwrap().into()),closure_hash:None,document_hash:None}
}
fn snapshot(scope:Scope,token:Option<String>,profile:&Path)->ProviderSnapshot{
    let time=time();let source_revision=binding(profile);
    ProviderSnapshot{provider:"native-fixture-nix-profile".into(),expected_token:token,source_truth:SourceTruth::UserApp,time:time.clone(),source_revision:source_revision.clone(),complete:true,
        nodes:vec![Node{id:"native-profile".into(),kind:"generation".into(),scope,provider:"native-fixture-nix-profile".into(),stable_key:format!("profile:{:x}",Sha256::digest(profile.as_os_str().as_encoded_bytes())),
            properties:json!({"generation":source_revision.generation,"profile":source_revision.profile}),source_truth:SourceTruth::UserApp,realtime_ns:time.realtime_ns}]}
}
#[test]
#[ignore="requires an enrolled NixOS guest; mutates only a uniquely owned fixture profile"]
fn actual_profile_generations_reconcile_without_overwriting_native_or_intended_state(){
    assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|l|l=="ID=nixos"));
    assert_eq!(fs::read_to_string("/etc/aios/guest-role").unwrap().trim(),"development");
    let uid=unsafe{libc::geteuid()};assert!(uid>=1000);let scope=Scope::User(uid);
    let initial_time=time();let directory=std::env::temp_dir().join(format!("horizon-native-graph-{}-{}",std::process::id(),initial_time.monotonic_ns));
    fs::create_dir(&directory).unwrap();fs::set_permissions(&directory,fs::Permissions::from_mode(0o700)).unwrap();let fixture=Fixture(directory);
    let graph=fixture.0.join("graph");fs::create_dir(&graph).unwrap();fs::set_permissions(&graph,fs::Permissions::from_mode(0o700)).unwrap();let profile=fixture.0.join("profile");
    let installed=fs::canonicalize("/run/current-system").unwrap();let software=fs::canonicalize("/run/current-system/sw").unwrap();assert_ne!(installed,software);
    let first_command=set(&profile,&installed);let first_revision=binding(&profile);
    let store=GraphStore::open(&graph,scope).unwrap();
    // Intended state here is explicitly a fixture, not the machine's managed
    // manifest. It must remain distinct from the actual user-profile source.
    store.upsert_nodes(vec![Node{id:"fixture-intended".into(),kind:"generation".into(),scope,provider:"fixture-intent".into(),stable_key:"fixture-intended".into(),properties:json!({"expected":installed}),source_truth:SourceTruth::Intended,realtime_ns:initial_time.realtime_ns}]).unwrap();
    let first=store.apply_provider_snapshot(snapshot(scope,None,&profile)).unwrap();
    assert_eq!(store.reconciliation_plan("native-fixture-nix-profile".into(),time(),first_revision.clone()).unwrap().reason,None);
    let second_command=set(&profile,&software);let second_revision=binding(&profile);assert_ne!(first_revision,second_revision);
    assert_eq!(store.reconciliation_plan("native-fixture-nix-profile".into(),time(),second_revision.clone()).unwrap().reason,Some(ReconcileReason::RevisionChanged));
    let dirty=store.ingest_provider_events("native-fixture-nix-profile".into(),vec![ProviderEvent{id:"actual-generation-change".into(),entity_id:Some("native-profile".into()),kind:EventKind::GenerationChanged,time:time(),origin_transaction_id:None,payload:json!({"before":first_revision,"after":second_revision})}]).unwrap();
    assert_eq!(store.apply_provider_snapshot(snapshot(scope,Some(first.token),&profile)),Err(Error::IdentityChanged));
    store.apply_provider_snapshot(snapshot(scope,Some(dirty.token),&profile)).unwrap();
    let rows=store.nodes(vec!["native-profile".into(),"fixture-intended".into()]).unwrap();assert_eq!(rows.len(),2);
    assert_eq!(rows[0].properties["profile"],software.to_str().unwrap());assert_eq!(rows[0].source_truth,SourceTruth::UserApp);
    assert_eq!(rows[1].properties["expected"],installed.to_str().unwrap());assert_eq!(rows[1].source_truth,SourceTruth::Intended);
    assert_eq!(binding(&profile),second_revision);assert_eq!(fs::canonicalize("/run/current-system").unwrap(),installed);
    assert_eq!(store.reconciliation_plan("native-fixture-nix-profile".into(),time(),second_revision.clone()).unwrap().reason,None);
    println!("AIOS_NATIVE_GRAPH_PROFILE={}",json!({"evidence_kind":"real-native-isolated-user-nix-profile-graph-library","uid":uid,"boot_id":initial_time.boot,
        "commands":[first_command,second_command],"before":first_revision,"after":second_revision,"revision_conflict_detected":true,
        "stale_snapshot_refused":true,"live_profile_reconciled":true,"native_profile_not_overwritten":true,"fixture_intended_separate":true,
        "system_generation_mutated":false,"installed_graph_service_qualified":false}));
    drop(store);
}
