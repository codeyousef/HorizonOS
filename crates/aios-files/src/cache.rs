//! Private derived content, never authority. Startup drops material belonging
//! to lost volatile grants. Root purges are transactional; originals are untouched.
use super::{ErrorCode, Owner, Result, identity, open_root, same_root};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{fs::{self, File, OpenOptions}, os::{fd::AsRawFd, unix::fs::{MetadataExt, OpenOptionsExt}}, path::{Path, PathBuf}, time::Duration};

const APP_ID: i64 = 0x484f4643;
const SCHEMA: &str = "
CREATE TABLE cache_scope(singleton INTEGER PRIMARY KEY CHECK(singleton=1), uid INTEGER NOT NULL);
CREATE TABLE documents(handle TEXT PRIMARY KEY, root TEXT NOT NULL, owner TEXT NOT NULL, identity TEXT NOT NULL, expires INTEGER NOT NULL);
CREATE INDEX documents_root ON documents(root);
CREATE TABLE chunks(handle TEXT PRIMARY KEY REFERENCES documents(handle) ON DELETE CASCADE, value TEXT NOT NULL);
CREATE TABLE previews(handle TEXT PRIMARY KEY REFERENCES documents(handle) ON DELETE CASCADE, value TEXT NOT NULL);
CREATE TABLE snippets(handle TEXT PRIMARY KEY REFERENCES documents(handle) ON DELETE CASCADE, value TEXT NOT NULL);
";
fn sql(_: rusqlite::Error) -> ErrorCode { ErrorCode::ResourceExhausted }
fn safe_file(path: &Path, uid: u32) -> Result<Option<(u64,u64)>> {
    let info=match fs::symlink_metadata(path) {
        Ok(info)=>info, Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),
        Err(_)=>return Err(ErrorCode::TargetChanged),
    };
    if !info.is_file() || info.uid()!=uid || info.mode()&0o777!=0o600 || info.nlink()!=1 {
        return Err(ErrorCode::PermissionDenied);
    }
    Ok(Some((info.dev(),info.ino())))
}

pub(crate) struct Cache {
    connection: Connection, directory: File, directory_identity: super::Identity,
    path: PathBuf, file_identity: (u64,u64), uid: u32, lock_identity: (u64,u64), _lock: File,
}
impl Cache {
    pub(crate) fn open(directory: &Path) -> Result<Self> {
        if !directory.is_absolute() { return Err(ErrorCode::InvalidArgument); }
        let uid=nix::unistd::geteuid().as_raw();
        let (pinned,dir_identity)=open_root(directory)?;
        if dir_identity.uid!=uid || dir_identity.mode&0o777!=0o700 { return Err(ErrorCode::PermissionDenied); }
        let path=directory.join("content.sqlite3");
        let lock_path=directory.join("content.lock");
        safe_file(&lock_path,uid)?;
        let lock=OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW|nix::libc::O_CLOEXEC).open(&lock_path).map_err(|_|ErrorCode::PermissionDenied)?;
        let lock_identity=safe_file(&lock_path,uid)?.ok_or(ErrorCode::TargetChanged)?;
        let held=lock.metadata().map_err(|_|ErrorCode::TargetChanged)?;
        if (held.dev(),held.ino())!=lock_identity { return Err(ErrorCode::TargetChanged); }
        // SAFETY: flock operates on this live owned descriptor only. Retaining
        // it prevents two brokers from clearing each other's live task cache.
        if unsafe{nix::libc::flock(lock.as_raw_fd(),nix::libc::LOCK_EX|nix::libc::LOCK_NB)}!=0 {
            return Err(ErrorCode::Conflict);
        }
        let existing=safe_file(&path,uid)?;
        for name in ["content.sqlite3-journal","content.sqlite3-wal","content.sqlite3-shm"] {
            if safe_file(&directory.join(name),uid)?.is_some() && existing.is_none() { return Err(ErrorCode::TargetChanged); }
        }
        if existing.is_none() {
            let file=OpenOptions::new().write(true).create_new(true).mode(0o600)
                .custom_flags(nix::libc::O_NOFOLLOW|nix::libc::O_CLOEXEC).open(&path).map_err(|_|ErrorCode::PermissionDenied)?;
            file.sync_all().map_err(|_|ErrorCode::ResourceExhausted)?;
        }
        let file_identity=safe_file(&path,uid)?.ok_or(ErrorCode::TargetChanged)?;
        if fs::metadata(&path).map_err(|_|ErrorCode::TargetChanged)?.len()>128*1024*1024 { return Err(ErrorCode::ResourceExhausted); }
        let mut connection=Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_WRITE|OpenFlags::SQLITE_OPEN_NOFOLLOW|OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(sql)?;
        if safe_file(&path,uid)?!=Some(file_identity) { return Err(ErrorCode::TargetChanged); }
        connection.busy_timeout(Duration::from_millis(250)).map_err(sql)?;
        let app:i64=connection.pragma_query_value(None,"application_id",|r|r.get(0)).map_err(sql)?;
        let version:i64=connection.pragma_query_value(None,"user_version",|r|r.get(0)).map_err(sql)?;
        let tables:i64=connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0)).map_err(sql)?;
        let objects:i64=connection.query_row("SELECT count(*) FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0)).map_err(sql)?;
        let page_size:i64=connection.pragma_query_value(None,"page_size",|r|r.get(0)).map_err(sql)?;
        if page_size!=4096 || !((app==0 && version==0 && tables==0 && objects==0) || (app==APP_ID && version==1 && tables==5 && objects==6)) {
            return Err(ErrorCode::UnsupportedSchema);
        }
        let integrity:String=connection.query_row("PRAGMA quick_check(1)",[],|r|r.get(0)).map_err(sql)?;
        if integrity!="ok" { return Err(ErrorCode::TargetChanged); }
        let (_,named)=open_root(directory)?;
        if !same_root(&named,&dir_identity) || !same_root(&identity(&pinned)?,&dir_identity) {
            return Err(ErrorCode::TargetChanged);
        }
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL; PRAGMA journal_mode=DELETE; PRAGMA max_page_count=32768;").map_err(sql)?;
        let tx=connection.transaction().map_err(sql)?;
        if version==0 {
            tx.execute_batch(SCHEMA).map_err(sql)?;
            tx.execute("INSERT INTO cache_scope VALUES(1,?1)",[uid]).map_err(sql)?;
            tx.pragma_update(None,"application_id",APP_ID).map_err(sql)?;
            tx.pragma_update(None,"user_version",1).map_err(sql)?;
        } else {
            let owner:i64=tx.query_row("SELECT uid FROM cache_scope WHERE singleton=1",[],|r|r.get(0)).map_err(sql)?;
            if owner!=i64::from(uid) { return Err(ErrorCode::PermissionDenied); }
            // Verify exact tables/indexes, refusing drift before purge.
            for statement in SCHEMA.split(';').map(str::trim).filter(|s|!s.is_empty()) {
                let name=statement.split_whitespace().nth(2).and_then(|token|token.split('(').next()).ok_or(ErrorCode::UnsupportedSchema)?;
                let actual:Option<String>=tx.query_row("SELECT sql FROM sqlite_master WHERE name=?1",[name],|r|r.get(0)).optional().map_err(sql)?;
                if actual.as_deref()!=Some(statement) { return Err(ErrorCode::UnsupportedSchema); }
            }
        }
        // No root/handle grant survives broker restart. Never recover authority
        // from persisted text, process IDs, old boot IDs or a SQLite row.
        tx.execute("DELETE FROM documents",[]).map_err(sql)?;
        tx.commit().map_err(sql)?;
        let value=Self{connection,directory:pinned,directory_identity:dir_identity,path,file_identity,uid,lock_identity,_lock:lock};
        value.verify()?; Ok(value)
    }
    fn verify(&self)->Result<()> {
        let (_,named)=open_root(self.path.parent().ok_or(ErrorCode::TargetChanged)?)?;
        if !same_root(&named,&self.directory_identity) || !same_root(&identity(&self.directory)?,&self.directory_identity)
            || safe_file(&self.path,self.uid)?!=Some(self.file_identity) { return Err(ErrorCode::TargetChanged); }
        if safe_file(&self.path.with_file_name("content.lock"),self.uid)?!=Some(self.lock_identity) { return Err(ErrorCode::TargetChanged); }
        for name in ["content.sqlite3-journal","content.sqlite3-wal","content.sqlite3-shm"] {
            safe_file(&self.path.with_file_name(name),self.uid)?;
        }
        Ok(())
    }
    pub(crate) fn put(&mut self,handle:&str,root:&str,owner:&Owner,identity:&str,expires:u64,text:&str,now:u64)->Result<()> {
        self.verify()?;
        if text.len()>super::MAX_READ_BYTES || expires>i64::MAX as u64 || now>i64::MAX as u64 { return Err(ErrorCode::ResourceExhausted); }
        let owner=serde_json::to_string(owner).map_err(|_|ErrorCode::InvalidArgument)?;
        let tx=self.connection.transaction().map_err(sql)?;
        tx.execute("DELETE FROM documents WHERE expires<=?1",[now as i64]).map_err(sql)?;
        let count:i64=tx.query_row("SELECT count(*) FROM documents WHERE handle<>?1",[handle],|r|r.get(0)).map_err(sql)?;
        if count>=256 { return Err(ErrorCode::ResourceExhausted); }
        tx.execute("INSERT INTO documents VALUES(?1,?2,?3,?4,?5) ON CONFLICT(handle) DO UPDATE SET root=excluded.root,owner=excluded.owner,identity=excluded.identity,expires=excluded.expires",params![handle,root,owner,identity,expires as i64]).map_err(sql)?;
        let preview:String=text.chars().take(8192).collect();
        let snippet:String=text.chars().take(2048).collect();
        for (statement,value) in [("INSERT INTO chunks VALUES(?1,?2) ON CONFLICT(handle) DO UPDATE SET value=excluded.value",text),
            ("INSERT INTO previews VALUES(?1,?2) ON CONFLICT(handle) DO UPDATE SET value=excluded.value",preview.as_str()),
            ("INSERT INTO snippets VALUES(?1,?2) ON CONFLICT(handle) DO UPDATE SET value=excluded.value",snippet.as_str())] {
            tx.execute(statement,params![handle,value]).map_err(sql)?;
        }
        tx.commit().map_err(sql)?; self.verify()
    }
    pub(crate) fn snippet(&self,handle:&str,owner:&Owner,identity:&str,now:u64)->Result<String> {
        self.verify()?;
        let now=i64::try_from(now).map_err(|_|ErrorCode::InvalidArgument)?;
        let owner=serde_json::to_string(owner).map_err(|_|ErrorCode::InvalidArgument)?;
        let value=self.connection.query_row("SELECT s.value FROM snippets s JOIN documents d USING(handle) WHERE d.handle=?1 AND d.owner=?2 AND d.identity=?3 AND d.expires>?4",params![handle,owner,identity,now],|r|r.get(0)).optional().map_err(sql)?.ok_or(ErrorCode::TargetNotFound)?;
        self.verify()?; Ok(value)
    }
    pub(crate) fn purge(&mut self,root:&str)->Result<usize> {
        self.verify()?;
        let tx=self.connection.transaction().map_err(sql)?;
        let count:i64=tx.query_row("SELECT (SELECT count(*) FROM chunks c JOIN documents d USING(handle) WHERE d.root=?1)+(SELECT count(*) FROM previews p JOIN documents d USING(handle) WHERE d.root=?1)+(SELECT count(*) FROM snippets s JOIN documents d USING(handle) WHERE d.root=?1)",[root],|r|r.get(0)).map_err(sql)?;
        tx.execute("DELETE FROM documents WHERE root=?1",[root]).map_err(sql)?;
        tx.commit().map_err(sql)?; self.verify()?; usize::try_from(count).map_err(|_|ErrorCode::TargetChanged)
    }
}
