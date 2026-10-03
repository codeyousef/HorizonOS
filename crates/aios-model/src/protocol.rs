//! Inference has no tool executor. These schemas constrain proposals only.
use aios_protocol::{MAX_TASK_BYTES, contracts::{ErrorCode, parse_tool_call}};
use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Profile { Normal, Low, High }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode { Decision, FinalAnswer }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadTool { SystemInfo, SystemServiceStatus }

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    pub profile: Profile,
    pub system_prompt: String,
    pub user_prompt: String,
    pub response_mode: ResponseMode,
    pub allowed_tools: Vec<ReadTool>,
    pub evidence_ids: Vec<String>,
    pub deadline_ms: u32,
}

impl Generation {
    pub fn validate(&self) -> Result<(), ErrorCode> {
        if self.profile != Profile::Normal { return Err(ErrorCode::ModelUnavailable); }
        if self.system_prompt.is_empty() || self.system_prompt.len() > 16384 ||
            self.user_prompt.is_empty() || self.user_prompt.len() > 48000 ||
            self.system_prompt.contains('\0') || self.user_prompt.contains('\0') ||
            self.deadline_ms == 0 || self.deadline_ms > 90000 ||
            self.allowed_tools.len() > 8 || self.evidence_ids.len() > 64 {
            return Err(ErrorCode::InvalidArgument);
        }
        if self.response_mode == ResponseMode::FinalAnswer && !self.allowed_tools.is_empty() {
            return Err(ErrorCode::InvalidArgument);
        }
        let mut ids = HashSet::new();
        for id in &self.evidence_ids {
            if id.is_empty() || id.len() > 128 ||
                !id.bytes().all(|b| b.is_ascii_alphanumeric() || b"_:-".contains(&b)) ||
                !ids.insert(id) { return Err(ErrorCode::InvalidArgument); }
        }
        let mut tools = Vec::new();
        for tool in &self.allowed_tools {
            if tools.contains(tool) { return Err(ErrorCode::InvalidArgument); }
            tools.push(*tool);
        }
        Ok(())
    }
    pub fn output_budget(&self) -> u32 {
        match self.response_mode { ResponseMode::Decision => 192, ResponseMode::FinalAnswer => 768 }
    }
    pub fn grammar(&self) -> Result<String, ErrorCode> {
        self.validate()?;
        let mut root = "root ::= ws (answer | clarification | abstain".to_owned();
        for tool in &self.allowed_tools {
            root.push_str(match tool { ReadTool::SystemInfo => " | system-info", ReadTool::SystemServiceStatus => " | service-status" });
        }
        root.push_str(") ws\n");
        let ids = self.evidence_ids.iter().map(|id| {
            // A grammar terminal containing the JSON-encoded ID. Both levels
            // are escaped by serde; client text is never grammar source.
            serde_json::to_string(&serde_json::to_string(id).unwrap()).unwrap()
        }).collect::<Vec<_>>().join(" | ");
        root.push_str(r#"answer ::= "{" ws "\"kind\"" ws ":" ws "\"answer\"" ws "," ws "\"text\"" ws ":" ws string ws "," ws "\"evidence_ids\"" ws ":" ws "[" ws references ws "]" ws "}"
clarification ::= "{" ws "\"kind\"" ws ":" ws "\"clarification\"" ws "," ws "\"question\"" ws ":" ws string ws "}"
abstain ::= "{" ws "\"kind\"" ws ":" ws "\"abstain\"" ws "," ws "\"reason\"" ws ":" ws string ws "}"
system-info ::= "{" ws "\"kind\"" ws ":" ws "\"tool_call\"" ws "," ws "\"action_id\"" ws ":" ws "\"system.info\"" ws "," ws "\"arguments\"" ws ":" ws "{" ws "}" ws "}"
service-status ::= "{" ws "\"kind\"" ws ":" ws "\"tool_call\"" ws "," ws "\"action_id\"" ws ":" ws "\"system.service_status\"" ws "," ws "\"arguments\"" ws ":" ws "{" ws "\"service_id\"" ws ":" ws string ws "}" ws "}"
string ::= "\"" ([^"\\\x00-\x1F] | "\\" (["\\/bfnrt] | "u" [0-9a-fA-F]{4}))* "\""
ws ::= [ \t\n\r]*
"#);
        if ids.is_empty() { root.push_str("references ::= \"\"\n"); }
        else { root.push_str(&format!("references ::= (reference (ws \",\" ws reference)*)?\nreference ::= {ids}\n")); }
        if root.len() > 32768 { return Err(ErrorCode::ContextBudgetExceeded); }
        Ok(root)
    }
    pub fn parse_output(&self, output: &str) -> Result<Value, ErrorCode> {
        #[derive(Deserialize)] struct Kind { kind: String }
        if output.len() > MAX_TASK_BYTES { return Err(ErrorCode::ModelOutputInvalid); }
        let kind: Kind = serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)?;
        match kind.kind.as_str() {
            "answer" => {
                #[derive(Deserialize)] #[serde(deny_unknown_fields)]
                struct Answer { kind: String, text: String, evidence_ids: Vec<String> }
                let answer: Answer = serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)?;
                let mut seen = HashSet::new();
                if answer.kind != "answer" || answer.text.trim().is_empty() || answer.text.len() > 8192 ||
                    answer.evidence_ids.len() > 64 || answer.evidence_ids.iter().any(|id| !self.evidence_ids.contains(id) || !seen.insert(id)) {
                    return Err(ErrorCode::ModelOutputInvalid);
                }
            },
            "clarification" | "abstain" => {
                #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Clarification { kind: String, question: String }
                #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Abstain { kind: String, reason: String }
                let text = if kind.kind == "clarification" {
                    let parsed: Clarification = serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)?;
                    if parsed.kind != "clarification" { return Err(ErrorCode::ModelOutputInvalid); } parsed.question
                } else {
                    let parsed: Abstain = serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)?;
                    if parsed.kind != "abstain" { return Err(ErrorCode::ModelOutputInvalid); } parsed.reason
                };
                if text.trim().is_empty() || text.len() > 4096 { return Err(ErrorCode::ModelOutputInvalid); }
            },
            "tool_call" => {
                let action = parse_tool_call(output.as_bytes()).map_err(|_| ErrorCode::ModelOutputInvalid)?;
                let tool = match action {
                    aios_protocol::contracts::Action::SystemInfo => ReadTool::SystemInfo,
                    aios_protocol::contracts::Action::SystemServiceStatus(_) => ReadTool::SystemServiceStatus,
                    _ => return Err(ErrorCode::ModelOutputInvalid),
                };
                if !self.allowed_tools.contains(&tool) { return Err(ErrorCode::ModelOutputInvalid); }
            },
            _ => return Err(ErrorCode::ModelOutputInvalid),
        }
        serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)
    }
}

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
        for raw in [r#"{"kind":"answer","text":"observed","evidence_ids":["ev_other"]}"#,
            r#"{"kind":"answer","text":"x","text":"y","evidence_ids":[]}"#,
            r#"{"kind":"answer","text":"x","evidence_ids":[],"approved":true}"#,
            r#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#] {
            assert_eq!(request.parse_output(raw),Err(ErrorCode::ModelOutputInvalid));
        }
        let mut request = request;request.response_mode = ResponseMode::Decision;request.allowed_tools.push(ReadTool::SystemInfo);
        assert!(request.parse_output(r#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#).is_ok());
        assert!(request.parse_output(r#"{"kind":"tool_call","action_id":"shell.run","arguments":{}}"#).is_err());
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
            assert_eq!(candidate.parse_output(&output.to_string()), Err(ErrorCode::ModelOutputInvalid));
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
