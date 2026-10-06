use super::*;
use std::{os::unix::fs::{PermissionsExt,symlink},sync::atomic::{AtomicU64,Ordering}};
static NEXT:AtomicU64=AtomicU64::new(0);
struct Temporary(std::path::PathBuf);
impl Temporary {fn new()->Self{let path=std::env::temp_dir().join(format!("horizon-graph-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir(&path).unwrap();fs::set_permissions(&path,fs::Permissions::from_mode(0o700)).unwrap();Self(path)}}
impl Drop for Temporary {fn drop(&mut self){fs::remove_dir_all(&self.0).unwrap();}}
fn node(id:&str)->Node {Node{id:id.into(),kind:"service".into(),scope:Scope::User(1000),provider:"native-systemd".into(),stable_key:id.into(),properties:serde_json::json!({"active":false}),source_truth:SourceTruth::Running,realtime_ns:1}}
#[test] fn native_sqlite_migration_wal_scope_and_reopen() {
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();
    store.upsert_nodes(vec![node("service:a")]).unwrap();assert_eq!(store.nodes(vec!["service:a".into()]).unwrap()[0].revision,1);
    assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::Busy)));
    let uid=unsafe{libc::geteuid()};for name in ["graph.sqlite3","graph.sqlite3-wal","graph.sqlite3-shm"]{assert!(safe_file(&temp.0.join(name),uid).unwrap().is_some());}
    drop(store);assert!(matches!(GraphStore::open(&temp.0,Scope::User(1001)),Err(Error::WrongScope)));
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert_eq!(store.nodes(vec!["service:a".into()]).unwrap().len(),1);drop(store);
    let connection=Connection::open(temp.0.join("graph.sqlite3")).unwrap();let mode:String=connection.query_row("PRAGMA journal_mode",[],|r|r.get(0)).unwrap();assert_eq!(mode,"wal");
    let version:i64=connection.pragma_query_value(None,"user_version",|r|r.get(0)).unwrap();assert_eq!(version,VERSION);
    connection.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    assert!(connection.execute("INSERT INTO observations VALUES('o','p','absent','boot',1,1,'{}','public',?1)",["a".repeat(64)]).is_err());
}
#[test] fn atomic_batch_preserves_identity_and_source_of_truth() {
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();
    let mut conflict=node("a");conflict.source_truth=SourceTruth::Intended;
    assert_eq!(store.upsert_nodes(vec![node("b"),conflict]),Err(Error::IdentityChanged));assert!(store.nodes(vec!["b".into()]).unwrap().is_empty());
    let mut foreign=node("c");foreign.scope=Scope::User(1001);assert_eq!(store.upsert_nodes(vec![foreign]),Err(Error::WrongScope));
    let mut intended=node("intended-a");intended.stable_key="a".into();intended.source_truth=SourceTruth::Intended;
    store.upsert_nodes(vec![intended]).unwrap();let rows=store.nodes(vec!["a".into(),"intended-a".into()]).unwrap();assert_eq!(rows.len(),2);assert_ne!(rows[0].source_truth,rows[1].source_truth);
    let mut stale=node("a");stale.realtime_ns=0;assert_eq!(store.upsert_nodes(vec![stale]),Err(Error::IdentityChanged));
}
#[test] fn serialized_concurrent_updates_preserve_every_revision() {
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();
    thread::scope(|scope|{for _ in 0..8 {let store=&store;scope.spawn(move||{for _ in 0..8 {store.upsert_nodes(vec![node("a")]).unwrap();}});}});
    assert_eq!(store.nodes(vec!["a".into()]).unwrap()[0].revision,65);
    assert_eq!(store.upsert_nodes(vec![node("a");65]),Err(Error::ResourceExhausted));
}
#[test] fn incompatible_or_non_graph_database_is_preserved() {
    let temp=Temporary::new();drop(GraphStore::open(&temp.0,Scope::System).unwrap());let path=temp.0.join("graph.sqlite3");
    let connection=Connection::open(&path).unwrap();connection.pragma_update(None,"user_version",2).unwrap();drop(connection);
    let before=fs::read(&path).unwrap();assert!(matches!(GraphStore::open(&temp.0,Scope::System),Err(Error::Incompatible)));assert_eq!(fs::read(&path).unwrap(),before);
    let other=Temporary::new();let path=other.0.join("graph.sqlite3");let connection=Connection::open(&path).unwrap();connection.execute_batch("CREATE TABLE ledger(id INTEGER PRIMARY KEY)").unwrap();drop(connection);fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();
    let before=fs::read(&path).unwrap();assert!(matches!(GraphStore::open(&other.0,Scope::System),Err(Error::Incompatible)));assert_eq!(fs::read(&path).unwrap(),before);
}
#[test] fn symlink_or_schema_drift_is_refused_without_reset() {
    let temp=Temporary::new();let other=Temporary::new();symlink(&other.0,temp.0.join("linked")).unwrap();assert!(matches!(GraphStore::open(&temp.0.join("linked"),Scope::System),Err(Error::IdentityChanged)));
    drop(GraphStore::open(&temp.0,Scope::System).unwrap());let connection=Connection::open(temp.0.join("graph.sqlite3")).unwrap();connection.execute_batch("ALTER TABLE nodes ADD COLUMN fabricated TEXT").unwrap();drop(connection);
    assert!(matches!(GraphStore::open(&temp.0,Scope::System),Err(Error::Incompatible)));
}
#[test] fn large_or_deep_payloads_are_refused_before_enqueue() {
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();
    let mut large=node("a");large.properties=Value::String("x".repeat(8193));assert_eq!(store.upsert_nodes(vec![large]),Err(Error::ResourceExhausted));
    let mut deep=node("a");let mut value=Value::Null;for _ in 0..33{value=Value::Array(vec![value]);}deep.properties=value;
    assert_eq!(store.upsert_nodes(vec![deep]),Err(Error::ResourceExhausted));assert!(store.nodes(vec!["a".into()]).unwrap().is_empty());
}
fn observation_time(mono:u64)->crate::graph::ObservationTime {
    crate::graph::ObservationTime{boot:crate::graph::BootId::parse("11111111-1111-4111-8111-111111111111").unwrap(),realtime_ns:100,monotonic_ns:mono}
}
fn records()->(ObservationInput,EvidenceInput){
    (ObservationInput{id:"observation:a".into(),provider:"native-systemd".into(),entity_id:"a".into(),entity_revision:1,
        time:observation_time(1),payload:serde_json::json!({"active":false}),sensitivity:Sensitivity::Private},
     EvidenceInput{id:"evidence:a".into(),scope:Scope::User(1000),locator:SourceLocator::Service{unit:"example.service".into(),snapshot_id:"snapshot:a".into()},
        excerpt:"inactive".into(),authenticated_binding:"origin:1.99:scope:a".into(),access_lifetime_ns:30_000_000_000,
        freshness_class:crate::graph::FreshnessClass::Service,source_revision:crate::graph::SourceRevision::default()})
}
fn resolve(store:&GraphStore,now:crate::graph::ObservationTime,purpose:ReadPurpose)->Result<ResolvedEvidence>{
    store.resolve_evidence("evidence:a".into(),"origin:1.99:scope:a".into(),now,crate::graph::SourceRevision::default(),purpose)
}
#[test] fn immutable_observation_evidence_hashes_and_foreign_binding_refusal(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();
    let (observation,evidence)=records();store.append_evidence(observation.clone(),evidence.clone()).unwrap();
    let got=resolve(&store,observation_time(2),ReadPurpose::Current).unwrap();assert_eq!(got.payload,observation.payload);assert_eq!(got.observation_hash.len(),64);assert_eq!(got.evidence_hash.len(),64);assert_eq!(got.freshness,crate::graph::Freshness::Current);
    assert_eq!(store.resolve_evidence("evidence:a".into(),"origin:1.100:scope:a".into(),observation_time(2),crate::graph::SourceRevision::default(),ReadPurpose::Current),Err(Error::WrongScope));
    assert_eq!(store.append_evidence(observation.clone(),evidence.clone()),Err(Error::Invalid));
    let mut other=observation;other.id="observation:b".into();assert_eq!(store.append_evidence(other,evidence),Err(Error::Invalid));drop(store);
    let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();let count:i64=db.query_row("SELECT count(*) FROM observations",[],|r|r.get(0)).unwrap();assert_eq!(count,1);
}
#[test] fn evidence_monotonic_access_expiry_and_explicit_diagnostic_freshness(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();let (o,e)=records();store.append_evidence(o,e).unwrap();
    let mut old=observation_time(5_000_000_002);old.realtime_ns=0;
    assert_eq!(resolve(&store,old.clone(),ReadPurpose::Current),Err(Error::StaleEvidence));let got=resolve(&store,old,ReadPurpose::Diagnostic).unwrap();assert_eq!(got.freshness,crate::graph::Freshness::Stale);assert_eq!(got.captured,observation_time(1));
    assert_eq!(resolve(&store,observation_time(30_000_000_001),ReadPurpose::Diagnostic),Err(Error::StaleEvidence));
    let mut reboot=observation_time(2);reboot.boot=crate::graph::BootId::parse("22222222-2222-4222-8222-222222222222").unwrap();assert_eq!(resolve(&store,reboot,ReadPurpose::Diagnostic),Err(Error::StaleEvidence));
    store.upsert_nodes(vec![node("a")]).unwrap();assert_eq!(resolve(&store,observation_time(2),ReadPurpose::Current),Err(Error::StaleEvidence));
}
#[test] fn scope_revision_and_locator_validation_precede_persistence(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();
    let (mut o,mut e)=records();e.scope=Scope::User(1001);assert_eq!(store.append_evidence(o.clone(),e),Err(Error::WrongScope));
    let (_,mut e)=records();o.entity_revision=2;assert_eq!(store.append_evidence(o,e.clone()),Err(Error::IdentityChanged));
    let (mut foreign_provider,evidence)=records();foreign_provider.provider="unrelated-provider".into();assert_eq!(store.append_evidence(foreign_provider,evidence),Err(Error::IdentityChanged));
    let (o,_)=records();e.locator=SourceLocator::File{scope_handle:"file-scope:a".into(),display_uri:"javascript:alert(1)".into(),content_hash:"a".repeat(64),range:DocumentRange::Pages{first:1,last:1}};assert_eq!(store.append_evidence(o.clone(),e.clone()),Err(Error::Invalid));
    e.locator=SourceLocator::File{scope_handle:"file-scope:a".into(),display_uri:"file:///display-only/path".into(),content_hash:"a".repeat(64),range:DocumentRange::Pages{first:1,last:1}};
    store.append_evidence(o,e).unwrap();let got=resolve(&store,observation_time(2),ReadPurpose::Current).unwrap();assert_eq!(got.viewer_target,ViewerTarget::ScopedFile{scope_handle:"file-scope:a".into(),content_hash:"a".repeat(64),range:DocumentRange::Pages{first:1,last:1}});
}
#[test] fn changed_locator_payload_or_provider_metadata_is_corrupt(){
    for mutation in ["UPDATE evidence SET excerpt='fabricated'","UPDATE observations SET payload_json='{}'","UPDATE observations SET provider='invented'","UPDATE observations SET sensitivity='public'"]{
        let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();let (o,e)=records();store.append_evidence(o,e).unwrap();drop(store);
        let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();db.execute_batch(mutation).unwrap();drop(db);
        let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert_eq!(resolve(&store,observation_time(2),ReadPurpose::Current),Err(Error::Corrupt));
    }
}
