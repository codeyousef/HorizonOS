//! Authorization receipts/native UI/polkit results below are explicitly fixtures.
use super::*;
use crate::caller::SessionIdentity;
pub(super) fn binding() -> Binding {
    Binding {
        plan_id: uuid::Uuid::new_v4().to_string(),
        plan_hash: "a".repeat(64),
        caller: CallerIdentity {
            uid: 1000,
            pid: 123,
            start_ticks: 42,
            boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
            sender: ":1.7".into(),
            bus_id: "b".repeat(32),
            session: Some(SessionIdentity {
                id: "c1".into(),
                remote: false,
                kind: "wayland".into(),
                class: "user".into(),
                state: "active".into(),
                active: true,
            }),
        },
        target: Target {
            installation_uuid: "6fa6c0ab-ecad-4ef9-975f-3f48b091c978".into(),
            dmi_uuid: "f8fa4ff3-4cae-4380-924a-bac62f58148e".into(),
            boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
            machine_id: "c".repeat(32),
            role: "development".into(),
            disk_serial: "AIOS_DEV_ROOT".into(),
            management_channel: "ssh-development".into(),
        },
        closure: format!("/nix/store/{}-system", "a".repeat(32)),
        impact_sha256: "e".repeat(64),
        policy_revision: "d".repeat(64),
        action: policy::ACTION.into(),
        frozen_at: 100,
        expires_at: 300100,
    }
}
fn owner() -> ServiceIdentity {
    ServiceIdentity {
        sender: ":1.4".into(),
        uid: 26,
        pid: 99,
        start_ticks: 25,
        boot_id: "ab31ff68-0c2d-4db1-8d6b-9a63189c6844".into(),
    }
}
fn confirmed(binding: &Binding) -> TrustedConfirmation {
    TrustedConfirmation {
        binding_sha256: binding.confirmation_digest().unwrap(),
    }
}
fn issued(binding: &Binding) -> Volatile {
    let mut volatile = Volatile::default();
    volatile
        .issue(
            binding.clone(),
            confirmed(binding),
            polkit::NativeAuthentication { owner: owner() },
            200,
        )
        .unwrap();
    volatile
}
#[test]
fn exact_receipt_is_single_use_and_status_carries_no_token() {
    let binding = binding();
    let mut volatile = issued(&binding);
    let grant = volatile.consume(&binding, &owner(), 250).unwrap();
    assert_eq!(grant.plan_id(), binding.plan_id);
    assert_eq!(grant.plan_hash(), binding.plan_hash);
    assert_eq!(grant.closure(), binding.closure);
    assert_eq!(
        volatile.consume(&binding, &owner(), 251).err(),
        Some(Error::AuthRequired)
    );
    assert_eq!(
        serde_json::to_string(&AuthorizationStatus::SystemAuthorized).unwrap(),
        "\"SYSTEM_AUTHORIZED\""
    );
}
#[test]
fn expiry_boundaries_clock_rewind_and_suspend_elapsed_time_invalidate_receipts() {
    let binding = binding();
    assert_eq!(
        issued(&binding).consume(&binding, &owner(), 300099).is_ok(),
        true
    );
    for now in [99, 199, 300100, 900000] {
        let mut volatile = issued(&binding);
        assert!(volatile.consume(&binding, &owner(), now).is_err());
        assert!(volatile.receipts.is_empty());
    }
    let mut wrong = binding.clone();
    wrong.expires_at += 1;
    assert_eq!(wrong.validate_time(250), Err(Error::Integrity));
}
#[test]
fn plan_policy_target_closure_action_and_expiry_drift_cannot_reuse_a_receipt() {
    let original = binding();
    for field in 0..11 {
        let mut current = original.clone();
        match field {
            0 => current.plan_hash = "e".repeat(64),
            1 => current.closure.push_str("-different"),
            2 => current.policy_revision = "f".repeat(64),
            3 => current.target.installation_uuid = uuid::Uuid::new_v4().to_string(),
            4 => current.target.boot_id = uuid::Uuid::new_v4().to_string(),
            5 => current.target.disk_serial = "OTHER".into(),
            6 => current.target.role = "production".into(),
            7 => current.action = policy::ELEVATED_ACTION.into(),
            8 => {
                current.frozen_at += 1;
                current.expires_at += 1;
            }
            9 => current.target.management_channel = "local-product".into(),
            10 => current.impact_sha256 = "f".repeat(64),
            _ => unreachable!(),
        }
        let mut volatile = issued(&original);
        assert!(
            volatile.consume(&current, &owner(), 250).is_err(),
            "field {field}"
        );
        assert!(volatile.receipts.is_empty());
        assert_ne!(
            original.confirmation_digest().unwrap(),
            current.confirmation_digest().unwrap()
        );
    }
}
#[test]
fn unrelated_reads_another_plan_another_uid_and_reconnected_client_have_no_authority() {
    let original = binding();
    let mut volatile = issued(&original);
    let mut unrelated = original.clone();
    unrelated.plan_id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        volatile.consume(&unrelated, &owner(), 250).err(),
        Some(Error::AuthRequired)
    );
    for field in 0..7 {
        let mut current = original.clone();
        match field {
            0 => current.caller.uid += 1,
            1 => current.caller.pid += 1,
            2 => current.caller.start_ticks += 1,
            3 => current.caller.sender = ":1.8".into(),
            4 => current.caller.bus_id = "e".repeat(32),
            5 => current.caller.session.as_mut().unwrap().id = "c2".into(),
            6 => current.caller.boot_id = uuid::Uuid::new_v4().to_string(),
            _ => unreachable!(),
        }
        assert_eq!(
            volatile.consume(&current, &owner(), 250).err(),
            Some(Error::Authority)
        );
    }
    assert!(volatile.consume(&original, &owner(), 250).is_ok());
}
#[test]
fn polkit_owner_restart_or_identity_change_invalidates_native_receipt() {
    let binding = binding();
    for field in 0..5 {
        let mut changed = owner();
        match field {
            0 => changed.sender = ":1.9".into(),
            1 => changed.uid += 1,
            2 => changed.pid += 1,
            3 => changed.start_ticks += 1,
            4 => changed.boot_id = uuid::Uuid::new_v4().to_string(),
            _ => unreachable!(),
        }
        assert!(issued(&binding).consume(&binding, &changed, 250).is_err());
    }
}
#[test]
fn confirmation_is_exact_and_repeated_authorization_cannot_replace_nonce_or_extend_expiry() {
    let binding = binding();
    let mut volatile = issued(&binding);
    let nonce = volatile.receipts[&binding.plan_id].nonce;
    assert!(!nonce.is_nil());
    assert_eq!(
        volatile.issue(
            binding.clone(),
            confirmed(&binding),
            polkit::NativeAuthentication { owner: owner() },
            299999
        ),
        Err(Error::Conflict)
    );
    assert_eq!(volatile.receipts[&binding.plan_id].nonce, nonce);
    let mut wrong = binding.clone();
    wrong.closure.push_str("-unapproved");
    assert_eq!(
        Volatile::default().issue(
            wrong,
            confirmed(&binding),
            polkit::NativeAuthentication { owner: owner() },
            250
        ),
        Err(Error::Integrity)
    );
}
#[test]
fn restart_restore_and_revocation_have_no_persistent_receipt() {
    let binding = binding();
    let mut volatile = issued(&binding);
    assert!(
        Volatile::default()
            .consume(&binding, &owner(), 250)
            .is_err()
    );
    volatile.receipts.clear();
    assert!(volatile.consume(&binding, &owner(), 250).is_err());
}
#[test]
fn installed_policy_paths_and_hashes_are_strict_and_caller_authority_fields_are_rejected() {
    let root = format!("/nix/store/{}-executor", "a".repeat(32));
    let authority = policy::Authority {
        schema_version: 1,
        policy_path: format!("{root}/share/aios/system-approval.json"),
        policy_sha256: sha256(policy::POLICY),
        action_path: format!("{root}/share/polkit-1/actions/org.aios.executor.policy"),
        action_sha256: sha256(policy::ACTIONS),
        polkit_uid: 26,
        polkit_package: format!("/nix/store/{}-polkit", "b".repeat(32)),
    };
    assert_eq!(authority.validate(), Ok(()));
    for field in 0..5 {
        let mut a = authority.clone();
        match field {
            0 => a.polkit_uid = 0,
            1 => a.policy_sha256 = "0".repeat(64),
            2 => a.action_path = "/tmp/client.policy".into(),
            3 => a.polkit_package.push_str("/../other"),
            4 => a.action_sha256 = "f".repeat(64),
            _ => unreachable!(),
        }
        assert!(a.validate().is_err());
    }
    let mut value = serde_json::to_value(&authority).unwrap();
    value["approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<policy::Authority>(value).is_err());
    let bytes = canonical(&authority).unwrap();
    let duplicate = [b"{\"schema_version\":1,".as_slice(), &bytes[1..]].concat();
    assert!(serde_json::from_slice::<policy::Authority>(&duplicate).is_err());
}
#[test]
fn nonroot_cannot_open_native_authorizer_and_real_boottime_is_monotonic() {
    assert_ne!(unsafe { libc::getuid() }, 0);
    assert_eq!(Authorizer::open().err(), Some(Error::Authority));
    let before = boottime_ms().unwrap();
    assert!(boottime_ms().unwrap() >= before);
}
