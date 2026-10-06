//! Recovery operates on a fixed graph file set under the exclusive owner lock.
//! No ledger paths, model paths, deletions or inferred database migrations.
use super::*;
use std::{io::{Read,Write},os::unix::fs::DirBuilderExt,path::PathBuf,ffi::CString};
use sha2::{Digest,Sha256};
const IDENTITY_VERSION:i64=2;
const MARKER:&str="graph.identity";
const PLAN:&str="graph.recovery";
const FILES:[&str;5]=["graph.sqlite3","graph.sqlite3-wal","graph.sqlite3-shm","graph.sqlite3-journal",MARKER];
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {version:i64,scope:i64,identity:identity::Identity}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArchivedFile {name:String,identity:identity::Identity,bytes:u64,sha256:String}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {version:i64,scope:i64,nonce:String,directory_identity:identity::Identity,files:Vec<ArchivedFile>}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct RecoveryReceipt {pub quarantine_directory:PathBuf,pub files:Vec<(String,u64,String)>}
fn uid()->u32{unsafe{libc::geteuid()}}
fn sync_directory(path:&Path)->Result<()>{File::open(path).and_then(|f|f.sync_all()).map_err(|_|Error::Storage)}
fn read_file(path:&Path)->Result<File>{
    let expected=safe_file(path,uid())?.ok_or(Error::IdentityChanged)?;
    let file=OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC).open(path).map_err(|_|Error::Storage)?;
    let meta=file.metadata().map_err(|_|Error::Storage)?;
    if (meta.dev(),meta.ino())!=expected {return Err(Error::IdentityChanged);} Ok(file)
}
fn read_json<T:serde::de::DeserializeOwned>(path:&Path)->Result<T>{
    let mut bytes=Vec::new();read_file(path)?.take(16385).read_to_end(&mut bytes).map_err(|_|Error::Storage)?;
    if bytes.len()>16384 {return Err(Error::ResourceExhausted);}
    serde_json::from_slice(&bytes).map_err(|_|Error::Corrupt)
}
fn read_identity_record<T:serde::de::DeserializeOwned>(path:&Path)->Result<T>{
    #[derive(Deserialize)] struct FormatVersion {version:i64}
    // Inspect and decode the same bounded raw bytes. Converting through Value
    // would collapse duplicate fields before the strict record decoder saw them.
    let value:Box<serde_json::value::RawValue>=read_json(path)?;
    let version:FormatVersion=serde_json::from_str(value.get()).map_err(|_|Error::Corrupt)?;
    if version.version!=IDENTITY_VERSION{return Err(Error::Incompatible);}
    serde_json::from_str(value.get()).map_err(|_|Error::Corrupt)
}
fn write_new<T:Serialize>(path:&Path,value:&T)->Result<()>{
    let bytes=serde_json::to_vec(value).map_err(|_|Error::Invalid)?;
    if bytes.len()>16384 {return Err(Error::ResourceExhausted);}
    let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).custom_flags(libc::O_NOFOLLOW|libc::O_CLOEXEC).open(path).map_err(|_|Error::Storage)?;
    file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|_|Error::Storage)?;
    sync_directory(path.parent().ok_or(Error::Invalid)?)
}
fn inventory(path:&Path,name:&str)->Result<ArchivedFile>{
    let mut file=read_file(path)?;let before=file.metadata().map_err(|_|Error::Storage)?;
    let mut hash=Sha256::new();let mut buffer=[0u8;65536];let mut bytes=0u64;
    loop{let count=file.read(&mut buffer).map_err(|_|Error::Storage)?;if count==0{break;}hash.update(&buffer[..count]);bytes=bytes.checked_add(count as u64).ok_or(Error::ResourceExhausted)?;}
    let after=file.metadata().map_err(|_|Error::Storage)?;
    if bytes!=before.len() || before.len()!=after.len() || before.mtime()!=after.mtime() || before.mtime_nsec()!=after.mtime_nsec()
        || safe_file(path,uid())?!=Some((before.dev(),before.ino())) {return Err(Error::IdentityChanged);}
    Ok(ArchivedFile{name:name.into(),identity:identity::Identity::observe(&file)?,bytes,sha256:format!("{:x}",hash.finalize())})
}
fn verify(path:&Path,entry:&ArchivedFile)->Result<()>{
    let actual=inventory(path,&entry.name)?;
    if actual.identity!=entry.identity || actual.bytes!=entry.bytes || actual.sha256!=entry.sha256 {return Err(Error::IdentityChanged);}Ok(())
}
pub(super) fn verify_stamp(directory:&Path,scope:Scope)->Result<()>{
    if safe_file(&directory.join(MARKER),uid())?.is_none(){return Ok(());}
    let marker:Marker=read_identity_record(&directory.join(MARKER))?;
    if marker.version!=IDENTITY_VERSION {return Err(Error::Incompatible);}
    if marker.scope!=scope.uid(){return Err(Error::WrongScope);}
    if identity::Identity::observe(&read_file(&directory.join("graph.sqlite3"))?)?!=marker.identity{return Err(Error::IdentityChanged);}Ok(())
}
pub(super) fn stamp(directory:&Path,scope:Scope)->Result<()>{
    if safe_file(&directory.join(MARKER),uid())?.is_some(){return verify_stamp(directory,scope);}
    let identity=identity::Identity::observe(&read_file(&directory.join("graph.sqlite3"))?)?;
    write_new(&directory.join(MARKER),&Marker{version:IDENTITY_VERSION,scope:scope.uid(),identity})
}
pub(super) fn damaged_header(directory:&Path)->Result<bool>{
    // Inspect only a marked native graph. SQLite must not examine or recover
    // a damaged header's journals before their exact bytes are preserved.
    if safe_file(&directory.join(MARKER),uid())?.is_none(){return Ok(false);}
    let mut bytes=Vec::new();read_file(&directory.join("graph.sqlite3"))?.take(100).read_to_end(&mut bytes).map_err(|_|Error::Storage)?;
    Ok(bytes.len()<100 || &bytes[..16]!=b"SQLite format 3\0")
}
fn destination(directory:&Path,plan:&Plan)->Result<PathBuf>{
    if plan.version!=IDENTITY_VERSION || plan.nonce.len()!=32 || !plan.nonce.bytes().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {return Err(Error::Corrupt);}
    if plan.files.len()<2 || plan.files.len()>FILES.len() {return Err(Error::Corrupt);}
    let mut seen=std::collections::HashSet::new();
    for file in &plan.files {
        if !FILES.contains(&file.name.as_str()) || !seen.insert(file.name.as_str()) || file.sha256.len()!=64
            || !file.sha256.bytes().all(|c|c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {return Err(Error::Corrupt);}
    }
    if !seen.contains("graph.sqlite3") || !seen.contains(MARKER){return Err(Error::Corrupt);}
    let target=directory.join(format!("graph.quarantine-{}",plan.nonce));private_directory(&target,uid())?;
    if identity::Identity::directory(&target)?!=plan.directory_identity{return Err(Error::IdentityChanged);}Ok(target)
}
pub(super) fn pending(directory:&Path,scope:Scope)->Result<Option<Plan>>{
    if safe_file(&directory.join(PLAN),uid())?.is_none(){return Ok(None);}
    let plan:Plan=read_identity_record(&directory.join(PLAN))?;
    if plan.scope!=scope.uid(){return Err(Error::WrongScope);}destination(directory,&plan)?;Ok(Some(plan))
}
pub(super) fn prepare(directory:&Path,scope:Scope)->Result<Plan>{
    // A healthy validated open seals native inode/scope ownership first.
    // An unmarked corrupt legacy/foreign file is never assumed rebuildable.
    if safe_file(&directory.join(MARKER),uid())?.is_none(){return Err(Error::Corrupt);}
    verify_stamp(directory,scope)?;
    let mut header=[0u8;100];let mut file=read_file(&directory.join("graph.sqlite3"))?;
    let count=file.read(&mut header).map_err(|_|Error::Storage)?;
    if count>=72 && &header[..16]==b"SQLite format 3\0" {
        let app=i64::from(u32::from_be_bytes(header[68..72].try_into().map_err(|_|Error::Corrupt)?));
        let version=i64::from(u32::from_be_bytes(header[60..64].try_into().map_err(|_|Error::Corrupt)?));
        if app!=APPLICATION_ID || version!=VERSION {return Err(Error::Incompatible);}
    }
    let mut files=Vec::new();for name in FILES{let path=directory.join(name);if safe_file(&path,uid())?.is_some(){files.push(inventory(&path,name)?);}}
    let mut random=[0u8;16];File::open("/dev/urandom").and_then(|mut f|f.read_exact(&mut random)).map_err(|_|Error::Storage)?;
    let nonce=random.iter().map(|b|format!("{b:02x}")).collect::<String>();let target=directory.join(format!("graph.quarantine-{nonce}"));
    fs::DirBuilder::new().mode(0o700).create(&target).map_err(|_|Error::Storage)?;sync_directory(directory)?;
    let directory_identity=identity::Identity::directory(&target)?;
    let plan=Plan{version:IDENTITY_VERSION,scope:scope.uid(),nonce,directory_identity,files};
    write_new(&directory.join(PLAN),&plan)?;Ok(plan)
}
fn rename_new(from:&Path,to:&Path)->Result<()>{
    use std::os::unix::ffi::OsStrExt;
    let from=CString::new(from.as_os_str().as_bytes()).map_err(|_|Error::Invalid)?;
    let to=CString::new(to.as_os_str().as_bytes()).map_err(|_|Error::Invalid)?;
    if unsafe{libc::renameat2(libc::AT_FDCWD,from.as_ptr(),libc::AT_FDCWD,to.as_ptr(),libc::RENAME_NOREPLACE)}!=0{return Err(Error::Storage);}Ok(())
}
pub(super) fn resume(directory:&Path,plan:&Plan)->Result<RecoveryReceipt>{
    let target=destination(directory,plan)?;
    // Validate the entire inventory before any move. After all old files are
    // archived, a prior interrupted replacement may already have new files.
    let all_archived=plan.files.iter().all(|e|safe_file(&target.join(&e.name),uid()).ok().flatten().is_some());
    for entry in &plan.files {
        let source=directory.join(&entry.name);let archived=target.join(&entry.name);
        match (safe_file(&source,uid())?,safe_file(&archived,uid())?){
            (Some(_),None)=>{if all_archived{return Err(Error::IdentityChanged);}verify(&source,entry)?;},
            (None,Some(_))=>verify(&archived,entry)?,
            (Some(_),Some(_)) if all_archived && identity::Identity::observe(&read_file(&source)?)?!=entry.identity=>verify(&archived,entry)?,
            _=>return Err(Error::IdentityChanged),
        }
    }
    if !all_archived {
        for name in FILES {if !plan.files.iter().any(|e|e.name==name) && safe_file(&directory.join(name),uid())?.is_some(){return Err(Error::IdentityChanged);}}
        for entry in &plan.files {
            if safe_file(&target.join(&entry.name),uid())?.is_none(){
                verify(&directory.join(&entry.name),entry)?;
                // Flush data before durable renames. No unlink or overwrite.
                read_file(&directory.join(&entry.name))?.sync_all().map_err(|_|Error::Storage)?;
                rename_new(&directory.join(&entry.name),&target.join(&entry.name))?;
                sync_directory(&target)?;sync_directory(directory)?;
            }
        }
    }
    Ok(RecoveryReceipt{quarantine_directory:target,files:plan.files.iter().map(|f|(f.name.clone(),f.bytes,f.sha256.clone())).collect()})
}
pub(super) fn finish(directory:&Path,receipt:&RecoveryReceipt)->Result<()>{
    rename_new(&directory.join(PLAN),&receipt.quarantine_directory.join("receipt.json"))?;
    sync_directory(&receipt.quarantine_directory)?;sync_directory(directory)
}

#[cfg(test)] pub(super) const FILES_FOR_TEST:[&str;5]=FILES;
#[cfg(test)] pub(super) fn nonce_for_test(plan:&Plan)->&str{&plan.nonce}
