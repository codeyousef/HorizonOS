use super::*;
use std::{io::Write,os::unix::fs::{PermissionsExt,DirBuilderExt,symlink},sync::atomic::{AtomicU64,Ordering}};
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
fn provider_snapshot(token:Option<String>,mono:u64,ids:&[&str])->ProviderSnapshot {
    let time=observation_time(mono);let mut nodes=Vec::new();for id in ids{let mut n=node(id);n.realtime_ns=time.realtime_ns;nodes.push(n);}
    ProviderSnapshot{provider:"native-systemd".into(),expected_token:token,source_truth:SourceTruth::Running,time,
        source_revision:crate::graph::SourceRevision::default(),complete:true,nodes}
}
#[test] fn complete_provider_replacement_is_atomic_and_partial_keeps_prior_nodes(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();
    let first=store.apply_provider_snapshot(provider_snapshot(None,1,&["a","b"])).unwrap();assert_eq!(first.status,ProviderStatus::Ready);
    let mut partial=provider_snapshot(Some(first.token.clone()),2,&["a"]);partial.complete=false;
    let partial=store.apply_provider_snapshot(partial).unwrap();assert_eq!(partial.status,ProviderStatus::Partial);assert_eq!(store.nodes(vec!["a".into(),"b".into()]).unwrap().len(),2);
    let fresh=store.apply_provider_snapshot(provider_snapshot(Some(partial.token),3,&["a"])).unwrap();assert_eq!(store.nodes(vec!["b".into()]).unwrap().len(),0);
    let mut conflict=provider_snapshot(Some(fresh.token.clone()),4,&["a","c"]);conflict.nodes[0].stable_key="changed-native-identity".into();
    assert_eq!(store.apply_provider_snapshot(conflict),Err(Error::IdentityChanged));assert_eq!(store.nodes(vec!["a".into(),"c".into()]).unwrap().len(),1);
    let state=store.reconciliation_plan("native-systemd".into(),observation_time(4),crate::graph::SourceRevision::default()).unwrap().state.unwrap();assert_eq!(state.token,fresh.token);
}
#[test] fn provider_reconciliation_uses_live_boot_revision_and_exact_period(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let provider="native-systemd".to_string();let revision=crate::graph::SourceRevision::default();
    assert_eq!(store.reconciliation_plan(provider.clone(),observation_time(1),revision.clone()).unwrap().reason,Some(ReconcileReason::Unknown));
    store.apply_provider_snapshot(provider_snapshot(None,10,&["a"])).unwrap();
    assert_eq!(store.reconciliation_plan(provider.clone(),observation_time(900_000_000_009),revision.clone()).unwrap().reason,None);
    assert_eq!(store.reconciliation_plan(provider.clone(),observation_time(900_000_000_010),revision.clone()).unwrap().reason,Some(ReconcileReason::Periodic));
    assert_eq!(store.reconciliation_plan(provider.clone(),observation_time(9),revision.clone()).unwrap().reason,Some(ReconcileReason::ClockUnknown));
    let mut reboot=observation_time(11);reboot.boot=crate::graph::BootId::parse("22222222-2222-4222-8222-222222222222").unwrap();
    assert_eq!(store.reconciliation_plan(provider.clone(),reboot,revision.clone()).unwrap().reason,Some(ReconcileReason::BootChanged));
    let mut changed=revision;changed.profile=Some("native-manual-generation".into());
    assert_eq!(store.reconciliation_plan(provider,observation_time(11),changed).unwrap().reason,Some(ReconcileReason::RevisionChanged));
}
fn provider_event(id:&str)->ProviderEvent{ProviderEvent{id:id.into(),entity_id:Some("a".into()),kind:EventKind::Changed,time:observation_time(2),origin_transaction_id:None,payload:serde_json::json!({"changed":true})}}
#[test] fn deduplicated_events_invalidate_snapshot_and_reject_racing_replacement(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let provider="native-systemd".to_string();
    let first=store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();
    let dirty=store.ingest_provider_events(provider.clone(),vec![provider_event("event:a"),provider_event("event:a")]).unwrap();assert_eq!(dirty.status,ProviderStatus::ReconcileRequired);assert_ne!(first.token,dirty.token);
    let duplicate=store.ingest_provider_events(provider.clone(),vec![provider_event("event:a")]).unwrap();assert_eq!(duplicate.token,dirty.token);
    let mut rebound=provider_event("event:a");rebound.payload=Value::Null;assert_eq!(store.ingest_provider_events(provider.clone(),vec![rebound]),Err(Error::IdentityChanged));
    assert_eq!(store.apply_provider_snapshot(provider_snapshot(Some(first.token),3,&["a"])),Err(Error::IdentityChanged));
    let other=store.ingest_provider_events(provider.clone(),vec![provider_event("event:b")]).unwrap();assert_ne!(other.token,dirty.token);
    assert_eq!(store.apply_provider_snapshot(provider_snapshot(Some(dirty.token),3,&["a"])),Err(Error::IdentityChanged));
    store.apply_provider_snapshot(provider_snapshot(Some(other.token),4,&["a"])).unwrap();
    assert_eq!(store.reconciliation_plan(provider,observation_time(5),crate::graph::SourceRevision::default()).unwrap().reason,None);
    drop(store);let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();let count:i64=db.query_row("SELECT count(*) FROM events",[],|r|r.get(0)).unwrap();assert_eq!(count,2);
}
#[test] fn event_flood_retention_and_explicit_loss_reconcile_without_inference(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let provider="native-systemd".to_string();
    store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();
    for batch in 0..66{store.ingest_provider_events(provider.clone(),(0..64).map(|n|provider_event(&format!("event:{}",batch*64+n))).collect()).unwrap();}
    store.report_event_loss();let plan=store.reconciliation_plan(provider,observation_time(3),crate::graph::SourceRevision::default()).unwrap();assert_eq!(plan.reason,Some(ReconcileReason::Invalidated));assert_eq!(plan.state.unwrap().error.unwrap()["code"],"event_queue_overflow");
    drop(store);let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();let count:i64=db.query_row("SELECT count(*) FROM events",[],|r|r.get(0)).unwrap();assert_eq!(count,4096);
}
#[test] fn native_edges_require_declared_relation_and_hypotheses_remain_separate(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a"),node("b")]).unwrap();
    let (mut o,e)=records();o.payload=serde_json::json!({"relations":[{"to_id":"b","relation":"ordered_after"}]});store.append_evidence(o,e).unwrap();
    let edge=EdgeInput{id:"edge:ordering".into(),from_id:"a".into(),to_id:"b".into(),relation:Relation::OrderedAfter,evidence_id:"evidence:a".into()};
    let binding="origin:1.99:scope:a".to_string();let revision=crate::graph::SourceRevision::default();
    store.append_observed_edges(vec![edge.clone()],binding.clone(),observation_time(2),revision.clone()).unwrap();
    let mut inferred=edge.clone();inferred.id="edge:hypothesis".into();inferred.relation=Relation::DependsOn;
    assert_eq!(store.append_observed_edges(vec![inferred.clone()],binding.clone(),observation_time(2),revision.clone()),Err(Error::Invalid));
    store.append_hypotheses(vec![inferred],binding.clone(),observation_time(2),revision.clone()).unwrap();
    let edges=store.edges(vec![edge.id,"edge:hypothesis".into()],binding,observation_time(2),revision.clone(),ReadPurpose::Current).unwrap();
    assert_eq!(edges[0].certainty,Certainty::Observed);assert_eq!(edges[0].relation,Relation::OrderedAfter);assert_eq!(edges[1].certainty,Certainty::Hypothesis);assert_eq!(edges[1].provider,"reasoning-hypothesis");
    assert_eq!(store.edges(vec!["edge:ordering".into()],"origin:foreign".into(),observation_time(2),revision,ReadPurpose::Current),Err(Error::WrongScope));
}
#[test] fn provider_loss_invalidates_young_evidence_and_tampered_edges_are_refused(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.apply_provider_snapshot(provider_snapshot(None,1,&["a","b"])).unwrap();
    let (mut o,e)=records();o.payload=serde_json::json!({"relations":[{"to_id":"b","relation":"ordered_after"}]});store.append_evidence(o,e).unwrap();
    let edge=EdgeInput{id:"edge:a".into(),from_id:"a".into(),to_id:"b".into(),relation:Relation::OrderedAfter,evidence_id:"evidence:a".into()};
    let binding="origin:1.99:scope:a".to_string();let revision=crate::graph::SourceRevision::default();store.append_observed_edges(vec![edge],binding.clone(),observation_time(2),revision.clone()).unwrap();
    store.report_event_loss();assert_eq!(resolve(&store,observation_time(3),ReadPurpose::Current),Err(Error::StaleEvidence));
    assert_eq!(resolve(&store,observation_time(3),ReadPurpose::Diagnostic).unwrap().freshness,crate::graph::Freshness::Stale);
    drop(store);let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();db.execute_batch("UPDATE edges SET relation='depends_on'").unwrap();drop(db);
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert_eq!(store.edges(vec!["edge:a".into()],binding,observation_time(3),revision,ReadPurpose::Diagnostic),Err(Error::Corrupt));
}
#[test] fn actual_full_queue_returns_bounded_error_and_persists_loss_before_next_read(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();
    let (started,ready)=mpsc::sync_channel(1);let (resume,wait)=mpsc::sync_channel(1);
    // A test-only rendezvous holds the actual actor, allowing deterministic
    // saturation of its real bounded queue without relying on scheduler timing.
    struct Resume(SyncSender<()>);impl Drop for Resume{fn drop(&mut self){let _=self.0.send(());}}
    store.send(Request::TestPause(started,wait)).unwrap();let release=Resume(resume);ready.recv().unwrap();
    let mut replies=Vec::new();for _ in 0..QUEUE{let (reply,receive)=mpsc::sync_channel(1);store.send(Request::Read(vec!["a".into()],reply)).unwrap();replies.push(receive);}
    let (reply,_)=mpsc::sync_channel(1);assert_eq!(store.send(Request::Read(vec!["a".into()],reply)),Err(Error::ResourceExhausted));drop(release);
    for reply in replies{assert_eq!(reply.recv().unwrap().unwrap().len(),1);}
    let state=store.reconciliation_plan("native-systemd".into(),observation_time(2),crate::graph::SourceRevision::default()).unwrap().state.unwrap();
    assert_eq!(state.status,ProviderStatus::ReconcileRequired);assert_eq!(state.error.unwrap()["code"],"event_queue_overflow");
    assert_eq!(store.apply_provider_snapshot(provider_snapshot(Some(state.token),3,&["a"])).unwrap().status,ProviderStatus::Ready);
}
#[test] fn snapshot_provider_scope_count_and_context_bounds_preserve_prior_state(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let first=store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();
    let mut foreign=provider_snapshot(Some(first.token.clone()),2,&["a"]);foreign.nodes[0].scope=Scope::User(1001);assert_eq!(store.apply_provider_snapshot(foreign),Err(Error::WrongScope));
    let mut big=provider_snapshot(Some(first.token.clone()),2,&["a"]);big.nodes=vec![node("a");8193];assert_eq!(store.apply_provider_snapshot(big),Err(Error::ResourceExhausted));
    let mut revision=crate::graph::SourceRevision::default();revision.profile=Some("x".repeat(2049));
    assert_eq!(store.reconciliation_plan("native-systemd".into(),observation_time(2),revision),Err(Error::ResourceExhausted));
    assert_eq!(store.ingest_provider_events("foreign-provider".into(),vec![provider_event("foreign-event")]),Err(Error::WrongScope));
    for n in 0..127{let mut snapshot=provider_snapshot(None,2,&[]);snapshot.provider=format!("provider:{n}");store.apply_provider_snapshot(snapshot).unwrap();}
    let mut excess=provider_snapshot(None,2,&[]);excess.provider="provider:excess".into();assert_eq!(store.apply_provider_snapshot(excess),Err(Error::ResourceExhausted));
    assert_eq!(store.reconciliation_plan("native-systemd".into(),observation_time(2),crate::graph::SourceRevision::default()).unwrap().state.unwrap().token,first.token);
    assert_eq!(store.nodes(vec!["a".into()]).unwrap().len(),1);
}
#[test] fn reopened_owner_requires_resampling_even_in_the_same_boot(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let before=store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();drop(store);
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let plan=store.reconciliation_plan("native-systemd".into(),observation_time(2),crate::graph::SourceRevision::default()).unwrap();
    assert_eq!(plan.reason,Some(ReconcileReason::Invalidated));let state=plan.state.unwrap();assert_eq!(state.error.unwrap()["code"],"graph_owner_started");assert_ne!(state.token,before.token);
    assert_eq!(store.apply_provider_snapshot(provider_snapshot(Some(before.token),3,&["a"])),Err(Error::IdentityChanged));
    assert_eq!(store.apply_provider_snapshot(provider_snapshot(Some(state.token),3,&["a"])).unwrap().status,ProviderStatus::Ready);
}

fn corrupt_owned_graph(temp:&Temporary)->Vec<(String,Vec<u8>)>{
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();drop(store);
    fs::write(temp.0.join("graph.sqlite3"),b"damaged native graph header").unwrap();
    for name in ["graph.sqlite3-wal","graph.sqlite3-shm","graph.sqlite3-journal"]{
        OpenOptions::new().write(true).create_new(true).mode(0o600).open(temp.0.join(name)).unwrap().write_all(name.as_bytes()).unwrap();
    }
    recovery::FILES_FOR_TEST.iter().map(|name|(name.to_string(),fs::read(temp.0.join(name)).unwrap())).collect()
}
#[test] fn corrupt_owned_graph_preserves_every_file_and_rebuilds_unknown(){
    let temp=Temporary::new();let before=corrupt_owned_graph(&temp);
    let ledger=b"ledger must survive graph recovery";fs::write(temp.0.join("transactions.sqlite3"),ledger).unwrap();
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let receipt=store.recovery_receipt().unwrap().clone();
    for (name,bytes) in before {assert_eq!(fs::read(receipt.quarantine_directory.join(name)).unwrap(),bytes);}
    assert_eq!(receipt.files.len(),5);assert!(receipt.quarantine_directory.join("receipt.json").exists());assert!(!temp.0.join("graph.recovery").exists());
    assert_eq!(fs::read(temp.0.join("transactions.sqlite3")).unwrap(),ledger);assert!(store.nodes(vec!["a".into()]).unwrap().is_empty());
    assert_eq!(store.reconciliation_plan("native-systemd".into(),observation_time(2),crate::graph::SourceRevision::default()).unwrap().reason,Some(ReconcileReason::Unknown));
    store.upsert_nodes(vec![node("b")]).unwrap();drop(store);
    let reopened=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert!(reopened.recovery_receipt().is_none());assert_eq!(reopened.nodes(vec!["b".into()]).unwrap().len(),1);
}
#[test] fn recovery_resumes_each_partial_move_without_losing_old_sidecars(){
    for count in 0..=5 {
        let temp=Temporary::new();let before=corrupt_owned_graph(&temp);let plan=recovery::prepare(&temp.0,Scope::User(1000)).unwrap();
        let destination=temp.0.join(format!("graph.quarantine-{}",recovery::nonce_for_test(&plan)));
        for (name,_) in before.iter().take(count){fs::rename(temp.0.join(name),destination.join(name)).unwrap();}
        let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert!(store.recovery_receipt().is_some());
        for (name,bytes) in &before{assert_eq!(fs::read(destination.join(name)).unwrap(),*bytes);}
        assert!(store.nodes(vec!["a".into()]).unwrap().is_empty());
    }
}
#[test] fn interrupted_replacement_reopens_only_a_compatible_new_graph(){
    let temp=Temporary::new();corrupt_owned_graph(&temp);let plan=recovery::prepare(&temp.0,Scope::User(1000)).unwrap();recovery::resume(&temp.0,&plan).unwrap();
    let lock=OpenOptions::new().read(true).write(true).open(temp.0.join("graph.lock")).unwrap();
    let mut replacement=Database::open_locked(&temp.0,Scope::User(1000),lock).unwrap();replacement.write(Scope::User(1000),vec![node("b")]).unwrap();drop(replacement);recovery::stamp(&temp.0,Scope::User(1000)).unwrap();
    let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();assert!(store.recovery_receipt().is_some());assert_eq!(store.nodes(vec!["b".into()]).unwrap().len(),1);
}
#[test] fn unmarked_corruption_wrong_scope_and_substituted_inode_never_reset(){
    let unknown=Temporary::new();let path=unknown.0.join("graph.sqlite3");OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).unwrap().write_all(b"unrecognized damaged store").unwrap();
    let before=fs::read(&path).unwrap();assert!(matches!(GraphStore::open(&unknown.0,Scope::System),Err(Error::Corrupt)));assert_eq!(fs::read(path).unwrap(),before);assert!(!unknown.0.join("graph.recovery").exists());
    let temp=Temporary::new();corrupt_owned_graph(&temp);assert!(matches!(GraphStore::open(&temp.0,Scope::User(1001)),Err(Error::WrongScope)));assert!(!temp.0.join("graph.recovery").exists());
    fs::rename(temp.0.join("graph.sqlite3"),temp.0.join("old.sqlite3")).unwrap();OpenOptions::new().write(true).create_new(true).mode(0o600).open(temp.0.join("graph.sqlite3")).unwrap().write_all(b"another damaged file").unwrap();
    assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::IdentityChanged)));assert!(!temp.0.join("graph.recovery").exists());
}
#[test] fn pending_recovery_refuses_changed_inventory_and_retains_plan(){
    let temp=Temporary::new();let before=corrupt_owned_graph(&temp);recovery::prepare(&temp.0,Scope::User(1000)).unwrap();
    fs::write(temp.0.join("graph.sqlite3-wal"),b"substituted WAL").unwrap();
    assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::IdentityChanged)));assert!(temp.0.join("graph.recovery").exists());assert_eq!(fs::read(temp.0.join("graph.sqlite3")).unwrap(),before[0].1);
}
#[test] fn unsafe_sidecars_and_quarantine_destinations_are_refused(){
    for hard_link in [false,true] {
        let temp=Temporary::new();corrupt_owned_graph(&temp);let original=temp.0.join("graph.sqlite3-wal");fs::rename(&original,temp.0.join("retained-wal")).unwrap();
        if hard_link{fs::hard_link(temp.0.join("retained-wal"),&original).unwrap();}else{symlink(temp.0.join("retained-wal"),&original).unwrap();}
        assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::IdentityChanged)));assert!(!temp.0.join("graph.recovery").exists());
    }
    let temp=Temporary::new();corrupt_owned_graph(&temp);let plan=recovery::prepare(&temp.0,Scope::User(1000)).unwrap();let target=temp.0.join(format!("graph.quarantine-{}",recovery::nonce_for_test(&plan)));
    fs::rename(&target,temp.0.join("retained-quarantine")).unwrap();fs::DirBuilder::new().mode(0o700).create(&target).unwrap();
    assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::IdentityChanged)));assert!(temp.0.join("graph.recovery").exists());
}
#[test] fn structural_graph_corruption_quarantines_but_schema_drift_does_not(){
    use std::io::{Seek,SeekFrom,Write};
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.upsert_nodes(vec![node("a")]).unwrap();drop(store);
    let db=Connection::open(temp.0.join("graph.sqlite3")).unwrap();let page:i64=db.query_row("SELECT rootpage FROM sqlite_master WHERE name='nodes'",[],|r|r.get(0)).unwrap();let size:i64=db.pragma_query_value(None,"page_size",|r|r.get(0)).unwrap();drop(db);
    let mut file=OpenOptions::new().write(true).open(temp.0.join("graph.sqlite3")).unwrap();file.seek(SeekFrom::Start(((page-1)*size) as u64)).unwrap();file.write_all(&[0]).unwrap();file.sync_all().unwrap();drop(file);
    let bytes=fs::read(temp.0.join("graph.sqlite3")).unwrap();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let receipt=store.recovery_receipt().unwrap();assert_eq!(fs::read(receipt.quarantine_directory.join("graph.sqlite3")).unwrap(),bytes);
}

#[test] fn actual_committed_wal_snapshot_is_preserved_before_header_recovery(){
    let temp=Temporary::new();let store=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();store.apply_provider_snapshot(provider_snapshot(None,1,&["a"])).unwrap();
    let names=["graph.sqlite3","graph.sqlite3-wal","graph.sqlite3-shm"];
    let snapshot=names.iter().map(|n|(n.to_string(),fs::read(temp.0.join(n)).unwrap())).collect::<Vec<_>>();
    assert!(snapshot[1].1.len()>32);drop(store);
    for (name,bytes) in &snapshot {let mut f=OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(temp.0.join(name)).unwrap();f.write_all(bytes).unwrap();f.sync_all().unwrap();}
    let mut file=OpenOptions::new().write(true).open(temp.0.join("graph.sqlite3")).unwrap();file.write_all(b"?").unwrap();file.sync_all().unwrap();drop(file);
    let before=names.iter().map(|n|(n.to_string(),fs::read(temp.0.join(n)).unwrap())).collect::<Vec<_>>();
    let rebuilt=GraphStore::open(&temp.0,Scope::User(1000)).unwrap();let receipt=rebuilt.recovery_receipt().unwrap();
    for (name,bytes) in before {assert_eq!(fs::read(receipt.quarantine_directory.join(name)).unwrap(),bytes);}
    assert!(rebuilt.nodes(vec!["a".into()]).unwrap().is_empty());
}
#[test] fn incompatible_interrupted_replacement_blocks_with_archives_intact(){
    let temp=Temporary::new();let before=corrupt_owned_graph(&temp);let plan=recovery::prepare(&temp.0,Scope::User(1000)).unwrap();let receipt=recovery::resume(&temp.0,&plan).unwrap();
    let lock=OpenOptions::new().read(true).write(true).open(temp.0.join("graph.lock")).unwrap();drop(Database::open_locked(&temp.0,Scope::User(1000),lock).unwrap());
    let connection=Connection::open(temp.0.join("graph.sqlite3")).unwrap();connection.pragma_update(None,"user_version",2).unwrap();drop(connection);let replacement=fs::read(temp.0.join("graph.sqlite3")).unwrap();
    assert!(matches!(GraphStore::open(&temp.0,Scope::User(1000)),Err(Error::Incompatible)));assert!(temp.0.join("graph.recovery").exists());assert_eq!(fs::read(temp.0.join("graph.sqlite3")).unwrap(),replacement);
    for (name,bytes) in before {assert_eq!(fs::read(receipt.quarantine_directory.join(name)).unwrap(),bytes);}
}

#[test] fn orphan_sidecar_cannot_attach_to_an_absent_or_empty_unmarked_database(){
    for empty in [false,true] {
        let temp=Temporary::new();let wal=temp.0.join("graph.sqlite3-wal");OpenOptions::new().write(true).create_new(true).mode(0o600).open(&wal).unwrap().write_all(b"orphan native WAL").unwrap();
        if empty {OpenOptions::new().write(true).create_new(true).mode(0o600).open(temp.0.join("graph.sqlite3")).unwrap();}
        assert!(matches!(GraphStore::open(&temp.0,Scope::System),Err(Error::IdentityChanged)));assert_eq!(fs::read(wal).unwrap(),b"orphan native WAL");assert_eq!(temp.0.join("graph.sqlite3").exists(),empty);assert!(!temp.0.join("graph.identity").exists());
    }
}
