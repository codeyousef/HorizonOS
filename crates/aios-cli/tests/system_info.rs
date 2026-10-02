//! Real NixOS provider smoke. Run only in a verified guest, never on the host.
use std::process::Command;
use aios_protocol::contracts::{ProviderResult, ResultStatus};
use aios_system::SystemInfo;

#[test]
fn real_nixos_system_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_aiosctl")).args(["system", "info", "--json"]).output().unwrap();
    assert!(output.status.success(), "provider failed: {}", String::from_utf8_lossy(&output.stderr));
    let result: ProviderResult<SystemInfo> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result.schema_version, 1);
    assert!(matches!(result.status, ResultStatus::Ok));
    assert!(result.complete);
    assert!(result.error.is_none());
    let data = result.data.unwrap();
    assert_eq!(data.os_id, "nixos");
    assert_eq!(data.boot_id, std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap().trim());
    assert_eq!(data.current_closure, std::fs::canonicalize("/run/current-system").unwrap().to_str().unwrap());
    assert_eq!(data.architecture, std::env::consts::ARCH);
    assert_eq!(data.virtualization, "kvm");
    assert!(data.generation.is_some());
    println!("AIOS_SYSTEM_INFO={}", String::from_utf8(output.stdout).unwrap().trim());
}

#[test]
fn shell_and_client_authority_flags_are_unavailable() {
    for args in [vec!["shell", "run", "id"], vec!["system", "info", "--json", "--approved"], vec!["system", "info", "--uid", "0"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_aiosctl")).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}
