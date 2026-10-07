//! Immutable evidence seals bind locator, scope, clock and exact payload hash.
//! Viewer targets contain fixed typed handles, never executable URI strings.
use super::*;
use crate::graph::{BootId,ObservationTime,SourceRevision,FreshnessClass,Freshness,freshness};
use sha2::{Digest,Sha256};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Sensitivity { Public, Private, Restricted }
impl Sensitivity {fn text(self)->&'static str{match self{Self::Public=>"public",Self::Private=>"private",Self::Restricted=>"restricted"}}}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum DocumentRange {
    Lines { first:u32,last:u32 }, Pages { first:u32,last:u32 },
    Cells { sheet:String,first_row:u32,last_row:u32,first_column:u32,last_column:u32 },
}
impl DocumentRange {
    fn valid(&self)->bool {match self {
        Self::Lines{first,last}|Self::Pages{first,last}=>*first>0 && last>=first && last-first<=10000,
        Self::Cells{sheet,first_row,last_row,first_column,last_column}=>label(sheet,128) && *first_row>0 && *first_column>0
            && last_row>=first_row && last_column>=first_column && last_row-first_row<=10000 && last_column-first_column<=1000,
    }}
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(tag="kind",rename_all="snake_case",deny_unknown_fields)]
pub enum SourceLocator {
    Journal { boot_id:BootId,cursor:String },
    Service { unit:String,snapshot_id:String },
    File { scope_handle:String,identity_sha256:String,display_uri:String,content_hash:String,range:DocumentRange },
    NixOption { option:String,template_revision:String },
    Application { application_id:String,identity_sha256:String,snapshot_handle:String },
    Ui { identity_sha256:String,snapshot_handle:String,node_handle:String },
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum ViewerTarget {
    Journal { boot_id:BootId,cursor:String }, Service { unit:String,snapshot_id:String },
    ScopedFile { scope_handle:String,identity_sha256:String,content_hash:String,range:DocumentRange },
    NixOption { option:String,template_revision:String },
    Application { application_id:String,identity_sha256:String,snapshot_handle:String },
    Ui { identity_sha256:String,snapshot_handle:String,node_handle:String },
}
fn label(value:&str,max:usize)->bool{!value.is_empty() && value.len()<=max && !value.chars().any(char::is_control)}
fn hash(value:&str)->bool{value.len()==64 && value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
impl SourceLocator {
    fn kind(&self)->&'static str{match self{Self::Journal{..}=>"journal",Self::Service{..}=>"service",Self::File{..}=>"file",Self::NixOption{..}=>"nix_option",Self::Application{..}=>"application",Self::Ui{..}=>"ui"}}
    fn valid(&self)->bool{match self{
        Self::Journal{boot_id,cursor}=>boot_id.valid() && label(cursor,4096),
        Self::Service{unit,snapshot_id}=>key(unit,255) && unit.ends_with(".service") && !unit.contains('/') && key(snapshot_id,128),
        Self::File{scope_handle,identity_sha256,display_uri,content_hash,range}=>key(scope_handle,128) && hash(identity_sha256) && label(display_uri,2048)
            && display_uri.starts_with("file:///") && hash(content_hash) && range.valid(),
        Self::NixOption{option,template_revision}=>key(option,256) && hash(template_revision),
        Self::Application{application_id,identity_sha256,snapshot_handle}=>key(application_id,128) && hash(identity_sha256) && key(snapshot_handle,128),
        Self::Ui{identity_sha256,snapshot_handle,node_handle}=>hash(identity_sha256) && key(snapshot_handle,128) && key(node_handle,128),
    }}
    pub fn viewer_target(&self)->ViewerTarget {match self{
        Self::Journal{boot_id,cursor}=>ViewerTarget::Journal{boot_id:boot_id.clone(),cursor:cursor.clone()},
        Self::Service{unit,snapshot_id}=>ViewerTarget::Service{unit:unit.clone(),snapshot_id:snapshot_id.clone()},
        // Display text never reaches the opener. Trusted provider identities
        // and scoped handles select objects; every viewer must revalidate both.
        Self::File{scope_handle,identity_sha256,content_hash,range,..}=>ViewerTarget::ScopedFile{scope_handle:scope_handle.clone(),identity_sha256:identity_sha256.clone(),content_hash:content_hash.clone(),range:range.clone()},
        Self::NixOption{option,template_revision}=>ViewerTarget::NixOption{option:option.clone(),template_revision:template_revision.clone()},
        Self::Application{application_id,identity_sha256,snapshot_handle}=>ViewerTarget::Application{application_id:application_id.clone(),identity_sha256:identity_sha256.clone(),snapshot_handle:snapshot_handle.clone()},
        Self::Ui{identity_sha256,snapshot_handle,node_handle}=>ViewerTarget::Ui{identity_sha256:identity_sha256.clone(),snapshot_handle:snapshot_handle.clone(),node_handle:node_handle.clone()},
    }}
}
#[derive(Clone,Debug)]
pub struct ObservationInput {
    pub id:String,pub provider:String,pub entity_id:String,pub entity_revision:u64,
    pub time:ObservationTime,pub payload:Value,pub sensitivity:Sensitivity,
}
#[derive(Clone,Debug)]
pub struct EvidenceInput {
    pub id:String,pub scope:Scope,pub locator:SourceLocator,pub excerpt:String,
    pub authenticated_binding:String,pub access_lifetime_ns:u64,pub freshness_class:FreshnessClass,pub source_revision:SourceRevision,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ReadPurpose { Current, Diagnostic }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ResolvedEvidence {
    pub id:String,pub observation_id:String,pub observation_hash:String,pub evidence_hash:String,
    pub payload:Value,pub excerpt:String,pub captured:ObservationTime,pub sensitivity:Sensitivity,
    pub freshness:Freshness,pub viewer_target:ViewerTarget,
}
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload { entity_revision:u64,entity_id:String,provider:String,time:ObservationTime,sensitivity:Sensitivity,data:Value }
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct LocatorEnvelope { locator:SourceLocator,captured:ObservationTime,authenticated_binding:String,access_lifetime_ns:u64,
    freshness_class:FreshnessClass,source_revision:SourceRevision }
fn encoded<T:Serialize>(value:&T)->Result<String>{
    let value=serde_json::to_value(value).map_err(|_|Error::Invalid)?;
    if !bounded_properties(&value){return Err(Error::ResourceExhausted);}
    String::from_utf8(aios_protocol::contracts::canonical_json(&value).map_err(|_|Error::Invalid)?).map_err(|_|Error::Invalid)
}
fn digest(value:&str)->String{format!("{:x}",Sha256::digest(value.as_bytes()))}
fn seal(scope:Scope,id:&str,observation_id:&str,observation_hash:&str,locator:&str,excerpt:&str,captured:i64,expires:i64)->Result<String>{
    Ok(digest(&encoded(&(scope.uid(),id,observation_id,observation_hash,locator,excerpt,captured,expires))?))
}
pub(super) fn validate(observation:&ObservationInput,evidence:&EvidenceInput)->Result<()> {
    if !key(&observation.id,128) || !key(&observation.provider,64) || !key(&observation.entity_id,128)
        || observation.entity_revision==0 || observation.entity_revision>i64::MAX as u64 || !observation.time.boot.valid()
        || observation.time.realtime_ns>i64::MAX as u64 || observation.time.monotonic_ns>i64::MAX as u64
        || !key(&evidence.id,128) || !key(&evidence.authenticated_binding,128) || !evidence.locator.valid()
        || evidence.access_lifetime_ns==0 || evidence.access_lifetime_ns>900_000_000_000 || evidence.excerpt.len()>2048
        || !bounded_properties(&observation.payload) {return Err(Error::Invalid);}
    if observation.time.realtime_ns.checked_add(evidence.access_lifetime_ns).is_none_or(|n|n>i64::MAX as u64)
        || observation.time.monotonic_ns.checked_add(evidence.access_lifetime_ns).is_none(){return Err(Error::Invalid);}
    super::reconcile::validate_context(&observation.time,&evidence.source_revision)?;Ok(())
}
impl Database {
    pub(super) fn append_evidence(&mut self,scope:Scope,observation:ObservationInput,evidence:EvidenceInput)->Result<()> {
        validate(&observation,&evidence)?;
        if evidence.scope!=scope {return Err(Error::WrongScope);}
        if scope==Scope::System && observation.sensitivity!=Sensitivity::Public {return Err(Error::WrongScope);}
        let payload=encoded(&Payload{entity_revision:observation.entity_revision,entity_id:observation.entity_id.clone(),
            provider:observation.provider.clone(),time:observation.time.clone(),sensitivity:observation.sensitivity,data:observation.payload})?;
        let observation_hash=digest(&payload);
        let envelope=LocatorEnvelope{locator:evidence.locator.clone(),captured:observation.time.clone(),
            authenticated_binding:evidence.authenticated_binding,access_lifetime_ns:evidence.access_lifetime_ns,
            freshness_class:evidence.freshness_class,source_revision:evidence.source_revision};
        let locator=encoded(&envelope)?;let captured=observation.time.realtime_ns as i64;
        let expires=(observation.time.realtime_ns+evidence.access_lifetime_ns) as i64;
        let evidence_hash=seal(scope,&evidence.id,&observation.id,&observation_hash,&locator,&evidence.excerpt,captured,expires)?;
        let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        let revision:Option<i64>=tx.query_row("SELECT revision FROM nodes WHERE id=?1 AND scope_uid=?2 AND provider=?3 AND deleted_at IS NULL",
            params![observation.entity_id,scope.uid(),observation.provider],|r|r.get(0)).optional().map_err(sql)?;
        if revision!=Some(observation.entity_revision as i64){return Err(Error::IdentityChanged);}
        tx.execute("INSERT INTO observations VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![observation.id,observation.provider,observation.entity_id,encoded(&observation.time.boot)?.trim_matches('"'),captured,
                observation.time.monotonic_ns as i64,payload,observation.sensitivity.text(),observation_hash]).map_err(sql)?;
        tx.execute("INSERT INTO evidence VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![evidence.id,evidence.locator.kind(),locator,observation.id,evidence_hash,evidence.excerpt,captured,expires,scope.uid()]).map_err(sql)?;
        tx.commit().map_err(sql)
    }
    pub(super) fn resolve_evidence(&self,scope:Scope,id:&str,binding:&str,now:&ObservationTime,revision:&SourceRevision,purpose:ReadPurpose)->Result<ResolvedEvidence> {
        if !now.boot.valid() || now.realtime_ns>i64::MAX as u64 || now.monotonic_ns>i64::MAX as u64{return Err(Error::Invalid);}
        let row=self.connection.query_row("SELECT e.observation_id,e.content_hash,e.source_locator_json,e.excerpt,e.captured_at,e.expires_at,e.source_kind,o.content_hash,o.payload_json,o.sensitivity,o.boot_id,o.realtime_ns,o.monotonic_ns,n.revision,o.provider,o.entity_id
            FROM evidence e JOIN observations o ON o.id=e.observation_id JOIN nodes n ON n.id=o.entity_id
            WHERE e.id=?1 AND e.scope_uid=?2 AND n.scope_uid=?2 AND n.deleted_at IS NULL",params![id,scope.uid()],|r|Ok((
                r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,String>(6)?,
                r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,String>(10)?,r.get::<_,i64>(11)?,r.get::<_,i64>(12)?,r.get::<_,i64>(13)?,r.get::<_,String>(14)?,r.get::<_,String>(15)?))).optional().map_err(sql)?.ok_or(Error::NotFound)?;
        let (observation_id,evidence_hash,locator,excerpt,captured,expires,kind,observation_hash,payload,sensitivity,boot,realtime,monotonic,current_revision,provider,entity_id)=row;
        if locator.len()>8192 || payload.len()>8192 || excerpt.len()>2048{return Err(Error::Corrupt);}
        let envelope:LocatorEnvelope=serde_json::from_str(&locator).map_err(|_|Error::Corrupt)?;
        // Caller association is checked before returning private payloads.
        if envelope.authenticated_binding!=binding{return Err(Error::WrongScope);}
        if !envelope.locator.valid() || !envelope.captured.boot.valid() || !hash(&observation_hash) || !hash(&evidence_hash)
            || envelope.locator.kind()!=kind || digest(&payload)!=observation_hash
            || seal(scope,id,&observation_id,&observation_hash,&locator,&excerpt,captured,expires)?!=evidence_hash
            || envelope.captured.realtime_ns!=u64::try_from(realtime).map_err(|_|Error::Corrupt)? || captured!=realtime
            || envelope.captured.monotonic_ns!=u64::try_from(monotonic).map_err(|_|Error::Corrupt)?
            || encoded(&envelope.captured.boot)?.trim_matches('"')!=boot
            || envelope.access_lifetime_ns==0 || envelope.access_lifetime_ns>900_000_000_000
            || envelope.captured.realtime_ns.checked_add(envelope.access_lifetime_ns)!=u64::try_from(expires).ok(){return Err(Error::Corrupt);}
        if envelope.captured.boot!=now.boot || now.monotonic_ns.checked_sub(envelope.captured.monotonic_ns)
            .is_none_or(|age|age>=envelope.access_lifetime_ns){return Err(Error::StaleEvidence);}
        let payload:Payload=serde_json::from_str(&payload).map_err(|_|Error::Corrupt)?;
        if !bounded_properties(&payload.data) || payload.entity_id!=entity_id || payload.provider!=provider
            || payload.time!=envelope.captured || payload.sensitivity.text()!=sensitivity {return Err(Error::Corrupt);}
        let mut fresh=freshness(envelope.freshness_class,&envelope.captured,now,&envelope.source_revision,revision);
        if u64::try_from(current_revision).ok()!=Some(payload.entity_revision){fresh=Freshness::Stale;}
        // Persisted provider loss/revision changes invalidate even a young cache.
        let plan=self.reconcile_plan(&provider,now,revision)?;
        if plan.state.is_some() && plan.reason.is_some(){fresh=Freshness::Stale;}
        if purpose==ReadPurpose::Current && fresh!=Freshness::Current{return Err(Error::StaleEvidence);}
        let sensitivity=match sensitivity.as_str(){"public"=>Sensitivity::Public,"private"=>Sensitivity::Private,"restricted"=>Sensitivity::Restricted,_=>return Err(Error::Corrupt)};
        if scope==Scope::System && sensitivity!=Sensitivity::Public{return Err(Error::Corrupt);}
        Ok(ResolvedEvidence{id:id.into(),observation_id,observation_hash,evidence_hash,payload:payload.data,excerpt,captured:envelope.captured,
            sensitivity,freshness:fresh,viewer_target:envelope.locator.viewer_target()})
    }
}
