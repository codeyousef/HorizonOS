//! Native library qualification with controlled public messages, not installed
//! broker/grant qualification. This test never publishes raw journal contents.
use aios_system::journal::{self, Query, Source, Unit};
use aios_protocol::contracts::ErrorCode;
use serde_json::{json, Value};
use std::{fs, io::Write, os::unix::fs::MetadataExt, process::{Command, Stdio}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use uuid::Uuid;
unsafe extern "C" { fn geteuid() -> u32; }
fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros().try_into().unwrap() }

#[test]
#[ignore="controlled journal writes; run only through verified guest journal-inspection"]
fn native_filters_redaction_and_exact_cursor_continuation() {
    assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|line|line=="ID=nixos"));
    assert_eq!(fs::read_to_string("/etc/aios/guest-role").unwrap().trim(),"development");
    let boot=fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim().to_owned();
    let uid=unsafe { geteuid() };
    let tag=format!("horizon-journal-check-{}",Uuid::new_v4().simple());
    let start=now();
    for message in [format!("{tag} service failed after startup."),format!("{tag} PASSWORD=fake-private-value"),format!("{tag} retry scheduled.")] {
        let mut child=Command::new("/run/current-system/sw/bin/systemd-cat").args(["--identifier",&tag,"--priority","notice"])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        writeln!(child.stdin.take().unwrap(),"{message}").unwrap();assert!(child.wait().unwrap().success());
    }
    // Independent upstream read is limited to the known public fixture tag/UID.
    // Its raw fake secret and fields are never printed or attached as evidence.
    let deadline=Instant::now()+Duration::from_secs(5);
    let original:Vec<Value>=loop {
        let out=Command::new("/run/current-system/sw/bin/journalctl").args(["--quiet","--no-pager","--output=json",
            "--output-fields=MESSAGE,__CURSOR,__REALTIME_TIMESTAMP,_UID,_SYSTEMD_UNIT,_BOOT_ID,PRIORITY",
            &format!("--boot={}",boot.replace('-',"")),"--identifier",&tag,"--lines=3",&format!("_UID={uid}")])
            .stdin(Stdio::null()).output().unwrap();
        assert!(out.stderr.len()<4096);
        assert!(out.status.success(),"fixed journalctl fixture exited {}; diagnostic: {}",out.status,String::from_utf8_lossy(&out.stderr));
        assert!(out.stdout.len()<131072);
        let rows:Vec<Value>=String::from_utf8(out.stdout).unwrap().lines().map(|line|serde_json::from_str(line).unwrap()).collect();
        if rows.len()==3 { break rows; }
        assert!(Instant::now()<deadline,"fixed journal messages were not observed");thread::sleep(Duration::from_millis(20));
    };
    let until=now();
    let query=Query::new(Source::User,None,&boot,start,until,7,200).unwrap();
    let all=journal::read(&query,None).unwrap_or_else(|error| {
        // Fixed storage metadata only; never print journal messages or arbitrary paths.
        let machine=fs::read_to_string("/etc/machine-id").unwrap();
        assert!(machine.trim().len()==32 && machine.trim().bytes().all(|b|b.is_ascii_hexdigit()));
        for root in ["/run/log/journal","/var/log/journal"] {
            let path=std::path::Path::new(root).join(machine.trim());
            for parent in path.ancestors() {
                if let Ok(m)=fs::symlink_metadata(parent) {
                    eprintln!("fixed journal directory {} uid={} gid={} mode={:o} symlink={}",parent.display(),m.uid(),m.gid(),m.mode(),m.is_symlink());
                }
            }
            for name in ["system.journal".to_owned(),format!("user-{uid}.journal")] {
                if let Ok(m)=fs::symlink_metadata(path.join(&name)) {
                    eprintln!("fixed journal file {name} uid={} gid={} mode={:o} bytes={}",m.uid(),m.gid(),m.mode(),m.len());
                }
            }
        }
        panic!("native journal permission failures must not be skipped: {error:?}");
    });
    assert!(all.continuation.is_none(),"controlled recent fixture exceeded the required bound");
    for row in &original {
        assert_eq!(row["_UID"].as_str().unwrap(),uid.to_string());
        assert_eq!(row["_BOOT_ID"].as_str().unwrap(),boot.replace('-',""));
        let cursor=row["__CURSOR"].as_str().unwrap();
        let entry=all.entries.iter().find(|entry|entry.locator().to_bytes()==cursor.as_bytes()).expect("fixture entry missing from native query");
        assert_eq!(entry.priority,5);assert_eq!(entry.source,Source::User);
        if row["MESSAGE"].as_str().unwrap().contains("PASSWORD") {
            assert!(entry.message.redacted());assert!(!entry.message.text().contains("fake-private-value"));
        } else { assert_eq!(entry.message.text(),row["MESSAGE"].as_str().unwrap());assert!(!entry.message.redacted()); }
    }
    let excluded=journal::read(&Query::new(Source::User,None,&boot,start,until,4,200).unwrap(),None).unwrap();
    assert!(excluded.entries.iter().all(|e|!original.iter().any(|r|r["__CURSOR"].as_str().unwrap().as_bytes()==e.locator().to_bytes())));
    let first_time=original[0]["__REALTIME_TIMESTAMP"].as_str().unwrap().parse().unwrap();
    let first_only=journal::read(&Query::new(Source::User,None,&boot,start,first_time,7,200).unwrap(),None).unwrap();
    for row in &original[1..] { assert!(!first_only.entries.iter().any(|e|e.locator().to_bytes()==row["__CURSOR"].as_str().unwrap().as_bytes())); }
    if let Some(unit)=original[0]["_SYSTEMD_UNIT"].as_str() {
        let selected=journal::read(&Query::new(Source::User,Some(Unit::System(unit.into())),&boot,start,until,7,200).unwrap(),None).unwrap();
        assert!(original.iter().all(|row|selected.entries.iter().any(|e|e.locator().to_bytes()==row["__CURSOR"].as_str().unwrap().as_bytes())));
    } else { panic!("required unit-scoped native fixture has no authoritative unit"); }
    let page_query=Query::new(Source::User,None,&boot,start,until,7,1).unwrap();
    let first=journal::read(&page_query,None).unwrap();assert_eq!(first.entries.len(),1);
    let cursor=first.continuation.as_ref().expect("required continuation missing");
    let second=journal::read(&page_query,Some(cursor)).unwrap();assert_eq!(second.entries.len(),1);
    assert_ne!(first.entries[0].locator(),second.entries[0].locator());
    let changed=Query::new(Source::User,None,&boot,start,until,4,1).unwrap();
    assert!(matches!(journal::read(&changed,Some(cursor)),Err(ErrorCode::PermissionDenied)));
    let absent=Query::new(Source::User,None,"11111111-1111-4111-8111-111111111111",start,until,7,200).unwrap();
    assert!(matches!(journal::read(&absent,None),Err(ErrorCode::TargetNotFound)));
    let system=journal::read(&Query::new(Source::System,Some(Unit::System("sshd.service".into())),&boot,0,until,7,5).unwrap(),None).unwrap();
    assert!(!system.entries.is_empty(),"required real system-service journal source missing");
    assert!(system.entries.iter().all(|e|e.source==Source::System));
    let kernel=journal::read(&Query::new(Source::Kernel,None,&boot,0,until,7,5).unwrap(),None).unwrap();
    assert!(!kernel.entries.is_empty(),"required real kernel journal source missing");
    assert!(kernel.entries.iter().all(|e|e.source==Source::Kernel));
    println!("AIOS_NATIVE_JOURNAL={}",json!({"evidence_kind":"real-native-journal-library-controlled-public-messages",
        "installed_broker":false,"uid":uid,"boot_id":boot,"controlled_messages":3,"redaction":true,
        "unit_filter":true,"own_uid_filter":true,"system_source":true,"kernel_source":true,"time_filter":true,"priority_filter":true,
        "entry_limit":true,"exact_cursor_continuation":true,"changed_query_refused":true,"absent_boot_refused":true}));
}
