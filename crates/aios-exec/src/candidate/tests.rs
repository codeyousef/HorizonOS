//! Filesystem fixtures run as the enrolled guest dev UID; no root qualification.
use super::*;
use serde_json::json;
use std::os::unix::fs::symlink;
pub(crate) struct Fixture {
    pub(crate) base: PathBuf,
    pub(crate) template: InstalledTemplate,
    pub(crate) store: CandidateStore,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fn clean(p: &Path) {
            if p.is_dir() && !p.is_symlink() {
                let _ = fs::set_permissions(p, Permissions::from_mode(0o700));
                if let Ok(entries) = fs::read_dir(p) {
                    for e in entries.flatten() {
                        clean(&e.path());
                    }
                }
            }
        }
        clean(&self.base);
        let _ = fs::remove_dir_all(&self.base);
    }
}
pub(crate) fn fixture() -> Fixture {
    let base = std::env::temp_dir().join(format!("aios-candidate-fixture-{}", Uuid::new_v4()));
    fs::create_dir(&base).unwrap();
    let source = base.join("installed");
    fs::create_dir(&source).unwrap();
    let output = base.join("candidates");
    fs::create_dir(&output).unwrap();
    fs::set_permissions(&output, Permissions::from_mode(0o755)).unwrap();
    let lock = b"fixture locked inputs";
    let mut packages = vec![];
    for (id, attribute, cap, version, bin) in [
        (
            "kate",
            json!(["kdePackages", "kate"]),
            "desktop_application",
            "fixture-1",
            "kate",
        ),
        (
            "postgresql-17",
            json!(["postgresql_17"]),
            "postgresql17",
            "17.11",
            "pg_isready",
        ),
    ] {
        let mut p = json!({"id":id,"attribute":attribute,"display_name":id,"version":version,"licenses":["MIT"],"unfree":false,"platform":"x86_64-linux","binaries":[bin],"desktop_ids":[],"capability":cap});
        p["metadata_revision"] = json!(sha256(&canonical(&p).unwrap()));
        packages.push(p);
    }
    let options = [
        ("power_policy.profile_on_ac", "power_profile"),
        ("power_policy.profile_on_battery", "power_profile"),
        ("services.openssh.enabled", "boolean"),
        ("services.openssh.open_firewall", "boolean"),
        ("services.postgresql.enabled", "boolean"),
        (
            "services.postgresql.listen_mode",
            "postgresql_listen_mode",
        ),
        ("services.postgresql.package_id", "package_id"),
    ]
    .into_iter()
    .map(|(id, value_kind)| {
        let mut option = json!({"id":id,"value_kind":value_kind});
        option["metadata_revision"] = json!(sha256(&canonical(&option).unwrap()));
        option
    })
    .collect::<Vec<_>>();
    let content = json!({"schema_version":1,"base_template_revision":"1".repeat(64),"lock_sha256":sha256(lock),"nixpkgs_revision":"774debe7a0d1b496e35677ad955a1011c6ff74f3","installation_state_version":"26.05","platform":"x86_64-linux","packages":packages,"options":options});
    let catalog = canonical(
        &json!({"catalog_revision":sha256(&canonical(&content).unwrap()),"content":content}),
    )
    .unwrap();
    let data = [
        ("catalog.json", catalog),
        ("flake.lock", lock.to_vec()),
        (
            "flake.nix",
            b"# fixed template fixture, no actual Nix evaluation".to_vec(),
        ),
        ("modules/fixed.nix", b"# trusted source fixture".to_vec()),
    ];
    let mut files = vec![];
    for (path, bytes) in data {
        let destination = source.join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, &bytes).unwrap();
        fs::set_permissions(&destination, Permissions::from_mode(0o444)).unwrap();
        files.push(FileEntry {
            path: path.into(),
            mode: 0o644,
            size: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = TemplateManifest {
        schema_version: 1,
        files,
    };
    let bytes = canonical(&manifest).unwrap();
    fs::write(source.join("template.json"), &bytes).unwrap();
    fs::set_permissions(source.join("template.json"), Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(source.join("modules"), Permissions::from_mode(0o555)).unwrap();
    fs::set_permissions(&source, Permissions::from_mode(0o555)).unwrap();
    let owner = unsafe { libc::geteuid() };
    let template = InstalledTemplate::load(&source, &sha256(&bytes), owner, false).unwrap();
    let store = CandidateStore::load(&output, owner, false).unwrap();
    Fixture {
        base,
        template,
        store,
    }
}
fn defaults(f: &Fixture) -> Compiled {
    f.template
        .catalog
        .compile(&canonical(&f.template.catalog.defaults()).unwrap())
        .unwrap()
}
#[test]
fn immutable_copy_contains_canonical_data_and_exact_revisions() {
    let f = fixture();
    let state = defaults(&f);
    let c = f.store.prepare(&f.template, &state).unwrap();
    f.store.verify(&c).unwrap();
    assert_eq!(fs::read(c.path().join(MANAGED)).unwrap(), state.bytes);
    assert_eq!(c.manifest().managed_sha256, state.digest);
    assert_eq!(c.manifest().catalog_revision, f.template.catalog.revision());
    assert_eq!(
        c.manifest().lock_sha256,
        f.template.catalog.content().lock_sha256
    );
    assert_eq!(fs::metadata(c.path()).unwrap().mode() & 0o777, 0o555);
    assert!(
        c.manifest()
            .files
            .iter()
            .all(|file| fs::metadata(c.path().join(&file.path)).unwrap().mode() & 0o777 == 0o444)
    );
}
#[test]
fn repeated_publication_deduplicates_without_replacing() {
    let f = fixture();
    let state = defaults(&f);
    let one = f.store.prepare(&f.template, &state).unwrap();
    let inode = fs::metadata(one.path()).unwrap().ino();
    let two = f.store.prepare(&f.template, &state).unwrap();
    assert_eq!(one.digest(), two.digest());
    assert_eq!(fs::metadata(two.path()).unwrap().ino(), inode);
    assert_eq!(fs::read_dir(&f.store.path).unwrap().count(), 1);
}
#[test]
fn different_managed_state_has_distinct_registered_digest() {
    let f = fixture();
    let one = f.store.prepare(&f.template, &defaults(&f)).unwrap();
    let mut state = f.template.catalog.defaults();
    state.system_packages.push("kate".into());
    let compiled = f
        .template
        .catalog
        .compile(&canonical(&state).unwrap())
        .unwrap();
    let two = f.store.prepare(&f.template, &compiled).unwrap();
    assert_ne!(one.digest(), two.digest());
    f.store.verify(&one).unwrap();
    f.store.verify(&two).unwrap();
}
#[test]
fn template_mutation_is_rejected_and_own_staging_is_cleaned() {
    let f = fixture();
    let source = f.base.join("installed/modules/fixed.nix");
    fs::set_permissions(&source, Permissions::from_mode(0o644)).unwrap();
    fs::write(&source, b"malicious change").unwrap();
    fs::set_permissions(&source, Permissions::from_mode(0o444)).unwrap();
    assert!(f.store.prepare(&f.template, &defaults(&f)).is_err());
    assert_eq!(fs::read_dir(&f.store.path).unwrap().count(), 0);
}
#[test]
fn source_symlink_and_unlisted_files_are_not_channels() {
    let f = fixture();
    let source = f.base.join("installed");
    fs::set_permissions(&source, Permissions::from_mode(0o755)).unwrap();
    symlink("/etc/passwd", source.join("unexpected")).unwrap();
    fs::set_permissions(&source, Permissions::from_mode(0o555)).unwrap();
    assert!(f.store.prepare(&f.template, &defaults(&f)).is_err());
}
#[test]
fn corrupt_existing_candidate_is_never_replaced_or_removed() {
    let f = fixture();
    let state = defaults(&f);
    let c = f.store.prepare(&f.template, &state).unwrap();
    let target = c.path().join(MANAGED);
    fs::set_permissions(&target, Permissions::from_mode(0o644)).unwrap();
    fs::write(&target, b"{}").unwrap();
    fs::set_permissions(&target, Permissions::from_mode(0o444)).unwrap();
    assert!(f.store.prepare(&f.template, &state).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"{}");
    assert_eq!(fs::read_dir(&f.store.path).unwrap().count(), 1);
}
#[test]
fn published_hardlink_is_rejected_but_nix_template_optimisation_is_allowed() {
    let f = fixture();
    let state = defaults(&f);
    let source = f.base.join("installed/flake.nix");
    fs::hard_link(&source, f.base.join("store-optimisation")).unwrap();
    let c = f.store.prepare(&f.template, &state).unwrap();
    f.store.verify(&c).unwrap();
    fs::hard_link(
        c.path().join("flake.nix"),
        f.base.join("unsafe-candidate-link"),
    )
    .unwrap();
    assert_eq!(f.store.verify(&c), Err(Error::Integrity));
}
#[test]
fn forged_compiler_result_cannot_select_data_or_catalog() {
    let f = fixture();
    let mut state = defaults(&f);
    state.state.system_packages.push("kate".into());
    assert!(f.store.prepare(&f.template, &state).is_err());
    let mut state = defaults(&f);
    state.digest = "0".repeat(64);
    assert!(f.store.prepare(&f.template, &state).is_err());
}
#[test]
fn traversal_reserved_and_conflicting_paths_fail_before_copy() {
    for name in [
        "../escape",
        "/absolute",
        "a//b",
        "a/./b",
        "a/../b",
        "a\\b",
        "managed.json",
        "candidate.json",
        "template.json",
    ] {
        let mut f = fixture();
        f.template.manifest.files[0].path = name.into();
        assert!(validate_entries(&f.template.manifest.files, false).is_err());
    }
}
#[test]
fn source_and_store_anchors_cannot_be_replaced_between_operations() {
    let f = fixture();
    let state = defaults(&f);
    fs::rename(&f.store.path, f.base.join("original-candidates")).unwrap();
    fs::create_dir(&f.store.path).unwrap();
    fs::set_permissions(&f.store.path, Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        f.store.prepare(&f.template, &state).err(),
        Some(Error::TargetChanged)
    );
    assert_eq!(fs::read_dir(&f.store.path).unwrap().count(), 0);
}
#[test]
fn candidate_digest_symlink_cannot_overwrite_external_target() {
    let f = fixture();
    let state = defaults(&f);
    let c = f.store.prepare(&f.template, &state).unwrap();
    let digest = c.digest().to_string();
    fs::set_permissions(c.path(), Permissions::from_mode(0o755)).unwrap();
    fs::rename(c.path(), f.base.join("saved-candidate")).unwrap();
    fs::set_permissions(
        f.base.join("saved-candidate"),
        Permissions::from_mode(0o555),
    )
    .unwrap();
    let other = f.base.join("external");
    fs::create_dir(&other).unwrap();
    fs::write(other.join("sentinel"), b"keep").unwrap();
    symlink(&other, f.store.path.join(digest)).unwrap();
    assert!(f.store.prepare(&f.template, &state).is_err());
    assert_eq!(fs::read(other.join("sentinel")).unwrap(), b"keep");
}
#[test]
fn production_constructors_deny_nonroot_without_fixture_override() {
    if unsafe { libc::geteuid() } != 0 {
        assert_eq!(CandidateStore::open().err(), Some(Error::Authority));
        assert_eq!(
            InstalledTemplate::open(
                "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-template",
                &"0".repeat(64)
            )
            .err(),
            Some(Error::Authority)
        );
    }
}
