//! Deterministic effect/health fixtures with real SQLite persistence.
//! No fixture performs NixOS activation, boot writes or root service changes.
use aios_guard::*;
use rusqlite::{Connection, params};
use std::{fs, os::unix::fs::DirBuilderExt, path::PathBuf};
use uuid::Uuid;
fn h(c: char) -> String {
    std::iter::repeat_n(c, 64).collect()
}
fn closure(name: &str) -> Closure {
    Closure {
        path: format!("/nix/store/{}-{name}", "a".repeat(32)),
        activation_sha256: h('1'),
        kernel_sha256: h('2'),
        initrd_sha256: h('3'),
        boot_adapter_sha256: h('4'),
    }
}
fn model(name: &str) -> ModelArtifact {
    ModelArtifact {
        store_path: format!("/nix/store/{}-{name}", "b".repeat(32)),
        manifest_sha256: h('5'),
    }
}
fn plan() -> Plan {
    Plan {
        schema_version: 1,
        transaction_id: Uuid::new_v4().to_string(),
        identity: Identity {
            installation_uuid: Uuid::new_v4().to_string(),
            dmi_uuid: Uuid::new_v4().to_string(),
            boot_id: Uuid::new_v4().to_string(),
            machine_id: "c".repeat(32),
            role: "development".into(),
            disk_serial: "AIOS_DEV_ROOT".into(),
            management_channel: "ssh-development".into(),
        },
        source_digest: h('6'),
        candidate_digest: h('7'),
        nixpkgs_revision: NIXPKGS_REVISION.into(),
        prior: Prior {
            running: closure("running"),
            profile: closure("profile"),
            boot: closure("boot"),
            managed_sha256: h('8'),
            model: Some(model("old-model")),
        },
        candidate: Candidate::System {
            closure: closure("candidate"),
        },
        managed_sha256: h('9'),
        retained_guard_sha256: h('a'),
        guard_timeout_seconds: 180,
        health: HealthPolicy {
            required_mounts: vec!["/".into(), "/nix".into()],
            baseline_units: vec![
                UnitHealth {
                    name: "sshd.service".into(),
                    active: true,
                    failed: false,
                },
                UnitHealth {
                    name: "dbus.service".into(),
                    active: true,
                    failed: false,
                },
                UnitHealth {
                    name: "aios-observer.service".into(),
                    active: false,
                    failed: true,
                },
            ],
            required_apis: vec![
                ApiHealth {
                    api: Api::Executor,
                    uid: None,
                    healthy: true,
                },
                ApiHealth {
                    api: Api::Graph,
                    uid: None,
                    healthy: true,
                },
            ],
            baseline_user_units: vec![UserUnit {
                uid: 1000,
                name: "aios-sessiond.service".into(),
                expected_executable_sha256: h('c'),
            }],
            required_user_units: vec![UserUnit {
                uid: 1000,
                name: "aios-sessiond.service".into(),
                expected_executable_sha256: h('b'),
            }],
            action_validators: vec![],
        },
    }
}
struct Fixture {
    observation: Observation,
    effects: Vec<Effect>,
    fail: Option<&'static str>,
    database: PathBuf,
    txid: String,
    baseline_user_units: Vec<UserUnit>,
    required_user_units: Vec<UserUnit>,
    candidate_managed_sha256: String,
    health_fails_on_test: bool,
    guard_lost_on_test: bool,
    corrupt_committed: bool,
    clock: u64,
    expire_on: Option<&'static str>,
    advance_on: Option<&'static str>,
    drift_after_profile: bool,
    guard_lost_after_managed: bool,
    observe_failure_after_test: bool,
    clock_failure_after_test: bool,
    fail_observe: bool,
    fail_clock: bool,
}
impl Adapter for Fixture {
    fn boot_time_ms(&mut self) -> Result<u64> {
        if self.fail_clock {
            self.fail_clock = false;
            return Err(Error::Adapter);
        }
        Ok(self.clock)
    }
    fn observe(&mut self) -> Result<Observation> {
        if self.fail_observe {
            self.fail_observe = false;
            return Err(Error::Adapter);
        }
        Ok(self.observation.clone())
    }
    fn apply(&mut self, effect: &Effect) -> Result<()> {
        // Independent connection witnesses that effect intent is already durable.
        let database = Connection::open(&self.database).unwrap();
        let (state, event): (String, Vec<u8>) = database
            .query_row(
                "SELECT state,event FROM guard_events WHERE id=?1 ORDER BY revision DESC LIMIT 1",
                [&self.txid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        if matches!(effect, Effect::DisarmGuard { .. }) {
            assert!(state == "COMMITTED" || state == "ROLLED_BACK");
        } else {
            assert_eq!(
                serde_json::from_slice::<Option<Effect>>(&event)
                    .unwrap()
                    .as_ref(),
                Some(effect)
            );
        }
        self.effects.push(effect.clone());
        let label = match effect {
            Effect::TestSystem { .. } | Effect::TestModel { .. } => "test",
            Effect::SetProfile { .. } => "profile",
            Effect::InstallBoot { .. } => "boot",
            Effect::PublishManaged { .. } => "managed",
            Effect::RecoverSystem { .. } | Effect::RecoverModel { .. } => "recover",
            _ => "other",
        };
        if self.expire_on == Some(label) {
            self.clock = 181000;
            self.expire_on = None;
        }
        if self.advance_on == Some(label) {
            self.clock = 7000;
            self.advance_on = None;
        }
        if self.fail == Some(label) {
            self.fail = None;
            return Err(Error::Adapter);
        }
        match effect {
            Effect::ArmGuard {
                executable_sha256, ..
            } => self.observation.retained_guard_sha256 = Some(executable_sha256.clone()),
            Effect::TestSystem { closure } => {
                self.observation.running = closure.clone();
                self.observation.user_units = self.required_user_units.clone();
                self.observation.managed_sha256 = self.candidate_managed_sha256.clone();
            }
            Effect::RecoverSystem { closure } => {
                self.observation.running = closure.clone();
                self.observation.user_units = self.baseline_user_units.clone();
            }
            Effect::TestModel { artifact } | Effect::RecoverModel { artifact } => {
                self.observation.model = Some(artifact.clone())
            }
            Effect::SetProfile { closure } => self.observation.profile = closure.clone(),
            Effect::InstallBoot { closure } => self.observation.boot = closure.clone(),
            Effect::PublishManaged { artifact_sha256 } => {
                self.observation.managed_sha256 = artifact_sha256.clone()
            }
            Effect::DisarmGuard { .. } => self.observation.retained_guard_sha256 = None,
            _ => {}
        }
        if matches!(effect, Effect::TestSystem { .. } | Effect::TestModel { .. }) {
            if self.health_fails_on_test {
                self.observation.system_bus = false;
            }
            if self.guard_lost_on_test {
                self.observation.retained_guard_sha256 = None;
            }
        }
        if self.drift_after_profile && matches!(effect, Effect::SetProfile { .. }) {
            self.observation.identity.boot_id = Uuid::new_v4().to_string();
        }
        if self.guard_lost_after_managed && matches!(effect, Effect::PublishManaged { .. }) {
            self.observation.retained_guard_sha256 = None;
        }
        if matches!(effect, Effect::TestSystem { .. } | Effect::TestModel { .. }) {
            self.fail_observe = self.observe_failure_after_test;
            self.fail_clock = self.clock_failure_after_test;
        }
        if matches!(
            effect,
            Effect::RecoverSystem { .. } | Effect::RecoverModel { .. }
        ) {
            self.observation.system_bus = true;
        }
        if self.corrupt_committed
            && matches!(effect,Effect::PublishManaged{artifact_sha256} if artifact_sha256==&h('9'))
        {
            self.observation.boot = "wrong boot default".into();
        }
        Ok(())
    }
}
struct Harness {
    plan: Plan,
    ledger: Ledger,
    fixture: Fixture,
    directory: PathBuf,
}
impl Harness {
    fn new(mut plan: Plan) -> Self {
        if matches!(plan.candidate, Candidate::ModelOnly { .. }) {
            plan.health.required_apis.push(ApiHealth {
                api: Api::Model,
                uid: None,
                healthy: true,
            });
        }
        let directory = std::env::temp_dir().join(format!("aios-guard-fixture-{}", Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let path = directory.join("ledger.sqlite");
        let db = Connection::open(&path).unwrap();
        let observation = Observation {
            identity: plan.identity.clone(),
            running: plan.prior.running.path.clone(),
            profile: plan.prior.profile.path.clone(),
            boot: plan.prior.boot.path.clone(),
            managed_sha256: plan.prior.managed_sha256.clone(),
            model: plan.prior.model.clone(),
            system_bus: true,
            mounts: plan.health.required_mounts.clone(),
            units: plan.health.baseline_units.clone(),
            apis: plan.health.required_apis.clone(),
            user_units: plan.health.baseline_user_units.clone(),
            validated_actions: vec![],
            retained_guard_sha256: None,
        };
        Self {
            fixture: Fixture {
                observation,
                effects: vec![],
                fail: None,
                database: path,
                txid: plan.transaction_id.clone(),
                baseline_user_units: plan.health.baseline_user_units.clone(),
                required_user_units: plan.health.required_user_units.clone(),
                candidate_managed_sha256: plan.managed_sha256.clone(),
                health_fails_on_test: false,
                guard_lost_on_test: false,
                corrupt_committed: false,
                clock: 1000,
                expire_on: None,
                advance_on: None,
                drift_after_profile: false,
                guard_lost_after_managed: false,
                observe_failure_after_test: false,
                clock_failure_after_test: false,
                fail_observe: false,
                fail_clock: false,
            },
            ledger: Ledger::new(db).unwrap(),
            plan,
            directory,
        }
    }
    fn engine(&mut self) -> Engine {
        Engine::register(self.plan.clone(), &mut self.ledger).unwrap()
    }
    fn verified(&mut self) -> Engine {
        let mut e = self.engine();
        assert_eq!(
            e.arm(&mut self.ledger, &mut self.fixture, 1000).unwrap(),
            State::GuardArmed
        );
        assert_eq!(
            e.test(&mut self.ledger, &mut self.fixture, 1001).unwrap(),
            State::Verifying
        );
        e
    }
    fn heartbeat(&self, e: &Engine) -> Heartbeat {
        Heartbeat {
            schema_version: 1,
            transaction_id: self.plan.transaction_id.clone(),
            identity: self.plan.identity.clone(),
            plan_digest: self.plan.digest().unwrap(),
            candidate_digest: self.plan.candidate_digest.clone(),
            nonce: e.challenge().transport_value().into(),
        }
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn exact_test_then_profile_boot_managed_commit_and_disarm_order() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
        State::Committed
    );
    let candidate = match &f.plan.candidate {
        Candidate::System { closure } => closure.path.clone(),
        _ => unreachable!(),
    };
    assert_eq!(
        f.fixture.effects[2..],
        [
            Effect::TestSystem {
                closure: candidate.clone()
            },
            Effect::SetProfile {
                closure: candidate.clone()
            },
            Effect::InstallBoot { closure: candidate },
            Effect::PublishManaged {
                artifact_sha256: f.plan.managed_sha256.clone()
            },
            Effect::DisarmGuard {
                transaction_id: f.plan.transaction_id.clone()
            }
        ]
    );
    assert_eq!(
        f.ledger.state(&f.plan.transaction_id).unwrap(),
        State::Committed
    );
    assert!(f.ledger.event_count(&f.plan.transaction_id).unwrap() > 8);
}
#[test]
fn missing_stale_wrong_nonce_identity_plan_or_candidate_heartbeat_cannot_commit() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003),
        Err(Error::Heartbeat)
    );
    for field in 0..6 {
        let mut beat = f.heartbeat(&e);
        match field {
            0 => beat.nonce = h('0'),
            1 => beat.identity.boot_id = Uuid::new_v4().to_string(),
            2 => beat.plan_digest = h('0'),
            3 => beat.candidate_digest = h('0'),
            4 => beat.transaction_id = Uuid::new_v4().to_string(),
            _ => beat.identity.installation_uuid = Uuid::new_v4().to_string(),
        };
        assert_eq!(e.heartbeat(beat, 1003), Err(Error::Heartbeat));
    }
    e.heartbeat(f.heartbeat(&e), 1003).unwrap();
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 6004),
        Err(Error::Heartbeat)
    );
    assert!(
        !f.fixture
            .effects
            .iter()
            .any(|x| matches!(x, Effect::SetProfile { .. } | Effect::InstallBoot { .. }))
    );
}
#[test]
fn heartbeat_deadline_equality_is_expired_and_clock_reversal_is_denied() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1005).unwrap();
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1004),
        Err(Error::Heartbeat)
    );
    assert_eq!(e.heartbeat(f.heartbeat(&e), 181000), Err(Error::Expired));
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 181000),
        Err(Error::Expired)
    );
}
#[test]
fn fresh_heartbeat_gates_a_commit_that_finishes_within_the_guard_deadline() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.advance_on = Some("profile");
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
        State::Committed
    );
    assert_eq!(f.fixture.clock, 7000);
}
#[test]
fn ssh_loss_timeout_restores_three_distinct_prior_pointers() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    assert_eq!(
        e.tick(&mut f.ledger, &mut f.fixture, 181000).unwrap(),
        State::RolledBack
    );
    assert_eq!(f.fixture.observation.running, f.plan.prior.running.path);
    assert_eq!(f.fixture.observation.profile, f.plan.prior.profile.path);
    assert_eq!(f.fixture.observation.boot, f.plan.prior.boot.path);
}
#[test]
fn changed_boot_or_installation_never_runs_recovery_on_another_target() {
    for boot in [false, true] {
        let mut f = Harness::new(plan());
        let mut e = f.verified();
        if boot {
            f.fixture.observation.identity.boot_id = Uuid::new_v4().to_string();
        } else {
            f.fixture.observation.identity.installation_uuid = Uuid::new_v4().to_string();
        }
        let before = f.fixture.effects.len();
        assert_eq!(
            e.tick(&mut f.ledger, &mut f.fixture, 181000),
            Err(Error::RecoveryRequired)
        );
        assert_eq!(f.fixture.effects.len(), before);
        assert_eq!(
            f.ledger.state(&f.plan.transaction_id).unwrap(),
            State::RecoveryRequired
        );
    }
}
#[test]
fn crashed_controller_reconciles_without_replaying_activation_or_old_challenge() {
    let mut f = Harness::new(plan());
    let e = f.verified();
    let before = f.fixture.effects.len();
    drop(e);
    assert_eq!(
        Engine::reconcile(&mut f.ledger, &f.plan.transaction_id, &mut f.fixture).unwrap(),
        State::RolledBack
    );
    assert!(
        !f.fixture.effects[before..]
            .iter()
            .any(|x| matches!(x, Effect::TestSystem { .. }))
    );
}
#[test]
fn restart_finishes_disarm_after_durable_terminal_commit() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.fail = Some("other");
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003),
        Err(Error::Adapter)
    );
    assert_eq!(
        f.ledger.state(&f.plan.transaction_id).unwrap(),
        State::Committed
    );
    assert!(f.fixture.observation.retained_guard_sha256.is_some());
    assert_eq!(
        Engine::reconcile(&mut f.ledger, &f.plan.transaction_id, &mut f.fixture).unwrap(),
        State::Committed
    );
    assert_eq!(f.fixture.observation.retained_guard_sha256, None);
}
#[test]
fn model_failure_restores_only_model_and_manifest_without_system_profile_or_boot_mutation() {
    let mut p = plan();
    p.candidate = Candidate::ModelOnly {
        artifact: model("new-model"),
    };
    let mut f = Harness::new(p);
    let mut e = f.verified();
    assert_eq!(
        e.tick(&mut f.ledger, &mut f.fixture, 181000).unwrap(),
        State::RolledBack
    );
    assert_eq!(f.fixture.observation.model, f.plan.prior.model);
    assert!(!f.fixture.effects.iter().any(|x| matches!(
        x,
        Effect::RecoverSystem { .. } | Effect::SetProfile { .. } | Effect::InstallBoot { .. }
    )));
}
#[test]
fn model_commit_preserves_system_profile_and_boot() {
    let mut p = plan();
    p.candidate = Candidate::ModelOnly {
        artifact: model("new-model"),
    };
    let mut f = Harness::new(p);
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap();
    assert_eq!(f.fixture.observation.profile, f.plan.prior.profile.path);
    assert_eq!(f.fixture.observation.boot, f.plan.prior.boot.path);
}
#[test]
fn commit_failures_at_each_pointer_restore_prior_and_never_claim_committed() {
    for phase in ["profile", "boot", "managed"] {
        let mut f = Harness::new(plan());
        let mut e = f.verified();
        e.heartbeat(f.heartbeat(&e), 1002).unwrap();
        f.fixture.fail = Some(phase);
        assert_eq!(
            e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
            State::RolledBack
        );
        assert_eq!(
            f.ledger.state(&f.plan.transaction_id).unwrap(),
            State::RolledBack
        );
    }
}
#[test]
fn recovery_failure_is_explicit_and_blocks_new_transactions() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    f.fixture.fail = Some("recover");
    assert_eq!(
        e.tick(&mut f.ledger, &mut f.fixture, 181000),
        Err(Error::RecoveryRequired)
    );
    assert_eq!(
        Engine::register(plan(), &mut f.ledger).err(),
        Some(Error::Conflict)
    );
}
#[test]
fn baseline_degradation_is_preserved_but_new_protected_failure_is_recovered() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    f.fixture.observation.units.push(UnitHealth {
        name: "aios-build.service".into(),
        active: false,
        failed: true,
    });
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003),
        Err(Error::RecoveryRequired)
    );
}
#[test]
fn bus_mount_api_and_user_activation_checks_are_independent() {
    for missing in 0..4 {
        let mut f = Harness::new(plan());
        let mut e = f.engine();
        match missing {
            0 => f.fixture.observation.system_bus = false,
            1 => f.fixture.observation.mounts.clear(),
            2 => f.fixture.observation.apis.clear(),
            _ => f.fixture.observation.user_units.clear(),
        };
        assert_eq!(
            e.arm(&mut f.ledger, &mut f.fixture, 1000),
            Err(Error::Health)
        );
        assert!(f.fixture.effects.is_empty());
        assert_eq!(
            f.ledger.state(&f.plan.transaction_id).unwrap(),
            State::Rejected
        );
    }
}
#[test]
fn baseline_and_candidate_user_executables_are_checked_in_their_own_phase() {
    let mut f = Harness::new(plan());
    assert_ne!(
        f.plan.health.baseline_user_units,
        f.plan.health.required_user_units
    );
    let mut e = f.engine();
    assert_eq!(
        e.arm(&mut f.ledger, &mut f.fixture, 1000).unwrap(),
        State::GuardArmed
    );
    assert_eq!(
        f.fixture.observation.user_units,
        f.plan.health.baseline_user_units
    );
    assert_eq!(
        e.test(&mut f.ledger, &mut f.fixture, 1001).unwrap(),
        State::Verifying
    );
    assert_eq!(
        f.fixture.observation.user_units,
        f.plan.health.required_user_units
    );
}
#[test]
fn lost_or_changed_retained_guard_during_activation_cannot_commit() {
    let mut f = Harness::new(plan());
    let mut e = f.engine();
    e.arm(&mut f.ledger, &mut f.fixture, 1000).unwrap();
    f.fixture.guard_lost_on_test = true;
    assert_eq!(
        e.test(&mut f.ledger, &mut f.fixture, 1001).unwrap(),
        State::RolledBack
    );
}
#[test]
fn kernel_initrd_and_boot_adapter_changes_require_separate_reboot() {
    for which in 0..3 {
        let mut p = plan();
        if let Candidate::System { closure } = &mut p.candidate {
            match which {
                0 => closure.kernel_sha256 = h('f'),
                1 => closure.initrd_sha256 = h('f'),
                _ => closure.boot_adapter_sha256 = h('f'),
            }
        }
        let mut f = Harness::new(p);
        let mut e = f.engine();
        assert_eq!(
            e.arm(&mut f.ledger, &mut f.fixture, 1000).unwrap(),
            State::AwaitingReboot
        );
        assert!(f.fixture.effects.is_empty());
    }
}
#[test]
fn arbitrary_paths_commands_unknown_fields_duplicate_fields_and_bad_versions_are_rejected() {
    let p = plan();
    let json = serde_json::to_string(&p).unwrap();
    let bytes = json.replacen("{", "{\"command\":\"true\",", 1);
    assert!(Plan::from_json(bytes.as_bytes()).is_err());
    let duplicate = json.replacen("{", "{\"schema_version\":1,", 1);
    assert!(Plan::from_json(duplicate.as_bytes()).is_err());
    for path in [
        "/etc/nixos",
        "/nix/store/../host",
        "/nix/store/a-name; true",
        "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-name/../../x",
    ] {
        let mut q = p.clone();
        q.prior.running.path = path.into();
        assert!(q.validate().is_err());
    }
    let mut q = p.clone();
    q.schema_version = 2;
    assert!(q.validate().is_err());
    q = p.clone();
    q.guard_timeout_seconds = 59;
    assert!(q.validate().is_err());
    q = p.clone();
    q.health.baseline_units.push(UnitHealth {
        name: "user-supplied.service".into(),
        active: true,
        failed: false,
    });
    assert!(q.validate().is_err());
    q = p;
    q.health.baseline_user_units.clear();
    assert_eq!(q.validate(), Err(Error::InvalidPlan));
}
#[test]
fn canonical_digest_ignores_input_key_order_and_binds_health_and_all_pointers() {
    let p = plan();
    let reordered = serde_json::to_vec(&serde_json::to_value(&p).unwrap()).unwrap();
    assert_eq!(
        Plan::from_json(&reordered).unwrap().digest().unwrap(),
        p.digest().unwrap()
    );
    let mut q = p.clone();
    q.prior.boot = closure("other-boot");
    assert_ne!(q.digest().unwrap(), p.digest().unwrap());
    q = p.clone();
    q.health.required_mounts.push("/var".into());
    assert_ne!(q.digest().unwrap(), p.digest().unwrap());
}
#[test]
fn ledger_corruption_and_stale_state_prevent_effects() {
    let mut f = Harness::new(plan());
    let e = f.engine();
    let db = Connection::open(&f.fixture.database).unwrap();
    db.execute(
        "UPDATE guard_transactions SET digest=?1 WHERE id=?2",
        params![h('0'), f.plan.transaction_id],
    )
    .unwrap();
    drop(e);
    assert_eq!(
        Engine::reconcile(&mut f.ledger, &f.plan.transaction_id, &mut f.fixture),
        Err(Error::Ledger)
    );
    assert!(f.fixture.effects.is_empty());
}
#[test]
fn exact_activation_commands_have_no_rebuild_shell_mutable_source_or_bypass_environment() {
    let p = plan();
    let nix =
        activation::RetainedNix::from_store_package(&format!("/nix/store/{}-nix", "a".repeat(32)))
            .unwrap();
    let candidate = match &p.candidate {
        Candidate::System { closure } => closure.path.clone(),
        _ => unreachable!(),
    };
    let effect = Effect::TestSystem {
        closure: candidate.clone(),
    };
    let cmd = activation::command(&p, &effect, &nix).unwrap().unwrap();
    assert_eq!(
        cmd.executable,
        format!("{candidate}/bin/switch-to-configuration")
    );
    assert_eq!(cmd.args, ["test"]);
    assert!(cmd.clear_environment);
    assert!(!cmd.environment.contains_key("NIXOS_NO_CHECK"));
    assert!(!cmd.environment.contains_key("NIXOS_NO_SYNC"));
    assert!(
        activation::command(
            &p,
            &Effect::TestSystem {
                closure: closure("unapproved").path
            },
            &nix
        )
        .is_err()
    );
    let cmd = activation::command(
        &p,
        &Effect::SetProfile {
            closure: candidate.clone(),
        },
        &nix,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        cmd.args,
        [
            "--profile",
            "/nix/var/nix/profiles/system",
            "--set",
            &candidate
        ]
    );
}

#[test]
fn deadline_expiring_inside_test_or_commit_prevents_success_and_restores_prior() {
    for phase in ["test", "profile", "boot", "managed"] {
        let mut f = Harness::new(plan());
        if phase == "test" {
            let mut e = f.engine();
            e.arm(&mut f.ledger, &mut f.fixture, 1000).unwrap();
            f.fixture.expire_on = Some(phase);
            assert_eq!(
                e.test(&mut f.ledger, &mut f.fixture, 1001).unwrap(),
                State::RolledBack
            );
        } else {
            let mut e = f.verified();
            e.heartbeat(f.heartbeat(&e), 1002).unwrap();
            f.fixture.expire_on = Some(phase);
            assert_eq!(
                e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
                State::RolledBack
            );
        }
        assert_eq!(
            f.ledger.state(&f.plan.transaction_id).unwrap(),
            State::RolledBack
        );
    }
}
#[test]
fn stale_event_revision_cannot_execute_an_effect_even_if_state_name_matches() {
    let mut f = Harness::new(plan());
    let mut e = f.engine();
    Connection::open(&f.fixture.database)
        .unwrap()
        .execute(
            "UPDATE guard_transactions SET revision=revision+1 WHERE id=?1",
            [&f.plan.transaction_id],
        )
        .unwrap();
    assert_eq!(
        e.arm(&mut f.ledger, &mut f.fixture, 1000),
        Err(Error::State)
    );
    assert!(f.fixture.effects.is_empty());
}

#[test]
fn incomplete_health_policy_cannot_omit_bus_ssh_or_mandatory_product_apis() {
    for missing in 0..3 {
        let mut p = plan();
        match missing {
            0 => p.health.baseline_units.retain(|u| u.name != "dbus.service"),
            1 => p.health.baseline_units.retain(|u| u.name != "sshd.service"),
            _ => p.health.required_apis.clear(),
        };
        assert_eq!(p.validate(), Err(Error::InvalidPlan));
    }
}
#[test]
fn unknown_or_partial_ledger_schema_is_not_regenerated() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE guard_schema(version INTEGER);INSERT INTO guard_schema VALUES(2);",
    )
    .unwrap();
    assert!(Ledger::new(db).is_err());
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE guard_schema(version INTEGER);INSERT INTO guard_schema VALUES(1);",
    )
    .unwrap();
    assert!(Ledger::new(db).is_err());
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE guard_events(something TEXT);")
        .unwrap();
    assert!(Ledger::new(db).is_err());
}
#[test]
fn final_pointer_disagreement_recovers_even_after_commands_report_success() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.corrupt_committed = true;
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
        State::RolledBack
    );
}

#[test]
fn contradictory_or_duplicate_health_facts_cannot_hide_a_failure() {
    for field in 0..5 {
        let mut f = Harness::new(plan());
        let mut e = f.engine();
        match field {
            0 => f
                .fixture
                .observation
                .units
                .push(f.fixture.observation.units[0].clone()),
            1 => f.fixture.observation.apis.push(ApiHealth {
                api: Api::Executor,
                uid: None,
                healthy: false,
            }),
            2 => f
                .fixture
                .observation
                .user_units
                .push(f.fixture.observation.user_units[0].clone()),
            3 => f.fixture.observation.mounts.push("/".into()),
            _ => f.fixture.observation.units[0].failed = true,
        };
        assert_eq!(
            e.arm(&mut f.ledger, &mut f.fixture, 1000),
            Err(Error::Health)
        );
        assert!(f.fixture.effects.is_empty());
    }
}
#[test]
fn target_drift_between_commit_effects_stops_further_mutations() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.drift_after_profile = true;
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003),
        Err(Error::RecoveryRequired)
    );
    assert_eq!(
        f.ledger.state(&f.plan.transaction_id).unwrap(),
        State::RecoveryRequired
    );
    assert!(matches!(
        f.fixture.effects.last(),
        Some(Effect::SetProfile { .. })
    ));
}
#[test]
fn losing_guard_during_pointer_publication_cannot_finish_commit() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.guard_lost_after_managed = true;
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003).unwrap(),
        State::RolledBack
    );
}
#[test]
fn observation_or_clock_failure_after_activation_recovers_prior_state() {
    for clock in [false, true] {
        let mut f = Harness::new(plan());
        let mut e = f.engine();
        e.arm(&mut f.ledger, &mut f.fixture, 1000).unwrap();
        f.fixture.observe_failure_after_test = !clock;
        f.fixture.clock_failure_after_test = clock;
        assert_eq!(
            e.test(&mut f.ledger, &mut f.fixture, 1001).unwrap(),
            State::RolledBack
        );
        assert_eq!(f.fixture.observation.running, f.plan.prior.running.path);
    }
}
#[test]
fn unavailable_recovery_observation_persists_recovery_required() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    f.fixture.fail_observe = true;
    assert_eq!(
        e.tick(&mut f.ledger, &mut f.fixture, 181000),
        Err(Error::RecoveryRequired)
    );
    assert_eq!(
        f.ledger.state(&f.plan.transaction_id).unwrap(),
        State::RecoveryRequired
    );
}

#[test]
fn unlisted_model_drift_during_system_change_cannot_be_committed() {
    let mut f = Harness::new(plan());
    let mut e = f.verified();
    e.heartbeat(f.heartbeat(&e), 1002).unwrap();
    f.fixture.observation.model = Some(model("unlisted-model"));
    assert_eq!(
        e.commit(&mut f.ledger, &mut f.fixture, 1003),
        Err(Error::RecoveryRequired)
    );
    assert_eq!(
        f.ledger.state(&f.plan.transaction_id).unwrap(),
        State::RecoveryRequired
    );
    assert!(
        !f.fixture
            .effects
            .iter()
            .any(|e| matches!(e, Effect::DisarmGuard { .. }))
    );
}
#[test]
fn non_nix_base32_store_hash_is_rejected() {
    let mut p = plan();
    p.prior.running.path = format!("/nix/store/{}-invalid-hash", "e".repeat(32));
    assert_eq!(p.validate(), Err(Error::InvalidPlan));
}
