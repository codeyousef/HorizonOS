use super::*;

fn fixture_mounts() -> Vec<u8> {
    [
        "1 0 0:32 /@root / rw,relatime - btrfs /dev/vda2 rw,compress=zstd",
        "2 1 0:32 /@nix /nix rw,relatime - btrfs /dev/vda2 rw,compress=zstd",
        "3 1 0:32 /@var /var rw,relatime - btrfs /dev/vda2 rw,compress=zstd",
        "4 1 0:32 /@home /home rw,relatime - btrfs /dev/vda2 rw,compress=zstd",
        "5 1 253:1 / /boot rw,nosuid - vfat /dev/vda1 rw,fmask=0077",
    ]
    .join("\n")
    .into_bytes()
}
fn unit(name: &str, active: &str) -> UnitEvidence {
    UnitEvidence {
        name: name.into(),
        state: Some(UnitState {
            id: name.into(),
            load_state: "loaded".into(),
            active_state: active.into(),
            sub_state: if active == "failed" {
                "failed"
            } else {
                "running"
            }
            .into(),
            invocation_id: vec![0; 16],
            job_id: 0,
        }),
    }
}
#[test]
fn systemd_alias_requires_both_names_from_the_same_authenticated_unit() {
    let names = vec!["dbus.service".into(), "dbus-broker.service".into()];
    assert_eq!(
        unit_identity("dbus.service", "dbus-broker.service", &names),
        Ok(())
    );
    assert_eq!(
        unit_identity("dbus.service", "sshd.service", &names),
        Err(Error::Integrity)
    );
    assert_eq!(
        unit_identity(
            "dbus.service",
            "dbus-broker.service",
            &["dbus-broker.service".into()]
        ),
        Err(Error::Integrity)
    );
    assert_eq!(
        unit_identity("/etc/dbus.service", "dbus-broker.service", &names),
        Err(Error::Integrity)
    );
    assert_eq!(
        unit_identity(
            "dbus.service",
            "dbus-broker.service",
            &[
                "dbus.service".into(),
                "dbus.service".into(),
                "dbus-broker.service".into()
            ]
        ),
        Err(Error::Integrity)
    );
}
fn fixture() -> HealthEvidence {
    let target = Target {
        installation_uuid: "11111111-1111-4111-8111-111111111111".into(),
        dmi_uuid: "22222222-2222-4222-8222-222222222222".into(),
        boot_id: "33333333-3333-4333-8333-333333333333".into(),
        machine_id: "a".repeat(32),
        role: "development".into(),
        disk_serial: "AIOS_FIXTURE".into(),
        management_channel: "ssh-development".into(),
    };
    HealthEvidence {
        target,
        manager: ManagerEvidence {
            bus_id: "a".repeat(32),
            owner: ":1.1".into(),
            uid: 0,
            pid: 1,
            start_ticks: 1,
            executable: "/fixture/systemd".into(),
            executable_sha256: "a".repeat(64),
        },
        mounts: mounts(&fixture_mounts(), "/dev/vda2").unwrap(),
        units: PROTECTED_UNITS
            .iter()
            .map(|name| unit(name, "active"))
            .collect(),
        core_baseline_healthy: true,
        product_apis_verified: false,
        user_service_activation_verified: false,
        action_postconditions_verified: false,
        authenticated_host_heartbeat_verified: false,
    }
}
#[test]
fn fixed_mount_identity_and_subvolume_are_required() {
    assert_eq!(mounts(&fixture_mounts(), "/dev/vda2").unwrap().len(), 5);
    let text = String::from_utf8(fixture_mounts()).unwrap();
    for bad in [
        text.replace("/@nix", "/@var"),
        text.replace("btrfs", "tmpfs"),
        text.replace("/dev/vda2", "/dev/vdb2"),
        text.replace("vfat", "ext4"),
    ] {
        assert_eq!(
            mounts(bad.as_bytes(), "/dev/vda2"),
            Err(Error::TargetChanged)
        );
    }
}
#[test]
fn ambiguous_or_malformed_mounts_do_not_become_healthy() {
    let text = String::from_utf8(fixture_mounts()).unwrap();
    for bad in [
        format!("{text}\n6 1 0:32 /@root / rw - btrfs /dev/vda2 rw"),
        text.replace("2 1", "1 1"),
        text.replace("0:32", "invalid"),
        text.replace("rw,relatime", "rw,ro"),
        text.replace(" - ", " "),
        text.replace("rw,compress=zstd", "rw,ro"),
    ] {
        assert_eq!(mounts(bad.as_bytes(), "/dev/vda2"), Err(Error::Integrity));
    }
    assert_eq!(mounts(b"", "/dev/vda2"), Err(Error::Health));
    assert_eq!(
        mounts(&vec![b' '; 1024 * 1024 + 1], "/dev/vda2"),
        Err(Error::Integrity)
    );
}
#[test]
fn readonly_or_missing_mount_is_observed_and_fails_core_health() {
    let mut v = fixture();
    assert!(core_healthy(&v.target, &v.mounts, &v.units));
    v.mounts[0].writable = false;
    assert!(!core_healthy(&v.target, &v.mounts, &v.units));
    v.mounts = mounts(&fixture_mounts(), "/dev/vda2").unwrap();
    v.mounts.retain(|m| m.path != "/home");
    assert!(!core_healthy(&v.target, &v.mounts, &v.units));
    let text = String::from_utf8(fixture_mounts())
        .unwrap()
        .replace("rw,relatime", "ro,relatime");
    assert!(
        mounts(text.as_bytes(), "/dev/vda2")
            .unwrap()
            .iter()
            .filter(|m| m.filesystem == "btrfs")
            .all(|m| !m.writable)
    );
    assert_eq!(
        mounts(
            text.replace("rw,compress=zstd", "rw,ro").as_bytes(),
            "/dev/vda2"
        ),
        Err(Error::Integrity)
    );
}
#[test]
fn missing_transitioning_or_queued_units_are_not_active() {
    let mut v = fixture();
    let i = v
        .units
        .iter()
        .position(|u| u.name == "sshd.service")
        .unwrap();
    v.units[i].state = None;
    assert!(!core_healthy(&v.target, &v.mounts, &v.units));
    for active in [
        "reloading",
        "activating",
        "deactivating",
        "maintenance",
        "refreshing",
        "failed",
        "inactive",
    ] {
        assert!(!unit("sshd.service", active).active());
    }
    let mut queued = unit("sshd.service", "active");
    queued.state.as_mut().unwrap().job_id = 1;
    assert!(!queued.active());
    let mut malformed = unit("sshd.service", "active").state.unwrap();
    malformed.invocation_id.clear();
    assert_eq!(malformed.validate(), Err(Error::Integrity));
    malformed.invocation_id = vec![0; 16];
    malformed.active_state = "unknown".into();
    assert_eq!(malformed.validate(), Err(Error::Integrity));
}
#[test]
fn baseline_degraded_is_distinct_from_new_failure_or_lost_active_unit() {
    let mut old = fixture();
    old.units[2] = unit("aios-state.service", "failed");
    assert_eq!(compare(&old, &old), Ok(()));
    let mut current = old.clone();
    current.units[3] = unit("aios-exec.service", "failed");
    assert_eq!(compare(&old, &current), Err(Error::Health));
    current = old.clone();
    current.units[3].state = None;
    assert_eq!(compare(&old, &current), Err(Error::Health));
    current = old.clone();
    current.units[2] = unit("aios-state.service", "active");
    assert_eq!(compare(&old, &current), Ok(()));
}
#[test]
fn target_mount_or_core_health_changes_reject_comparison() {
    let old = fixture();
    let mut current = old.clone();
    current.target.boot_id = "4".repeat(36);
    assert_eq!(compare(&old, &current), Err(Error::TargetChanged));
    current = old.clone();
    current.mounts[0].source = "/dev/vdb2".into();
    assert_eq!(compare(&old, &current), Err(Error::Health));
    current = old.clone();
    current.core_baseline_healthy = false;
    assert_eq!(compare(&old, &current), Err(Error::Health));
}
#[test]
fn actual_pid1_bus_and_fixed_units_are_readonly() {
    let connection = crate::caller::connect_native_timeout(Duration::from_millis(250)).unwrap();
    let bus = Proxy::new(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .unwrap();
    let owner: String = bus.call("GetNameOwner", &(MANAGER,)).unwrap();
    let credentials: zbus::fdo::ConnectionCredentials = bus
        .call("GetConnectionCredentials", &(owner.as_str(),))
        .unwrap();
    assert_eq!(credentials.unix_user_id(), Some(0));
    assert_eq!(credentials.process_id(), Some(1));
    let observed = units(&connection, &owner, Instant::now()).unwrap();
    assert_eq!(observed.len(), PROTECTED_UNITS.len());
    for name in ["dbus.service", "sshd.service"] {
        assert!(observed.iter().any(|u| u.name == name && u.active()));
    }
    let after: String = bus.call("GetNameOwner", &(MANAGER,)).unwrap();
    assert_eq!(owner, after);
    assert!(matches!(NativeHealth::capture(), Err(Error::Authority)));
    println!(
        "AIOS_NATIVE_HEALTH_BUS {}",
        serde_json::json!({"evidence_kind":"actual-nonroot-fixed-systemd-readonly-observation", "manager_owner":owner,
        "manager_uid":0,"manager_pid":1,"units":observed,"native_root_health_capability_minted":false,
        "product_apis_verified":false,"activation_performed":false})
    );
}
