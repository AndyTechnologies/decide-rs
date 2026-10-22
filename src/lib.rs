//! decide-rs core: a thin wrapper around the `julia1` CPU runtime for the Julia-1
//! decision model. No inference servers and no auto-downloads inside the
//! engine: it is always loaded from a caller-supplied local checkpoint
//! directory. Networking exists only in the explicit [`checkpoint`] module
//! (revision-pinned, sha256-verified download; ADR 0001 D1).
pub mod adapter;
pub mod checkpoint;
pub mod schema;

use julia1::{Answers, Engine, EngineOptions, State};
use serde_json::Value;
use std::path::Path;

/// Load a Julia-1 engine from a LOCAL checkpoint directory.
///
/// Never calls `Engine::from_pretrained` or any download function; the model
/// must already exist at `model_path`.
pub fn load_engine(model_path: &Path) -> anyhow::Result<Engine> {
    Engine::load(
        model_path,
        EngineOptions {
            strict_encoding: true,
            head_length: 512,
            ..Default::default()
        },
    )
}

/// A short, realistic support-ticket state text.
pub fn sample_state() -> &'static str {
    "Ticket #4821: checkout payments started failing after this morning's release. \
     Two similar reports in the queue; customer says revenue is being lost."
}

/// One questions map mixing all three types in a single call: a `choice`
/// (department routing), a `noul` (urgency), and a `score` (frustration).
/// Instructions are plain strings, as required by the typed API.
pub fn sample_questions() -> Value {
    serde_json::json!({
        "route": {
            "type": "choice",
            "instructions": "Which team should handle this ticket?",
            "criteria": {
                "billing": "Payments, invoices, refunds and billing disputes",
                "platform": "Checkout, API and platform outages",
                "account": "Account access, identity and permissions"
            }
        },
        "urgent": {
            "type": "noul",
            "instructions": "Is this ticket urgent?",
            "criteria": {
                "false": "Routine request with no time pressure",
                "true": "Urgent: revenue impact, outage or same-day deadline"
            }
        },
        "frustration": {
            "type": "score",
            "instructions": "How frustrated is the customer?",
            "criteria": [
                "Calm: neutral wording, no complaints",
                "Annoyed: mild complaints or repetition",
                "Angry: explicit anger, threats to cancel or escalation demands"
            ]
        }
    })
}

/// Validate `questions` against the typed schema BEFORE inference (ADR 0001).
pub fn run_decisions(engine: &Engine, state: &str, questions: &Value) -> anyhow::Result<Answers> {
    schema::parse_request(&Value::String(state.to_owned()), questions).map_err(|errors| {
        let details: Vec<String> = errors
            .iter()
            .map(|e| format!("  {}: {}", e.path, e.message))
            .collect();
        anyhow::anyhow!("request rejected before inference:\n{}", details.join("\n"))
    })?;
    engine.predict_typed(&State::from(state), questions)
}

/// Render answers as JSON via `julia1::answers_to_json`.
pub fn answers_json(answers: &Answers) -> Value {
    julia1::answers_to_json(answers)
}

/// Default install location for the Julia-1 checkpoint: `$XDG_DATA_HOME/
/// decide-rs/julia-checkpoint` (typically `~/.local/share/...`). The relative
/// project-local `julia-checkpoint` directory is a development convenience,
/// not the installed default.
pub fn installed_model_path() -> std::path::PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|home| std::path::PathBuf::from(home).join(".local/share"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    data_home.join("decide-rs/julia-checkpoint")
}

/// Resolve the model directory for binaries, in priority order:
/// 1. `DECIDE_RS_MODEL_PATH` (explicit override)
/// 2. the installed location (see [`installed_model_path`]) when it exists
/// 3. the relative project-local `julia-checkpoint` (development runs)
pub fn resolve_model_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("DECIDE_RS_MODEL_PATH") {
        return std::path::PathBuf::from(path);
    }
    let installed = installed_model_path();
    if installed.is_dir() {
        return installed;
    }
    std::path::PathBuf::from("julia-checkpoint")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn question<'a>(questions: &'a Value, id: &str) -> &'a Value {
        questions
            .as_object()
            .and_then(|q| q.get(id))
            .unwrap_or_else(|| panic!("missing question {id}"))
    }

    fn instructions_are_plain_string(q: &Value) -> bool {
        q.get("instructions").is_some_and(Value::is_string)
    }

    #[test]
    fn sample_questions_has_exactly_three_entries_and_types() {
        let questions = sample_questions();
        let obj = questions.as_object().expect("questions must be a map");
        assert_eq!(obj.len(), 3, "expected exactly three questions");
        let mut types: Vec<&str> = obj
            .values()
            .map(|q| q.get("type").and_then(Value::as_str).expect("type string"))
            .collect();
        types.sort_unstable();
        assert_eq!(types, ["choice", "noul", "score"]);
    }

    #[test]
    fn choice_question_has_nonempty_criteria_and_string_instructions() {
        let questions = sample_questions();
        let q = question(&questions, "route");
        assert!(instructions_are_plain_string(q));
        let criteria = q
            .get("criteria")
            .and_then(Value::as_object)
            .expect("choice criteria must be a map");
        assert!(!criteria.is_empty(), "choice criteria must be nonempty");
        assert!(criteria.values().all(Value::is_string));
    }

    #[test]
    fn noul_question_criteria_maps_false_and_true() {
        let questions = sample_questions();
        let q = question(&questions, "urgent");
        assert!(instructions_are_plain_string(q));
        let criteria = q
            .get("criteria")
            .and_then(Value::as_object)
            .expect("noul criteria must be a map");
        assert!(criteria.contains_key("false") && criteria.contains_key("true"));
        assert!(criteria.values().all(Value::is_string));
    }

    #[test]
    fn score_question_criteria_is_ordered_list_with_at_least_two_levels() {
        let questions = sample_questions();
        let q = question(&questions, "frustration");
        assert!(instructions_are_plain_string(q));
        let criteria = q
            .get("criteria")
            .and_then(Value::as_array)
            .expect("score criteria must be an ordered list");
        assert!(
            criteria.len() >= 2,
            "score rubric needs at least two levels, got {}",
            criteria.len()
        );
        assert!(criteria.iter().all(Value::is_string));
    }
}
