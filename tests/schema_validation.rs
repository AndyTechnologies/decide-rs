//! Schema tests: no network, no model weights, no inference.
//! Invalid requests must be rejected with stable paths and messages.

use decide_rs::schema::{QuestionType, RequestState, parse_request};
use serde_json::{Value, json};

fn q(qtype: &str, criteria: Value) -> Value {
    json!({ "type": qtype, "instructions": "Decide this.", "criteria": criteria })
}
fn ch(c: Value) -> Value {
    q("choice", c)
}
fn sc(c: Value) -> Value {
    q("score", c)
}
fn nl(c: Value) -> Value {
    q("noul", c)
}
fn one(question: Value) -> Value {
    json!({ "q": question })
}
fn sq() -> Value {
    decide_rs::sample_questions()
}

/// Assert exactly one violation at `path` whose message contains `fragment`.
fn bad(state: Value, questions: Value, path: &str, fragment: &str) {
    let mut errors = parse_request(&state, &questions).err().expect("rejected");
    assert_eq!(errors.len(), 1, "got {errors:?}");
    let error = errors.remove(0);
    assert_eq!(error.path, path, "{error:?}");
    assert!(error.message.contains(fragment), "{:?}", error.message);
}
fn field(question: Value, fld: &str, fragment: &str) {
    let path = format!("questions.q.{fld}");
    bad(json!("s"), one(question), &path, fragment);
}

// ---------------------------------------------------------------- valid input

#[test]
fn parses_mixed_choice_noul_score_request() {
    let request = parse_request(&json!("state text"), &sq()).expect("must parse");
    assert_eq!(request.state, RequestState::Text("state text".into()));
    let kinds: Vec<QuestionType> = request.questions.iter().map(|(_, q)| q.qtype).collect();
    for expected in [
        QuestionType::Choice,
        QuestionType::Noul,
        QuestionType::Score,
    ] {
        assert!(kinds.contains(&expected), "missing {expected:?}");
    }
}

#[test]
fn valid_requests_preserve_structure() {
    let state = json!({ "ticket": 4821, "open": true, "tags": ["payments", "urgent"] });
    let request = parse_request(&state, &sq()).expect("must parse");
    assert_eq!(
        request.state,
        RequestState::Json(state),
        "never stringified"
    );
    let bare = json!({ "type": "noul", "instructions": "Urgent?" });
    let request = parse_request(&json!("s"), &one(bare)).expect("must parse");
    assert_eq!(request.questions[0].1.criteria, Value::Null);
}

// ---------------------------------------------------------------- invalid input

#[test]
fn rejects_state_violations() {
    bad(json!(null), sq(), "state", "state is required");
    bad(json!(42), sq(), "state", "got number");
    bad(json!(true), sq(), "state", "got boolean");
}

#[test]
fn rejects_question_container_violations() {
    bad(json!("s"), json!({}), "questions", "nonempty map");
    bad(json!("s"), json!(["nope"]), "questions", "nonempty map");
    let empty_id = json!({ "": ch(json!({ "a": "A" })) });
    bad(json!("s"), empty_id, "questions", "non-empty");
}

#[test]
fn rejects_unknown_or_missing_type() {
    field(q("ranking", json!({})), "type", "unknown");
    let no_type = one(json!({ "instructions": "x" }));
    bad(
        json!("s"),
        no_type,
        "questions.q.type",
        "choice, noul or score",
    );
}

#[test]
fn rejects_structured_instructions_without_coercion() {
    let obj = json!({ "type": "choice", "instructions": { "x": 1 }, "criteria": { "a": "A" } });
    field(obj, "instructions", "plain string");
    let missing = json!({ "type": "choice", "criteria": { "a": "A" } });
    field(missing, "instructions", "plain string");
}

#[test]
fn rejects_criteria_shape_and_empty_ids() {
    field(ch(json!("nope")), "criteria", "an object");
    field(sc(json!({})), "criteria", "an array");
    field(nl(json!("nope")), "criteria", "an object");
    let missing = json!({ "type": "choice", "instructions": "x" });
    field(missing, "criteria", "an object");
    field(ch(json!({ "": "E", "b": "Ok" })), "criteria", "non-empty");
}

// ---------------------------------------------------------------- determinism

#[test]
fn invalid_request_yields_identical_error_vectors() {
    let state = json!(null);
    let questions = json!({
        "b": ch(json!(["not an object"])),
        "a": q("ranking", json!({})),
    });
    let first = parse_request(&state, &questions).err().expect("rejected");
    let second = parse_request(&state, &questions).err().expect("rejected");
    assert_eq!(first, second, "same input must yield identical errors");
    let paths: Vec<&str> = first.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["state", "questions.b.criteria", "questions.a.type"]);
}
