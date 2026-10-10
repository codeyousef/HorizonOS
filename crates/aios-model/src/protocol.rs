//! Inference wire operations; the independently parsed generation contract is shared.
pub use aios_protocol::inference::{Generation, Profile, ResponseMode, ReadTool};
use aios_protocol::contracts::ErrorCode;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request { pub schema_version: u32, pub request_id: String, pub operation: Box<RawValue> }
pub enum Operation { Generate(Generation), GetStatus, GetResult(String), Cancel(String), Unload }

pub fn parse_operation(raw: &str) -> Result<Operation, ErrorCode> {
    #[derive(Deserialize)] struct Kind { kind: String }
    let kind: Kind = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
    match kind.kind.as_str() {
        "generate" => {
            #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Fields { kind: String, generation: Generation }
            let parsed: Fields = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
            if parsed.kind != "generate" { return Err(ErrorCode::InvalidArgument); }
            parsed.generation.validate()?; Ok(Operation::Generate(parsed.generation))
        },
        "get_result" | "cancel" => {
            #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Fields { kind: String, generation_id: String }
            let parsed: Fields = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
            if uuid::Uuid::parse_str(&parsed.generation_id).is_err() { return Err(ErrorCode::InvalidArgument); }
            if parsed.kind == "cancel" { Ok(Operation::Cancel(parsed.generation_id)) } else { Ok(Operation::GetResult(parsed.generation_id)) }
        },
        "get_status" | "unload" => {
            #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Fields { kind: String }
            let parsed: Fields = serde_json::from_str(raw).map_err(|_| ErrorCode::InvalidArgument)?;
            match parsed.kind.as_str() { "get_status" => Ok(Operation::GetStatus), "unload" => Ok(Operation::Unload), _ => Err(ErrorCode::InvalidArgument) }
        },
        _ => Err(ErrorCode::UnknownCapability),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Generation { Generation { profile: Profile::Normal, system_prompt: "system".into(), user_prompt: "question".into(),
        response_mode: ResponseMode::FinalAnswer, allowed_tools: vec![], evidence_ids: vec!["ev_one".into()], deadline_ms: 90000 } }
    #[test] fn output_proposals_cannot_gain_authority_or_references() {
        let request = request();
        assert!(request.parse_output(r#"{"kind":"answer","text":"observed","evidence_ids":["ev_one"]}"#).is_ok());
        assert!(request.parse_output(r#"{"kind":"answer","text":"observed","evidence_ids":["ev_one"],"claims":[{"kind":"observed","evidence_id":"ev_one","json_pointer":"/bytes","expected":42,"statement":"42 bytes observed"},{"kind":"hypothesis","statement":"Cause remains unknown"},{"kind":"missing_evidence","statement":"No causation evidence"}]}"#).is_ok());
        assert!(request.grammar().unwrap().contains("observed-claim"));
        assert_eq!(request.parse_output(r#"{"kind":"answer","text":"unsupported","evidence_ids":["ev_one"],"claims":[{"kind":"observed","evidence_id":"ev_one","json_pointer":"file:///etc/shadow","expected":42,"statement":"forged locator"}]}"#),Err(ErrorCode::ModelOutputInvalid));
        assert_eq!(request.parse_output(r#"{"kind":"answer","text":"observed","evidence_ids":["ev_other"]}"#),Err(ErrorCode::StaleEvidence));
        assert_eq!(request.parse_output(r#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#),Err(ErrorCode::PermissionDenied));
        for raw in [
            r#"{"kind":"answer","text":"x","text":"y","evidence_ids":[]}"#,
            r#"{"kind":"answer","text":"x","evidence_ids":[],"approved":true}"#] {
            assert_eq!(request.parse_output(raw),Err(ErrorCode::ModelOutputInvalid));
        }
        let mut request = request;request.response_mode = ResponseMode::Decision;request.allowed_tools.push(ReadTool::SystemInfo);
        assert!(request.parse_output(r#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#).is_ok());
        assert!(request.parse_output(r#"{"kind":"tool_call","action_id":"shell.run","arguments":{}}"#).is_err());
    }
    #[test] fn required_read_decision_cannot_answer_before_provider_evidence(){
        let mut request=request();request.response_mode=ResponseMode::ReadDecision;
        request.allowed_tools=vec![ReadTool::SystemServiceStatus];
        assert_eq!(request.output_budget(),192);
        aios_protocol::validation::validate(include_str!("../../../schemas/model-request.schema.json"),
            &serde_json::json!({"schema_version":1,"request_id":"4ba7f699-596d-4362-978c-70ac9cc69725","operation":{"kind":"generate","generation":request}})).unwrap();
        assert!(!request.grammar().unwrap().lines().next().unwrap().contains("answer"));
        assert_eq!(request.parse_output(r#"{"kind":"answer","text":"sshd is active","evidence_ids":["ev_one"]}"#),Err(ErrorCode::PermissionDenied));
        assert!(request.parse_output(r#"{"kind":"tool_call","action_id":"system.service_status","arguments":{"service_id":"selected"}}"#).is_ok());
        assert_eq!(request.parse_output(r#"{"kind":"tool_call","action_id":"packages.install","arguments":{}}"#),Err(ErrorCode::PermissionDenied));
    }
    #[test] fn optional_profiles_never_fall_back_and_mutating_tools_are_unavailable() {
        for profile in [Profile::Low, Profile::High] {
            let mut candidate = request();
            candidate.profile = profile;
            assert_eq!(candidate.validate(), Err(ErrorCode::ModelUnavailable));
            assert_eq!(candidate.grammar(), Err(ErrorCode::ModelUnavailable));
        }
        let mut candidate = request();
        candidate.response_mode = ResponseMode::Decision;
        candidate.allowed_tools = vec![ReadTool::SystemInfo, ReadTool::SystemServiceStatus];
        for action in ["packages.install", "system.service_restart", "files.move", "shell.run"] {
            let output = serde_json::json!({"kind":"tool_call","action_id":action,"arguments":{}});
            assert!(candidate.parse_output(&output.to_string()).is_err());
            assert!(serde_json::from_value::<ReadTool>(serde_json::Value::String(action.into())).is_err());
        }
    }
    #[test] fn client_cannot_supply_grammar_or_unknown_identity_fields() {
        for raw in [r#"{"kind":"get_status","uid":0}"#,r#"{"kind":"get_status","kind":"get_status"}"#,
            r#"{"kind":"generate","grammar":"root ::= anything"}"#] { assert!(parse_operation(raw).is_err()); }
        let mut request = request();request.evidence_ids = vec!["\" | arbitrary".into()];assert!(request.grammar().is_err());
        request.evidence_ids.clear();request.deadline_ms = 90001;assert!(request.validate().is_err());
    }
}
