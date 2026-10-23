//! Unit tests for the MCP server.

use decide_rs::adapter::Adapter;
use decide_rs::adapter::{AdaptedAnswer, ChoiceResult, NoulResult, ScoreResult};
use serde_json::{json, Value, Map};
use std::path::Path;
use std::sync::Arc;

#[test]
fn test_tool_schema_structure() {
    use rmcp::model::Tool;

    let mut schema_map = Map::new();
    schema_map.insert("type".to_string(), Value::String("object".to_string()));
    let mut props = Map::new();
    props.insert("state".to_string(), Value::String("string".to_string()));
    props.insert("questions".to_string(), Value::String("object".to_string()));
    schema_map.insert("properties".to_string(), Value::Object(props));
    schema_map.insert("required".to_string(), Value::Array(vec![
        Value::String("state".to_string()),
        Value::String("questions".to_string()),
    ]));
    let schema: Arc<Map<String, Value>> = Arc::new(schema_map);

    let tool = Tool::new("decide_choice", "Run a choice decision", schema);

    assert_eq!(tool.name.as_ref(), "decide_choice");
    assert_eq!(tool.description.as_deref(), Some("Run a choice decision"));
    assert!(tool.input_schema.contains_key("type"));
    assert!(tool.input_schema.contains_key("properties"));
    assert!(tool.input_schema.contains_key("required"));
}

#[test]
fn test_adapter_answers_to_json_structure() {
    // Test ChoiceResult serialization via answers_to_json
    let choice = AdaptedAnswer::Choice(ChoiceResult {
        winner: "billing".to_string(),
        probabilities: vec![("billing".to_string(), 0.7), ("platform".to_string(), 0.3)],
        confidence: 0.4,
    });

    let answers = vec![("route".to_string(), choice)];
    let Some(adapter) = create_mock_adapter() else {
        eprintln!("Model not found, skipping JSON serialization test");
        return;
    };
    let json = adapter.answers_to_json(&answers);
    
    assert!(json.get("answers").is_some());
    let answers_obj = json["answers"].as_object().unwrap();
    assert!(answers_obj.contains_key("route"));
    let route = &answers_obj["route"];
    assert_eq!(route["type"], "choice");
    assert_eq!(route["choice"], "billing");
    assert!(route["probabilities"]["billing"].is_number());

    // Test NoulResult serialization
    let noul = AdaptedAnswer::Noul(NoulResult { prob_true: 0.8 });
    let answers = vec![("urgent".to_string(), noul)];
    let json = adapter.answers_to_json(&answers);
    
    let answers_obj = json["answers"].as_object().unwrap();
    assert!(answers_obj.contains_key("urgent"));
    let urgent = &answers_obj["urgent"];
    assert_eq!(urgent["type"], "noul");
    assert_eq!(urgent["noul"], 0.8);

    // Test ScoreResult serialization
    let score = AdaptedAnswer::Score(ScoreResult {
        probabilities: vec![("Calm".to_string(), 0.1), ("Annoyed".to_string(), 0.3), ("Angry".to_string(), 0.6)],
        expected_score: 2.5,
        confidence: 0.2,
    });
    let answers = vec![("frustration".to_string(), score)];
    let json = adapter.answers_to_json(&answers);
    
    let answers_obj = json["answers"].as_object().unwrap();
    assert!(answers_obj.contains_key("frustration"));
    let frustration = &answers_obj["frustration"];
    assert_eq!(frustration["type"], "score");
    assert_eq!(frustration["score"], 2.5);
}

fn create_mock_adapter() -> Option<Adapter> {
    // Create a minimal adapter for testing JSON serialization
    // This uses a dummy engine - in real tests we'd use the actual engine
    let model_path = Path::new("julia-checkpoint");
    if !model_path.exists() {
        return None;
    }
    Adapter::new(model_path).ok()
}

#[cfg(test)]
mod integration {
    use super::*;

    // These tests require the model checkpoint to be present
    // They are ignored by default and run with --ignored
    #[ignore]
    #[test]
    fn test_adapter_run_choice() {
        let model_path = Path::new("julia-checkpoint");
        if !model_path.exists() {
            eprintln!("Model not found at {:?}, skipping test", model_path);
            return;
        }

        let adapter = Adapter::new(model_path).expect("Failed to load adapter");
        let questions = json!({
            "route": {
                "type": "choice",
                "instructions": "Which team?",
                "criteria": {
                    "billing": "Payments",
                    "platform": "API"
                }
            }
        });

        let result = adapter.run("Test state", &questions);
        assert!(result.is_ok());
        let answers = result.unwrap();
        assert!(!answers.is_empty());
        assert_eq!(answers[0].0, "route");
    }

    #[ignore]
    #[test]
    fn test_adapter_run_noul() {
        let model_path = Path::new("julia-checkpoint");
        if !model_path.exists() {
            eprintln!("Model not found at {:?}, skipping test", model_path);
            return;
        }

        let adapter = Adapter::new(model_path).expect("Failed to load adapter");
        let questions = json!({
            "urgent": {
                "type": "noul",
                "instructions": "Is this urgent?",
                "criteria": {
                    "false": "No",
                    "true": "Yes"
                }
            }
        });

        let result = adapter.run("Test state", &questions);
        assert!(result.is_ok());
        let answers = result.unwrap();
        assert!(!answers.is_empty());
        assert_eq!(answers[0].0, "urgent");
    }

    #[ignore]
    #[test]
    fn test_adapter_run_score() {
        let model_path = Path::new("julia-checkpoint");
        if !model_path.exists() {
            eprintln!("Model not found at {:?}, skipping test", model_path);
            return;
        }

        let adapter = Adapter::new(model_path).expect("Failed to load adapter");
        let questions = json!({
            "frustration": {
                "type": "score",
                "instructions": "How frustrated?",
                "criteria": [
                    "Calm",
                    "Annoyed",
                    "Angry"
                ]
            }
        });

        let result = adapter.run("Test state", &questions);
        assert!(result.is_ok());
        let answers = result.unwrap();
        assert!(!answers.is_empty());
        assert_eq!(answers[0].0, "frustration");
    }
}
#[cfg(test)]
mod tool_surface {
    use decide_rs::adapter::Adapter;
    use decide_rs::mcp::{self, ToolError};
    use rmcp::model::CallToolResult;
    use serde_json::{json, Value};

    fn adapter_if_model_available() -> Option<Adapter> {
        let path = std::path::Path::new("julia-checkpoint");
        if path.is_dir() {
            Adapter::new(path).ok()
        } else {
            None
        }
    }

    const ARGS: &str = r#"{
        "state": "Ticket #4821: checkout payments failing after release, revenue impact",
        "questions": {
            "route": {"type": "choice", "instructions": "Which team?", "criteria": {
                "billing": "Payments, invoices, refunds",
                "platform": "Checkout, API and platform outages"
            }}
        }
    }"#;

    #[test]
    fn unknown_tool_is_rejected_with_its_name() {
        let err = mcp::prepare_call(
            "decide_roulette",
            &serde_json::from_str::<Value>(ARGS).unwrap(),
        )
        .unwrap_err();
        match err {
            ToolError::UnknownTool(ref name) => assert_eq!(name, "decide_roulette"),
            other => panic!("expected UnknownTool, got {other:?}"),
        }
    }

    #[test]
    fn schema_failures_reference_stable_paths() {
        let args = json!({
            "state": "s",
            "questions": {
                "route": {"type": "choice", "instructions": "Which team?", "criteria": {
                    "billing": "Payments", "platform": "Payments"
                }}
            }
        });
        let err = mcp::prepare_call("decide_choice", &args).unwrap_err();
        match err {
            ToolError::Schema(errors) => {
                assert_eq!(errors.len(), 1, "one criteria violation only");
                assert_eq!(errors[0].path, "questions.route.criteria");
                assert!(errors[0].message.contains("duplicate"));
            }
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn missing_arguments_are_flagged_at_their_path() {
        let args = json!({"state": "s"});
        let err = mcp::prepare_call("decide_noul", &args).unwrap_err();
        match err {
            ToolError::Schema(errors) => {
                assert!(errors.iter().any(|e| e.path == "arguments.questions"),
                    "expected arguments.questions violation, got {errors:?}");
            }
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn tool_rejects_mismatched_question_type() {
        let args = json!({
            "state": "s",
            "questions": {
                "urgent": {"type": "noul", "instructions": "Is this urgent?",
                    "criteria": {"false": "Routine", "true": "Urgent"}}
            }
        });
        let err = mcp::prepare_call("decide_choice", &args).unwrap_err();
        match err {
            ToolError::Schema(errors) => {
                assert!(errors.iter().any(|e| e.path == "questions.urgent.type"
                    && e.message.contains("does not match tool decide_choice")),
                    "expected type-mismatch violation, got {errors:?}");
            }
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn non_string_state_is_canonically_serialized() {
        let args = json!({
            "state": {"summary": "payments failing", "impact": "revenue"},
            "questions": {
                "urgent": {"type": "noul", "instructions": "Is this urgent?",
                    "criteria": {"false": "Routine", "true": "Urgent"}}
            }
        });
        let prepared = mcp::prepare_call("decide_noul", &args)
            .expect("object state is valid for the tool contract");
        // Canonical serialization: compact JSON with sorted map keys.
        assert_eq!(
            prepared.state_text,
            r#"{"impact":"revenue","summary":"payments failing"}"#
        );
    }

    #[test]
    fn valid_call_returns_answers_with_created_structured_value() {
        let Some(adapter) = adapter_if_model_available() else {
            eprintln!("model not present; skipping real-inference assertion");
            return;
        };
        let args: Value = serde_json::from_str(ARGS).unwrap();
        let answers = mcp::run_tool(&adapter, "decide_choice", &args)
            .expect("valid call must succeed");
        let route = &answers["answers"]["route"];
        assert_eq!(route["type"], "choice");
        assert!(route["probabilities"].is_object());

        // The binary wraps this exact value in CallToolResult::structured:
        // text content mirrors structuredContent.
        let result = rmcp::model::CallToolResult::structured(answers.clone());
        assert_eq!(result.structured_content.as_ref().unwrap(), &answers);
        let text = result.content[0].as_text().unwrap().text.clone();
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), answers);
    }


}
