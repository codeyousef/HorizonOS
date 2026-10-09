//! Actual SQLite commits/reopens; targets, grants and build outputs are fixtures.
use super::*;
use crate::candidate::{
    Candidate,
    tests::{Fixture, fixture},
};
use aios_state::{DatabaseData, PreparationGrants};
use uuid::Uuid;
#[test]
fn native_sqlite_open_retains_nofollow_and_pinned_inodes() {
    use std::os::fd::AsRawFd;
    let directory = std::env::temp_dir().join(format!("aios-ledger-open-{}", Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let directory = directory.canonicalize().unwrap();
    let anchor = File::open(&directory).unwrap();
    let path = directory.join("ledger.sqlite");
    let file = OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(&path).unwrap();
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let alias = std::path::PathBuf::from(format!("/proc/self/fd/{}/ledger.sqlite", anchor.as_raw_fd()));
    assert!(Connection::open_with_flags(alias, flags).is_err());
    let connection = open_pinned_database(&path, &anchor, &file, flags).unwrap();
    connection.execute_batch("CREATE TABLE real_commit (id INTEGER); INSERT INTO real_commit VALUES (1)").unwrap();
    drop(connection);
    let reopened = open_pinned_database(&path, &anchor, &file, flags).unwrap();
    assert_eq!(reopened.query_row("SELECT id FROM real_commit", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
    drop(reopened);
    std::fs::rename(&path, directory.join("prior.sqlite")).unwrap();
    std::fs::write(&path, b"replacement").unwrap();
    assert!(matches!(open_pinned_database(&path, &anchor, &file, flags), Err(Error::Integrity)));
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(directory.join("prior.sqlite"), &path).unwrap();
    assert!(matches!(open_pinned_database(&path, &anchor, &file, flags), Err(Error::Integrity)));
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn schema_one_migrates_terminal_index_without_weakening_active_recovery() {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE broker_schema(version INTEGER NOT NULL);
             INSERT INTO broker_schema VALUES(1);
             CREATE TABLE plans(id TEXT PRIMARY KEY,subject INTEGER NOT NULL,prepared BLOB NOT NULL,digest TEXT NOT NULL,state TEXT NOT NULL,revision INTEGER NOT NULL,cancel INTEGER NOT NULL DEFAULT 0,resource BLOB,result BLOB,final BLOB,final_digest TEXT);
             CREATE UNIQUE INDEX one_active ON plans((1)) WHERE state NOT IN ('CANCELLED','REJECTED','FAILED');
             CREATE TABLE events(id TEXT NOT NULL,revision INTEGER NOT NULL,state TEXT NOT NULL,kind TEXT NOT NULL,PRIMARY KEY(id,revision));",
        )
        .unwrap();
    let ledger = Ledger::initialize(connection).unwrap();
    assert_eq!(
        ledger
            .connection
            .query_row("SELECT version FROM broker_schema", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        2
    );
    let index: String = ledger
        .connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='one_active'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(index.contains("'COMMITTED','ROLLED_BACK','CANCELLED','REJECTED','FAILED'"));
    assert!(!index.contains("RECOVERY_REQUIRED"));
}
fn closure(label: &str) -> String {
    format!("/nix/store/{}-{label}", "a".repeat(32))
}
fn prepare(f: &Fixture) -> (PreparedPlan, Candidate) {
    let catalog = f.template.catalog();
    let old = catalog
        .compile(&canonical(&catalog.defaults()).unwrap())
        .unwrap();
    let intent = Intent::InstallPackage {
        package_id: "kate".into(),
    };
    let preview = catalog
        .prepare(
            &old,
            intent.clone(),
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
        .unwrap();
    let compiled = catalog
        .compile(&canonical(&preview.candidate_manifest).unwrap())
        .unwrap();
    let candidate = f.store.prepare(&f.template, &compiled).unwrap();
    let plan = PreparedPlan {
        schema_version: 1,
        plan_id: Uuid::new_v4().to_string(),
        mode: PreparationMode::Act,
        target: Target {
            installation_uuid: Uuid::new_v4().to_string(),
            dmi_uuid: Uuid::new_v4().to_string(),
            boot_id: Uuid::new_v4().to_string(),
            machine_id: "b".repeat(32),
            role: "development".into(),
            disk_serial: "AIOS_FIXTURE".into(),
            management_channel: "ssh-development".into(),
        },
        requester: Requester {
            uid: 1000,
            logind_session: "c1".into(),
            bus_sender: ":1.15".into(),
        },
        intent_text: "Install Kate (fixture direct user request)".into(),
        intent,
        policy_revision: "3".repeat(64),
        capability_revision: "4".repeat(64),
        baseline: Baseline {
            intended_manifest_sha256: old.digest,
            running_closure: closure("running"),
            profile_closure: closure("profile"),
            boot_selected_closure: closure("selected-boot"),
            boot_metadata_sha256: "5".repeat(64),
        },
        candidate_sha256: candidate.digest().into(),
        template_sha256: f.template.digest().into(),
        preview,
        prepared_at_monotonic_ms: 100,
        preparation_expires_monotonic_ms: 300100,
    };
    (plan, candidate)
}
fn permission(plan: &PreparedPlan) -> ResourcePermission {
    ResourcePermission {
        requester_uid: 1000,
        boot_id: plan.target.boot_id.clone(),
        candidate_sha256: plan.candidate_sha256.clone(),
        plan_digest: plan.digest().unwrap(),
        max_build_bytes: 16 * 1024 * 1024 * 1024,
        max_download_bytes: 8 * 1024 * 1024 * 1024,
        recovery_reserve_bytes: 8 * 1024 * 1024 * 1024,
        expires_monotonic_ms: 300000,
        approved_cache: "https://cache.nixos.org".into(),
    }
}
fn built(plan: &PreparedPlan) -> VerifiedBuild {
    VerifiedBuild {
        result: BuildResult {
            candidate_sha256: plan.candidate_sha256.clone(),
            derivation: closure("candidate.drv"),
            closure: closure("candidate"),
            inventory_sha256: "6".repeat(64),
            added_paths: vec![closure("added")],
            removed_paths: vec![closure("removed")],
            nar_bytes: 12345,
            measured_download_bytes: None,
        },
        baselines_unchanged: true,
    }
}
fn ledger(f: &Fixture) -> Ledger {
    Ledger::initialize(Connection::open(f.base.join("fixture-ledger.sqlite")).unwrap()).unwrap()
}
fn registered(f: &Fixture) -> (Ledger, PreparedPlan, Candidate) {
    let (mut l, (p, c)) = (ledger(f), prepare(f));
    l.register(&p, &c, &f.store).unwrap();
    l.mark_planned(&p.plan_id, 1000).unwrap();
    (l, p, c)
}
fn building(f: &Fixture) -> (Ledger, PreparedPlan, Candidate) {
    let (mut l, p, c) = registered(f);
    l.start_build(
        &p.plan_id,
        1000,
        &p.target,
        &p.baseline,
        &permission(&p),
        200,
    )
    .unwrap();
    (l, p, c)
}
#[test]
fn separate_baselines_and_immutable_candidate_survive_reopen() {
    let f = fixture();
    let (mut l, p, c) = registered(&f);
    assert_ne!(p.baseline.running_closure, p.baseline.profile_closure);
    assert_ne!(p.baseline.profile_closure, p.baseline.boot_selected_closure);
    let count = l.history(&p.plan_id, 1000).unwrap().len();
    let status = l.register(&p, &c, &f.store).unwrap();
    assert_eq!(status.state, State::Planned);
    assert_eq!(l.history(&p.plan_id, 1000).unwrap().len(), count);
    drop(l);
    let l = ledger(&f);
    assert_eq!(
        canonical(&l.get_plan(&p.plan_id, 1000).unwrap()).unwrap(),
        canonical(&p).unwrap()
    );
}
#[test]
fn broker_restart_cancels_only_pre_effect_plans_and_preserves_build_lock() {
    let f = fixture();
    let (mut l, p, _) = registered(&f);
    l.cancel_abandoned_pre_effects().unwrap();
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Cancelled);
    assert_eq!(
        l.history(&p.plan_id, 1000).unwrap().last().unwrap().2,
        "broker-restarted-before-system-effects"
    );
    drop(l);
    let reopened = ledger(&f);
    assert_eq!(
        reopened.status(&p.plan_id, 1000).unwrap().state,
        State::Cancelled
    );
    drop(reopened);
    let f = fixture();
    let (mut l, p, _) = building(&f);
    l.cancel_abandoned_pre_effects().unwrap();
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Building);
    let (other, candidate) = prepare(&f);
    assert_eq!(
        l.register(&other, &candidate, &f.store).unwrap_err(),
        Error::Conflict
    );
}
#[test]
fn another_uid_and_changed_transaction_intent_cannot_reuse_plan() {
    let f = fixture();
    let (mut l, mut p, c) = registered(&f);
    assert_eq!(l.get_plan(&p.plan_id, 1001).err(), Some(Error::Authority));
    assert_eq!(
        l.request_cancel(&p.plan_id, 1001).err(),
        Some(Error::Authority)
    );
    p.intent_text = "Changed intent".into();
    assert_eq!(l.register(&p, &c, &f.store).err(), Some(Error::Conflict));
}
#[test]
fn one_active_system_change_until_terminal_and_no_build_autoreplay() {
    let f = fixture();
    let (mut l, p, _) = building(&f);
    let (other, c) = prepare(&f);
    assert_eq!(
        l.register(&other, &c, &f.store).err(),
        Some(Error::Conflict)
    );
    drop(l);
    let mut l = ledger(&f);
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Building);
    let cancel = l.request_cancel(&p.plan_id, 1000).unwrap();
    assert_eq!(cancel.state, State::Building);
    assert!(cancel.cancel_requested);
    assert_eq!(
        l.register(&other, &c, &f.store).err(),
        Some(Error::Conflict)
    );
    l.acknowledge_worker_stopped(
        &p.plan_id,
        1000,
        &VerifiedWorkerStop {
            plan_id: p.plan_id.clone(),
            candidate_sha256: p.candidate_sha256.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        l.register(&other, &c, &f.store).unwrap().state,
        State::Received
    );
}

#[test]
fn verified_worker_failure_is_terminal_and_releases_active_slot() {
    let f = fixture();
    let (mut ledger, plan, _) = building(&f);
    let stop = VerifiedWorkerStop {
        plan_id: plan.plan_id.clone(),
        candidate_sha256: plan.candidate_sha256.clone(),
    };
    let status = ledger
        .record_build_failure(&plan.plan_id, 1000, &stop)
        .unwrap();
    assert_eq!(status.state, State::Failed);
    assert!(!status.cancel_requested);
    let (other, candidate) = prepare(&f);
    assert_eq!(
        ledger.register(&other, &candidate, &f.store).unwrap().state,
        State::Received
    );
}
#[test]
fn resource_permission_is_separate_exact_bounded_and_expiring() {
    let f = fixture();
    let (mut l, p, _) = registered(&f);
    let valid = permission(&p);
    for case in 0..7 {
        let mut bad = valid.clone();
        match case {
            0 => bad.requester_uid = 1001,
            1 => bad.boot_id = Uuid::new_v4().to_string(),
            2 => bad.candidate_sha256 = "0".repeat(64),
            3 => bad.plan_digest = "0".repeat(64),
            4 => bad.approved_cache = "https://unapproved.example".into(),
            5 => bad.recovery_reserve_bytes = 0,
            _ => bad.expires_monotonic_ms = 200,
        };
        assert_eq!(
            l.start_build(&p.plan_id, 1000, &p.target, &p.baseline, &bad, 200)
                .err(),
            Some(Error::ResourcePermissionRequired)
        );
    }
    assert_eq!(
        l.start_build(&p.plan_id, 1000, &p.target, &p.baseline, &valid, 300100)
            .err(),
        Some(Error::Expired)
    );
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Planned);
}
#[test]
fn drift_in_each_pointer_or_boot_identity_stops_build() {
    let f = fixture();
    let (mut l, p, _) = registered(&f);
    for field in 0..4 {
        let mut baseline = p.baseline.clone();
        match field {
            0 => baseline.running_closure = closure("other"),
            1 => baseline.profile_closure = closure("other"),
            2 => baseline.boot_selected_closure = closure("other"),
            _ => baseline.boot_metadata_sha256 = "0".repeat(64),
        };
        assert_eq!(
            l.start_build(&p.plan_id, 1000, &p.target, &baseline, &permission(&p), 200)
                .err(),
            Some(Error::TargetChanged)
        );
    }
    let mut target = p.target.clone();
    target.boot_id = Uuid::new_v4().to_string();
    assert_eq!(
        l.start_build(&p.plan_id, 1000, &target, &p.baseline, &permission(&p), 200)
            .err(),
        Some(Error::TargetChanged)
    );
}
#[test]
fn foreign_candidate_and_unverified_build_output_are_not_bound() {
    let f = fixture();
    let (mut l, p, _) = building(&f);
    let mut bad = built(&p);
    bad.result.candidate_sha256 = "0".repeat(64);
    assert_eq!(
        l.record_build(&p.plan_id, 1000, &bad, &p.target, &p.baseline)
            .err(),
        Some(Error::Invalid)
    );
    let mut bad = built(&p);
    bad.baselines_unchanged = false;
    assert_eq!(
        l.record_build(&p.plan_id, 1000, &bad, &p.target, &p.baseline)
            .err(),
        Some(Error::TargetChanged)
    );
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Building);
}
#[test]
fn final_plan_binds_authorized_guard_handoff_and_terminal_commit() {
    let f = fixture();
    let (mut l, p, _) = building(&f);
    l.record_build(&p.plan_id, 1000, &built(&p), &p.target, &p.baseline)
        .unwrap();
    let final_plan = l.freeze(&p.plan_id, 1000, 400, false).unwrap();
    let status = l.status(&p.plan_id, 1000).unwrap();
    let hash = status.final_plan_sha256.clone().unwrap();
    assert_eq!(status.state, State::AwaitingApproval);
    assert_eq!(final_plan.build.closure, closure("candidate"));
    assert_eq!(final_plan.ordered_steps, STEPS);
    assert_eq!(final_plan.approval_expires_monotonic_ms, 300400);
    assert_eq!(
        l.authorize(&p.plan_id, 1000, &"0".repeat(64)).err(),
        Some(Error::State)
    );
    assert_eq!(
        l.authorize(&p.plan_id, 1000, &hash).unwrap().state,
        State::Authorized
    );
    let (other, candidate) = prepare(&f);
    assert_eq!(l.register(&other, &candidate, &f.store).err(), Some(Error::Conflict));
    assert_eq!(
        l.guard_complete(&p.plan_id, 1000, State::Committed)
            .unwrap()
            .state,
        State::Committed
    );
    assert_eq!(
        l.history(&p.plan_id, 1000).unwrap().last().unwrap().1,
        State::Committed
    );
    assert_eq!(l.register(&other, &candidate, &f.store).unwrap().state, State::Received);
}
#[test]
fn sqlite_abort_cannot_publish_success_and_failure_preserves_record() {
    let f = fixture();
    let (mut l, p, _) = registered(&f);
    l.connection.execute_batch("CREATE TRIGGER fixture_fail BEFORE UPDATE ON plans BEGIN SELECT RAISE(ABORT,'fixture disk full'); END;").unwrap();
    assert!(
        l.start_build(
            &p.plan_id,
            1000,
            &p.target,
            &p.baseline,
            &permission(&p),
            200
        )
        .is_err()
    );
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Planned);
    assert_eq!(l.history(&p.plan_id, 1000).unwrap().len(), 3);
}
#[test]
fn long_verified_build_gets_a_fresh_final_window_without_reusing_expired_build_consent() {
    let f = fixture();
    let (mut l, p, _) = building(&f);
    l.record_build(&p.plan_id, 1000, &built(&p), &p.target, &p.baseline)
        .unwrap();
    let frozen = l.freeze(&p.plan_id, 1000, 900000, false).unwrap();
    assert_eq!(frozen.frozen_at_monotonic_ms, 900000);
    assert_eq!(frozen.approval_expires_monotonic_ms, 1200000);
    assert_eq!(
        l.approval_snapshot(&p.plan_id, 1000).err(),
        Some(Error::Authority)
    );
    let f = fixture();
    let (mut l, p, _) = registered(&f);
    assert!(
        l.start_build(
            &p.plan_id,
            1000,
            &p.target,
            &p.baseline,
            &permission(&p),
            900000
        )
        .is_err()
    );
    assert_eq!(l.status(&p.plan_id, 1000).unwrap().state, State::Planned);
}
#[test]
fn validation_crash_can_resume_without_replaying_effects() {
    let f = fixture();
    let (mut l, (p, c)) = (ledger(&f), prepare(&f));
    l.register(&p, &c, &f.store).unwrap();
    let s = l.status(&p.plan_id, 1000).unwrap();
    l.transition(&p.plan_id, &s, State::Validating, "validation-started")
        .unwrap();
    drop(l);
    let mut l = ledger(&f);
    assert_eq!(
        l.mark_planned(&p.plan_id, 1000).unwrap().state,
        State::Planned
    );
    assert_eq!(l.history(&p.plan_id, 1000).unwrap().len(), 3);
}
#[test]
fn corrupted_plan_or_missing_schema_blocks_without_deleting_ledger() {
    let f = fixture();
    let (l, p, _) = registered(&f);
    l.connection
        .execute(
            "UPDATE plans SET prepared=?2 WHERE id=?1",
            params![p.plan_id, b"{}".as_slice()],
        )
        .unwrap();
    assert!(l.get_plan(&p.plan_id, 1000).is_err());
    drop(l);
    let l = ledger(&f);
    assert!(l.get_plan(&p.plan_id, 1000).is_err());
    l.connection
        .execute_batch("DROP INDEX one_active;")
        .unwrap();
    drop(l);
    assert!(
        Ledger::initialize(Connection::open(f.base.join("fixture-ledger.sqlite")).unwrap())
            .is_err()
    );
    assert!(f.base.join("fixture-ledger.sqlite").exists());
}
#[test]
fn full_ledger_refuses_a_new_transaction_without_partial_registration() {
    let f = fixture();
    let mut ledger = ledger(&f);
    let (plan, candidate) = prepare(&f);
    let page_count: i64 = ledger
        .connection
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    ledger
        .connection
        .pragma_update(None, "max_page_count", page_count)
        .unwrap();
    let mut sequence = 0_i64;
    for bytes in [8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16] {
        let payload = "x".repeat(bytes);
        if ledger
            .connection
            .execute(
                "INSERT INTO events(id,revision,state,kind) VALUES(?1,?2,'FAILED',?3)",
                params![format!("full-{sequence}"), sequence, payload],
            )
            .is_ok()
        {
            sequence += 1;
        }
    }
    assert_eq!(
        ledger.register(&plan, &candidate, &f.store).err(),
        Some(Error::Ledger)
    );
    let registered: i64 = ledger
        .connection
        .query_row(
            "SELECT count(*) FROM plans WHERE id=?1",
            [&plan.plan_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(registered, 0);
}
#[test]
fn malformed_schema_and_duplicate_fields_cannot_make_a_prepared_plan() {
    let f = fixture();
    let (p, _) = prepare(&f);
    let raw = canonical(&p).unwrap();
    let duplicate = format!(
        "{{\"schema_version\":1,{}",
        std::str::from_utf8(&raw[1..]).unwrap()
    );
    assert!(serde_json::from_str::<PreparedPlan>(&duplicate).is_err());
    let mut bad = p.clone();
    bad.preview.final_authorization_ready = true;
    assert_eq!(bad.validate(), Err(Error::Invalid));
    bad = p.clone();
    bad.baseline.boot_selected_closure = "/nix/store/arbitrary".into();
    assert_eq!(bad.validate(), Err(Error::Invalid));
    bad = p;
    bad.requester.uid = 0;
    assert_eq!(bad.validate(), Err(Error::Invalid));
}
#[test]
fn system_broker_blocks_recovery_classes_it_cannot_safely_execute() {
    let f = fixture();
    let (plan, _) = prepare(&f);
    for recovery in [
        Recovery::ReversibleUserSettingTargetBound,
        Recovery::CompensatableFileOperationReceiptBound,
        Recovery::DataMigrationBackupAndTestedRestoreRequired,
        Recovery::ExternallyIrreversibleFinalConfirmationRequired,
        Recovery::UnsupportedRecoveryBlocked,
    ] {
        let mut unsupported = plan.clone();
        unsupported.preview.recovery = recovery;
        assert_eq!(unsupported.validate(), Err(Error::Invalid));
    }
}
#[test]
fn root_ledger_constructor_has_no_nonroot_or_environment_override() {
    if unsafe { libc::geteuid() } != 0 {
        assert_eq!(Ledger::open().err(), Some(Error::Authority));
    }
}
