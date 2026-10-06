//! Filtering System1 observer. No raw UID, journal path or expression is a
//! request argument. Volatile authority/evidence is bound to native bus peers.
use super::Result;
use crate::caller::{CallerIdentity,VerifiedCaller};
use aios_protocol::contracts::{Action,ErrorCode};
use aios_system::journal::{self,Query,Source,Unit,user::NativeUser};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use std::{collections::{BTreeMap,BTreeSet},path::Path};
use time::{OffsetDateTime,format_description::well_known::Rfc3339};
use aios_policy::{CurrentResources,Resource,Scope};

#[derive(Deserialize,Serialize)]#[serde(deny_unknown_fields)]
struct Readers { schema_version:u32, users:Vec<String> }
fn reader(uid:u32)->Result<()> {
    let readers:Readers=crate::native::installed("journal-readers.json")?;
    let bytes=crate::native::read(Path::new("/etc/passwd"),Path::new("/etc"),false,65536)?;
    let passwd=std::str::from_utf8(&bytes).map_err(|_|ErrorCode::PolicyChanged)?;
    authorized_reader(&readers,passwd,uid)
}
fn authorized_reader(readers:&Readers,passwd:&str,uid:u32)->Result<()> {
    if readers.schema_version!=1 || readers.users.len()>256 || readers.users.is_empty()
        || readers.users.iter().collect::<BTreeSet<_>>().len()!=readers.users.len()
        || readers.users.iter().any(|n|n.is_empty() || n.len()>128 || !n.bytes().all(|b|b.is_ascii_alphanumeric() || b"_-".contains(&b))) {
        return Err(ErrorCode::PolicyChanged.into());
    }
    let mut allowed=false;let mut found=BTreeSet::new();
    for line in passwd.lines() {
        let fields:Vec<_>=line.split(':').collect();
        if fields.len()!=7 { return Err(ErrorCode::PolicyChanged.into()); }
        if readers.users.iter().any(|name|name==fields[0]) {
            if !found.insert(fields[0]) { return Err(ErrorCode::PolicyChanged.into()); }
            let actual=fields[2].parse::<u32>().map_err(|_|ErrorCode::PolicyChanged)?;
            if actual==0 || actual==u32::MAX { return Err(ErrorCode::PolicyChanged.into()); }
            if actual==uid { allowed=true; }
        }
    }
    if found.len()!=readers.users.len() { return Err(ErrorCode::PolicyChanged.into()); }
    if !allowed { return Err(ErrorCode::PermissionDenied.into()); } Ok(())
}
struct Cursor {
    owner:CallerIdentity, arguments_sha256:String, query:Query,
    continuation:journal::Continuation, expires:u64,
}
#[derive(Clone)]
enum NativeService { System(String), User(journal::user::unit::UserUnit) }
impl NativeService {
    fn verify(&self,id:&str)->Result<()> {
        match self { Self::System(name)=>{aios_system::services::read_service_status(name,id)?;}, Self::User(unit)=>unit.verify()? }
        Ok(())
    }
    fn query_unit(&self)->Unit { match self { Self::System(name)=>Unit::System(name.clone()), Self::User(unit)=>Unit::User(unit.name().into()) } }
    fn source(&self)->Source { match self { Self::System(_)=>Source::System, Self::User(_)=>Source::User } }
    fn digest(&self)->Result<String> {
        match self { Self::System(name)=>Ok(aios_policy::digest(name)?), Self::User(unit)=>Ok(aios_policy::digest(unit.identity())?) }
    }
}
struct Service { owner:CallerIdentity, unit:NativeService, expires:u64 }
struct Evidence {
    owner:CallerIdentity, expires:u64, payload:Value, cursor:String, boot:String, hash:String,
}
#[derive(Default)]
pub(super) struct JournalState { cursors:BTreeMap<String,Cursor>, evidence:BTreeMap<String,Evidence>, services:BTreeMap<String,Service> }
struct Resources(Vec<Resource>);
impl CurrentResources for Resources {
    fn resolve(&self,field:&str,kind:&str,handle:&str)->std::result::Result<String,ErrorCode> {
        self.0.iter().find(|r|r.field==field && r.kind==kind && r.handle==handle)
            .map(|r|r.identity_sha256.clone()).ok_or(ErrorCode::PermissionDenied)
    }
    fn dynamic_arguments(&self,_:&str,_:&Value,_:&Scope)->std::result::Result<(),ErrorCode> { Err(ErrorCode::UnsupportedCapability) }
}
fn micros(text:&str)->Result<u64> {
    let time=OffsetDateTime::parse(text,&Rfc3339).map_err(|_|ErrorCode::InvalidArgument)?;
    let nanos=time.unix_timestamp_nanos();
    if nanos<0 || nanos%1000!=0 { return Err(ErrorCode::InvalidArgument.into()); }
    u64::try_from(nanos/1000).map_err(|_|ErrorCode::InvalidArgument.into())
}
fn subject(c:&CallerIdentity)->aios_policy::Subject {
    aios_policy::Subject{uid:c.uid,pid:c.pid,start_ticks:c.start_ticks,boot_id:c.boot_id.clone(),
        session:c.session.as_ref().map(|s|aios_policy::Session{id:s.id.clone(),remote:s.remote,kind:s.kind.clone()}),
        client:aios_policy::Client::Bus{sender:c.sender.clone(),bus_id:c.bus_id.clone()}}
}
impl JournalState {
    fn cleanup(&mut self)->Result<()> {
        let now=aios_policy::boottime_ms()?;
        self.cursors.retain(|_,c|c.expires>now);self.evidence.retain(|_,e|e.expires>now);self.services.retain(|_,s|s.expires>now);Ok(())
    }
    pub(super) fn resolve_service(&mut self,caller:&VerifiedCaller,unit:&str)->Result<Value> {
        self.cleanup()?;reader(caller.identity().uid)?;
        aios_system::services::validate_service_name(unit)?;
        if self.services.len()>=4096 || self.services.values().filter(|s|s.owner.uid==caller.identity().uid).count()>=256 {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        let id=uuid::Uuid::new_v4().to_string();
        aios_system::services::read_service_status(unit,&id)?;
        self.services.insert(id.clone(),Service{owner:caller.identity().clone(),unit:NativeService::System(unit.into()),expires:aios_policy::boottime_ms()?.checked_add(30000).ok_or(ErrorCode::ResourceExhausted)?});
        Ok(super::envelope("resolve_log_service",json!({"service_id":id,"expires_after_ms":30000,"scope":"system"})))
    }
    pub(super) fn resolve_user_service(&mut self,caller:&VerifiedCaller,name:&str)->Result<Value> {
        self.cleanup()?;let owner=caller.identity();reader(owner.uid)?;
        aios_system::services::validate_service_name(name)?;
        if self.services.len()>=4096 || self.services.values().filter(|s|s.owner.uid==owner.uid).count()>=256 { return Err(ErrorCode::ResourceExhausted.into()); }
        let native=NativeUser::observe(&owner.sender,&owner.bus_id)?;
        if native.uid()!=owner.uid {return Err(ErrorCode::TargetChanged.into());}
        let unit=native.resolve_unit(name)?;native.verify()?;reader(owner.uid)?;
        let id=uuid::Uuid::new_v4().to_string();
        self.services.insert(id.clone(),Service{owner:owner.clone(),unit:NativeService::User(unit),expires:aios_policy::boottime_ms()?.checked_add(30000).ok_or(ErrorCode::ResourceExhausted)?});
        Ok(super::envelope("resolve_user_log_service",json!({"service_id":id,"expires_after_ms":30000,"scope":"user"})))
    }
    pub(super) fn logs(&mut self,caller:&VerifiedCaller,action:&Action)->Result<Value> {
        self.cleanup()?;let owner=caller.identity();reader(owner.uid)?;
        let mut arguments=action.arguments_value();
        let reference=arguments["cursor"].as_str().map(str::to_owned);
        arguments.as_object_mut().ok_or(ErrorCode::InvalidArgument)?.remove("cursor");
        let digest=aios_policy::digest(&arguments)?;
        let native=NativeUser::observe(&owner.sender,&owner.bus_id)?;
        if native.uid()!=owner.uid { return Err(ErrorCode::TargetChanged.into()); }
        // Service handles must originate at this observer. A user-broker handle
        // or guessed native name cannot silently become a privileged unit scope.
        let service=if let Some(id)=arguments["service_id"].as_str() {
            let service=self.services.get(id).ok_or(ErrorCode::TargetNotFound)?;
            if service.owner!=*owner { return Err(ErrorCode::PermissionDenied.into()); }
            service.unit.verify(id)?;
            Some(service.unit.clone())
        } else {None};
        let source=match arguments["source"].as_str() {
            Some("user")=>Source::User,Some("system")=>Source::System,Some("kernel")=>Source::Kernel,
            None if service.is_some()=>service.as_ref().unwrap().source(),
            _=>return Err(ErrorCode::InvalidArgument.into()),
        };
        let query=if let Some(id)=&reference {
            let cursor=self.cursors.get(id).ok_or(ErrorCode::TargetNotFound)?;
            if cursor.owner!=*owner { return Err(ErrorCode::PermissionDenied.into()); }
            if cursor.arguments_sha256!=digest { return Err(ErrorCode::StaleEvidence.into()); }
            cursor.query.clone()
        } else {
            let until=match arguments["until"].as_str(){Some(t)=>micros(t)?,None=>u64::try_from(OffsetDateTime::now_utc().unix_timestamp_nanos()/1000).map_err(|_|ErrorCode::InvalidArgument)?};
            let since=match arguments["since"].as_str(){Some(t)=>micros(t)?,None=>until.saturating_sub(600_000_000)};
            Query::for_native_user(&native,source,service.as_ref().map(NativeService::query_unit),arguments["boot_id"].as_str().unwrap_or("current"),
                since,until,arguments["priority_max"].as_u64().unwrap_or(7) as u8,
                arguments["max_entries"].as_u64().unwrap_or(20) as usize)?
        };
        let mut resources=Vec::new();
        if let Some(boot)=arguments["boot_id"].as_str() {
            resources.push(Resource{field:"boot_id".into(),kind:"scope-owner-expiry".into(),handle:boot.into(),identity_sha256:aios_policy::digest(&json!({"boot_id":query.boot_id(),"uid":owner.uid,"query":digest}))?});
        }
        if let Some(id)=&reference { resources.push(Resource{field:"cursor".into(),kind:"query-bound-cursor".into(),handle:id.clone(),identity_sha256:digest.clone()}); }
        if let (Some(unit),Some(id))=(&service,arguments["service_id"].as_str()) {
            resources.push(Resource{field:"service_id".into(),kind:"scope-owner-expiry".into(),handle:id.into(),identity_sha256:unit.digest()?});
        }
        let resources=Resources(resources);
        let policy=aios_policy::Policy::new(owner.boot_id.clone(),aios_policy::registry_revision())?;
        let request_id=uuid::Uuid::new_v4().to_string();
        let scope=Scope{actions:BTreeSet::from(["system.logs".into()]),resources:resources.0.iter().cloned().collect(),..Default::default()};
        let intent=policy.authenticated_user_intent(subject(owner),request_id.clone(),&arguments.to_string(),aios_policy::Mode::Ask)?;
        let grant=policy.grant_reads(intent,scope,aios_policy::boottime_ms()?,10_000)?;
        policy.check_read(&grant,&subject(owner),&request_id,action,&resources,aios_policy::boottime_ms()?)?;
        let resume=reference.as_ref().map(|id|&self.cursors.get(id).expect("checked private cursor").continuation);
        let read=journal::read(&query,resume)?;
        native.verify()?;reader(owner.uid)?;
        if let Some(id)=&reference {
            if self.cursors.get(id).is_none_or(|c|c.expires<=aios_policy::boottime_ms().unwrap_or(u64::MAX)) {
                return Err(ErrorCode::StaleEvidence.into());
            }
        }
        if let (Some(unit),Some(id))=(&service,arguments["service_id"].as_str()) {
            if self.services.get(id).is_none_or(|s|s.expires<=aios_policy::boottime_ms().unwrap_or(u64::MAX)) { return Err(ErrorCode::ApprovalExpired.into()); }
            unit.verify(id)?;
        }
        policy.check_read(&grant,&subject(owner),&request_id,action,&resources,aios_policy::boottime_ms()?)?;
        let expires=aios_policy::boottime_ms()?.checked_add(30000).ok_or(ErrorCode::ResourceExhausted)?;
        if self.cursors.len()>=4096 || self.cursors.values().filter(|c|c.owner.uid==owner.uid).count()>=256
            || self.evidence.len()+read.entries.len()+1>8192 || self.evidence.values().filter(|e|e.owner.uid==owner.uid).count()+read.entries.len()+1>1024 {
            return Err(ErrorCode::ResourceExhausted.into());
        }
        let mut rows=Vec::new();let mut records=Vec::new();
        let batch=uuid::Uuid::new_v4().to_string();
        for entry in read.entries {
            let id=uuid::Uuid::new_v4().to_string();
            let payload=json!({"timestamp":entry.timestamp,"priority":entry.priority,"service_id":arguments["service_id"].as_str(),
                "source":entry.source,"message":entry.message.text(),"redacted":entry.message.redacted(),"evidence_id":id});
            let evidence=Evidence{owner:owner.clone(),expires,hash:aios_policy::digest(&payload)?,payload:payload.clone(),
                cursor:entry.locator().to_str().map_err(|_|ErrorCode::PartialResult)?.into(),boot:query.boot_id().into()};
            rows.push(payload);records.push((id,evidence));
        }
        let next=read.continuation.as_ref().map(|_|uuid::Uuid::new_v4().to_string());
        let value=json!({"schema_version":1,"status":"ok","observed_at":OffsetDateTime::now_utc().format(&Rfc3339).map_err(|_|ErrorCode::PartialResult)?,
            "source":{"provider":"aios-native-journal-observer","provider_version":env!("CARGO_PKG_VERSION")},
            "evidence_ids":[batch],"complete":true,"next_cursor":next,"data":{"entries":rows,"missing_scope":[]},"error":null});
        aios_protocol::validation::validate_result("system.logs",&serde_json::to_vec(&value).map_err(|_|ErrorCode::InvalidArgument)?)?;
        // Sanitized payloads only; a batch evidence ID links the selected rows.
        self.evidence.insert(batch,Evidence{owner:owner.clone(),expires,payload:value["data"].clone(),cursor:String::new(),boot:query.boot_id().into(),hash:aios_policy::digest(&value["data"])?});
        for (id,evidence) in records { self.evidence.insert(id,evidence); }
        if let (Some(id),Some(continuation))=(next,read.continuation) {
            self.cursors.insert(id,Cursor{owner:owner.clone(),arguments_sha256:digest,query,continuation,expires});
        }
        Ok(value)
    }
    pub(super) fn evidence(&mut self,caller:&VerifiedCaller,id:&str)->Result<Value> {
        self.cleanup()?;reader(caller.identity().uid)?;
        let evidence=self.evidence.get(id).ok_or(ErrorCode::TargetNotFound)?;
        if evidence.owner!=*caller.identity() { return Err(ErrorCode::PermissionDenied.into()); }
        Ok(super::envelope("journal_evidence",json!({"evidence_id":id,"source_kind":"journal","source_locator":{"cursor":evidence.cursor,"boot_id":evidence.boot},
            "content_sha256":evidence.hash,"payload":evidence.payload,"expires_at_boottime_ms":evidence.expires})))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Synthetic policy fixtures. Installed peer/journal access is qualified
    // separately; these tests do not grant the model account native access.
    const PASSWD:&str="root:x:0:0:root:/root:/bin/sh\nalice:x:1000:100:Alice:/home/alice:/bin/sh\naios-model:x:991:991:Model:/var/lib/aios-model:/bin/false\n";
    fn readers(names:&[&str])->Readers { Readers{schema_version:1,users:names.iter().map(|n|(*n).into()).collect()} }
    #[test]
    fn existence_in_passwd_does_not_authorize_a_system_account() {
        let policy=readers(&["alice"]);
        assert!(authorized_reader(&policy,PASSWD,1000).is_ok());
        for uid in [0,991,1001,u32::MAX] {
            assert_eq!(authorized_reader(&policy,PASSWD,uid).unwrap_err().code,ErrorCode::PermissionDenied);
        }
    }
    #[test]
    fn missing_or_ambiguous_installed_reader_configuration_fails_closed() {
        for policy in [readers(&[]),readers(&["alice","alice"]),readers(&["absent"]),readers(&["root"]),readers(&["alice/../../root"])] {
            assert_eq!(authorized_reader(&policy,PASSWD,1000).unwrap_err().code,ErrorCode::PolicyChanged);
        }
        let duplicate=format!("{PASSWD}alice:x:1001:100:Duplicate:/home/other:/bin/sh\n");
        assert_eq!(authorized_reader(&readers(&["alice"]),&duplicate,1000).unwrap_err().code,ErrorCode::PolicyChanged);
        let mut future=readers(&["alice"]);future.schema_version=2;
        assert_eq!(authorized_reader(&future,PASSWD,1000).unwrap_err().code,ErrorCode::PolicyChanged);
    }
    #[test]
    fn time_bounds_are_timezone_normalized_and_never_silently_rounded() {
        assert_eq!(micros("1970-01-01T03:00:01.000001+03:00").unwrap(),1_000_001);
        assert_eq!(micros("1970-01-01T00:00:00Z").unwrap(),0);
        for invalid in ["1969-12-31T23:59:59Z","1970-01-01T00:00:00.000000001Z","not-a-date"] {
            assert_eq!(micros(invalid).unwrap_err().code,ErrorCode::InvalidArgument);
        }
    }
}
