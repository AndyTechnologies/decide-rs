//! Tool surface shared by the MCP server.
//!
//! Pure decision logic and wire-neutral definitions; rmcp types stay in the
//! binary, so the engine crate keeps no MCP dependency.

use serde_json::Value;

use crate::adapter::Adapter;
use crate::schema::SchemaError;

/// Tool names exposed by the MCP server, in listing order.
pub const TOOL_NAMES: &[&str] = &["decide/choice", "decide/noul", "decide/score"];

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

/// Wire-neutral tool definitions: (name, description, input schema).
pub fn tool_specs() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("decide/choice", "Run a categorical choice decision between named criteria (returns probabilities and the recommended option)", shared_input_schema()),
        ("decide/noul", "Run a yes/no (noul) decision; returns the probability of the \"true\" label", shared_input_schema()),
        ("decide/score", "Run a rubric score decision over ordered levels; returns level probabilities and the expected score", shared_input_schema()),
    ]
}

/// A validated tool call, ready for inference.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCall {
    /// State text ready for the engine (non-string states are serialized).
    pub state_text: String,
    /// Validated questions map.
    pub questions: Value,
}

/// Validate one tool call without touching the engine: name membership plus
/// arguments shape. Schema errors reference stable JSON paths.
pub fn prepare_call(name: &str, args: &Value) -> Result<PreparedCall, ToolError> {
    if !TOOL_NAMES.contains(&name) {
        return Err(ToolError::UnknownTool(name.to_owned()));
    }
    let args = args.as_object().ok_or_else(|| ToolError::Schema(vec![SchemaError {
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
    let _ = request;
    let state_text: String = state
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| state.to_string());
    Ok(PreparedCall {
        state_text,
        questions: questions.clone(),
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
