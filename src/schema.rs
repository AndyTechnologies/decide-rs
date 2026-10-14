//! Typed request schema: deterministic validation BEFORE inference (ADR 0001 D3).
//! Violations are collected as structured [`SchemaError`]s with stable paths.

use serde_json::Value;

/// Request state as-is: a string, or JSON object/array, never stringified.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestState {
    Text(String),
    Json(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionType {
    Choice,
    Noul,
    Score,
}

/// One validated question; `criteria` is raw JSON, shape-checked only.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub qtype: QuestionType,
    pub instructions: String,
    pub criteria: Value,
}

/// A validated request; `questions` keeps the input document order.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub state: RequestState,
    pub questions: Vec<(String, Question)>,
}

/// One violation: a stable JSON `path` plus a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub path: String,
    pub message: String,
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[derive(Default)]
struct Errors(Vec<SchemaError>);

impl Errors {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(SchemaError {
            path: path.into(),
            message: message.into(),
        });
    }
}

/// Keep `criteria` only when its container shape matches the question type.
fn shape(
    criteria: Option<&Value>,
    want_object: bool,
    msg: &str,
    cpath: &str,
    errors: &mut Errors,
) -> Option<Value> {
    let ok = match criteria {
        Some(Value::Object(_)) => want_object,
        Some(Value::Array(_)) => !want_object,
        _ => false,
    };
    if ok {
        criteria.cloned()
    } else {
        errors.push(cpath, msg);
        None
    }
}

impl QuestionType {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "choice" => Some(Self::Choice),
            "noul" => Some(Self::Noul),
            "score" => Some(Self::Score),
            _ => None,
        }
    }

    fn parse_criteria(
        self,
        qpath: &str,
        criteria: Option<&Value>,
        errors: &mut Errors,
    ) -> Option<Value> {
        let cpath = format!("{qpath}.criteria");
        match self {
            Self::Choice => {
                let raw = shape(criteria, true, "choice needs an object", &cpath, errors);
                if let Some(Value::Object(map)) = &raw
                    && map.keys().any(|k| k.is_empty())
                {
                    errors.push(&cpath, "choice ids must be non-empty");
                }
                raw
            }
            Self::Score => shape(criteria, false, "score needs an array", &cpath, errors),
            Self::Noul => match criteria {
                None => Some(Value::Null),
                other => shape(other, true, "noul needs an object", &cpath, errors),
            },
        }
    }
}

/// Parse and validate `(state, questions)`; violations are collected in document order.
pub fn parse_request(state: &Value, questions: &Value) -> Result<Request, Vec<SchemaError>> {
    let mut errors = Errors::default();
    let parsed_state = match state {
        Value::String(s) => Some(RequestState::Text(s.clone())),
        Value::Object(_) | Value::Array(_) => Some(RequestState::Json(state.clone())),
        Value::Null => {
            errors.push("state", "state is required: string, object or array");
            None
        }
        other => {
            let kind = json_kind(other);
            errors.push(
                "state",
                format!("state must be string/object/array, got {kind}"),
            );
            None
        }
    };
    let Some(qmap) = questions.as_object().filter(|m| !m.is_empty()) else {
        errors.push("questions", "questions must be a nonempty map");
        return Err(errors.0);
    };
    let mut parsed = Vec::with_capacity(qmap.len());
    for (qid, qvalue) in qmap {
        if qid.is_empty() {
            errors.push("questions", "question ids must be non-empty");
            continue;
        }
        let qpath = format!("questions.{qid}");
        let Some(qobj) = qvalue.as_object() else {
            let kind = json_kind(qvalue);
            errors.push(&qpath, format!("question must be an object, got {kind}"));
            continue;
        };
        let tpath = format!("{qpath}.type");
        let qtype = match qobj.get("type").and_then(Value::as_str) {
            Some(name) => QuestionType::parse(name).or_else(|| {
                errors.push(tpath, format!("unknown question type \"{name}\""));
                None
            }),
            None => {
                errors.push(tpath, "type must be choice, noul or score");
                None
            }
        };
        let ipath = format!("{qpath}.instructions");
        let instructions = match qobj.get("instructions") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(other) => {
                let kind = json_kind(other);
                errors.push(
                    ipath,
                    format!("instructions must be a plain string, got {kind}"),
                );
                None
            }
            None => {
                errors.push(ipath, "instructions is required (plain string)");
                None
            }
        };
        let criteria =
            qtype.and_then(|t| t.parse_criteria(&qpath, qobj.get("criteria"), &mut errors));
        if let (Some(qtype), Some(instructions), Some(criteria)) = (qtype, instructions, criteria) {
            parsed.push((
                qid.clone(),
                Question {
                    qtype,
                    instructions,
                    criteria,
                },
            ));
        }
    }
    if errors.0.is_empty() {
        Ok(Request {
            state: parsed_state.expect("state validated"),
            questions: parsed,
        })
    } else {
        Err(errors.0)
    }
}
