//! Native provider snapshots and invalidations. These are observations, never
//! edits to managed manifests or transactions, and never inference requests.
use super::*;
use crate::graph::{ObservationTime, SourceRevision};
use sha2::{Digest,Sha256};
use std::collections::BTreeSet;
const SNAPSHOT_NODES:usize=8192;
const SNAPSHOT_BYTES:usize=8*1024*1024;
const PROVIDERS:i64=128;
const RETAIN_EVENTS:i64=4096;
const PERIOD_NS:u64=900_000_000_000;
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ProviderStatus { Unknown, Ready, Partial, Failed, ReconcileRequired }
impl ProviderStatus {
    fn text(self)->&'static str{match self{Self::Unknown=>"unknown",Self::Ready=>"ready",Self::Partial=>"partial",Self::Failed=>"failed",Self::ReconcileRequired=>"reconcile_required"}}
    fn parse(s:&str)->Result<Self>{match s{"unknown"=>Ok(Self::Unknown),"ready"=>Ok(Self::Ready),"partial"=>Ok(Self::Partial),"failed"=>Ok(Self::Failed),"reconcile_required"=>Ok(Self::ReconcileRequired),_=>Err(Error::Corrupt)}}
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint { invalidations:u64, last_snapshot:Option<ObservationTime>, #[serde(default)] last_observation:Option<ObservationTime>, source_revision:SourceRevision,
    source_truth:Option<SourceTruth>, snapshot_hash:Option<String> }
impl Default for Checkpoint {fn default()->Self{Self{invalidations:0,last_snapshot:None,last_observation:None,source_revision:SourceRevision::default(),source_truth:None,snapshot_hash:None}}}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ProviderState {pub provider:String,pub token:String,pub status:ProviderStatus,pub last_success:Option<u64>,pub error:Option<Value>}
#[derive(Clone,Debug)]
pub struct ProviderSnapshot {pub provider:String,pub expected_token:Option<String>,pub source_truth:SourceTruth,
    pub time:ObservationTime,pub source_revision:SourceRevision,pub complete:bool,pub nodes:Vec<Node>,
    /// Exact absence verified by a native retained identity, never inferred from
    /// exclusion in an incomplete enumeration. No execution authority.
    pub verified_absent_ids:Vec<String>}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EventKind { Changed, Removed, Lost, GenerationChanged, Boot }
#[derive(Clone,Debug)]
pub struct ProviderEvent {pub id:String,pub entity_id:Option<String>,pub kind:EventKind,pub time:ObservationTime,
    pub origin_transaction_id:Option<String>,pub payload:Value}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ReconcileReason { Unknown, Invalidated, Incomplete, BootChanged, RevisionChanged, ClockUnknown, Periodic }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ReconcilePlan {pub state:Option<ProviderState>,pub reason:Option<ReconcileReason>}
fn canonical<T:Serialize>(value:&T)->Result<String>{
    let value=serde_json::to_value(value).map_err(|_|Error::Invalid)?;
    if !bounded_properties(&value){return Err(Error::ResourceExhausted);}
    String::from_utf8(aios_protocol::contracts::canonical_json(&value).map_err(|_|Error::Invalid)?).map_err(|_|Error::Invalid)
}
fn digest(s:&str)->String{format!("{:x}",Sha256::digest(s.as_bytes()))}
fn valid_time(t:&ObservationTime)->bool{t.boot.valid() && t.realtime_ns<=i64::MAX as u64 && t.monotonic_ns<=i64::MAX as u64}
fn valid_token(t:&str)->bool{t.len()==64 && t.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
pub(super) fn validate_context(time:&ObservationTime,revision:&SourceRevision)->Result<()> {
    if !valid_time(time){return Err(Error::Invalid);}
    if [&revision.generation,&revision.profile,&revision.closure_hash,&revision.document_hash].into_iter().flatten().any(|s|s.len()>2048){return Err(Error::ResourceExhausted);}
    canonical(revision)?;Ok(())
}
pub(super) fn validate_snapshot(s:&ProviderSnapshot)->Result<()> {
    if !key(&s.provider,64) || !valid_time(&s.time) || s.expected_token.as_ref().is_some_and(|t|!valid_token(t)){return Err(Error::Invalid);}
    if s.nodes.len().saturating_add(s.verified_absent_ids.len())>SNAPSHOT_NODES {return Err(Error::ResourceExhausted);}
    validate_context(&s.time,&s.source_revision)?;
    let mut total=0usize;let mut ids=BTreeSet::new();let mut identities=BTreeSet::new();
    for n in &s.nodes {
        if !key(&n.id,128) || !key(&n.kind,64) || !key(&n.stable_key,256) || n.provider!=s.provider || n.source_truth!=s.source_truth
            || n.realtime_ns!=s.time.realtime_ns || !ids.insert(&n.id) || !identities.insert((&n.kind,&n.stable_key)){return Err(Error::Invalid);}
        if !bounded_properties(&n.properties){return Err(Error::ResourceExhausted);}
        total=total.checked_add(canonical(&n.properties)?.len()+n.id.len()+n.kind.len()+n.stable_key.len()+256).ok_or(Error::ResourceExhausted)?;
        if total>SNAPSHOT_BYTES{return Err(Error::ResourceExhausted);}
    }
    for id in &s.verified_absent_ids {
        if !key(id,128) || !ids.insert(id){return Err(Error::Invalid);}
        total=total.checked_add(id.len()+64).ok_or(Error::ResourceExhausted)?;
        if total>SNAPSHOT_BYTES{return Err(Error::ResourceExhausted);}
    }
    Ok(())
}
pub(super) fn validate_events(provider:&str,events:&[ProviderEvent])->Result<()> {
    if !key(provider,64) || events.is_empty() || events.len()>BATCH{return Err(Error::Invalid);}
    for e in events {
        if !key(&e.id,128) || !valid_time(&e.time) || e.entity_id.as_ref().is_some_and(|n|!key(n,128))
            || e.origin_transaction_id.as_ref().is_some_and(|n|!key(n,128)) || !bounded_properties(&e.payload){return Err(Error::Invalid);}
        // Bound the entire stored envelope before enqueueing, not just data.
        event_payload(provider,e)?;
    }Ok(())
}
fn event_payload(provider:&str,event:&ProviderEvent)->Result<String>{canonical(&serde_json::json!({"provider":provider,"time":event.time,"data":event.payload}))}
struct Row {cursor:String,status:String,last_success:Option<i64>,error:Option<String>}
impl Row {
    fn checkpoint(&self)->Result<Checkpoint>{if self.cursor.len()>8192{return Err(Error::Corrupt);}serde_json::from_str(&self.cursor).map_err(|_|Error::Corrupt)}
    fn state(&self,provider:&str)->Result<ProviderState>{
        let error=self.error.as_ref().map(|s|{if s.len()>8192{return Err(Error::Corrupt);}serde_json::from_str(s).map_err(|_|Error::Corrupt)}).transpose()?;
        Ok(ProviderState{provider:provider.into(),token:digest(&canonical(&(&self.cursor,&self.status,self.last_success,&self.error))?),status:ProviderStatus::parse(&self.status)?,
            last_success:self.last_success.map(|v|u64::try_from(v).map_err(|_|Error::Corrupt)).transpose()?,error})
    }
}
fn row(connection:&Connection,provider:&str)->Result<Option<Row>>{connection.query_row("SELECT cursor_json,status,last_success,error_json FROM provider_state WHERE provider=?1",[provider],|r|Ok(Row{cursor:r.get(0)?,status:r.get(1)?,last_success:r.get(2)?,error:r.get(3)?})).optional().map_err(sql)}
fn persist(connection:&Connection,provider:&str,checkpoint:&Checkpoint,status:ProviderStatus,last_success:Option<i64>,error:Option<&str>)->Result<()> {
    if row(connection,provider)?.is_none(){let count:i64=connection.query_row("SELECT count(*) FROM provider_state",[],|r|r.get(0)).map_err(sql)?;if count>=PROVIDERS{return Err(Error::ResourceExhausted);}}
    connection.execute("INSERT INTO provider_state(provider,cursor_json,last_success,status,error_json) VALUES(?1,?2,?3,?4,?5)
        ON CONFLICT(provider) DO UPDATE SET cursor_json=excluded.cursor_json,last_success=excluded.last_success,status=excluded.status,error_json=excluded.error_json",
        params![provider,canonical(checkpoint)?,last_success,status.text(),error]).map_err(sql)?;Ok(())
}
impl Database {
    pub(super) fn reconcile_plan(&self,provider:&str,now:&ObservationTime,revision:&SourceRevision)->Result<ReconcilePlan>{
        if !key(provider,64) || !valid_time(now){return Err(Error::Invalid);}canonical(revision)?;
        let Some(row)=row(&self.connection,provider)? else{return Ok(ReconcilePlan{state:None,reason:Some(ReconcileReason::Unknown)});};
        let state=row.state(provider)?;let checkpoint=row.checkpoint()?;
        let reason=if state.status==ProviderStatus::ReconcileRequired {Some(ReconcileReason::Invalidated)}
            else if state.status!=ProviderStatus::Ready {Some(ReconcileReason::Incomplete)}
            else if let Some(old)=checkpoint.last_snapshot {
                if !valid_time(&old){return Err(Error::Corrupt);}
                if old.boot!=now.boot{Some(ReconcileReason::BootChanged)}
                else if checkpoint.source_revision!=*revision{Some(ReconcileReason::RevisionChanged)}
                else {match now.monotonic_ns.checked_sub(old.monotonic_ns){None=>Some(ReconcileReason::ClockUnknown),Some(age) if age>=PERIOD_NS=>Some(ReconcileReason::Periodic),_=>None}}
            }else{Some(ReconcileReason::Unknown)};
        Ok(ReconcilePlan{state:Some(state),reason})
    }
    pub(super) fn snapshot(&mut self,scope:Scope,snapshot:ProviderSnapshot)->Result<ProviderState>{
        validate_snapshot(&snapshot)?;if snapshot.nodes.iter().any(|n|n.scope!=scope){return Err(Error::WrongScope);}
        let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        let previous=row(&tx,&snapshot.provider)?;
        let actual=previous.as_ref().map(|r|r.state(&snapshot.provider).map(|s|s.token)).transpose()?;
        if actual!=snapshot.expected_token{return Err(Error::IdentityChanged);}
        let mut checkpoint=previous.as_ref().map(Row::checkpoint).transpose()?.unwrap_or_default();
        if checkpoint.source_truth.is_some_and(|truth|truth!=snapshot.source_truth){return Err(Error::IdentityChanged);}
        if let Some(old)=checkpoint.last_observation.as_ref().or(checkpoint.last_snapshot.as_ref()) {
            if !valid_time(old){return Err(Error::Corrupt);}
            if old.boot==snapshot.time.boot && snapshot.time.monotonic_ns<old.monotonic_ns{return Err(Error::IdentityChanged);}
        }
        // An incomplete enumeration updates only facts actually observed. It
        // cannot infer absence. An explicit native retained-identity exit can
        // remove exactly that node, after validating scope/provider/truth.
        for id in &snapshot.verified_absent_ids {
            let binding:Option<(i64,String,String)>=tx.query_row("SELECT scope_uid,provider,source_truth FROM nodes WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(sql)?;
            if binding.is_some_and(|(uid,provider,truth)|uid!=scope.uid() || provider!=snapshot.provider || truth!=snapshot.source_truth.text()){return Err(Error::WrongScope);}
            tx.execute("UPDATE nodes SET deleted_at=?2,revision=revision+1 WHERE id=?1 AND deleted_at IS NULL",params![id,snapshot.time.realtime_ns as i64]).map_err(sql)?;
        }
        let mut hashes=Vec::with_capacity(snapshot.nodes.len());
        if snapshot.complete {
            tx.execute("UPDATE nodes SET deleted_at=?1 WHERE provider=?2 AND scope_uid=?3 AND source_truth=?4 AND deleted_at IS NULL",
                params![snapshot.time.realtime_ns as i64,snapshot.provider,scope.uid(),snapshot.source_truth.text()]).map_err(sql)?;
        }
        for n in &snapshot.nodes {
            let json=canonical(&n.properties)?;hashes.push(digest(&canonical(&(&n.id,&n.kind,&n.stable_key,&json))?));
            let changed=tx.execute("INSERT INTO nodes(id,kind,scope_uid,provider,stable_key,properties_json,source_truth,first_seen,last_seen,revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,1)
                ON CONFLICT(id) DO UPDATE SET properties_json=excluded.properties_json,last_seen=max(nodes.last_seen,excluded.last_seen),revision=nodes.revision+1,deleted_at=NULL
                WHERE nodes.scope_uid=excluded.scope_uid AND nodes.kind=excluded.kind AND nodes.provider=excluded.provider AND nodes.stable_key=excluded.stable_key AND nodes.source_truth=excluded.source_truth",
                params![n.id,n.kind,scope.uid(),snapshot.provider,n.stable_key,json,snapshot.source_truth.text(),snapshot.time.realtime_ns as i64]).map_err(sql)?;
            if changed!=1{return Err(Error::IdentityChanged);}
        }
        checkpoint.last_observation=Some(snapshot.time.clone());checkpoint.source_truth=Some(snapshot.source_truth);
        if snapshot.complete {
            hashes.sort(); // Stable complete-snapshot hash ignores order.
            let bytes=serde_json::to_vec(&hashes).map_err(|_|Error::Invalid)?;
            checkpoint.last_snapshot=Some(snapshot.time.clone());checkpoint.source_revision=snapshot.source_revision;
            checkpoint.snapshot_hash=Some(format!("{:x}",Sha256::digest(bytes)));
            persist(&tx,&snapshot.provider,&checkpoint,ProviderStatus::Ready,Some(snapshot.time.realtime_ns as i64),None)?;
        }else{
            // Last complete checkpoint/hash/success remain unchanged; Partial
            // never promotes known subset observations to a complete census.
            persist(&tx,&snapshot.provider,&checkpoint,ProviderStatus::Partial,previous.as_ref().and_then(|r|r.last_success),Some("{\"code\":\"incomplete_snapshot\"}"))?;
        }
        tx.commit().map_err(sql)?;row(&self.connection,&snapshot.provider)?.ok_or(Error::Corrupt)?.state(&snapshot.provider)
    }
    pub(super) fn events(&mut self,scope:Scope,provider:&str,events:Vec<ProviderEvent>)->Result<ProviderState>{
        validate_events(provider,&events)?;let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        let previous=row(&tx,provider)?;let mut checkpoint=previous.as_ref().map(Row::checkpoint).transpose()?.unwrap_or_default();let mut changed=false;
        for event in events {
            if let Some(entity)=&event.entity_id {
                let known:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM nodes WHERE id=?1 AND provider=?2 AND scope_uid=?3)",params![entity,provider,scope.uid()],|r|r.get(0)).map_err(sql)?;
                if !known{return Err(Error::WrongScope);}
            }
            let kind=canonical(&event.kind)?.trim_matches('"').to_owned();let boot=canonical(&event.time.boot)?.trim_matches('"').to_owned();let payload=event_payload(provider,&event)?;
            let old:Option<(Option<String>,String,String,i64,Option<String>,String)>=tx.query_row("SELECT entity_id,event_type,boot_id,timestamp,origin_transaction_id,payload_json FROM events WHERE event_id=?1",[&event.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(sql)?;
            let value=(event.entity_id,kind,boot,event.time.realtime_ns as i64,event.origin_transaction_id,payload);
            if let Some(old)=old {if old!=value{return Err(Error::IdentityChanged);}continue;}
            tx.execute("INSERT INTO events(event_id,entity_id,event_type,boot_id,timestamp,origin_transaction_id,payload_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![event.id,value.0,value.1,value.2,value.3,value.4,value.5]).map_err(sql)?;changed=true;
        }
        if changed {
            checkpoint.invalidations=checkpoint.invalidations.checked_add(1).ok_or(Error::ResourceExhausted)?;
            persist(&tx,provider,&checkpoint,ProviderStatus::ReconcileRequired,previous.as_ref().and_then(|r|r.last_success),Some("{\"code\":\"provider_event\"}"))?;
            tx.execute("DELETE FROM events WHERE sequence <= (SELECT coalesce(max(sequence),0)-?1 FROM events)",[RETAIN_EVENTS]).map_err(sql)?;
        }else if previous.is_none(){return Err(Error::Corrupt);}
        tx.commit().map_err(sql)?;row(&self.connection,provider)?.ok_or(Error::Corrupt)?.state(provider)
    }
    pub(super) fn overflow(&mut self)->Result<()> {self.invalidate_all(false)}
    pub(super) fn owner_started(&mut self)->Result<()> {self.invalidate_all(true)}
    fn invalidate_all(&mut self,owner_started:bool)->Result<()> {
        let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        let providers={let mut q=tx.prepare("SELECT provider FROM provider_state ORDER BY provider LIMIT 129").map_err(sql)?;let rows=q.query_map([],|r|r.get::<_,String>(0)).map_err(sql)?;rows.collect::<std::result::Result<Vec<_>,_>>().map_err(sql)?};
        if providers.len()>PROVIDERS as usize{return Err(Error::Corrupt);}
        for provider in providers {
            let previous=row(&tx,&provider)?.ok_or(Error::Corrupt)?;let mut checkpoint=previous.checkpoint()?;
            checkpoint.invalidations=checkpoint.invalidations.checked_add(1).ok_or(Error::ResourceExhausted)?;
            persist(&tx,&provider,&checkpoint,ProviderStatus::ReconcileRequired,previous.last_success,Some(if owner_started{"{\"code\":\"graph_owner_started\"}"}else{"{\"code\":\"event_queue_overflow\"}"}))?;
        }
        tx.commit().map_err(sql)
    }
}
