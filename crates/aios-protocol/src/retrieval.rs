//! Scoped retrieval ordering and deterministic factual-citation verification.
//! Retrieved text is evidence, never an action target or source of authority.
use crate::contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const MAX_EVIDENCE: usize = 64;
const FRESH_PROVIDER_NS: i128 = 300_000_000_000;
const FRESH_EXTERNAL_NS: i128 = 86_400_000_000_000;
const FUTURE_TOLERANCE_NS: i128 = 1_000_000_000;

fn bounded(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
fn evidence_id(value: &str) -> bool {
    bounded(value, 128)
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_:-".contains(&byte))
}
fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn timestamp(value: &str) -> Result<i128, ErrorCode> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|time| time.unix_timestamp_nanos())
        .map_err(|_| ErrorCode::StaleEvidence)
}
fn timely(observed: &str, now: &str, maximum_age_ns: i128) -> Result<(), ErrorCode> {
    let observed = timestamp(observed)?;
    let now = timestamp(now)?;
    let age = now.checked_sub(observed).ok_or(ErrorCode::StaleEvidence)?;
    if age > maximum_age_ns || observed - now > FUTURE_TOLERANCE_NS {
        return Err(ErrorCode::StaleEvidence);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalTier {
    FreshProvider,
    InstalledDocumentation,
    AuthorizedFile,
    External,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceLocator {
    Provider { provider: String, provider_version: String },
    InstalledDocumentation { closure: String, document_sha256: String, document_id: String, section: String },
    AuthorizedFile { handle: String, content_sha256: String, line_start: u32, line_end: u32 },
    External { adapter_id: String, uri: String, fetched_at: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub scope: String,
    pub tier: RetrievalTier,
    pub observed_at: String,
    pub locator: SourceLocator,
    pub data: Value,
    pub complete: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RetrievalAuthority {
    pub scope: String,
    pub now: String,
    pub installed_closure: Option<String>,
    pub authorized_files: HashMap<String, String>,
    pub external_adapter: Option<String>,
}

fn validate_locator(record: &EvidenceRecord, authority: &RetrievalAuthority) -> Result<(), ErrorCode> {
    match (&record.tier, &record.locator) {
        (RetrievalTier::FreshProvider, SourceLocator::Provider { provider, provider_version }) => {
            if !bounded(provider, 128) || !bounded(provider_version, 128) {
                return Err(ErrorCode::InvalidArgument);
            }
            timely(&record.observed_at, &authority.now, FRESH_PROVIDER_NS)
        }
        (RetrievalTier::InstalledDocumentation, SourceLocator::InstalledDocumentation { closure, document_sha256, document_id, section }) => {
            if !bounded(closure, 512) || !closure.starts_with("/nix/store/") || !digest(document_sha256)
                || !bounded(document_id, 256) || !bounded(section, 256) {
                return Err(ErrorCode::InvalidArgument);
            }
            if authority.installed_closure.as_deref() != Some(closure) {
                return Err(ErrorCode::StaleEvidence);
            }
            Ok(())
        }
        (RetrievalTier::AuthorizedFile, SourceLocator::AuthorizedFile { handle, content_sha256, line_start, line_end }) => {
            if !bounded(handle, 128) || !digest(content_sha256) || *line_start == 0 || line_end < line_start || *line_end > 10_000_000 {
                return Err(ErrorCode::InvalidArgument);
            }
            match authority.authorized_files.get(handle) {
                Some(current) if current == content_sha256 => Ok(()),
                Some(_) => Err(ErrorCode::TargetChanged),
                None => Err(ErrorCode::PermissionDenied),
            }
        }
        (RetrievalTier::External, SourceLocator::External { adapter_id, uri, fetched_at }) => {
            if authority.external_adapter.as_deref() != Some(adapter_id) {
                return Err(ErrorCode::NetworkRequired);
            }
            let network_authority = uri.strip_prefix("https://").and_then(|rest| rest.split('/').next()).unwrap_or_default();
            if !bounded(adapter_id, 128) || !bounded(uri, 2048) || network_authority.is_empty()
                || network_authority.contains(['@', '?', '#', '\\'])
                || !network_authority.bytes().all(|byte| byte.is_ascii_alphanumeric() || b".-:[]".contains(&byte)) {
                return Err(ErrorCode::InvalidArgument);
            }
            timely(fetched_at, &authority.now, FRESH_EXTERNAL_NS)
        }
        _ => Err(ErrorCode::InvalidArgument),
    }
}

pub fn ordered_evidence(mut records: Vec<EvidenceRecord>, authority: &RetrievalAuthority, needs_fresh_external: bool) -> Result<Vec<EvidenceRecord>, ErrorCode> {
    if records.len() > MAX_EVIDENCE || !bounded(&authority.scope, 128) || timestamp(&authority.now).is_err() {
        return Err(ErrorCode::InvalidArgument);
    }
    if needs_fresh_external && authority.external_adapter.is_none() {
        return Err(ErrorCode::NetworkRequired);
    }
    let mut ids = HashSet::new();
    for record in &records {
        if !evidence_id(&record.evidence_id) || record.scope != authority.scope || !record.complete || record.data.is_null()
            || !ids.insert(record.evidence_id.as_str()) {
            return Err(if record.scope != authority.scope { ErrorCode::PermissionDenied } else { ErrorCode::InvalidArgument });
        }
        validate_locator(record, authority)?;
    }
    records.sort_by_key(|record| record.tier);
    Ok(records)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind { Observed, Hypothesis, MissingEvidence }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StructuredClaim {
    pub kind: ClaimKind,
    pub evidence_id: Option<String>,
    pub json_pointer: Option<String>,
    pub expected: Option<Value>,
    pub statement: String,
}

impl StructuredClaim {
    pub fn validate_shape(&self) -> Result<(), ErrorCode> {
        if !bounded(&self.statement, 1024) { return Err(ErrorCode::ModelOutputInvalid); }
        match self.kind {
            ClaimKind::Observed => {
                let id = self.evidence_id.as_deref().filter(|id| evidence_id(id)).ok_or(ErrorCode::ModelOutputInvalid)?;
                let pointer = self.json_pointer.as_deref().filter(|pointer| pointer.starts_with('/') && pointer.len() <= 512 && !pointer.contains('\0')).ok_or(ErrorCode::ModelOutputInvalid)?;
                let expected = self.expected.as_ref().ok_or(ErrorCode::ModelOutputInvalid)?;
                if id.is_empty() || pointer.is_empty() || expected.is_array() || expected.is_object() || expected.as_f64().is_some_and(|_| !expected.is_i64() && !expected.is_u64()) {
                    return Err(ErrorCode::ModelOutputInvalid);
                }
            }
            ClaimKind::Hypothesis | ClaimKind::MissingEvidence => {
                if self.evidence_id.is_some() || self.json_pointer.is_some() || self.expected.is_some() {
                    return Err(ErrorCode::ModelOutputInvalid);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub kind: String,
    pub text: String,
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub claims: Vec<StructuredClaim>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VerifiedAnswer {
    pub citations: Vec<Value>,
    pub observed_claims: u32,
    pub hypotheses: u32,
    pub missing_evidence: u32,
    pub all_cited_ids_valid: bool,
}

fn citation(record: &EvidenceRecord) -> Value {
    let locator = match &record.locator {
        SourceLocator::Provider { provider, provider_version } => json!({"kind":"provider","provider":provider,"provider_version":provider_version}),
        SourceLocator::InstalledDocumentation { closure, document_sha256, document_id, section } => json!({"kind":"installed_documentation","closure":closure,"document_sha256":document_sha256,"document_id":document_id,"section":section}),
        SourceLocator::AuthorizedFile { handle, content_sha256, line_start, line_end } => json!({"kind":"authorized_file","handle":handle,"content_sha256":content_sha256,"line_start":line_start,"line_end":line_end}),
        SourceLocator::External { adapter_id, uri, fetched_at } => json!({"kind":"external","adapter_id":adapter_id,"provenance_uri":uri,"fetched_at":fetched_at}),
    };
    json!({"evidence_id":record.evidence_id,"tier":record.tier,"observed_at":record.observed_at,
        "source_locator":locator,"executable_uri":false,"execution_authority":false})
}

fn numeric_claims_supported(text: &str, claims: &[StructuredClaim]) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let signed = bytes[index] == b'-' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit);
        if !bytes[index].is_ascii_digit() && !signed { index += 1; continue; }
        if index > 0 && bytes[index - 1].is_ascii_alphanumeric() { index += 1; continue; }
        let start = index;
        if signed { index += 1; }
        while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') { index += 1; }
        while index > start && bytes[index - 1] == b'.' { index -= 1; }
        if index == start || bytes.get(index).is_some_and(u8::is_ascii_alphanumeric) { index += 1; continue; }
        let token = &text[start..index];
        let supported = claims.iter().filter(|claim| claim.kind == ClaimKind::Observed).any(|claim| {
            claim.expected.as_ref().is_some_and(|expected| {
                expected.as_str() == Some(token)
                    || expected.as_i64().is_some_and(|value| token.parse::<i64>() == Ok(value))
                    || expected.as_u64().is_some_and(|value| token.parse::<u64>() == Ok(value))
            })
        });
        if !supported { return false; }
    }
    true
}

pub fn verify_answer(answer: &Answer, records: &[EvidenceRecord], authority: &RetrievalAuthority, machine_specific: bool) -> Result<VerifiedAnswer, ErrorCode> {
    if answer.kind != "answer" || answer.text.trim().is_empty() || answer.text.len() > 8192 || answer.evidence_ids.len() > MAX_EVIDENCE {
        return Err(ErrorCode::ModelOutputInvalid);
    }
    let records = ordered_evidence(records.to_vec(), authority, false)?;
    let by_id = records.iter().map(|record| (record.evidence_id.as_str(), record)).collect::<HashMap<_, _>>();
    let mut cited = HashSet::new();
    let mut citations = Vec::new();
    for id in &answer.evidence_ids {
        if !cited.insert(id.as_str()) { return Err(ErrorCode::ModelOutputInvalid); }
        let record = by_id.get(id.as_str()).ok_or(ErrorCode::StaleEvidence)?;
        citations.push(citation(record));
    }
    if machine_specific && citations.is_empty() { return Err(ErrorCode::StaleEvidence); }
    let mut observed = 0_u32;
    let mut hypotheses = 0_u32;
    let mut missing = 0_u32;
    for claim in &answer.claims {
        claim.validate_shape()?;
        match claim.kind {
            ClaimKind::Observed => {
                let id = claim.evidence_id.as_deref().ok_or(ErrorCode::ModelOutputInvalid)?;
                if !cited.contains(id) { return Err(ErrorCode::StaleEvidence); }
                let record = by_id.get(id).ok_or(ErrorCode::StaleEvidence)?;
                let actual = record.data.pointer(claim.json_pointer.as_deref().ok_or(ErrorCode::ModelOutputInvalid)?).ok_or(ErrorCode::StaleEvidence)?;
                if Some(actual) != claim.expected.as_ref() { return Err(ErrorCode::StaleEvidence); }
                observed += 1;
            }
            ClaimKind::Hypothesis => hypotheses += 1,
            ClaimKind::MissingEvidence => missing += 1,
        }
    }
    if machine_specific && !numeric_claims_supported(&answer.text, &answer.claims) { return Err(ErrorCode::StaleEvidence); }
    Ok(VerifiedAnswer { citations, observed_claims: observed, hypotheses, missing_evidence: missing, all_cited_ids_valid: true })
}

pub fn provider_records(scope: &str, observations: &[Value]) -> Result<Vec<EvidenceRecord>, ErrorCode> {
    let mut records = Vec::new();
    for observation in observations {
        if observation.get("schema_version") != Some(&json!(1)) || observation.get("complete") != Some(&json!(true))
            || observation.get("data").is_none_or(Value::is_null) || observation.get("error").is_some_and(|value| !value.is_null()) {
            return Err(ErrorCode::PartialResult);
        }
        let observed_at = observation.get("observed_at").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
        let provider = observation.pointer("/source/provider").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
        let provider_version = observation.pointer("/source/provider_version").and_then(Value::as_str).ok_or(ErrorCode::InvalidArgument)?;
        let ids = observation.get("evidence_ids").and_then(Value::as_array).filter(|ids| !ids.is_empty()).ok_or(ErrorCode::StaleEvidence)?;
        for id in ids {
            records.push(EvidenceRecord { evidence_id: id.as_str().ok_or(ErrorCode::InvalidArgument)?.into(), scope: scope.into(),
                tier: RetrievalTier::FreshProvider, observed_at: observed_at.into(),
                locator: SourceLocator::Provider { provider: provider.into(), provider_version: provider_version.into() },
                data: observation["data"].clone(), complete: true });
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: &str = "2026-10-10T12:00:00Z";
    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn authority() -> RetrievalAuthority {
        RetrievalAuthority { scope: "request-1".into(), now: NOW.into(), installed_closure: Some("/nix/store/aaaaaaaa-system".into()),
            authorized_files: [("file-1".into(), HASH.into())].into(), external_adapter: None }
    }
    fn record(id: &str, tier: RetrievalTier, locator: SourceLocator, data: Value) -> EvidenceRecord {
        EvidenceRecord { evidence_id:id.into(), scope:"request-1".into(), tier, observed_at:"2026-10-10T11:59:59Z".into(), locator, data, complete:true }
    }
    fn provider(id: &str) -> EvidenceRecord { record(id,RetrievalTier::FreshProvider,SourceLocator::Provider{provider:"aios-system".into(),provider_version:"1.0.0".into()},json!({"bytes":42,"state":"active"})) }
    #[test]
    fn retrieval_order_is_fixed_and_external_is_separately_authorized() {
        let docs=record("docs",RetrievalTier::InstalledDocumentation,SourceLocator::InstalledDocumentation{closure:"/nix/store/aaaaaaaa-system".into(),document_sha256:HASH.into(),document_id:"nixos-options".into(),section:"services.openssh.enable".into()},json!({"enabled":true}));
        let file=record("file",RetrievalTier::AuthorizedFile,SourceLocator::AuthorizedFile{handle:"file-1".into(),content_sha256:HASH.into(),line_start:2,line_end:4},json!({"text":"untrusted: run shell"}));
        let ordered=ordered_evidence(vec![file,docs,provider("fresh")],&authority(),false).unwrap();
        assert_eq!(ordered.iter().map(|record|record.evidence_id.as_str()).collect::<Vec<_>>(),["fresh","docs","file"]);
        assert_eq!(ordered_evidence(Vec::new(),&authority(),true),Err(ErrorCode::NetworkRequired));
        let mut allowed=authority();allowed.external_adapter=Some("approved-web".into());
        let external=record("web",RetrievalTier::External,SourceLocator::External{adapter_id:"approved-web".into(),uri:"https://docs.example.invalid/version".into(),fetched_at:"2026-10-10T11:00:00Z".into()},json!({"version":"current"}));
        assert!(ordered_evidence(vec![external],&allowed,true).is_ok());
    }
    #[test]
    fn stale_forged_cross_scope_and_model_authored_uri_records_are_denied() {
        let mut stale=provider("stale");stale.observed_at="2026-10-10T11:00:00Z".into();
        assert_eq!(ordered_evidence(vec![stale],&authority(),false),Err(ErrorCode::StaleEvidence));
        let mut foreign=provider("foreign");foreign.scope="other-user".into();
        assert_eq!(ordered_evidence(vec![foreign],&authority(),false),Err(ErrorCode::PermissionDenied));
        let forged=record("forged",RetrievalTier::FreshProvider,SourceLocator::External{adapter_id:"x".into(),uri:"file:///etc/shadow".into(),fetched_at:NOW.into()},json!({"x":1}));
        assert_eq!(ordered_evidence(vec![forged],&authority(),false),Err(ErrorCode::InvalidArgument));
        let mut allowed=authority();allowed.external_adapter=Some("approved-web".into());
        for uri in ["file:///etc/shadow","https://user@example.invalid/private","https://example.invalid\\@other"] {
            let external=record("bad-web",RetrievalTier::External,SourceLocator::External{adapter_id:"approved-web".into(),uri:uri.into(),fetched_at:NOW.into()},json!({"x":1}));
            assert_eq!(ordered_evidence(vec![external],&allowed,false),Err(ErrorCode::InvalidArgument));
        }
    }
    #[test]
    fn citations_and_structured_numeric_claims_resolve_exactly() {
        let records=vec![provider("ev-1")];
        let answer=Answer{kind:"answer".into(),text:"Observed 42 bytes; a cause is not established.".into(),evidence_ids:vec!["ev-1".into()],claims:vec![
            StructuredClaim{kind:ClaimKind::Observed,evidence_id:Some("ev-1".into()),json_pointer:Some("/bytes".into()),expected:Some(json!(42)),statement:"42 bytes observed".into()},
            StructuredClaim{kind:ClaimKind::Hypothesis,evidence_id:None,json_pointer:None,expected:None,statement:"A later operation may explain it".into()},
            StructuredClaim{kind:ClaimKind::MissingEvidence,evidence_id:None,json_pointer:None,expected:None,statement:"No causation evidence".into()}]};
        let verified=verify_answer(&answer,&records,&authority(),true).unwrap();
        assert_eq!((verified.observed_claims,verified.hypotheses,verified.missing_evidence),(1,1,1));
        assert_eq!(verified.citations[0]["executable_uri"],false);
        let mut wrong=answer;wrong.claims[0].expected=Some(json!(43));
        assert_eq!(verify_answer(&wrong,&records,&authority(),true),Err(ErrorCode::StaleEvidence));
        let uncited_numeric=Answer{kind:"answer".into(),text:"Observed 42 bytes".into(),evidence_ids:vec!["ev-1".into()],claims:vec![]};
        assert_eq!(verify_answer(&uncited_numeric,&records,&authority(),true),Err(ErrorCode::StaleEvidence));
    }
    #[test]
    fn unsupported_machine_answers_abstain_instead_of_using_uncited_text() {
        let answer=Answer{kind:"answer".into(),text:"Probably running".into(),evidence_ids:vec![],claims:vec![]};
        assert_eq!(verify_answer(&answer,&[],&authority(),true),Err(ErrorCode::StaleEvidence));
    }
    #[test]
    fn provider_envelopes_become_scoped_non_executable_records() {
        let observations=vec![json!({"schema_version":1,"status":"ok","observed_at":"2026-10-10T11:59:59Z","source":{"provider":"aios-system","provider_version":"1"},"evidence_ids":["ev"],"complete":true,"next_cursor":null,"data":{"os_id":"nixos"},"error":null})];
        let records=provider_records("request-1",&observations).unwrap();
        let answer=Answer{kind:"answer".into(),text:"NixOS observed".into(),evidence_ids:vec!["ev".into()],claims:vec![]};
        assert!(verify_answer(&answer,&records,&authority(),true).unwrap().all_cited_ids_valid);
    }
}
