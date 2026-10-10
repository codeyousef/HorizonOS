use aios_protocol::retrieval::{Answer, ClaimKind, EvidenceRecord, RetrievalAuthority, RetrievalTier,
    SourceLocator, StructuredClaim, ordered_evidence, verify_answer};
use aios_protocol::contracts::ErrorCode;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[test]
fn installed_catalog_citations_bind_the_running_closure_and_exact_option_revision() {
    let closure = fs::canonicalize("/run/current-system").expect("NixOS running closure");
    let closure = closure.to_str().expect("UTF-8 closure").to_owned();
    assert!(closure.starts_with("/nix/store/"));
    let catalog_path = format!("{closure}/etc/aios/catalog.json");
    let catalog_bytes = fs::read(&catalog_path).expect("installed catalog");
    let catalog: Value = serde_json::from_slice(&catalog_bytes).expect("installed catalog JSON");
    let option = catalog.pointer("/content/options/0").expect("reviewed installed option").clone();
    let section = option["id"].as_str().expect("option id").to_owned();
    let revision = option["metadata_revision"].clone();
    let document_sha256 = format!("{:x}", Sha256::digest(&catalog_bytes));
    let now = OffsetDateTime::now_utc().format(&Rfc3339).unwrap();
    let record = EvidenceRecord {
        evidence_id: "installed-option".into(), scope: "held-out-request".into(),
        tier: RetrievalTier::InstalledDocumentation, observed_at: now.clone(),
        locator: SourceLocator::InstalledDocumentation { closure: closure.clone(), document_sha256,
            document_id: "aios-installed-catalog".into(), section },
        data: option, complete: true,
    };
    let authority = RetrievalAuthority { scope: "held-out-request".into(), now,
        installed_closure: Some(closure), authorized_files: Default::default(), external_adapter: None };
    let answer = Answer { kind: "answer".into(), text: "The installed option metadata revision is cited.".into(),
        evidence_ids: vec!["installed-option".into()], claims: vec![StructuredClaim { kind: ClaimKind::Observed,
            evidence_id: Some("installed-option".into()), json_pointer: Some("/metadata_revision".into()),
            expected: Some(revision), statement: "Installed metadata revision observed".into() }] };
    let verified = verify_answer(&answer, std::slice::from_ref(&record), &authority, true).unwrap();
    assert_eq!(verified.observed_claims, 1);
    assert_eq!(verified.citations[0]["source_locator"]["closure"], authority.installed_closure.as_deref().unwrap());
    assert_eq!(verified.citations[0]["executable_uri"], false);
    let mut stale = authority;
    stale.installed_closure = Some("/nix/store/00000000000000000000000000000000-stale".into());
    assert_eq!(ordered_evidence(vec![record], &stale, false), Err(ErrorCode::StaleEvidence));
    assert_eq!(verified.all_cited_ids_valid, true, "every rendered ID resolved against installed evidence");
    assert_ne!(catalog["catalog_revision"], json!(null));
}
