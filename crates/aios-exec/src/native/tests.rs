//! Native enrollment values are fixtures; no root/native runtime proof is minted.
use super::*;
fn fixture() -> (Enrollment, Observed) {
    let e = Enrollment {
        schema_version: 1,
        os_id: "nixos".into(),
        os_version: "26.05".into(),
        installation_uuid: "6fa6c0ab-ecad-4ef9-975f-3f48b091c978".into(),
        dmi_uuid: "f8fa4ff3-4cae-4380-924a-bac62f58148e".into(),
        guest_role: "development".into(),
        disk_serial: "AIOS_DEV_ROOT".into(),
        disk_device: "vda".into(),
        root_partition: "vda2".into(),
        root_filesystem: "btrfs".into(),
        management_channel: "ssh-development".into(),
    };
    let o = Observed {
        target: Target {
            installation_uuid: e.installation_uuid.clone(),
            dmi_uuid: e.dmi_uuid.clone(),
            boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
            machine_id: "f8fa4ff34cae4380924abac62f58148e".into(),
            role: e.guest_role.clone(),
            disk_serial: e.disk_serial.clone(),
            management_channel: e.management_channel.clone(),
        },
        os_id: e.os_id.clone(),
        os_version: e.os_version.clone(),
        partition: "/dev/vda2".into(),
        filesystem: "btrfs".into(),
        disk_parent: "vda".into(),
    };
    (e, o)
}
#[test]
fn enrollment_checks_all_observed_dimensions() {
    let (e, o) = fixture();
    assert_eq!(check(&e, &o), Ok(()));
    for i in 0..11 {
        let mut changed = o.clone();
        match i {
            0 => changed.os_id = "cachyos".into(),
            1 => changed.os_version = "25.11".into(),
            2 => changed.target.installation_uuid = uuid::Uuid::new_v4().to_string(),
            3 => changed.target.dmi_uuid = uuid::Uuid::new_v4().to_string(),
            4 => changed.target.role = "production".into(),
            5 => changed.target.disk_serial = "FOREIGN".into(),
            6 => changed.target.management_channel = "local-product".into(),
            7 => changed.partition = "/dev/vdb2".into(),
            8 => changed.filesystem = "ext4".into(),
            9 => changed.disk_parent = "vdb".into(),
            10 => changed.target.boot_id = "malformed".into(),
            _ => unreachable!(),
        };
        assert!(check(&e, &changed).is_err(), "dimension {i}");
    }
}
#[test]
fn malformed_and_unsafe_enrollment_is_rejected() {
    let (e, _) = fixture();
    for i in 0..8 {
        let mut v = e.clone();
        match i {
            0 => v.schema_version = 2,
            1 => v.installation_uuid = uuid::Uuid::nil().to_string(),
            2 => v.dmi_uuid = "not-a-uuid".into(),
            3 => v.disk_device = "../../host".into(),
            4 => v.root_partition = "vda2/serial".into(),
            5 => v.guest_role = "host".into(),
            6 => v.management_channel = "local-product".into(),
            7 => v.os_id = "linux".into(),
            _ => unreachable!(),
        };
        assert_eq!(v.validate(), Err(Error::Invalid));
    }
}
#[test]
fn authority_schema_and_revision_shape_are_strict() {
    let a = InstalledAuthority {
        schema_version: 1,
        template_path: format!("/nix/store/{}-template", "a".repeat(32)),
        manifest_sha256: "1".repeat(64),
        base_template_revision: "2".repeat(64),
        catalog_revision: "3".repeat(64),
        lock_sha256: "4".repeat(64),
    };
    assert_eq!(a.validate(), Ok(()));
    let mut bad = a.clone();
    bad.template_path.push_str("/../mutable");
    assert_eq!(bad.validate(), Err(Error::Invalid));
    bad = a.clone();
    bad.manifest_sha256 = "UPPERCASE".into();
    assert_eq!(bad.validate(), Err(Error::Invalid));
    let mut value = serde_json::to_value(a).unwrap();
    value["approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<InstalledAuthority>(value).is_err());
}
#[test]
fn duplicate_and_unknown_installed_fields_are_rejected() {
    let (e, _) = fixture();
    let data = canonical(&e).unwrap();
    let duplicate = [b"{\"schema_version\":1,".as_slice(), &data[1..]].concat();
    assert!(serde_json::from_slice::<Enrollment>(&duplicate).is_err());
    let mut v = serde_json::to_value(e).unwrap();
    v["uid"] = serde_json::json!(0);
    assert!(serde_json::from_value::<Enrollment>(v).is_err());
}
#[test]
fn os_release_parsing_is_literal_and_unambiguous() {
    assert_eq!(
        os_value("ID=nixos\nVERSION_ID=\"26.05\"\n", "VERSION_ID"),
        Ok("26.05".into())
    );
    for s in [
        "ID=nixos\nID=cachyos",
        "ID=\"nixos\\\"\"",
        "ID=",
        "NAME=nixos",
        "ID=\"",
    ] {
        assert!(os_value(s, "ID").is_err());
    }
}
#[test]
fn namespace_mapping_must_cover_actual_root_and_full_uid_range() {
    assert!(full_root_mapping(b"         0          0 4294967295\n"));
    for map in [
        b"0 1000 1".as_slice(),
        b"0 0 1",
        b"0 0 4294967295\n1000 1000 1",
        b"0 0",
        b"0 0 4294967296",
        b"not a mapping",
    ] {
        assert!(!full_root_mapping(map));
    }
}
#[test]
fn valid_boot_or_machine_change_invalidates_frozen_target() {
    let (e, o) = fixture();
    assert_eq!(check_snapshot(&e, &o, &o.target), Ok(()));
    let mut boot = o.clone();
    boot.target.boot_id = uuid::Uuid::new_v4().to_string();
    assert_eq!(check(&e, &boot), Ok(()));
    assert_eq!(
        check_snapshot(&e, &boot, &o.target),
        Err(Error::TargetChanged)
    );
    let mut machine = o.clone();
    machine.target.machine_id = "a".repeat(32);
    assert_eq!(check(&e, &machine), Ok(()));
    assert_eq!(
        check_snapshot(&e, &machine, &o.target),
        Err(Error::TargetChanged)
    );
}
#[test]
fn nonroot_cannot_mint_native_target_or_installed_template() {
    if unsafe { libc::getuid() } != 0 {
        assert_eq!(VerifiedTarget::enroll().err(), Some(Error::Authority));
        assert_eq!(
            crate::candidate::InstalledTemplate::from_installed().err(),
            Some(Error::Authority)
        );
    }
}
#[test]
fn dev_owned_files_are_not_installed_authority() {
    if unsafe { libc::getuid() } == 0 {
        return;
    }
    let path = std::env::temp_dir().join(format!("aios-native-fixture-{}", uuid::Uuid::new_v4()));
    fs::write(&path, b"{}").unwrap();
    assert_eq!(
        read(&path, Path::new("/"), true, 65536),
        Err(Error::Ownership)
    );
    fs::remove_file(path).unwrap();
}
#[test]
fn relative_resolution_cannot_escape_its_fixed_scope() {
    assert_eq!(normalized(Path::new("relative")), Err(Error::Invalid));
    assert_eq!(normalized(Path::new("/../../host")), Err(Error::Invalid));
    assert_eq!(
        normalized(Path::new("/sys/class/block/../../devices")),
        Ok(PathBuf::from("/sys/devices"))
    );
    assert_eq!(
        resolved(Path::new("/etc"), Path::new("/nix/store")),
        Err(Error::Ownership)
    );
}
