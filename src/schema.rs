//! Typed request schema: deterministic validation BEFORE inference (ADR 0001 D3).
//! Violations are collected as structured [`SchemaError`]s with stable paths.

use serde_json::{Map, Value};

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

/// Validated per-type criteria; limits and uniqueness enforced.
#[derive(Debug, Clone, PartialEq)]
pub enum Criteria {
    Choice(Vec<(String, String)>), // (id, description) pairs, 2-20, unique descriptions
    Noul { neg: String, pos: String }, // labels for the fixed false/true keys
    Score(Vec<String>),            // ordered levels, 2-10, unique
}

/// One validated question with its typed `criteria`.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub qtype: QuestionType,
    pub instructions: String,
    pub criteria: Criteria,
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

fn text(value: &Value) -> Option<&str> {
    value.as_str().filter(|s| !s.is_empty())
}

/// Reject a value already seen among the alternatives of one criteria list.
fn push_unique<'a>(
    seen: &mut Vec<&'a str>,
    value: &'a str,
    kind: &str,
    path: &str,
    errors: &mut Errors,
) {
    if seen.contains(&value) {
        errors.push(path, format!("duplicate {kind}: \"{value}\""));
    } else {
        seen.push(value);
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
    /// The string literal accepted by the schema for this type.
    pub fn qtype_name(self) -> &'static str {
        match self {
            Self::Choice => "choice",
            Self::Noul => "noul",
            Self::Score => "score",
        }
    }

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
    ) -> Option<Criteria> {
        let before = errors.0.len();
        let cpath = format!("{qpath}.criteria");
        let parsed = match self {
            Self::Choice => {
                let Some(Value::Object(map)) =
                    shape(criteria, true, "choice needs an object", &cpath, errors)
                else {
                    return None;
                };
                if !(2..=20).contains(&map.len()) {
                    let n = map.len();
                    errors.push(&cpath, format!("choice needs 2-20 entries, got {n}"));
                }
                let mut pairs = Vec::with_capacity(map.len());
                let mut seen: Vec<&str> = Vec::with_capacity(map.len());
                for (id, description) in &map {
                    if id.is_empty() {
                        errors.push(&cpath, "choice ids must be non-empty");
                    }
                    let Some(desc) = text(&description) else {
                        errors.push(&cpath, "choice description must be non-empty");
                        continue;
                    };
                    push_unique(&mut seen, desc, "choice description", &cpath, errors);
                    pairs.push((id.clone(), desc.to_owned()));
                }
                Criteria::Choice(pairs)
            }
            Self::Score => {
                let Some(Value::Array(levels)) =
                    shape(criteria, false, "score needs an array", &cpath, errors)
                else {
                    return None;
                };
                if !(2..=10).contains(&levels.len()) {
                    let n = levels.len();
                    errors.push(&cpath, format!("score needs 2-10 levels, got {n}"));
                }
                let mut parsed_levels = Vec::with_capacity(levels.len());
                let mut seen: Vec<&str> = Vec::with_capacity(levels.len());
                for (index, level) in levels.iter().enumerate() {
                    let Some(desc) = text(level) else {
                        errors.push(&cpath, format!("score level at index {index} is invalid"));
                        continue;
                    };
                    push_unique(&mut seen, desc, "score level", &cpath, errors);
                    parsed_levels.push(desc.to_owned());
                }
                Criteria::Score(parsed_levels)
            }
            Self::Noul => match criteria {
                None | Some(Value::Null) => Criteria::Noul {
                    neg: "false".into(),
                    pos: "true".into(),
                },
                Some(Value::Object(map)) => return parse_noul(map, &cpath, errors),
                Some(other) => {
                    let kind = json_kind(other);
                    errors.push(&cpath, format!("noul needs an object, got {kind}"));
                    return None;
                }
            },
        };
        (errors.0.len() == before).then_some(parsed)
    }
}

/// Noul criteria: exactly the keys `false` and `true` with nonempty labels.
fn parse_noul(map: &Map<String, Value>, cpath: &str, errors: &mut Errors) -> Option<Criteria> {
    let before = errors.0.len();
    for key in ["false", "true"] {
        if !map.contains_key(key) {
            errors.push(cpath, format!("noul missing \"{key}\""));
        }
        if let Some(value) = map.get(key)
            && text(value).is_none()
        {
            errors.push(cpath, format!("noul \"{key}\" must be non-empty"));
        }
    }
    for key in map.keys() {
        if key != "false" && key != "true" {
            errors.push(cpath, format!("noul unexpected \"{key}\""));
        }
    }
    let get = |k: &str| map.get(k).and_then(text).unwrap_or_default().to_owned();
    (errors.0.len() == before).then(|| Criteria::Noul {
        neg: get("false"),
        pos: get("true"),
    })
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
