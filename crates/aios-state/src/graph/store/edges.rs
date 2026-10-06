//! Bounded evidence-backed relations. Native declared facts and model hypotheses
//! have separate insertion APIs and remain distinguishable on every read.
use super::*;
use crate::graph::{ObservationTime,SourceRevision};
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Relation { ProvidedBy, ConfiguredBy, StartedBy, DependsOn, Owns, MountedAt, UsesDriver, RoutesTo, ObservedWith, OrderedAfter }
impl Relation {
    fn text(self)->&'static str{match self{Self::ProvidedBy=>"provided_by",Self::ConfiguredBy=>"configured_by",Self::StartedBy=>"started_by",Self::DependsOn=>"depends_on",Self::Owns=>"owns",Self::MountedAt=>"mounted_at",Self::UsesDriver=>"uses_driver",Self::RoutesTo=>"routes_to",Self::ObservedWith=>"observed_with",Self::OrderedAfter=>"ordered_after"}}
    fn parse(s:&str)->Result<Self>{match s{"provided_by"=>Ok(Self::ProvidedBy),"configured_by"=>Ok(Self::ConfiguredBy),"started_by"=>Ok(Self::StartedBy),"depends_on"=>Ok(Self::DependsOn),"owns"=>Ok(Self::Owns),"mounted_at"=>Ok(Self::MountedAt),"uses_driver"=>Ok(Self::UsesDriver),"routes_to"=>Ok(Self::RoutesTo),"observed_with"=>Ok(Self::ObservedWith),"ordered_after"=>Ok(Self::OrderedAfter),_=>Err(Error::Corrupt)}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Certainty { Observed, Hypothesis }
impl Certainty {fn text(self)->&'static str{match self{Self::Observed=>"observed",Self::Hypothesis=>"hypothesis"}}}
#[derive(Clone,Debug)]
pub struct EdgeInput {pub id:String,pub from_id:String,pub to_id:String,pub relation:Relation,pub evidence_id:String}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct StoredEdge {pub id:String,pub from_id:String,pub to_id:String,pub relation:Relation,pub provider:String,pub evidence_id:String,
    pub certainty:Certainty,pub observed_at:u64,pub freshness:crate::graph::Freshness}
pub(super) fn valid(edges:&[EdgeInput])->bool{!edges.is_empty() && edges.len()<=BATCH && edges.iter().all(|e|key(&e.id,128)&&key(&e.from_id,128)&&key(&e.to_id,128)&&key(&e.evidence_id,128))}
fn declared(payload:&Value,edge:&EdgeInput)->bool {
    // A citation's existence is not proof of a relation. Only exact relations
    // explicitly declared in the sealed native observation can be Observed.
    payload.get("relations").and_then(Value::as_array).is_some_and(|relations|relations.len()<=BATCH && relations.iter().any(|r|r.as_object().is_some_and(|r|r.len()==2 && r.get("to_id")==Some(&Value::String(edge.to_id.clone())) && r.get("relation")==Some(&Value::String(edge.relation.text().into())))))
}
impl Database {
    fn edge_source(&self,scope:Scope,edge:&EdgeInput)->Result<(String,i64)> {
        self.connection.query_row("SELECT o.provider,e.expires_at FROM evidence e JOIN observations o ON o.id=e.observation_id
            JOIN nodes a ON a.id=o.entity_id JOIN nodes b ON b.id=?1
            WHERE e.id=?2 AND o.entity_id=?3 AND a.scope_uid=?4 AND b.scope_uid=?4 AND e.scope_uid=?4 AND a.deleted_at IS NULL AND b.deleted_at IS NULL",
            params![edge.to_id,edge.evidence_id,edge.from_id,scope.uid()],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(sql)?.ok_or(Error::WrongScope)
    }
    pub(super) fn append_edges(&mut self,scope:Scope,edges:Vec<EdgeInput>,certainty:Certainty,binding:&str,now:&ObservationTime,revision:&SourceRevision)->Result<()> {
        if !valid(&edges){return Err(Error::Invalid);}
        let mut sealed=Vec::with_capacity(edges.len());
        for edge in &edges {
            let evidence=self.resolve_evidence(scope,&edge.evidence_id,binding,now,revision,ReadPurpose::Current)?;
            if certainty==Certainty::Observed && !declared(&evidence.payload,edge){return Err(Error::Invalid);}
            let (provider,expires)=self.edge_source(scope,edge)?;
            let provider=if certainty==Certainty::Observed{provider}else{"reasoning-hypothesis".into()};
            sealed.push((provider,evidence.captured.realtime_ns as i64,expires));
        }
        let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        for (edge,(provider,captured,expires)) in edges.into_iter().zip(sealed){
            tx.execute("INSERT INTO edges VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![edge.id,edge.from_id,edge.relation.text(),edge.to_id,provider,edge.evidence_id,captured,expires,certainty.text()]).map_err(sql)?;
        }
        tx.commit().map_err(sql)
    }
    pub(super) fn read_edges(&self,scope:Scope,ids:Vec<String>,binding:&str,now:&ObservationTime,revision:&SourceRevision,purpose:ReadPurpose)->Result<Vec<StoredEdge>> {
        if ids.is_empty() || ids.len()>BATCH || ids.iter().any(|id|!key(id,128)){return Err(Error::Invalid);}
        let mut result=Vec::new();
        for id in ids {
            let row=self.connection.query_row("SELECT e.from_id,e.to_id,e.relation,e.provider,e.evidence_id,e.observed_at,e.valid_until,e.certainty FROM edges e
                JOIN nodes a ON a.id=e.from_id JOIN nodes b ON b.id=e.to_id WHERE e.id=?1 AND a.scope_uid=?2 AND b.scope_uid=?2 AND a.deleted_at IS NULL AND b.deleted_at IS NULL",
                params![id,scope.uid()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?))).optional().map_err(sql)?;
            let Some((from_id,to_id,relation,provider,evidence_id,captured,expires,certainty))=row else{continue;};
            let relation=Relation::parse(&relation)?;let certainty=match certainty.as_str(){"observed"=>Certainty::Observed,"hypothesis"=>Certainty::Hypothesis,_=>return Err(Error::Corrupt)};
            let edge=EdgeInput{id:id.clone(),from_id:from_id.clone(),to_id:to_id.clone(),relation,evidence_id:evidence_id.clone()};
            if !valid(std::slice::from_ref(&edge)){return Err(Error::Corrupt);}
            let evidence=self.resolve_evidence(scope,&evidence_id,binding,now,revision,purpose)?;
            let (native_provider,native_expires)=self.edge_source(scope,&edge)?;
            if captured!=i64::try_from(evidence.captured.realtime_ns).map_err(|_|Error::Corrupt)? || expires!=native_expires
                || (certainty==Certainty::Observed && (provider!=native_provider || !declared(&evidence.payload,&edge)))
                || (certainty==Certainty::Hypothesis && provider!="reasoning-hypothesis"){return Err(Error::Corrupt);}
            result.push(StoredEdge{id,from_id,to_id,relation,provider,evidence_id,certainty,observed_at:evidence.captured.realtime_ns,freshness:evidence.freshness});
        }Ok(result)
    }
}
