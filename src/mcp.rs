//! Tool surface shared by the MCP server.
//!
//! Pure decision logic and wire-neutral definitions; rmcp types stay in the
//! binary, so the engine crate keeps no MCP dependency.

use serde_json::Value;

use crate::adapter::Adapter;
use crate::schema::SchemaError;

/// Tool names exposed by the MCP server, in listing order. Names follow the
/// MCP pattern recommendation (`a-z0-9`, `_`, `-`, `.`), so interoperable
/// clients can rely on them; each tool only accepts questions of its type.
pub const TOOL_NAMES: &[&str] = &["decide_choice", "decide_noul", "decide_score"];

/// The question type each tool accepts: `decide_choice` only accepts
/// `choice` questions, `decide_noul` only `noul`, `decide_score` only `score.
pub fn tool_question_type(name: &str) -> Option<&'static str> {
    match name {
        "decide_choice" => Some("choice"),
        "decide_noul" => Some("noul"),
        "decide_score" => Some("score"),
        _ => None,
    }
}

/// Why a `tools/call` failed, before any rmcp mapping.
#[derive(Debug)]
pub enum ToolError {
    /// Unknown tool name.
    UnknownTool(String),
    /// Invalid arguments or request rejected before inference.
    Schema(Vec<SchemaError>),
    /// Inference or serialization failure (not the caller's fault).
    Engine(anyhow::Error),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "unknown tool \"{name}\""),
            Self::Schema(errors) => {
                let details: Vec<String> = errors
                    .iter()
                    .map(|e| format!("  {}: {}", e.path, e.message))
                    .collect();
                write!(f, "request rejected before inference:\n{}", details.join("\n"))
            }
            Self::Engine(err) => write!(f, "{err:#}"),
        }
    }
}

/// The shared input schema for every decide tool: `state` plus `questions`.
fn shared_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "state": {"type": "string", "description": "State text (or object/array); what happened"},
            "questions": {"type": "object", "description": "Map of question id to typed question object"}
        },
        "required": ["state", "questions"]
    })
}

/// Determine the engine-facing text for a validated state. The schema accepts
/// string, object or array states; objects and arrays serialize canonically
/// (compact, map keys recursively sorted) before the engine sees a string.
pub fn canonical_state_text(state: &crate::schema::RequestState) -> String {
    match state {
        crate::schema::RequestState::Text(text) => text.clone(),
        crate::schema::RequestState::Json(value) => canonical_json_string(value),
    }
}

/// Compact JSON with object keys recursively sorted, so equal states always
/// serialize to equal text regardless of input key order.
fn canonical_json_string(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> = keys
                .iter()
                .map(|key| format!("{}:{}", serde_json::to_string(key).expect("key"), canonical_json_string(&map[*key])))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical_json_string).collect();
            format!("[{}]", inner.join(","))
        }
        other => serde_json::to_string(other).expect("serializing a Value never fails"),
    }
}

/// Wire-neutral tool definitions: (name, description, input schema).
pub fn tool_specs() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "decide_choice",
            "Run a categorical choice decision between named criteria (returns probabilities and the recommended option); questions must all be type choice",
            shared_input_schema(),
        ),
        (
            "decide_noul",
            "Run a yes/no (noul) decision; returns the probability of the \"true\" label; questions must all be type noul",
            shared_input_schema(),
        ),
        (
            "decide_score",
            "Run a rubric score decision over ordered levels; returns level probabilities and the expected score; questions must all be type score",
            shared_input_schema(),
        ),
    ]
}

/// A validated request, ready for inference.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCall {
    /// State text ready for the engine (non-string states are serialized
    /// canonically; see [`canonical_state_text`]).
    pub state_text: String,
    /// Validated questions map.
    pub questions: Value,
    /// Per-question type (`question id` -> `choice|noul|score`), in order.
    pub question_types: Vec<(String, &'static str)>,
}

/// Validate one tool call without touching the engine: name membership plus
/// arguments shape. Schema errors reference stable JSON paths.
pub fn prepare_call(name: &str, args: &Value) -> Result<PreparedCall, ToolError> {
    if !TOOL_NAMES.contains(&name) {
        return Err(ToolError::UnknownTool(name.to_owned()));
    }
    let expected = tool_question_type(name).expect("name is a known tool");
    let prepared = parse_arguments(args)?;
    for (qid, actual) in &prepared.question_types {
        if *actual != expected {
            return Err(ToolError::Schema(vec![SchemaError {
                path: format!("questions.{qid}.type"),
                message: format!(
                    "question type {actual} does not match tool {name} (expected {expected})"
                ),
            }]));
        }
    }
    Ok(prepared)
}

/// Validate a mixed-type request (the generic `run` surface): same argument
/// shape, full typed schema validation and canonical state, without a tool
/// type constraint.
pub fn parse_arguments(args: &Value) -> Result<PreparedCall, ToolError> {
    let args = args
        .as_object()
        .ok_or_else(|| ToolError::Schema(vec![SchemaError {
            path: "arguments".into(),
            message: "arguments must be an object".into(),
        }]))?;
    let Some(state) = args.get("state") else {
        return Err(ToolError::Schema(vec![SchemaError {
            path: "arguments.state".into(),
            message: "state is required: string, object or array".into(),
        }]));
    };
    let Some(questions) = args.get("questions") else {
        return Err(ToolError::Schema(vec![SchemaError {
            path: "arguments.questions".into(),
            message: "questions are required: nonempty map".into(),
        }]));
    };
    // Full typed schema validation BEFORE inference (schema errors carry paths).
    let request = crate::schema::parse_request(state, questions)
        .map_err(ToolError::Schema)?;
    Ok(PreparedCall {
        state_text: canonical_state_text(&request.state),
        questions: questions.clone(),
        question_types: request
            .questions
            .iter()
            .map(|(qid, q)| (qid.clone(), q.qtype.qtype_name()))
            .collect(),
    })
}

/// Run one tool call: validate, infer once, return the answers JSON.
///
/// `args` is the raw `arguments` object of the call (`{state, questions}`).
pub fn run_tool(adapter: &Adapter, name: &str, args: &Value) -> Result<Value, ToolError> {
    let prepared = prepare_call(name, args)?;
    let answers = adapter
        .run(&prepared.state_text, &prepared.questions)
        .map_err(ToolError::Engine)?;
    Ok(adapter.answers_to_json(&answers))
}
