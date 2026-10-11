//! Optional synthetic NixOS-driver component qualification. This exercises the
//! real UID/mount/cache boundaries, not public UI consent or installed D-Bus.
use aios_files::{Access, Manager, Owner};
use aios_protocol::contracts::ErrorCode;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::MetadataExt, path::Path, process::Command, thread, time::{Duration, Instant}};

fn fixture_identity() -> Value {
    let expected=option_env!("AIOS_SCOPE_QUALIFICATION_UUID").expect("qualification-only package required");
    let path=Path::new("/run/aios-scope-qualification/identity.json");
    let info=fs::symlink_metadata(path).expect("driver identity record missing");
    assert!(info.is_file() && info.uid()==0 && info.mode()&0o022==0 && info.nlink()==1);
    let record:Value=serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(record["dmi_uuid"],expected);
    // The raw DMI UUID is root-readable. The driver checks it before each
    // operation and seals it in this root-owned record for the user probe.
    assert_eq!(fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim(),record["boot_id"].as_str().unwrap());
    assert_eq!(fs::read_to_string("/etc/machine-id").unwrap().trim(),record["machine_id"].as_str().unwrap());
    assert_eq!(fs::read_to_string("/sys/block/vda/serial").unwrap().trim(),"root");
    let marker:Value=serde_json::from_slice(&fs::read("/etc/aios/scope-qualification.json").unwrap()).unwrap();
    assert_eq!(marker["dmi_uuid"],record["dmi_uuid"]);
    assert_eq!(marker["installation_uuid"],record["installation_uuid"]);
    assert_eq!(marker["guest_role"],"development");
    assert_eq!(marker["management_channel"],"nixos-test-driver");
    let key=Command::new("/run/current-system/sw/bin/ssh-keygen").args(["-lf","/etc/ssh/ssh_host_ed25519_key.pub","-E","sha256"]).output().unwrap();
    assert!(key.status.success());
    assert_eq!(String::from_utf8(key.stdout).unwrap().split_whitespace().nth(1).unwrap(),record["host_key_fingerprint"].as_str().unwrap());
    record
}

fn phase(name:&str) {
    fixture_identity();
    fs::write(Path::new("/home/alice/scope-probe").join(name),b"ready\n").unwrap();
}
fn wait(name:&str) {
    let deadline=Instant::now()+Duration::from_secs(120);
    while !Path::new("/home/alice/scope-probe").join(name).exists() {
        assert!(Instant::now()<deadline,"driver handshake timed out");thread::sleep(Duration::from_millis(100));
    }
    fixture_identity();
}

fn alice() {
    let identity=fixture_identity();let uid=nix::unistd::geteuid().as_raw();assert_eq!(uid,1000);
    let owner=Owner{uid,boot_id:identity["boot_id"].as_str().unwrap().into(),session_id:None,client_binding_sha256:"a".repeat(64)};
    let mut files=Manager::for_user_home(Path::new("/home/alice")).unwrap();
    let proposal=files.propose_xdg_roots(owner.clone(),Path::new("/home/alice"),100).unwrap();
    let id=proposal.roots.iter().find(|r|r.display_path=="/home/alice/Documents").unwrap().root_id.clone();
    assert_eq!(files.issue_handle(&owner,&id,Path::new("note.txt"),Access::Content,101),Err(ErrorCode::PermissionDenied));
    // Deliberate internal enrollment for component tests. Native UI consent is
    // a separate mandatory gate, never inferred from this call.
    let grant=files.enroll(&owner,&proposal.proposal_id,&[id],&[Access::Content],101).unwrap().remove(0);
    let note=files.issue_handle(&owner,&grant.root_id,Path::new("note.txt"),Access::Content,102).unwrap();
    assert_eq!(files.read_and_cache(&owner,&note.file_handle,128,103).unwrap(),"نص آمن / safe text\n".as_bytes());
    assert_eq!(files.cached_snippet(&owner,&note.file_handle,104).unwrap(),"نص آمن / safe text\n");
    assert_eq!(files.issue_handle(&owner,&grant.root_id,Path::new("foreign.txt"),Access::Content,104),Err(ErrorCode::PermissionDenied));
    assert_eq!(files.issue_handle(&owner,&grant.root_id,Path::new("escape.txt"),Access::Content,104),Err(ErrorCode::PermissionDenied));
    let old_mount=files.issue_handle(&owner,&grant.root_id,Path::new("mounted/note.txt"),Access::Content,104).unwrap();
    phase("ready");wait("mounted");
    assert_eq!(files.read(&owner,&old_mount.file_handle,128,105),Err(ErrorCode::PermissionDenied));
    assert_eq!(files.issue_handle(&owner,&grant.root_id,Path::new("mounted/note.txt"),Access::Content,105),Err(ErrorCode::PermissionDenied));
    phase("mount-denied");wait("root-replaced");
    assert_eq!(files.read(&owner,&note.file_handle,128,106),Err(ErrorCode::TargetChanged));
    assert_eq!(files.cached_snippet(&owner,&note.file_handle,106),Err(ErrorCode::TargetChanged));
    let receipt=files.revoke(&owner,&grant.root_id).unwrap();
    assert!(receipt.access_blocked && !receipt.purge_pending);assert_eq!(receipt.cached_records_purged,3);
    assert_eq!(files.cached_snippet(&owner,&note.file_handle,107),Err(ErrorCode::PermissionDenied));
    fs::write("/home/alice/scope-probe/result.json",serde_json::to_vec_pretty(&json!({
        "schema_version":1,"identity":identity,"uid":uid,
        "evidence_kind":"native-synthetic-UID-mount-cache-component-test",
        "preconsent_denied":true,"foreign_owned_readable_file_denied":true,
        "symlink_escape_denied":true,"new_mount_denied":true,"retained_handle_mount_change_denied":true,
        "replaced_root_blocks_cached_snippet":true,"persistent_purge":receipt,
        "native_ui_consent_qualified":false,"production_installation_qualified":false
    })).unwrap()).unwrap();
}
fn bob() {
    fixture_identity();assert_eq!(nix::unistd::geteuid().as_raw(),1001);
    assert!(matches!(Manager::with_cache(Path::new("/home/alice/.local/state/aios/files")),Err(ErrorCode::PermissionDenied)));
    println!("actual-foreign-uid-cache-denied");
}
fn main() {
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [mode] if mode=="alice"=>alice(), [mode] if mode=="bob"=>bob(),
        _=>panic!("fixed qualification role required"),
    }
}
