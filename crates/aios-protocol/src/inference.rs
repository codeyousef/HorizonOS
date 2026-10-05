//! Inference has no tool executor. These schemas constrain proposals only.
use crate::{MAX_TASK_BYTES, contracts::{ErrorCode, parse_tool_call}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Profile { Normal, Low, High }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode { Decision, ReadDecision, FinalAnswer }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadTool { SystemInfo, SystemServiceStatus }

#[derive(Debug, Deserialize, Serialize)]
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
        match self.response_mode { ResponseMode::Decision | ResponseMode::ReadDecision => 192, ResponseMode::FinalAnswer => 768 }
    }
    pub fn grammar(&self) -> Result<String, ErrorCode> {
        self.validate()?;
        let mut root = "root ::= ws (clarification | abstain".to_owned();
        if self.response_mode!=ResponseMode::ReadDecision{root.push_str(" | answer");}
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
                if self.response_mode==ResponseMode::ReadDecision{return Err(ErrorCode::PermissionDenied);}
                #[derive(Deserialize)] #[serde(deny_unknown_fields)]
                struct Answer { kind: String, text: String, evidence_ids: Vec<String> }
                let answer: Answer = serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)?;
                let mut seen = HashSet::new();
                if answer.kind != "answer" || answer.text.trim().is_empty() || answer.text.len() > 8192 ||
                    answer.evidence_ids.len() > 64 || answer.evidence_ids.iter().any(|id| !seen.insert(id)) {
                    return Err(ErrorCode::ModelOutputInvalid);
                }
                if answer.evidence_ids.iter().any(|id| !self.evidence_ids.contains(id)){return Err(ErrorCode::StaleEvidence);}
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
                #[derive(Deserialize)] #[serde(deny_unknown_fields)]
                struct Tool { kind:String,action_id:String,arguments:Box<serde_json::value::RawValue> }
                let proposal:Tool=serde_json::from_str(output).map_err(|_|ErrorCode::ModelOutputInvalid)?;
                if proposal.kind!="tool_call"{return Err(ErrorCode::ModelOutputInvalid);}
                crate::registry::capability(&proposal.action_id)?;
                let tool=match proposal.action_id.as_str(){
                    "system.info"=>ReadTool::SystemInfo,
                    "system.service_status"=>ReadTool::SystemServiceStatus,
                    _=>return Err(ErrorCode::PermissionDenied),
                };
                if !self.allowed_tools.contains(&tool){return Err(ErrorCode::PermissionDenied);}
                // Resolve capability/offer before validating its arguments.
                // A malformed forbidden action never gains a repair attempt.
                let _=proposal.arguments;
                parse_tool_call(output.as_bytes()).map_err(|_|ErrorCode::ModelOutputInvalid)?;
            },
            _ => return Err(ErrorCode::ModelOutputInvalid),
        }
        serde_json::from_str(output).map_err(|_| ErrorCode::ModelOutputInvalid)
    }
}


// Inference prompts are private to each request in both the client and daemon.
impl Drop for Generation {
    fn drop(&mut self) {
        for text in [&mut self.system_prompt, &mut self.user_prompt] {
            for byte in unsafe { text.as_bytes_mut() } { unsafe { std::ptr::write_volatile(byte,0); } }
            std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst); text.clear();
        }
    }
}
