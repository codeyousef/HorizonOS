//! Real installed System1 qualification. Writes only controlled public fixtures;
//! independent journalctl reads are restricted to that tag and the native UID.
use serde_json::{json,Value};
use std::{fs,io::Write,process::{Command,Stdio},thread,time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
use time::{OffsetDateTime,format_description::well_known::Rfc3339};
use zbus::blocking::{Connection,Proxy};
fn now()->u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros().try_into().unwrap() }
fn timestamp(value:u64)->String { OffsetDateTime::from_unix_timestamp_nanos(i128::from(value)*1000).unwrap().format(&Rfc3339).unwrap() }
fn request(arguments:Value)->String {
    json!({"schema_version":1,"request_id":uuid::Uuid::new_v4().to_string(),"operation":{"kind":"invoke",
        "tool_call":{"kind":"tool_call","action_id":"system.logs","arguments":arguments}}}).to_string()
}
fn call(proxy:&Proxy<'_>,member:&str,args:impl serde::Serialize+zbus::zvariant::DynamicType)->Value {
    let response:String=proxy.call(member,&args).unwrap();serde_json::from_str(&response).unwrap()
}
fn denied(proxy:&Proxy<'_>,member:&str,args:impl serde::Serialize+zbus::zvariant::DynamicType,code:&str) {
    let error=proxy.call::<_,_,String>(member,&args).unwrap_err();
    let zbus::Error::MethodError(name,_,_)=error else {panic!("expected explicit installed denial")};
    assert_eq!(name.as_str(),format!("org.aios.Error.{code}"));
}
fn logs(proxy:&Proxy<'_>,args:Value)->Value {
    let value=call(proxy,"Logs",(request(args),));
    aios_protocol::validation::validate_result("system.logs",&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(value["source"]["provider"],"aios-native-journal-observer");
    assert_eq!(value["complete"],true);value
}
fn verify_records(proxy:&Proxy<'_>,original:&[Value],result:&Value,boot:&str,uid:u32) {
    let entries=result["data"]["entries"].as_array().unwrap();
    for expected in original {
        assert_eq!(expected["_UID"],uid.to_string());assert_eq!(expected["_BOOT_ID"],boot.replace('-',""));
        let raw=expected["MESSAGE"].as_str().unwrap();
        // A multiline stdout write can yield distinct journal entries with
        // the same microsecond timestamp. Match the authoritative native
        // cursor as well; timestamp alone can select a different message.
        let candidates:Vec<_>=entries.iter().filter(|row|row["timestamp"].as_str().is_some_and(|s|OffsetDateTime::parse(s,&Rfc3339).unwrap().unix_timestamp_nanos()/1000==expected["__REALTIME_TIMESTAMP"].as_str().unwrap().parse::<i128>().unwrap()))
            .map(|row|(row,call(proxy,"GetJournalEvidence",(row["evidence_id"].as_str().unwrap(),))))
            .filter(|(_,evidence)|evidence["data"]["source_locator"]["cursor"]==expected["__CURSOR"]).collect();
        assert_eq!(candidates.len(),1,"exact native journal cursor must resolve once");
        let (row,evidence)=candidates.into_iter().next().unwrap();
        assert_eq!(row["priority"],5);assert_eq!(row["source"],"user");
        if raw.contains("PASSWORD") {assert_eq!(row["redacted"],true);assert!(!row.to_string().contains("fake-private-value"));}
        else {assert_eq!(row["message"],raw);assert_eq!(row["redacted"],false);}
        assert_eq!(evidence["data"]["source_locator"]["cursor"],expected["__CURSOR"]);
        assert_eq!(evidence["data"]["source_locator"]["boot_id"],boot);
        assert_eq!(evidence["data"]["payload"],*row);
        assert_eq!(evidence["data"]["content_sha256"],aios_policy::digest(row).unwrap());
    }
    let batch=call(proxy,"GetJournalEvidence",(result["evidence_ids"][0].as_str().unwrap(),));
    assert_eq!(batch["data"]["source_locator"]["boot_id"],boot);
    assert_eq!(batch["data"]["source_locator"]["cursor"],"");
    assert_eq!(batch["data"]["payload"],result["data"]);
    assert_eq!(batch["data"]["content_sha256"],aios_policy::digest(&result["data"]).unwrap());
    assert!(!result.to_string().contains("fake-private-value"));
}

// Only a UUID-named own-user transient unit is created. Keep its actual
// invocation for cleanup; a substituted/restarted unit is never stopped.
struct UserUnitFixture { name:String, invocation:String }
fn user_command(program:&str)->Command {
    let uid=unsafe{libc::geteuid()};let mut command=Command::new(format!("/run/current-system/sw/bin/{program}"));
    command.env("XDG_RUNTIME_DIR",format!("/run/user/{uid}"))
        .env("DBUS_SESSION_BUS_ADDRESS",format!("unix:path=/run/user/{uid}/bus"));command
}
impl UserUnitFixture {
    fn invocation(&self)->String {
        let output=user_command("systemctl").args(["--user","show","--value","--property=InvocationID",&self.name]).output().unwrap();
        assert!(output.status.success());let value=String::from_utf8(output.stdout).unwrap().trim().to_owned();
        assert_eq!(value.len(),32);assert!(value.bytes().all(|b|b.is_ascii_hexdigit()));assert_ne!(value,"0".repeat(32));value
    }
    fn stop(&mut self) {
        if self.invocation.is_empty(){return;}
        assert_eq!(self.invocation(),self.invocation,"owned fixture invocation changed; cleanup refused");
        assert!(user_command("systemctl").args(["--user","stop",&self.name]).status().unwrap().success());
        self.invocation.clear();
    }
}
impl Drop for UserUnitFixture {
    fn drop(&mut self) {
        if !self.invocation.is_empty() {
            let cleanup=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||self.stop()));
            if cleanup.is_err(){eprintln!("OWNED_USER_UNIT_CLEANUP_REFUSED {}",self.name);}
        }
    }
}
fn user_unit_records(proxy:&Proxy<'_>,foreign:&Proxy<'_>,boot:&str,uid:u32)->String {
    let mut fixture=UserUnitFixture{name:format!("horizon-journal-fixture-{}.service",uuid::Uuid::new_v4().simple()),invocation:String::new()};
    let tag=format!("horizon-observer-userunit-{}",uuid::Uuid::new_v4().simple());
    let messages=[format!("{tag} user {uid} startup failed."),format!("{tag} PASSWORD=fake-private-value"),format!("{tag} user {uid} retry scheduled.")];
    let script=format!("for value in {}: print(value)",serde_json::to_string(&messages).unwrap());
    let since=now();
    // The oneshot start job completes after the logger exits. RemainAfterExit
    // keeps the unit loaded for identity checks; --wait would wait for stop.
    let output=user_command("systemd-run").args(["--user","--quiet","--unit",&fixture.name,
        "--property=Type=oneshot","--property=RemainAfterExit=yes","/run/current-system/sw/bin/systemd-cat",
        "--priority=notice","--identifier",&tag,"/run/current-system/sw/bin/python3","-c",&script]).output().unwrap();
    assert!(output.status.success(),"owned user fixture creation failed");fixture.invocation=fixture.invocation();
    let deadline=Instant::now()+Duration::from_secs(5);
    let original:Vec<Value>=loop {
        let output=Command::new("/run/current-system/sw/bin/journalctl").args(["--quiet","--no-pager","--output=json",
            "--output-fields=MESSAGE,__CURSOR,__REALTIME_TIMESTAMP,_UID,_SYSTEMD_USER_UNIT,_BOOT_ID,PRIORITY",
            "--boot","--identifier",&tag,"--lines=3",&format!("_UID={uid}"),&format!("_SYSTEMD_USER_UNIT={}",fixture.name)])
            .stdin(Stdio::null()).output().unwrap();
        assert!(output.status.success());assert!(output.stdout.len()<131072 && output.stderr.len()<4096);
        let rows:Vec<Value>=String::from_utf8(output.stdout).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
        if rows.len()==3{break rows;}assert!(Instant::now()<deadline);thread::sleep(Duration::from_millis(20));
    };
    assert!(original.iter().all(|row|row["_SYSTEMD_USER_UNIT"]==fixture.name));
    let until=now();let resolved=call(proxy,"ResolveUserLogService",(fixture.name.as_str(),));
    assert_eq!(resolved["data"]["scope"],"user");let handle=resolved["data"]["service_id"].as_str().unwrap();
    let args=json!({"service_id":handle,"boot_id":boot,"since":timestamp(since),"until":timestamp(until),"max_entries":200});
    let observed=logs(proxy,args.clone());verify_records(proxy,&original,&observed,boot,uid);
    assert!(observed["data"]["entries"].as_array().unwrap().iter().all(|row|row["service_id"]==handle));
    denied(foreign,"Logs",(request(args.clone()),),"PERMISSION_DENIED");
    denied(foreign,"GetJournalEvidence",(observed["evidence_ids"][0].as_str().unwrap(),),"PERMISSION_DENIED");
    let mut wrong=args;wrong["source"]=json!("system");denied(proxy,"Logs",(request(wrong),),"INVALID_ARGUMENT");
    denied(proxy,"ResolveUserLogService",("../sshd.service",),"INVALID_ARGUMENT");
    denied(proxy,"ResolveUserLogService",("horizon-missing-unit-11111111111111111111111111111111.service",),"TARGET_NOT_FOUND");
    // Restart our own fixed fixture only. The old invocation-bound read scope
    // must fail, and a fresh explicit resolution may authorize the new unit.
    assert_eq!(fixture.invocation(),fixture.invocation);
    assert!(user_command("systemctl").args(["--user","restart",&fixture.name]).status().unwrap().success());
    let previous=fixture.invocation.clone();fixture.invocation=fixture.invocation();assert_ne!(fixture.invocation,previous);
    denied(proxy,"Logs",(request(json!({"service_id":handle})),),"TARGET_CHANGED");
    let renewed=call(proxy,"ResolveUserLogService",(fixture.name.as_str(),));
    let fresh=renewed["data"]["service_id"].as_str().unwrap().to_owned();assert_ne!(fresh,handle);
    assert!(!logs(proxy,json!({"service_id":fresh}))["data"]["entries"].as_array().unwrap().is_empty());
    fixture.stop();
    println!("AIOS_INSTALLED_JOURNAL_USER_UNIT={}",json!({"uid":uid,"name":fixture.name,"controlled_messages":3,
        "own_user_manager_and_kernel_socket_bound":true,"native_user_unit_filter":true,"redacted_evidence_verified":true,
        "foreign_sender_handle_and_evidence_refused":true,"source_mismatch_refused":true,"changed_invocation_refused":true,
        "fresh_resolution_distinct":true,"owned_cleanup_complete":true}));
    fresh
}

#[test]
#[ignore="requires the newly installed root System1 observer in an enrolled NixOS guest"]
fn installed_journal_filters_private_cursors_and_sanitized_evidence() {
    assert!(fs::read_to_string("/etc/os-release").unwrap().lines().any(|line|line=="ID=nixos"));
    assert_eq!(fs::read_to_string("/etc/aios/guest-role").unwrap().trim(),"development");
    let uid=unsafe {libc::geteuid()};assert!(uid>=1000);
    let boot=fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim().to_owned();
    let connection=Connection::system().unwrap();
    let bus=Proxy::new(&connection,"org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus").unwrap();
    let owner:String=bus.call("GetNameOwner",&("org.aios.System1",)).unwrap();
    let observer_uid:u32=bus.call("GetConnectionUnixUser",&(&owner,)).unwrap();assert_eq!(observer_uid,0);
    let observer_pid:u32=bus.call("GetConnectionUnixProcessID",&(&owner,)).unwrap();
    let main=Command::new("/run/current-system/sw/bin/systemctl").args(["show","--value","--property=MainPID","aios-execd.service"]).output().unwrap();
    assert!(main.status.success());assert_eq!(String::from_utf8(main.stdout).unwrap().trim().parse::<u32>().unwrap(),observer_pid);
    let installed=fs::canonicalize("/run/current-system/sw/bin/aios-execd").unwrap();assert!(installed.starts_with("/nix/store"));
    let proxy=Proxy::new(&connection,"org.aios.System1","/org/aios/System1","org.aios.System1").unwrap();
    let tag=format!("horizon-observer-check-{}",uuid::Uuid::new_v4().simple());
    let since=now();
    for message in [format!("{tag} startup failed."),format!("{tag} PASSWORD=fake-private-value"),format!("{tag} retry scheduled.")] {
        let mut writer=Command::new("/run/current-system/sw/bin/systemd-cat").args(["--identifier",&tag,"--priority","notice"])
            .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
        writeln!(writer.stdin.take().unwrap(),"{message}").unwrap();assert!(writer.wait().unwrap().success());
    }
    let deadline=Instant::now()+Duration::from_secs(5);
    let original:Vec<Value>=loop {
        let output=Command::new("/run/current-system/sw/bin/journalctl").args(["--quiet","--no-pager","--output=json",
            "--output-fields=MESSAGE,__CURSOR,__REALTIME_TIMESTAMP,_UID,_SYSTEMD_UNIT,_BOOT_ID,PRIORITY",
            &format!("--boot={}",boot.replace('-',"")),"--identifier",&tag,"--lines=3",&format!("_UID={uid}")])
            .stdin(Stdio::null()).output().unwrap();
        assert!(output.status.success());assert!(output.stdout.len()<131072 && output.stderr.len()<4096);
        let rows:Vec<Value>=String::from_utf8(output.stdout).unwrap().lines().map(|row|serde_json::from_str(row).unwrap()).collect();
        if rows.len()==3 {break rows;}assert!(Instant::now()<deadline);thread::sleep(Duration::from_millis(20));
    };
    let until=now();let arguments=json!({"source":"user","boot_id":"current","since":timestamp(since),"until":timestamp(until),"max_entries":200});
    let all=logs(&proxy,arguments.clone());verify_records(&proxy,&original,&all,&boot,uid);
    // Required prior-boot qualification: run this probe once before rebooting
    // the same enrolled guest. Only the fixed public fixture pattern is read.
    let previous=Command::new("/run/current-system/sw/bin/journalctl").args(["--quiet","--no-pager","--output=json",
        "--output-fields=MESSAGE,__CURSOR,__REALTIME_TIMESTAMP,_UID,_BOOT_ID,PRIORITY,SYSLOG_IDENTIFIER",
        "--boot=-1","--case-sensitive=yes","--grep=^horizon-observer-check-[0-9a-f]{32} (startup failed[.]|PASSWORD=fake-private-value|retry scheduled[.])$",
        "--lines=3",&format!("_UID={uid}")]).stdin(Stdio::null()).output().unwrap();
    assert!(previous.status.success(),"prior-boot fixture access required");
    assert!(previous.stdout.len()<131072 && previous.stderr.len()<4096);
    let historical:Vec<Value>=String::from_utf8(previous.stdout).unwrap().lines().map(|row|serde_json::from_str(row).unwrap()).collect();
    assert_eq!(historical.len(),3,"three controlled prior-boot messages required");
    let historical_boot=uuid::Uuid::parse_str(historical[0]["_BOOT_ID"].as_str().unwrap()).unwrap().to_string();
    assert_ne!(historical_boot,boot);
    let old_tag=historical[0]["SYSLOG_IDENTIFIER"].as_str().unwrap();
    assert!(historical.iter().all(|r|r["SYSLOG_IDENTIFIER"]==old_tag));
    for suffix in ["startup failed.","PASSWORD=fake-private-value","retry scheduled."] {
        assert!(historical.iter().any(|r|r["MESSAGE"]==format!("{old_tag} {suffix}")));
    }
    let times:Vec<u64>=historical.iter().map(|r|r["__REALTIME_TIMESTAMP"].as_str().unwrap().parse().unwrap()).collect();
    let old_arguments=json!({"source":"user","boot_id":historical_boot,"since":timestamp(*times.iter().min().unwrap()),
        "until":timestamp(*times.iter().max().unwrap()),"max_entries":200});
    let old=logs(&proxy,old_arguments.clone());verify_records(&proxy,&historical,&old,&historical_boot,uid);
    let mut wrong_boot=old_arguments;wrong_boot["boot_id"]=json!(boot);
    assert!(logs(&proxy,wrong_boot)["data"]["entries"].as_array().unwrap().is_empty(),"historical records leaked into current boot");
    let mut priority=arguments.clone();priority["priority_max"]=json!(4);
    assert!(logs(&proxy,priority)["data"]["entries"].as_array().unwrap().iter().all(|r|!r["message"].as_str().unwrap().starts_with(&tag)));
    let mut narrow=arguments.clone();narrow["since"]=json!(timestamp(until));
    assert!(logs(&proxy,narrow)["data"]["entries"].as_array().unwrap().iter().all(|r|!r["message"].as_str().unwrap().starts_with(&tag)));
    let mut page=arguments.clone();page["max_entries"]=json!(1);
    let first=logs(&proxy,page.clone());assert_eq!(first["data"]["entries"].as_array().unwrap().len(),1);
    let cursor=first["next_cursor"].as_str().expect("required installed continuation");page["cursor"]=json!(cursor);
    let second=logs(&proxy,page.clone());assert_eq!(second["data"]["entries"].as_array().unwrap().len(),1);
    assert_ne!(first["data"]["entries"][0]["evidence_id"],second["data"]["entries"][0]["evidence_id"]);
    let first_evidence=call(&proxy,"GetJournalEvidence",(first["data"]["entries"][0]["evidence_id"].as_str().unwrap(),));
    let second_evidence=call(&proxy,"GetJournalEvidence",(second["data"]["entries"][0]["evidence_id"].as_str().unwrap(),));
    assert_ne!(first_evidence["data"]["source_locator"]["cursor"],second_evidence["data"]["source_locator"]["cursor"]);
    let other=Connection::system().unwrap();let foreign=Proxy::new(&other,"org.aios.System1","/org/aios/System1","org.aios.System1").unwrap();
    let user_service=user_unit_records(&proxy,&foreign,&boot,uid);
    denied(&foreign,"Logs",(request(page.clone()),),"PERMISSION_DENIED");
    denied(&foreign,"GetJournalEvidence",(first["data"]["entries"][0]["evidence_id"].as_str().unwrap(),),"PERMISSION_DENIED");
    let expiring_page=page.clone();
    page["priority_max"]=json!(4);denied(&proxy,"Logs",(request(page),),"STALE_EVIDENCE");
    let mut absent=arguments.clone();absent["boot_id"]=json!("11111111-1111-4111-8111-111111111111");
    denied(&proxy,"Logs",(request(absent),),"TARGET_NOT_FOUND");
    let mut forged=arguments;forged["uid"]=json!(0);denied(&proxy,"Logs",(request(forged),),"INVALID_ARGUMENT");
    let service=call(&proxy,"ResolveLogService",("sshd.service",));let id=service["data"]["service_id"].as_str().unwrap();
    denied(&foreign,"Logs",(request(json!({"service_id":id,"source":"system"})),),"PERMISSION_DENIED");
    let system=logs(&proxy,json!({"service_id":id,"source":"system","boot_id":boot,"since":timestamp(0),"until":timestamp(now()),"max_entries":5}));
    assert!(!system["data"]["entries"].as_array().unwrap().is_empty());
    assert!(system["data"]["entries"].as_array().unwrap().iter().all(|r|r["source"]=="system" && r["service_id"]==id));
    denied(&proxy,"Logs",(request(json!({"service_id":id,"source":"kernel"})),),"INVALID_ARGUMENT");
    let kernel=logs(&proxy,json!({"source":"kernel","since":timestamp(0),"until":timestamp(now()),"max_entries":5}));
    assert!(!kernel["data"]["entries"].as_array().unwrap().is_empty());
    assert!(kernel["data"]["entries"].as_array().unwrap().iter().all(|r|r["source"]=="kernel"));
    // Exercise the installed observer's real monotonic lifetime, preserving the
    // original live connection and login. Missing handles must be explicit.
    thread::sleep(Duration::from_millis(30_100));
    denied(&proxy,"Logs",(request(expiring_page),),"TARGET_NOT_FOUND");
    denied(&proxy,"GetJournalEvidence",(first["data"]["entries"][0]["evidence_id"].as_str().unwrap(),),"TARGET_NOT_FOUND");
    denied(&proxy,"Logs",(request(json!({"service_id":id,"source":"system"})),),"TARGET_NOT_FOUND");
    denied(&proxy,"Logs",(request(json!({"service_id":user_service})),),"TARGET_NOT_FOUND");
    println!("AIOS_INSTALLED_JOURNAL={}",json!({"evidence_kind":"real-installed-native-journal-observer","installed_executable":installed,
        "observer_uid":observer_uid,"observer_pid":observer_pid,"uid":uid,"boot_id":boot,"controlled_messages":3,
        "own_uid_filter":true,"system_unit_filter":true,"kernel_source":true,"time_filter":true,"priority_filter":true,
        "entry_limit":true,"cursor_continuation":true,"cross_connection_refused":true,"query_drift_refused":true,
        "missing_boot_refused":true,"claimed_uid_refused":true,"redaction_before_evidence":true,"evidence_hash_verified":true,
        "expiry_refused":true,"historical_boot_filter":true,"historical_boot_id":historical_boot,
        "historical_controlled_messages":3,"batch_evidence_boot_verified":true,"user_unit_filter":true,
        "native_user_manager_bound":true,"unit_change_refused":true,"user_unit_expiry_refused":true}));
}
