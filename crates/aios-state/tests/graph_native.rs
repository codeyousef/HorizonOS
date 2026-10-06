//! Actual Nix profile generations in an isolated, owned guest fixture. This
//! does not activate the system, edit its profile, or imply installed wiring.
use aios_state::graph::{BootId,ObservationTime,SourceRevision};
use aios_state::graph::store::*;
use std::{fs,os::unix::fs::PermissionsExt,path::{Path,PathBuf},process::Command,time::{SystemTime,UNIX_EPOCH}};
use serde_json::json;
use sha2::{Digest,Sha256};
struct Fixture(PathBuf);
impl Drop for Fixture{fn drop(&mut self){fs::remove_dir_all(&self.0).unwrap();}}

#[test]
#[ignore="requires an enrolled NixOS guest; reads original immutable built configuration"]
fn real_built_configuration_is_hash_bound_and_does_not_claim_approval_or_runtime() {
    use aios_state::graph::built::{self,NativeBuiltSnapshot,PROVIDER,NODE};
    let fixture=Fixture(std::env::temp_dir().join(format!("aios-graph-built-{}-{}",std::process::id(),time().monotonic_ns)));
    fs::create_dir(&fixture.0).unwrap();fs::set_permissions(&fixture.0,fs::Permissions::from_mode(0o700)).unwrap();
    let graph=fixture.0.join("graph");fs::create_dir(&graph).unwrap();fs::set_permissions(&graph,fs::Permissions::from_mode(0o700)).unwrap();
    let store=GraphStore::open(&graph,Scope::System).unwrap();
    let snapshot=NativeBuiltSnapshot::collect(&store).unwrap();let data=snapshot.data().clone();
    let running=fs::canonicalize("/run/current-system").unwrap();assert_eq!(data.running_closure,running.to_str().unwrap());
    let manifest=fs::read(running.join("etc/aios/managed.json")).unwrap();let catalog=fs::read(running.join("etc/aios/catalog.json")).unwrap();
    assert_eq!(data.manifest_sha256,format!("{:x}",Sha256::digest(&manifest)));
    assert_eq!(data.catalog_sha256,format!("{:x}",Sha256::digest(&catalog)));
    assert_eq!(serde_json::to_value(&data.configuration).unwrap(),serde_json::from_slice::<serde_json::Value>(&manifest).unwrap());
    let state=snapshot.apply(&store).unwrap();assert_eq!(state.status,ProviderStatus::Ready);
    let row=store.nodes(vec![NODE.into()]).unwrap().remove(0);
    assert_eq!(row.source_truth,SourceTruth::Built);assert_eq!(row.provider,PROVIDER);
    assert_eq!(row.properties["approved_manifest_verified"],false);assert_eq!(row.properties["managed_transaction"],serde_json::Value::Null);
    assert_eq!(row.properties["runtime_postconditions_verified"],false);assert_eq!(row.properties["complete_package_inventory_verified"],false);
    assert_eq!(row.properties["manifest_sha256"],data.manifest_sha256);
    assert_eq!(built::observe().unwrap().1,data);
    let other=fixture.0.join("other");fs::create_dir(&other).unwrap();fs::set_permissions(&other,fs::Permissions::from_mode(0o700)).unwrap();
    let user=GraphStore::open(&other,Scope::User(unsafe{libc::geteuid()})).unwrap();
    assert!(matches!(NativeBuiltSnapshot::collect(&user),Err(aios_state::graph::native::Error::WrongScope)));
    let expired=NativeBuiltSnapshot::collect(&store).unwrap();std::thread::sleep(std::time::Duration::from_millis(2100));
    assert!(matches!(expired.apply(&store),Err(aios_state::graph::native::Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))));
    println!("AIOS_NATIVE_BUILT_CONFIGURATION={}",json!({"evidence_kind":"native-built-configuration-graph-library","data":data,
        "native_file_hashes_compared":true,"read_only_original_metadata":true,"approved_manifest_verified":false,
        "runtime_postconditions_verified":false,"expired_snapshot_refused":true,"installed_owner_wiring":false}));
}

#[test]
#[ignore="requires an enrolled NixOS guest; reads fixed system pointers only"]
fn real_system_pointers_are_separate_and_do_not_infer_boot_or_management() {
    use aios_state::graph::generations::{self, NativeGenerationSnapshot, RUNNING_PROVIDER, PROFILE_PROVIDER};
    let fixture=Fixture(std::env::temp_dir().join(format!("aios-graph-system-pointers-{}-{}",std::process::id(),time().monotonic_ns)));
    fs::create_dir(&fixture.0).unwrap();fs::set_permissions(&fixture.0,fs::Permissions::from_mode(0o700)).unwrap();
    let graph=fixture.0.join("graph");fs::create_dir(&graph).unwrap();fs::set_permissions(&graph,fs::Permissions::from_mode(0o700)).unwrap();
    let store=GraphStore::open(&graph,Scope::System).unwrap();
    let snapshot=NativeGenerationSnapshot::collect(&store).unwrap();let pointers=snapshot.pointers().clone();
    assert_eq!(pointers.running_closure,fs::canonicalize("/run/current-system").unwrap().to_str().unwrap());
    assert_eq!(pointers.selected_profile_closure.as_deref(),fs::canonicalize("/nix/var/nix/profiles/system").ok().as_ref().and_then(|p|p.to_str()));
    assert_eq!(pointers.bootloader_entry,None);assert_eq!(pointers.managed_transaction,None);
    let states=snapshot.apply(&store).unwrap();assert_eq!(states.0.status,ProviderStatus::Ready);
    assert_eq!(states.1.status,if pointers.selected_profile_generation.is_some(){ProviderStatus::Ready}else{ProviderStatus::Partial});
    let rows=store.nodes(vec!["generation:running-system".into(),"generation:selected-system-profile".into()]).unwrap();
    assert_eq!(rows.len(),2);assert_eq!(rows[0].provider,RUNNING_PROVIDER);assert_eq!(rows[0].source_truth,SourceTruth::Running);
    assert_eq!(rows[1].provider,PROFILE_PROVIDER);assert_eq!(rows[1].source_truth,SourceTruth::BootSelected);
    assert_eq!(rows[0].properties["running_closure"],pointers.running_closure);
    assert!(rows[0].properties.get("selected_profile_closure").is_none());
    assert_eq!(rows[1].properties["selected_profile_closure"],serde_json::to_value(&pointers.selected_profile_closure).unwrap());
    assert!(rows[1].properties.get("running_closure").is_none());
    assert_eq!(generations::observe().unwrap().1,pointers);
    let other=fixture.0.join("other");fs::create_dir(&other).unwrap();fs::set_permissions(&other,fs::Permissions::from_mode(0o700)).unwrap();
    let user=GraphStore::open(&other,Scope::User(unsafe{libc::geteuid()})).unwrap();
    assert!(matches!(NativeGenerationSnapshot::collect(&user),Err(aios_state::graph::native::Error::WrongScope)));
    let expired=NativeGenerationSnapshot::collect(&store).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert!(matches!(expired.apply(&store),Err(aios_state::graph::native::Error::Native(aios_protocol::contracts::ErrorCode::TargetChanged))));
    println!("AIOS_NATIVE_SYSTEM_POINTERS={}",json!({"evidence_kind":"native-system-pointer-graph-library","pointers":pointers,
        "source_truths_separate":true,"read_only_system_pointers":true,"boot_and_management_unknown":true,"expired_snapshot_refused":true,"installed_owner_wiring":false}));
}
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
    ProviderSnapshot{provider:"native-fixture-nix-profile".into(),expected_token:token,source_truth:SourceTruth::UserApp,time:time.clone(),source_revision:source_revision.clone(),complete:true,verified_absent_ids:vec![],
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

struct OwnedChild(std::process::Child);
impl OwnedChild{
    fn new()->Self{Self(Command::new("/run/current-system/sw/bin/sleep").arg("60").spawn().unwrap())}
    fn exit(&mut self){self.0.kill().unwrap();self.0.wait().unwrap();}
}
impl Drop for OwnedChild{fn drop(&mut self){if self.0.try_wait().unwrap().is_none(){self.0.kill().unwrap();self.0.wait().unwrap();}}}
fn process_fixture()->(Fixture,u32){
    assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|l|l=="ID=nixos"));assert_eq!(fs::read_to_string("/etc/aios/guest-role").unwrap().trim(),"development");
    let uid=unsafe{libc::geteuid()};assert!(uid>=1000);
    let path=std::env::temp_dir().join(format!("horizon-native-process-{}-{}",std::process::id(),time().monotonic_ns));
    fs::create_dir(&path).unwrap();fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).unwrap();(Fixture(path),uid)
}
fn observed_process_snapshot(store:&GraphStore)->(aios_state::graph::native::NativeProcessSnapshot,String){
    let snapshot=aios_state::graph::native::NativeProcessSnapshot::collect(store).unwrap();let state=snapshot.apply(store).unwrap();
    assert!(matches!(state.status,ProviderStatus::Ready|ProviderStatus::Partial));
    if !snapshot.is_complete(){assert_eq!(state.status,ProviderStatus::Partial);}
    (snapshot,format!("{:?}",state.status))
}
fn child_key(snapshot:&aios_state::graph::native::NativeProcessSnapshot,pid:u32)->String{
    snapshot.ids().find(|id|snapshot.read_live(id).is_ok_and(|o|o.observation.identity.pid==pid)).expect("native child absent from owned UID census").to_owned()
}
#[test]
#[ignore="requires an enrolled NixOS guest; owns only its graph and child process fixtures"]
fn actual_processes_match_native_identity_exit_reconciliation_and_untrusted_cache(){
    use aios_state::graph::native::{NativeProcessSnapshot,PROCESS_PROVIDER,Error as NativeError};
    use aios_protocol::contracts::ErrorCode;use std::os::unix::fs::MetadataExt;
    let (fixture,uid)=process_fixture();let store=GraphStore::open(&fixture.0,Scope::User(uid)).unwrap();let mut child=OwnedChild::new();
    let (snapshot,attempts)=observed_process_snapshot(&store);let key=child_key(&snapshot,child.0.id());let live=snapshot.read_live(&key).unwrap();
    let boot=fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim().to_owned();let stat=fs::read_to_string(format!("/proc/{}/stat",child.0.id())).unwrap();
    let fields=stat.rsplit_once(')').unwrap().1.split_whitespace().collect::<Vec<_>>();let start:u64=fields[19].parse().unwrap();let native_uid=fs::metadata(format!("/proc/{}",child.0.id())).unwrap().uid();
    assert_eq!(native_uid,uid);assert_eq!(live.observation.identity.pid,child.0.id());assert_eq!(live.observation.identity.uid,native_uid);assert_eq!(live.observation.identity.boot_id,boot);assert_eq!(live.observation.identity.start_time_ticks,start);
    let row=store.nodes(vec![key.clone()]).unwrap().remove(0);assert_eq!(row.source_truth,SourceTruth::Running);assert_eq!(row.provider,PROCESS_PROVIDER);
    assert_eq!(row.properties["observation"]["identity"]["pid"],child.0.id());assert_eq!(row.properties["observation"]["identity"]["start_time_ticks"],start);
    assert_eq!(row.properties["captured"]["boot"],boot);
    // Corrupt cache text cannot select PID 1, root or another executable for
    // the retained native live read. No product execution is attempted.
    let db=rusqlite::Connection::open(fixture.0.join("graph.sqlite3")).unwrap();db.execute("UPDATE nodes SET properties_json=?2 WHERE id=?1",rusqlite::params![key,serde_json::json!({"observation":{"identity":{"pid":1,"uid":0,"start_time_ticks":0,"boot_id":"forged"}}}).to_string()]).unwrap();drop(db);
    let after_cache_change=snapshot.read_live(&key).unwrap();assert_eq!(after_cache_change.observation.identity,live.observation.identity);
    let before_exit=NativeProcessSnapshot::collect(&store).unwrap();let retained_key=child_key(&before_exit,child.0.id());assert_eq!(retained_key,key);child.exit();
    assert!(matches!(before_exit.read_live(&key),Err(NativeError::Native(ErrorCode::TargetNotFound))));
    let partial=before_exit.apply(&store).unwrap();assert_eq!(partial.status,ProviderStatus::Partial);assert!(store.nodes(vec![key.clone()]).unwrap().is_empty());
    let (_,after_exit_attempts)=observed_process_snapshot(&store);assert!(store.nodes(vec![key.clone()]).unwrap().is_empty());
    let replacement=OwnedChild::new();let (replacement_snapshot,replacement_attempts)=observed_process_snapshot(&store);let replacement_key=child_key(&replacement_snapshot,replacement.0.id());assert_ne!(replacement_key,key);
    assert!(matches!(before_exit.read_live(&key),Err(NativeError::Native(ErrorCode::TargetNotFound))));
    println!("AIOS_NATIVE_GRAPH_PROCESSES={}",json!({"evidence_kind":"real-own-uid-native-pidfd-and-proc-graph-library","uid":uid,"boot":boot,"child_pid":child.0.id(),"start_ticks":start,"graph_id":key,
        "attempts":attempts,"after_exit_attempts":after_exit_attempts,"replacement_attempts":replacement_attempts,"native_identity_matches":true,"cache_cannot_rebind_native_handle":true,"access_denied":snapshot.has_access_denials(),"native_census_complete":snapshot.is_complete(),"verified_pidfd_exit_removed_exact_node":true,"partial_never_inferred_absence":true,"old_retained_handle_stayed_exited":true,"actual_pid_reuse_tested":false,"installed_graph_service_qualified":false}));
    drop(replacement_snapshot);drop(before_exit);drop(snapshot);drop(store);
}
#[test]
#[ignore="requires an enrolled NixOS guest; owns only its graph and child process fixtures"]
fn native_process_snapshot_deadline_and_database_scope_are_enforced(){
    use aios_state::graph::native::{NativeProcessSnapshot,Error as NativeError,PROCESS_PROVIDER};
    let (fixture,uid)=process_fixture();let store=GraphStore::open(&fixture.0,Scope::System).unwrap();assert!(matches!(NativeProcessSnapshot::collect(&store),Err(NativeError::WrongScope)));drop(store);
    let graph=fixture.0.join("own");fs::create_dir(&graph).unwrap();fs::set_permissions(&graph,fs::Permissions::from_mode(0o700)).unwrap();let store=GraphStore::open(&graph,Scope::User(uid)).unwrap();let child=OwnedChild::new();
    let (snapshot,attempts)=observed_process_snapshot(&store);let key=child_key(&snapshot,child.0.id());let expired=NativeProcessSnapshot::collect(&store).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2100));assert!(matches!(expired.apply(&store),Err(NativeError::Expired)));
    assert_eq!(expired.read_live(&key).unwrap().observation.identity.pid,child.0.id());
    assert_eq!(store.reconciliation_plan(PROCESS_PROVIDER.into(),time(),SourceRevision::default()).unwrap().reason,Some(ReconcileReason::Invalidated));
    println!("AIOS_NATIVE_GRAPH_PROCESS_EXPIRY={}",json!({"evidence_kind":"real-kernel-monotonic-own-process-snapshot-expiry","uid":uid,"attempts":attempts,"actual_wait_ms":2100,"expired_snapshot_refused":true,"live_read_reobserved_retained_process":true,"foreign_database_scope_refused":true,"installed_graph_service_qualified":false}));
    drop(expired);drop(snapshot);drop(store);
}

#[test]
#[ignore="requires the enrolled NixOS guest's real system bus and systemd manager"]
fn native_systemd_loaded_graph_matches_independent_service_state(){
    use aios_state::graph::native::{NativeSystemdSnapshot,SYSTEMD_PROVIDER,system_service_key};
    let (fixture,_uid)=process_fixture();let store=GraphStore::open(&fixture.0,Scope::System).unwrap();
    let snapshot=NativeSystemdSnapshot::collect(&store).unwrap();let state=snapshot.apply(&store).unwrap();assert!(matches!(state.status,ProviderStatus::Ready|ProviderStatus::Partial));
    let id=system_service_key("sshd.service").unwrap();let row=store.nodes(vec![id.clone()]).unwrap().remove(0);
    let output=Command::new("/run/current-system/sw/bin/systemctl").args(["show","sshd.service","--property=LoadState","--property=ActiveState","--property=SubState"]).output().unwrap();assert!(output.status.success());
    let text=String::from_utf8(output.stdout).unwrap();let expected=text.lines().map(|line|line.split_once('=').unwrap()).collect::<std::collections::BTreeMap<_,_>>();
    assert_eq!(row.properties["observation"]["load_state"],expected["LoadState"]);assert_eq!(row.properties["observation"]["active_state"],expected["ActiveState"]);assert_eq!(row.properties["observation"]["sub_state"],expected["SubState"]);
    assert!(row.properties["main_pid"].is_null());assert!(row.properties["result"].is_null());assert!(row.properties["ordering_after"].is_null());assert_eq!(row.provider,SYSTEMD_PROVIDER);assert_eq!(row.source_truth,SourceTruth::Running);
    println!("AIOS_NATIVE_GRAPH_SYSTEMD={}",json!({"evidence_kind":"real-root-manager-loaded-service-graph-library","unit":"sshd.service","id":id,"provider_status":state.status,"observed_services":snapshot.ids().unwrap().len(),"manager_owner":row.properties["manager_owner"],"boot":snapshot.captured().boot,"independent_systemctl":text,"unknown_properties_preserved":true,"installed_graph_daemon_qualified":false}));drop(store);
}

#[test]
#[ignore="requires enrolled NixOS guest, native udev/sysfs; no storage effect or installed owner claim"]
fn real_block_device_properties_match_native_udev_and_scoped_graph(){
    use aios_state::graph::devices::{NativeBlockSnapshot,PROVIDER};
    let fixture=Fixture(std::env::temp_dir().join(format!("aios-graph-block-{}-{}",std::process::id(),time().monotonic_ns)));
    fs::create_dir(&fixture.0).unwrap();fs::set_permissions(&fixture.0,fs::Permissions::from_mode(0o700)).unwrap();let root=fixture.0.join("graph");fs::create_dir(&root).unwrap();fs::set_permissions(&root,fs::Permissions::from_mode(0o700)).unwrap();
    let store=GraphStore::open(&root,Scope::System).unwrap();let snapshot=NativeBlockSnapshot::collect(&store).unwrap();
    let inventory=snapshot.inventory().clone();assert!(!inventory.devices.is_empty());
    let ids=snapshot.ids().unwrap();let result=snapshot.apply(&store).unwrap();let mut rows=Vec::new();for ids in ids.chunks(64){rows.extend(store.nodes(ids.to_vec()).unwrap());}assert_eq!(rows.len(),inventory.devices.len());
    assert_eq!(result.status,if inventory.complete{ProviderStatus::Ready}else{ProviderStatus::Partial});
    let device=&inventory.devices[0];
    assert_eq!(fs::read_to_string(Path::new(&device.syspath).join("dev")).unwrap().trim(),format!("{}:{}",device.major,device.minor));
    let argv=["/run/current-system/sw/bin/udevadm","info","--query=property","--path",device.syspath.as_str()];
    let independent=Command::new(argv[0]).args(&argv[1..]).output().unwrap();assert!(independent.status.success());assert!(independent.stdout.len()+independent.stderr.len()<65536);
    let properties:std::collections::BTreeMap<_,_>=std::str::from_utf8(&independent.stdout).unwrap().lines().filter_map(|l|l.split_once('=').filter(|(_,value)|!value.is_empty())).collect();
    for (property,value) in [("ID_SERIAL",&device.serial),("ID_SERIAL_SHORT",&device.serial_short),("ID_WWN",&device.wwn),("ID_BUS",&device.bus),("ID_MODEL",&device.model),("ID_VENDOR",&device.vendor)]{
        assert_eq!(properties.get(property).copied(),value.as_deref());
    }
    for row in &rows{assert_eq!(row.provider,PROVIDER);assert_eq!(row.source_truth,SourceTruth::Running);assert_eq!(row.properties["execution_authority"],false);assert_eq!(row.properties["live_identity_retained"],false);}
    let other=fixture.0.join("user");fs::create_dir(&other).unwrap();fs::set_permissions(&other,fs::Permissions::from_mode(0o700)).unwrap();
    let user=GraphStore::open(&other,Scope::User(unsafe{libc::geteuid()})).unwrap();
    assert!(matches!(NativeBlockSnapshot::collect(&user),Err(aios_state::graph::native::Error::WrongScope)));
    let expired=NativeBlockSnapshot::collect(&store).unwrap();std::thread::sleep(std::time::Duration::from_millis(2100));
    assert!(matches!(expired.apply(&store),Err(aios_state::graph::native::Error::Expired)));
    println!("AIOS_NATIVE_BLOCK_GRAPH={}",json!({"evidence_kind":"native-udev-block-disk-graph-library","device_count":rows.len(),"complete_block_enumeration":inventory.complete,
        "independent_udevadm_argv":argv,"independent_udevadm_exit":0,"selected_device":device,"missing_serials_stay_null":rows.iter().all(|row|!row.properties["device"]["serial"].is_null() || row.properties["identity_kind"]=="boot_scoped_kernel_locator" || row.properties["identity_kind"]=="udev_wwn"),"wrong_scope_refused":true,"expired_snapshot_refused":true,"storage_effect_performed":false,"installed_owner_verified":false}));
}
