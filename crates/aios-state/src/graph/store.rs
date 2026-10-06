//! Private rebuildable graph storage. No SQL/address/path from model output is
//! executed. Transaction ledgers are separate and are never opened or reset here.
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::{fs::{self,File,OpenOptions},os::unix::{fs::{MetadataExt,OpenOptionsExt},io::AsRawFd},
    path::Path,sync::mpsc::{self,SyncSender,TrySendError},thread::{self,JoinHandle},time::Duration};

mod evidence;
pub use evidence::{EvidenceInput, ObservationInput, ResolvedEvidence, SourceLocator, ViewerTarget, DocumentRange, Sensitivity, ReadPurpose};

const SCHEMA: &str = include_str!("schema.sql");
const APPLICATION_ID: i64 = 0x484f4752;
const VERSION: i64 = 1;
const QUEUE: usize = 16;
const BATCH: usize = 64;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Invalid, Storage, Corrupt, Incompatible, WrongScope, Busy, ResourceExhausted, IdentityChanged, StaleEvidence, NotFound }
pub type Result<T> = std::result::Result<T,Error>;
fn sql(error: rusqlite::Error) -> Error {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase) => Error::Corrupt,
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => Error::Busy,
        Some(rusqlite::ErrorCode::ConstraintViolation) => Error::Invalid,
        _ => Error::Storage,
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Scope { System, User(u32) }
impl Scope { fn uid(self)->i64 { match self { Self::System=>-1,Self::User(uid)=>i64::from(uid) } } }
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum SourceTruth { Intended, Built, Running, BootSelected, UserApp, Unmanaged }
impl SourceTruth {
    fn text(self)->&'static str { match self { Self::Intended=>"intended",Self::Built=>"built",Self::Running=>"running",
        Self::BootSelected=>"boot_selected",Self::UserApp=>"user_app",Self::Unmanaged=>"unmanaged" } }
}
#[derive(Clone,Debug)]
pub struct Node {
    pub id:String,pub kind:String,pub scope:Scope,pub provider:String,pub stable_key:String,
    pub properties:Value,pub source_truth:SourceTruth,pub realtime_ns:u64,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct StoredNode {
    pub id:String,pub kind:String,pub provider:String,pub stable_key:String,pub properties:Value,
    pub source_truth:SourceTruth,pub first_seen:u64,pub last_seen:u64,pub revision:u64,
}
fn key(s:&str,max:usize)->bool { !s.is_empty() && s.len()<=max && s.bytes().all(|b|b.is_ascii_alphanumeric() || b"-_:./".contains(&b)) }
fn bounded_properties(value:&Value)->bool {
    fn shape(value:&Value,depth:usize,budget:&mut usize)->bool {
        if depth>32 || *budget==0 {return false;} *budget-=1;
        match value {
            Value::String(s)=>s.len()<=8192,
            Value::Array(a)=>a.iter().all(|v|shape(v,depth+1,budget)),
            Value::Object(o)=>o.iter().all(|(k,v)|k.len()<=8192 && shape(v,depth+1,budget)),
            _=>true,
        }
    }
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self,bytes:&[u8])->std::io::Result<usize> {
            if bytes.len()>8192-self.0 {return Err(std::io::Error::other("graph properties exceed bound"));}
            self.0+=bytes.len();Ok(bytes.len())
        }
        fn flush(&mut self)->std::io::Result<()> {Ok(())}
    }
    shape(value,0,&mut 4096) && serde_json::to_writer(Counter(0),value).is_ok()
}
fn safe_file(path:&Path,uid:u32)->Result<Option<(u64,u64)>> {
    let meta=match fs::symlink_metadata(path) { Ok(m)=>m,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(_)=>return Err(Error::Storage) };
    if !meta.is_file() || meta.uid()!=uid || meta.mode()&0o777!=0o600 || meta.nlink()!=1 { return Err(Error::IdentityChanged); }
    Ok(Some((meta.dev(),meta.ino())))
}
fn private_directory(path:&Path,uid:u32)->Result<()> {
    let meta=fs::symlink_metadata(path).map_err(|_|Error::Storage)?;
    if !path.is_absolute() || fs::canonicalize(path).map_err(|_|Error::Storage)?!=path || !meta.is_dir()
        || meta.uid()!=uid || meta.mode()&0o777!=0o700 { return Err(Error::IdentityChanged); }
    Ok(())
}
struct Database { connection:Connection, _lock:File }
impl Database {
    fn open(directory:&Path,scope:Scope)->Result<Self> {
        let uid=unsafe {libc::geteuid()};private_directory(directory,uid)?;
        let lock_path=directory.join("graph.lock");safe_file(&lock_path,uid)?;
        let lock=OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
            .custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC).open(&lock_path).map_err(|_|Error::Storage)?;
        safe_file(&lock_path,uid)?;
        if unsafe {libc::flock(lock.as_raw_fd(),libc::LOCK_EX|libc::LOCK_NB)}!=0 { return Err(Error::Busy); }
        let path=directory.join("graph.sqlite3");
        for name in ["graph.sqlite3-wal","graph.sqlite3-shm","graph.sqlite3-journal"] { safe_file(&directory.join(name),uid)?; }
        let identity=match safe_file(&path,uid)? {
            Some(id)=>id,None=>{
                let file=OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC)
                    .open(&path).map_err(|_|Error::Storage)?;file.sync_all().map_err(|_|Error::Storage)?;
                safe_file(&path,uid)?.ok_or(Error::IdentityChanged)?
            }
        };
        let connection=Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX|OpenFlags::SQLITE_OPEN_NOFOLLOW).map_err(sql)?;
        if safe_file(&path,uid)?!=Some(identity) { return Err(Error::IdentityChanged); }
        connection.busy_timeout(Duration::from_millis(250)).map_err(sql)?;
        // Compatibility and integrity reads precede schema/WAL mutation.
        let application:i64=connection.pragma_query_value(None,"application_id",|row|row.get(0)).map_err(sql)?;
        let version:i64=connection.pragma_query_value(None,"user_version",|row|row.get(0)).map_err(sql)?;
        let tables:i64=connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|row|row.get(0)).map_err(sql)?;
        if !((application==0 && version==0 && tables==0) || (application==APPLICATION_ID && version==VERSION && tables==7)) { return Err(Error::Incompatible); }
        let integrity:String=connection.query_row("PRAGMA quick_check(1)",[],|row|row.get(0)).map_err(sql)?;
        if integrity!="ok" { return Err(Error::Corrupt); }
        if version==VERSION {
            Self::compatible(&connection)?;
            let actual:i64=connection.query_row("SELECT scope_uid FROM graph_metadata WHERE singleton=1",[],|row|row.get(0)).map_err(sql)?;
            if actual!=scope.uid() { return Err(Error::WrongScope); }
        }
        drop(connection);
        let mut connection=Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NO_MUTEX|OpenFlags::SQLITE_OPEN_NOFOLLOW).map_err(sql)?;
        if safe_file(&path,uid)?!=Some(identity) {return Err(Error::IdentityChanged);}
        connection.busy_timeout(Duration::from_millis(250)).map_err(sql)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL;").map_err(sql)?;
        if version==0 {
            let transaction=connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
            transaction.execute_batch(SCHEMA).map_err(sql)?;
            transaction.execute("INSERT INTO graph_metadata VALUES(1,?1)",[scope.uid()]).map_err(sql)?;
            transaction.pragma_update(None,"application_id",APPLICATION_ID).map_err(sql)?;
            transaction.pragma_update(None,"user_version",VERSION).map_err(sql)?;
            transaction.commit().map_err(sql)?;
            Self::compatible(&connection)?;
        }
        let mode:String=connection.query_row("PRAGMA journal_mode=WAL",[],|row|row.get(0)).map_err(sql)?;
        if mode!="wal" { return Err(Error::Incompatible); }
        connection.execute_batch("PRAGMA wal_autocheckpoint=1000; PRAGMA journal_size_limit=8388608;").map_err(sql)?;
        for name in ["graph.sqlite3","graph.sqlite3-wal","graph.sqlite3-shm"] { safe_file(&directory.join(name),uid)?; }
        Ok(Self {connection,_lock:lock})
    }
    fn compatible(connection:&Connection)->Result<()> {
        // Check actual DDL, not only a version marker that may survive drift.
        for statement in SCHEMA.split(';').map(str::trim).filter(|s|!s.is_empty()) {
            let name=statement.split_whitespace().nth(2).ok_or(Error::Incompatible)?;
            let actual:Option<String>=connection.query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",[name],|row|row.get(0)).optional().map_err(sql)?;
            if actual.as_deref()!=Some(statement) { return Err(Error::Incompatible); }
        }
        let violations:i64=connection.query_row("SELECT count(*) FROM pragma_foreign_key_check",[],|row|row.get(0)).map_err(sql)?;
        if violations!=0 { return Err(Error::Corrupt); }
        Ok(())
    }
    fn write(&mut self,scope:Scope,nodes:Vec<Node>)->Result<()> {
        if nodes.is_empty() || nodes.len()>BATCH {return Err(Error::Invalid);}
        let mut encoded=Vec::with_capacity(nodes.len());
        for node in &nodes {
            if node.scope!=scope {return Err(Error::WrongScope);}
            if !key(&node.id,128) || !key(&node.kind,64) || !key(&node.provider,64) || !key(&node.stable_key,256) || node.realtime_ns>i64::MAX as u64 {return Err(Error::Invalid);}
            if !bounded_properties(&node.properties) {return Err(Error::ResourceExhausted);}
            let value=aios_protocol::contracts::canonical_json(&node.properties).map_err(|_|Error::Invalid)?;
            if value.len()>8192 {return Err(Error::ResourceExhausted);}
            encoded.push(String::from_utf8(value).map_err(|_|Error::Invalid)?);
        }
        let tx=self.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql)?;
        for (node,json) in nodes.iter().zip(encoded) {
            let changed=tx.execute("INSERT INTO nodes(id,kind,scope_uid,provider,stable_key,properties_json,source_truth,first_seen,last_seen,revision) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,1)
                ON CONFLICT(id) DO UPDATE SET properties_json=excluded.properties_json,last_seen=excluded.last_seen,revision=nodes.revision+1,deleted_at=NULL
                WHERE nodes.scope_uid=excluded.scope_uid AND nodes.kind=excluded.kind AND nodes.provider=excluded.provider AND nodes.stable_key=excluded.stable_key
                AND nodes.source_truth=excluded.source_truth AND excluded.last_seen>=nodes.last_seen",
                params![node.id,node.kind,scope.uid(),node.provider,node.stable_key,json,node.source_truth.text(),node.realtime_ns as i64]).map_err(sql)?;
            if changed!=1 {return Err(Error::IdentityChanged);}
        }
        tx.commit().map_err(sql)
    }
    fn read(&self,scope:Scope,ids:Vec<String>)->Result<Vec<StoredNode>> {
        if ids.is_empty() || ids.len()>BATCH || ids.iter().any(|s|!key(s,128)) {return Err(Error::Invalid);}
        let mut result=Vec::with_capacity(ids.len());
        for id in ids {
            let node=self.connection.query_row("SELECT id,kind,provider,stable_key,properties_json,source_truth,first_seen,last_seen,revision FROM nodes WHERE id=?1 AND scope_uid=?2 AND deleted_at IS NULL",
                params![id,scope.uid()],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,i64>(6)?,row.get::<_,i64>(7)?,row.get::<_,i64>(8)?))).optional().map_err(sql)?;
            if let Some((id,kind,provider,stable_key,json,truth,first_seen,last_seen,revision))=node {
                if json.len()>8192 {return Err(Error::Corrupt);}
                let properties=serde_json::from_str(&json).map_err(|_|Error::Corrupt)?;
                if !bounded_properties(&properties) {return Err(Error::Corrupt);}
                let source_truth=match truth.as_str(){"intended"=>SourceTruth::Intended,"built"=>SourceTruth::Built,"running"=>SourceTruth::Running,
                    "boot_selected"=>SourceTruth::BootSelected,"user_app"=>SourceTruth::UserApp,"unmanaged"=>SourceTruth::Unmanaged,_=>return Err(Error::Corrupt)};
                let first_seen=u64::try_from(first_seen).map_err(|_|Error::Corrupt)?;
                let last_seen=u64::try_from(last_seen).map_err(|_|Error::Corrupt)?;
                let revision=u64::try_from(revision).map_err(|_|Error::Corrupt)?;
                result.push(StoredNode{id,kind,provider,stable_key,properties,source_truth,first_seen,last_seen,revision});
            }
        }
        Ok(result)
    }
}
enum Request { Write(Vec<Node>,SyncSender<Result<()>>), Read(Vec<String>,SyncSender<Result<Vec<StoredNode>>>),
    AppendEvidence(ObservationInput,EvidenceInput,SyncSender<Result<()>>),
    ResolveEvidence(String,String,super::ObservationTime,super::SourceRevision,ReadPurpose,SyncSender<Result<ResolvedEvidence>>) }
pub struct GraphStore { sender:Option<SyncSender<Request>>,worker:Option<JoinHandle<()>> }
impl GraphStore {
    /// The trusted owner selects Scope from authenticated native identity.
    /// Callers must not choose database scope from model request fields.
    pub fn open(directory:&Path,scope:Scope)->Result<Self> {
        let mut db=Database::open(directory,scope)?;let (sender,receiver)=mpsc::sync_channel(QUEUE);
        let worker=thread::Builder::new().name("aios-graph-store".into()).spawn(move||{
            while let Ok(request)=receiver.recv(){match request {
                Request::Write(nodes,reply)=>{let _=reply.send(db.write(scope,nodes));},
                Request::Read(ids,reply)=>{let _=reply.send(db.read(scope,ids));},
                Request::AppendEvidence(observation,evidence,reply)=>{let _=reply.send(db.append_evidence(scope,observation,evidence));},
                Request::ResolveEvidence(id,binding,now,revision,purpose,reply)=>{let _=reply.send(db.resolve_evidence(scope,&id,&binding,&now,&revision,purpose));},
            }}
        }).map_err(|_|Error::Storage)?;
        Ok(Self{sender:Some(sender),worker:Some(worker)})
    }
    fn send(&self,request:Request)->Result<()> {
        self.sender.as_ref().ok_or(Error::Storage)?.try_send(request).map_err(|error|match error {
            TrySendError::Full(_)=>Error::ResourceExhausted,TrySendError::Disconnected(_)=>Error::Storage,
        })
    }
    pub fn upsert_nodes(&self,nodes:Vec<Node>)->Result<()> {
        // Reject oversized allocations before enqueueing. Payload validation is
        // repeated by the sole worker inside the transaction boundary.
        if nodes.is_empty() || nodes.len()>BATCH || nodes.iter().any(|n| n.id.len()>128 || n.kind.len()>64 || n.provider.len()>64 || n.stable_key.len()>256
            || !bounded_properties(&n.properties)) {return Err(Error::ResourceExhausted);}
        let (reply,receiver)=mpsc::sync_channel(1);self.send(Request::Write(nodes,reply))?;receiver.recv().map_err(|_|Error::Storage)?
    }
    pub fn nodes(&self,ids:Vec<String>)->Result<Vec<StoredNode>> {
        if ids.is_empty() || ids.len()>BATCH || ids.iter().any(|id|id.len()>128) {return Err(Error::ResourceExhausted);}
        let (reply,receiver)=mpsc::sync_channel(1);self.send(Request::Read(ids,reply))?;receiver.recv().map_err(|_|Error::Storage)?
    }
    /// Bindings are selected by trusted code from the originating native client
    /// and active read scope. They are never accepted as model-issued grants.
    pub fn append_evidence(&self,observation:ObservationInput,evidence:EvidenceInput)->Result<()> {
        evidence::validate(&observation,&evidence)?;
        let (reply,receiver)=mpsc::sync_channel(1);self.send(Request::AppendEvidence(observation,evidence,reply))?;receiver.recv().map_err(|_|Error::Storage)?
    }
    /// This returns an evidence/viewer descriptor, not permission to open the
    /// source or execute an effect. Native viewers must recheck live authority.
    pub fn resolve_evidence(&self,id:String,authenticated_binding:String,now:super::ObservationTime,
        live_revision:super::SourceRevision,purpose:ReadPurpose)->Result<ResolvedEvidence> {
        if !key(&id,128) || !key(&authenticated_binding,128) {return Err(Error::Invalid);}
        let (reply,receiver)=mpsc::sync_channel(1);
        self.send(Request::ResolveEvidence(id,authenticated_binding,now,live_revision,purpose,reply))?;receiver.recv().map_err(|_|Error::Storage)?
    }
}
impl Drop for GraphStore {fn drop(&mut self){self.sender.take();if let Some(worker)=self.worker.take(){let _=worker.join();}}}

#[cfg(test)] mod tests;
