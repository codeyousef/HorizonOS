//! Descriptor-only document parsing. This module grants no file authority.
//! The broker must check its grant and source identity before handing off an
//! input descriptor and again before publishing the returned material.
use aios_protocol::contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, io::{Read,Write},
    os::{fd::{AsRawFd,FromRawFd}, unix::fs::MetadataExt}, process::{Command, Stdio}};

pub mod transport;

type Result<T> = std::result::Result<T, ErrorCode>;
pub const MAX_SOURCE: u64 = 50 * 1024 * 1024;
/// An explicit bounded-range request can inspect a larger source. CPU, memory
/// and retained-text limits still apply; this is never automatic indexing.
pub const MAX_RANGE_SOURCE: u64 = 512 * 1024 * 1024;
pub const MAX_TEXT: usize = 2 * 1024 * 1024;
pub const MAX_PAGES: u32 = 200;
pub const MAX_WIRE: u64 = 16 * 1024 * 1024;
const CHUNK_BYTES: usize = 8192;
pub const VERSION: &str = "aios-extract-v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub format: Format,
    pub range: Option<Selection>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum Format { Text, Pdf, Metadata }
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub enum Selection { Lines { start: u64, end: u64 }, Pages { start: u32, end: u32 } }
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum Limitation { SourceLimit, OutputLimit, PageLimit, RangeSelected, UnsupportedFormat,
    InvalidUtf8, EncryptedInteractiveRequired, NoExtractableText, MalformedPdf, LayoutBestEffort }
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub enum Locator {
    Lines { start: u64, end: u64, byte_start: u64, byte_end: u64 },
    /// Physical one-based page number; never a displayed PDF page label.
    Page { number: u32 },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chunk { pub locator: Locator, pub text: String }
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Extraction {
    pub schema_version: u32,
    pub extraction_version: String,
    pub source_bytes: u64,
    /// None means the whole source was not hashed; never a prefix digest.
    pub source_sha256: Option<String>,
    pub complete: bool,
    pub limitations: Vec<Limitation>,
    pub pages: Option<u32>,
    pub chunks: Vec<Chunk>,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 { return Err(ErrorCode::InvalidArgument); }
        match (self.format, self.range) {
            (_, None) => Ok(()),
            (Format::Text, Some(Selection::Lines { start, end })) if start > 0 && end >= start && end <= 9_007_199_254_740_991 && end-start < 100_000 => Ok(()),
            (Format::Pdf, Some(Selection::Pages { start, end })) if start > 0 && end >= start && end-start < MAX_PAGES => Ok(()),
            _ => Err(ErrorCode::InvalidArgument),
        }
    }
}
#[derive(Clone)]
pub(crate) struct PdfTools { pub info: &'static str, pub text: &'static str }
impl PdfTools {
    pub fn compiled() -> Option<Self> {
        Some(Self { info: option_env!("AIOS_PDFINFO")?, text: option_env!("AIOS_PDFTOTEXT")? })
    }
}

fn metadata(file: &File) -> Result<std::fs::Metadata> {
    let m = file.metadata().map_err(|_|ErrorCode::TargetChanged)?;
    // The transferred descriptor must not provide source-write authority.
    let flags = unsafe { nix::libc::fcntl(file.as_raw_fd(), nix::libc::F_GETFL) };
    if flags < 0 || flags & nix::libc::O_PATH != 0 || flags & nix::libc::O_ACCMODE != nix::libc::O_RDONLY || !m.is_file()
        || m.uid() != nix::unistd::geteuid().as_raw() { return Err(ErrorCode::PermissionDenied); }
    Ok(m)
}
fn same(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    (a.dev(),a.ino(),a.uid(),a.mode(),a.len(),a.mtime(),a.mtime_nsec(),a.ctime(),a.ctime_nsec())
        == (b.dev(),b.ino(),b.uid(),b.mode(),b.len(),b.mtime(),b.mtime_nsec(),b.ctime(),b.ctime_nsec())
}
fn empty(size: u64) -> Extraction {
    Extraction { schema_version:1, extraction_version:VERSION.into(), source_bytes:size,
        source_sha256:None, complete:false, limitations:Vec::new(), pages:None, chunks:Vec::new() }
}
fn unsupported(prefix: &[u8]) -> bool {
    [b"PK\x03\x04".as_slice(), b"\x7fELF", b"SQLite format 3\0", b"\x89PNG", b"\xff\xd8\xff", b"GIF87a", b"GIF89a", b"\x1f\x8b", b"7z\xbc\xaf\x27\x1c", b"Rar!", b"MZ"].iter().any(|magic|prefix.starts_with(magic))
        || (prefix.starts_with(b"RIFF") && prefix.get(8..12)==Some(b"WEBP"))
        || prefix.contains(&0)
}
fn secret_material(bytes:&[u8])->bool {
    let prefix=b"-----BEGIN ";
    bytes.windows(prefix.len()).enumerate().any(|(start,part)| {
        if part!=prefix {return false;}
        let label=&bytes[start+prefix.len()..];
        let Some(end)=label.windows(5).take(96).position(|p|p==b"-----") else{return false;};
        let label=&label[..end];
        label.iter().all(|b|b.is_ascii_uppercase() || *b==b' ')
            && (label.ends_with(b"PRIVATE KEY") || label==b"PGP PRIVATE KEY BLOCK")
    })
}
fn scan_secret(tail:&mut Vec<u8>,bytes:&[u8])->Result<()> {
    tail.extend_from_slice(bytes);
    if secret_material(tail) {return Err(ErrorCode::SecretScopeDenied);}
    if tail.len()>128 {tail.drain(..tail.len()-128);}
    Ok(())
}
fn digest(file: &File) -> Result<String> {
    use std::os::unix::fs::FileExt;
    let mut hash=Sha256::new();let mut pos=0u64;let mut buf=[0u8;65536];let mut tail=Vec::new();
    loop {
        let n=file.read_at(&mut buf,pos).map_err(|_|ErrorCode::TargetChanged)?;
        if n==0 {break;}
        pos=pos.checked_add(n as u64).ok_or(ErrorCode::ResourceExhausted)?;
        if pos>MAX_SOURCE {return Err(ErrorCode::TargetChanged);}
        scan_secret(&mut tail,&buf[..n])?;
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}",hash.finalize()))
}
fn push_text(out:&mut Extraction,text:&str,mut line:u64,mut offset:u64) {
    let mut rest=text;
    while !rest.is_empty() {
        let mut end=rest.len().min(CHUNK_BYTES);
        while !rest.is_char_boundary(end) {end-=1;}
        let part=&rest[..end];let newlines=part.bytes().filter(|b|*b==b'\n').count() as u64;
        let last=line+newlines-u64::from(part.ends_with('\n'));
        out.chunks.push(Chunk { locator:Locator::Lines { start:line,end:last,byte_start:offset,byte_end:offset+end as u64 },text:part.into() });
        line+=newlines;offset+=end as u64;rest=&rest[end..];
    }
}
fn text(file:&File,req:&Request,out:&mut Extraction)->Result<()> {
    use std::os::unix::fs::FileExt;
    let (start,end)=match req.range {Some(Selection::Lines { start,end })=>(start,end),_ =>(1,u64::MAX)};
    let scan_limit=if req.range.is_some() {MAX_RANGE_SOURCE}else{MAX_SOURCE};
    // Line numbering starts at the real source beginning. Never invent line
    // numbers by trusting an unverified byte-position hint.
    let mut buf=[0u8;65536];let(mut pos,mut line)=(0u64,1u64);
    let mut selected=Vec::new();let mut began=false;let mut first_line=1;let mut first_byte=0;let mut tail=Vec::new();
    while pos<scan_limit && line<=end {
        let count=buf.len().min((scan_limit-pos) as usize);
        let n=file.read_at(&mut buf[..count],pos).map_err(|_|ErrorCode::TargetChanged)?;
        if n==0 {break;}
        // A range that starts after a raw-key header must not return its body.
        // Include overlap so a header cannot hide across read boundaries.
        scan_secret(&mut tail,&buf[..n])?;
        for &b in &buf[..n] {
            if line>=start && line<=end {
                if !began {first_line=line;first_byte=pos;began=true;}
                if selected.len()==MAX_TEXT {out.limitations.push(Limitation::OutputLimit);break;}
                selected.push(b);
            }
            pos+=1;
            if b==b'\n' {line+=1;}
            if line>end {break;}
        }
        if out.limitations.contains(&Limitation::OutputLimit) {break;}
    }
    if out.source_bytes>MAX_SOURCE {out.limitations.push(Limitation::SourceLimit);}
    if req.range.is_some() {out.limitations.push(Limitation::RangeSelected);}
    let valid=match std::str::from_utf8(&selected) {
        Ok(s)=>s,
        Err(e) if e.error_len().is_none() && out.limitations.contains(&Limitation::OutputLimit) =>std::str::from_utf8(&selected[..e.valid_up_to()]).map_err(|_|ErrorCode::InvalidArgument)?,
        Err(_)=>{out.limitations.push(Limitation::InvalidUtf8);return Ok(());}
    };
    if secret_material(valid.as_bytes()) {return Err(ErrorCode::SecretScopeDenied);}
    push_text(out,valid,first_line,first_byte);
    out.complete=out.limitations.is_empty();Ok(())
}
fn sealed_pdf(file:&File)->Result<File> {
    use std::os::unix::fs::FileExt;
    let fd=unsafe {nix::libc::memfd_create(c"aios-extract-input".as_ptr(),nix::libc::MFD_CLOEXEC|nix::libc::MFD_ALLOW_SEALING)};
    if fd<0 {return Err(ErrorCode::ResourceExhausted);}
    let mut copy=unsafe {File::from_raw_fd(fd)};
    let mut buf=[0u8;65536];let mut offset=0;
    loop {
        let n=file.read_at(&mut buf,offset).map_err(|_|ErrorCode::TargetChanged)?;
        if n==0 {break;}
        offset+=n as u64;if offset>MAX_RANGE_SOURCE {return Err(ErrorCode::TargetChanged);}
        copy.write_all(&buf[..n]).map_err(|_|ErrorCode::ResourceExhausted)?;
    }
    // O_RDONLY alone is insufficient: /proc/self/fd/N can reopen an ordinary
    // file with new access flags. Seals make even that reopened FD immutable.
    let seals=nix::libc::F_SEAL_WRITE|nix::libc::F_SEAL_SHRINK|nix::libc::F_SEAL_GROW|nix::libc::F_SEAL_SEAL;
    if unsafe {nix::libc::fcntl(fd,nix::libc::F_ADD_SEALS,seals)}!=0 {return Err(ErrorCode::ResourceExhausted);}
    File::open(format!("/proc/self/fd/{fd}")).map_err(|_|ErrorCode::ResourceExhausted)
}
fn unique_info_field<'a>(data:&'a str,prefix:&str)->Option<&'a str> {
    let mut values=data.lines().filter_map(|line|line.strip_prefix(prefix));
    let first=values.next()?;
    if values.next().is_some() {return None;}
    Some(first.trim())
}
fn pdf(file:&File,req:&Request,out:&mut Extraction,tools:&PdfTools)->Result<()> {
    if out.source_bytes>MAX_SOURCE {
        out.limitations.push(Limitation::SourceLimit);
        if req.range.is_none() || out.source_bytes>MAX_RANGE_SOURCE {return Ok(());}
    }
    let snapshot=sealed_pdf(file)?;
    if out.source_bytes<=MAX_SOURCE {out.source_sha256=Some(digest(&snapshot)?);}
    // Both tools are compile-pinned; no request can supply a program, option,
    // password or path. Whole-tree bounds are enforced by the worker unit.
    // Only the sealed snapshot reaches the parser. Original descriptors are
    // CLOEXEC, and worker startup disables ptrace/proc access to the parent.
    let mut info=Command::new(tools.info).env_clear().env("LC_ALL","C").arg("/proc/self/fd/0").stdin(Stdio::from(snapshot.try_clone().map_err(|_|ErrorCode::TargetChanged)?)).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|_|ErrorCode::UnsupportedCapability)?;
    let stderr=info.stderr.take().ok_or(ErrorCode::ResourceExhausted)?;
    let errors=std::thread::spawn(move||{let mut data=Vec::new();stderr.take(65537).read_to_end(&mut data).map(|_|data)});
    let mut stdout=Vec::new();let read=info.stdout.take().ok_or(ErrorCode::ResourceExhausted)?.take(65537).read_to_end(&mut stdout);
    if read.is_err() || stdout.len()>65536 {let _=info.kill();}
    let status=info.wait().map_err(|_|ErrorCode::ResourceExhausted)?;
    let stderr=errors.join().map_err(|_|ErrorCode::ResourceExhausted)?.map_err(|_|ErrorCode::ResourceExhausted)?;
    read.map_err(|_|ErrorCode::ResourceExhausted)?;
    if stdout.len()>65536 || stderr.len()>65536 {return Err(ErrorCode::ResourceExhausted);}
    let data=String::from_utf8_lossy(&stdout);
    if stderr.windows(b"Incorrect password".len()).any(|s|s==b"Incorrect password") {out.limitations.push(Limitation::EncryptedInteractiveRequired);return Ok(());}
    if !status.success() {out.limitations.push(Limitation::MalformedPdf);return Ok(());}
    // Metadata strings can contain newlines and fake field labels. Accept
    // only unique control fields, never a first-match page count from a title.
    let Some(encrypted)=unique_info_field(&data,"Encrypted:").and_then(|v|v.split_whitespace().next()).filter(|v|matches!(*v,"yes"|"no")) else {out.limitations.push(Limitation::MalformedPdf);return Ok(());};
    if encrypted=="yes" {out.limitations.push(Limitation::EncryptedInteractiveRequired);return Ok(());}
    let Some(pages)=unique_info_field(&data,"Pages:").and_then(|v|v.parse::<u32>().ok()).filter(|n|*n>0) else {out.limitations.push(Limitation::MalformedPdf);return Ok(());};
    out.pages=Some(pages);
    let (start,end)=match req.range {Some(Selection::Pages {start,end})=>{out.limitations.push(Limitation::RangeSelected);(start,end.min(pages))},_ =>(1,pages.min(MAX_PAGES))};
    if pages>MAX_PAGES && req.range.is_none() {out.limitations.push(Limitation::PageLimit);}
    if start>pages {return Err(ErrorCode::InvalidArgument);}
    let mut child=Command::new(tools.text).env_clear().env("LC_ALL","C").args(["-layout","-enc","UTF-8","-eol","unix","-f",&start.to_string(),"-l",&end.to_string()]).arg("/proc/self/fd/0").arg("-")
        .stdin(Stdio::from(snapshot)).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|_|ErrorCode::UnsupportedCapability)?;
    let mut bytes=Vec::new();let pipe=child.stdout.take().ok_or(ErrorCode::ResourceExhausted)?;
    let read=pipe.take(MAX_TEXT as u64+1).read_to_end(&mut bytes);
    let truncated=bytes.len()>MAX_TEXT;
    if truncated || read.is_err() {let _=child.kill();}
    let status=child.wait().map_err(|_|ErrorCode::ResourceExhausted)?;
    read.map_err(|_|ErrorCode::ResourceExhausted)?;
    if truncated {bytes.truncate(MAX_TEXT);out.limitations.push(Limitation::OutputLimit);}
    else if !status.success() {out.limitations.push(Limitation::MalformedPdf);return Ok(());}
    let text=match std::str::from_utf8(&bytes) {Ok(s)=>s,Err(e) if truncated && e.error_len().is_none()=>std::str::from_utf8(&bytes[..e.valid_up_to()]).map_err(|_|ErrorCode::InvalidArgument)?,_=>return Err(ErrorCode::InvalidArgument)};
    if secret_material(text.as_bytes()) {return Err(ErrorCode::SecretScopeDenied);}
    let parts=text.split_terminator('\u{c}').collect::<Vec<_>>();
    if parts.len()>(end-start+1) as usize {return Err(ErrorCode::InvalidArgument);}
    let mut missing_text=false;
    for (i,part) in parts.into_iter().enumerate() {
        if part.trim().is_empty() {missing_text=true;}
        else {out.chunks.push(Chunk {locator:Locator::Page {number:start+i as u32},text:part.into()});}
    }
    if missing_text || out.chunks.is_empty() {out.limitations.push(Limitation::NoExtractableText);}
    out.complete=out.limitations.is_empty();
    out.limitations.push(Limitation::LayoutBestEffort);
    Ok(())
}

pub(crate) fn extract(file:&File,request:&Request,tools:Option<&PdfTools>)->Result<Extraction> {
    use std::os::unix::fs::FileExt;
    request.validate()?;let before=metadata(file)?;let mut out=empty(before.len());
    let flags=unsafe {nix::libc::fcntl(file.as_raw_fd(),nix::libc::F_GETFD)};
    if flags<0 || unsafe {nix::libc::fcntl(file.as_raw_fd(),nix::libc::F_SETFD,flags|nix::libc::FD_CLOEXEC)}!=0 {return Err(ErrorCode::PermissionDenied);}
    let mut prefix=[0u8;16];let n=file.read_at(&mut prefix,0).map_err(|_|ErrorCode::TargetChanged)?;let prefix=&prefix[..n];
    if request.format==Format::Metadata || (request.format!=Format::Pdf && unsupported(prefix)) || (request.format==Format::Text && prefix.starts_with(b"%PDF-")) {
        out.limitations.push(Limitation::UnsupportedFormat);
    } else {
        match request.format {
            Format::Text if before.len()<=MAX_SOURCE || (request.range.is_some() && before.len()<=MAX_RANGE_SOURCE)=>text(file,request,&mut out)?,
            Format::Text=>out.limitations.push(Limitation::SourceLimit),
            Format::Pdf if prefix.starts_with(b"%PDF-")=>pdf(file,request,&mut out,tools.ok_or(ErrorCode::UnsupportedCapability)?)?,
            _=>out.limitations.push(Limitation::UnsupportedFormat),
        }
    }
    if before.len()<=MAX_SOURCE && out.source_sha256.is_none() {out.source_sha256=Some(digest(file)?);}
    if !same(&before,&metadata(file)?) {return Err(ErrorCode::TargetChanged);}
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn input(bytes:&[u8])->(tempfile::NamedTempFile,File) {let mut t=tempfile::NamedTempFile::new().unwrap();t.write_all(bytes).unwrap();let f=File::open(t.path()).unwrap();(t,f)}
    fn req(range:Option<Selection>)->Request {Request {schema_version:1,format:Format::Text,range}}
    #[test]fn unicode_ranges_keep_exact_bytes_and_line_locations() {
        let bytes="first\nالعربية with English\nlast\n".as_bytes();let (_t,f)=input(bytes);
        let out=extract(&f,&req(Some(Selection::Lines {start:2,end:2})),None).unwrap();
        assert_eq!(out.chunks[0].text,"العربية with English\n");assert_eq!(out.chunks[0].locator,Locator::Lines {start:2,end:2,byte_start:6,byte_end:(bytes.len()-5) as u64});
        assert!(!out.complete);assert_eq!(out.source_sha256,Some(format!("{:x}",Sha256::digest(bytes))));
    }
    #[test]fn a_writable_or_non_regular_input_never_parses() {
        let (t,_)=input(b"data");assert_eq!(extract(t.as_file(),&req(None),None).unwrap_err(),ErrorCode::PermissionDenied);
        let f=File::open("/dev/null").unwrap();assert_eq!(extract(&f,&req(None),None).unwrap_err(),ErrorCode::PermissionDenied);
    }
    #[test]fn limits_and_invalid_encoding_are_explicit() {
        let mut b=vec![b'x';MAX_TEXT-1];b.extend_from_slice("ع".as_bytes());let (_t,f)=input(&b);
        let out=extract(&f,&req(None),None).unwrap();assert!(!out.complete);assert!(out.limitations.contains(&Limitation::OutputLimit));assert_eq!(out.chunks.iter().map(|c|c.text.len()).sum::<usize>(),MAX_TEXT-1);
        let (_t,f)=input(b"valid\n\xff");let out=extract(&f,&req(None),None).unwrap();assert!(out.chunks.is_empty());assert!(out.limitations.contains(&Limitation::InvalidUtf8));
    }
    #[test]fn metadata_is_the_default_for_binary_and_raw_keys_never_escape() {
        for bytes in [b"PK\x03\x04anything".as_slice(),b"SQLite format 3\0",b"\x89PNGabc",b"\x7fELFbin"] {
            let (_t,f)=input(bytes);let out=extract(&f,&req(None),None).unwrap();assert!(!out.complete);assert!(out.chunks.is_empty());
        }
        let key_marker=[b"-----BEGIN ".as_slice(),b"OPENSSH PRIVATE KEY-----"].concat();
        let (_t,f)=input(&key_marker);assert_eq!(extract(&f,&req(None),None).unwrap_err(),ErrorCode::SecretScopeDenied);
    }
    #[test]fn source_limit_has_no_false_full_digest_and_supports_bounded_lines() {
        let (t,f)=input(b"first\nsecond longer line\n");t.as_file().set_len(MAX_SOURCE+1).unwrap();
        let out=extract(&f,&req(None),None).unwrap();assert_eq!(out.source_sha256,None);assert!(out.chunks.is_empty());assert!(out.limitations.contains(&Limitation::SourceLimit));
        let out=extract(&f,&req(Some(Selection::Lines {start:1,end:1})),None).unwrap();assert_eq!(out.chunks[0].text,"first\n");assert_eq!(out.source_sha256,None);
    }
    #[test]fn typed_request_rejects_scope_program_path_and_bad_ranges() {
        assert!(serde_json::from_str::<Request>(r#"{"schema_version":1,"format":"text","range":null,"path":"/etc/passwd"}"#).is_err());
        assert!(req(Some(Selection::Lines {start:2,end:1})).validate().is_err());
        assert!(Request {schema_version:1,format:Format::Pdf,range:Some(Selection::Pages {start:1,end:201})}.validate().is_err());
        assert!(req(Some(Selection::Pages {start:1,end:1})).validate().is_err());
    }
    #[test]fn a_range_cannot_skip_a_private_key_header_or_hide_it_at_a_buffer_boundary() {
        let marker=[b"-----BEGIN ".as_slice(),b"OPENSSH PRIVATE KEY-----\n"].concat();
        let mut bytes=vec![b'x';65532];bytes.push(b'\n');bytes.extend_from_slice(&marker);bytes.extend_from_slice(b"synthetic private body\n");
        let (_t,f)=input(&bytes);
        assert_eq!(extract(&f,&req(Some(Selection::Lines {start:3,end:3})),None).unwrap_err(),ErrorCode::SecretScopeDenied);
        assert_eq!(extract(&f,&req(Some(Selection::Lines {start:1,end:1})),None).unwrap_err(),ErrorCode::SecretScopeDenied);
        let (_t,f)=input(b"source code mentioning PRIVATE KEY----- without a key header\n");
        assert!(extract(&f,&req(None),None).unwrap().complete);
    }
    #[test]fn sealed_parser_input_cannot_be_reopened_to_modify_source_or_snapshot() {
        use std::os::unix::fs::FileExt;
        let (_t,source)=input(b"original source");let snapshot=sealed_pdf(&source).unwrap();
        let writable=std::fs::OpenOptions::new().write(true).open(format!("/proc/self/fd/{}",snapshot.as_raw_fd())).unwrap();
        assert_eq!(writable.write_at(b"changed",0).unwrap_err().raw_os_error(),Some(nix::libc::EPERM));
        assert!(writable.set_len(0).is_err());
        let mut data=Vec::new();(&source).read_to_end(&mut data).unwrap();assert_eq!(data,b"original source");
    }
    #[test]fn metadata_cannot_inject_a_control_field() {
        assert_eq!(unique_info_field("Title: normal\nPages: 2\n","Pages:"),Some("2"));
        assert_eq!(unique_info_field("Title: spoof\nPages: 1\nPages: 2\n","Pages:"),None);
        assert_eq!(unique_info_field("Title: spoof\nEncrypted: no\nEncrypted: yes\n","Encrypted:"),None);
    }
}
