//! Strict first provider contract. Registry growth must remain explicit.
use serde::{Deserialize, Serialize};
use serde_json::{Value, value::RawValue};
use crate::MAX_TASK_BYTES;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument, UnsupportedSchema, UnknownCapability, UnsupportedCapability,
    TargetNotFound, TargetChanged, AuthRequired, PermissionDenied, ApprovalExpired,
    PlanChanged, PolicyChanged, Conflict, StaleEvidence, SecretScopeDenied,
    IndexNotReady, ModelUnavailable, ModelCrashed, ModelOutputInvalid,
    DeadlineExceeded, ContextBudgetExceeded, ResourceExhausted, BuildFailed,
    ActivationFailed, HealthCheckFailed, RollbackFailed, PartialResult, Cancelled,
    NetworkRequired, RebootRequired, RecoveryRequired,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus { Ok, Partial, Error, Pending, Cancelled }

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source { pub provider: String, pub provider_version: String }

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResult<T> {
    pub schema_version: u32,
    pub status: ResultStatus,
    pub observed_at: String,
    pub source: Source,
    pub evidence_ids: Vec<String>,
    pub complete: bool,
    pub next_cursor: Option<String>,
    pub data: Option<T>,
    pub error: Option<ProviderError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ToolKind { ToolCall }
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolEnvelope {
    kind: ToolKind,
    action_id: String,
    arguments: Box<RawValue>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyArguments {}

#[derive(Debug, PartialEq, Eq)]
pub enum Action { SystemInfo }

pub fn parse_tool_call(bytes: &[u8]) -> Result<Action, ErrorCode> {
    if bytes.is_empty() || bytes.len() > MAX_TASK_BYTES { return Err(ErrorCode::ResourceExhausted); }
    let envelope: ToolEnvelope = serde_json::from_slice(bytes).map_err(|_| ErrorCode::InvalidArgument)?;
    match envelope.kind { ToolKind::ToolCall => {} }
    match envelope.action_id.as_str() {
        "system.info" => {
            serde_json::from_str::<EmptyArguments>(envelope.arguments.get()).map_err(|_| ErrorCode::InvalidArgument)?;
            Ok(Action::SystemInfo)
        }
        _ => Err(ErrorCode::UnknownCapability),
    }
}

/// One integer-only canonical representation for security-sensitive values.
/// Typed deserialization must reject duplicate/unknown fields before this step.
pub fn canonical_json(value: &Value) -> Result<Vec<u8>, ErrorCode> {
    fn ordered(v: &Value) -> Result<Value, ErrorCode> {
        Ok(match v {
            Value::Number(n) if !n.is_i64() && !n.is_u64() => return Err(ErrorCode::InvalidArgument),
            Value::Array(a) => Value::Array(a.iter().map(ordered).collect::<Result<_, _>>()?),
            Value::Object(m) => {
                let mut keys = m.keys().collect::<Vec<_>>(); keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys { out.insert(k.clone(), ordered(&m[k])?); }
                Value::Object(out)
            },
            _ => v.clone(),
        })
    }
    serde_json::to_vec(&ordered(value)?).map_err(|_| ErrorCode::InvalidArgument)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_action_boundary() {
        assert_eq!(parse_tool_call(br#"{"kind":"tool_call","action_id":"system.info","arguments":{}}"#), Ok(Action::SystemInfo));
        for raw in [
            r#"{"kind":"tool_call","action_id":"system.info","arguments":{"admin":true}}"#,
            r#"{"kind":"tool_call","action_id":"system.info","arguments":[],"uid":0}"#,
            r#"{"kind":"tool_call","action_id":"system.info","arguments":{},"approved":true}"#,
            r#"{"kind":"tool_call","action_id":"system.info","action_id":"system.info","arguments":{}}"#,
            r#"{"kind":"tool_call","action_id":"system.info","arguments":{},"kind":"tool_call"}"#,
            r#"{"kind":"tool_call","action_id":"system.info","arguments":{}} trailing"#,
        ] { assert_eq!(parse_tool_call(raw.as_bytes()), Err(ErrorCode::InvalidArgument)); }
        assert_eq!(parse_tool_call(br#"{"kind":"tool_call","action_id":"shell.run","arguments":{"command":"id"}}"#), Err(ErrorCode::UnknownCapability));
        assert_eq!(parse_tool_call(&vec![b' '; MAX_TASK_BYTES + 1]), Err(ErrorCode::ResourceExhausted));
    }
    #[test]
    fn canonical_order_integer_units_and_unicode() {
        let value = serde_json::json!({"z": [true, null, "مرحبا"], "a": {"bytes":18446744073709551615_u64,"duration_ms":20}});
        let output = canonical_json(&value).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), r#"{"a":{"bytes":18446744073709551615,"duration_ms":20},"z":[true,null,"مرحبا"]}"#);
        assert_eq!(canonical_json(&serde_json::json!({"duration":0.5})), Err(ErrorCode::InvalidArgument));
    }
}
