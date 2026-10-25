/// Thin typed adapter between `julia1::Answers` and decide-rs structured results.
///
/// The adapter wraps the engine, calls `predict_typed` once, and translates the
/// raw softmax answers into typed [`AdaptedAnswer`] variants that carry the
/// caller's criterion IDs or rubric levels alongside the probabilities.
use crate::{load_engine, run_decisions};
use crate::mcp::AdapterError;
use std::cmp::Ordering;
use julia1::{Answer, Answers, Engine};
use crate::schema::{Criteria, Question, QuestionType, Request};
use serde_json::{json, Map, Value};
use std::path::Path;

/// Adapted result for all three question types.
#[derive(Debug)]
pub enum AdaptedAnswer {
    Choice(ChoiceResult),
    Noul(NoulResult),
    Score(ScoreResult),
}

/// Winner ID and full probability distribution keyed by criterion ID.
#[derive(Debug)]
pub struct ChoiceResult {
    pub winner: String,
    pub probabilities: Vec<(String, f64)>,
    pub confidence: f64,
}

/// Probability of the `"true"` key.
#[derive(Debug)]
pub struct NoulResult {
    pub prob_true: f64,
}

/// Probability distribution over rubric levels and expected score (levels start at 1).
#[derive(Debug)]
pub struct ScoreResult {
    pub probabilities: Vec<(String, f64)>,
    pub expected_score: f64,
    pub confidence: f64,
}

/// Typed inference adapter wrapping a single [`Engine`] instance.
pub struct Adapter {
    engine: Engine,
}

impl Adapter {
    /// Load the engine from `model_path` (cached once per Adapter lifetime).
    pub fn new(model_path: &Path) -> Result<Self, AdapterError> {
        load_engine(model_path)
            .map(|engine| Self { engine })
            .map_err(AdapterError::EngineLoad)
    }

    /// Create an adapter from an existing engine (takes ownership).
    pub fn new_from_engine(engine: Engine) -> Self {
        Self { engine }
    }

    /// Compute confidence from a probability vector: (n * max - 1) / (n - 1).
    /// Returns 0.0 for a uniform distribution, 1.0 for a degenerate distribution.
    /// For n == 1, returns 1.0 (though Jev requires n >= 2 for choice/score).
    pub fn compute_confidence(probs: &[f64]) -> f64 {
        let n = probs.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return 1.0;
        }
        let max = probs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let n_f = n as f64;
        (n_f * max - 1.0) / (n_f - 1.0)
    }

    /// Validate the request and run inference, returning adapted answers.
    pub fn run(&self, state: &str, questions: &serde_json::Value) -> Result<Vec<(String, AdaptedAnswer)>, AdapterError> {
        let request = crate::schema::parse_request(&serde_json::Value::String(state.to_owned()), questions)
            .map_err(AdapterError::Schema)?;
        let answers = crate::run_decisions(&self.engine, state, questions)
            .map_err(AdapterError::Engine)?;
        self.evaluate(&request, &answers)
    }

    /// Translate raw [`Answers`] into typed [`AdaptedAnswer`]s using schema context.
    pub fn evaluate(&self, request: &crate::schema::Request, answers: &Answers) -> Result<Vec<(String, AdaptedAnswer)>, AdapterError> {
        let mut results = Vec::with_capacity(answers.len());
        for (qid, answer) in answers.iter() {
            let question = request.questions.iter()
                .find(|(id, _)| id == qid)
                .ok_or_else(|| AdapterError::MissingQuestion(qid.clone()))?;
            let adapted = match &question.1 {
                _q @ crate::schema::Question {
                    qtype: crate::schema::QuestionType::Choice,
                    criteria: crate::schema::Criteria::Choice(criterion_ids),
                    ..
                } => AdaptedAnswer::Choice(Self::evaluate_choice(answer, criterion_ids)?),
                crate::schema::Question {
                    qtype: crate::schema::QuestionType::Noul,
                    ..
                } => AdaptedAnswer::Noul(Self::evaluate_noul(answer)?),
                _q @ crate::schema::Question {
                    qtype: crate::schema::QuestionType::Score,
                    criteria: crate::schema::Criteria::Score(levels),
                    ..
                } => AdaptedAnswer::Score(Self::evaluate_score(answer, levels)?),
                _ => return Err(AdapterError::TypeMismatch),
            };
            results.push((qid.clone(), adapted));
        }
        Ok(results)
    }

    pub fn evaluate_choice(answer: &Answer, criterion_ids: &[(String, String)]) -> Result<ChoiceResult, AdapterError> {
        if answer.probabilities.is_empty() {
            return Err(AdapterError::EmptyProbabilities);
        }
        if criterion_ids.len() != answer.probabilities.len() {
            return Err(AdapterError::LengthMismatch {
                expected: criterion_ids.len(),
                got: answer.probabilities.len(),
            });
        }

        let mut best_idx = 0usize;
        let mut best_prob = answer.probabilities[0];
        for (i, &prob) in answer.probabilities.iter().enumerate().skip(1) {
            match prob.partial_cmp(&best_prob) {
                Some(Ordering::Greater) => {
                    best_idx = i;
                    best_prob = prob;
                }
                Some(Ordering::Equal) => {
                    if i < best_idx {
                        best_idx = i;
                        best_prob = prob;
                    }
                }
                Some(Ordering::Less) => {}
                None => return Err(AdapterError::NaNProbability),
            }
        }
        let probabilities: Vec<(String, f64)> = answer.probabilities.iter().enumerate()
            .map(|(i, &p)| (criterion_ids[i].0.clone(), p))
            .collect();
        let winner = criterion_ids[best_idx].0.clone();
        let confidence = Self::compute_confidence(&answer.probabilities);
        Ok(ChoiceResult { winner, probabilities, confidence })
    }

    fn evaluate_noul(answer: &Answer) -> Result<NoulResult, AdapterError> {
        let true_idx = answer.keys.iter().position(|k| k == "true")
            .ok_or(AdapterError::MissingTrueKey)?;
        let prob_true = answer.probabilities.get(true_idx)
            .copied()
            .ok_or(AdapterError::EmptyProbabilities)?;
        Ok(NoulResult { prob_true })
    }

    pub fn evaluate_score(answer: &Answer, levels: &[String]) -> Result<ScoreResult, AdapterError> {
        if answer.probabilities.is_empty() {
            return Err(AdapterError::EmptyProbabilities);
        }
        if levels.len() != answer.probabilities.len() {
            return Err(AdapterError::LengthMismatch {
                expected: levels.len(),
                got: answer.probabilities.len(),
            });
        }
        let probabilities: Vec<(String, f64)> = levels.iter().map(|l| l.clone())
            .zip(answer.probabilities.iter().copied())
            .collect();
        let expected_score: f64 = answer.probabilities.iter().enumerate()
            .map(|(i, &p)| (i + 1) as f64 * p)
            .sum();
        let confidence = Self::compute_confidence(&answer.probabilities);
        Ok(ScoreResult { probabilities, expected_score, confidence })
    }

    /// Serialize adapted answers to JSON (mirrors `julia1::answers_to_json` structure).
    pub fn answers_to_json(&self, answers: &[(String, AdaptedAnswer)]) -> serde_json::Value {
        answers_to_json(answers)
    }
}

/// Free function for JSON serialization — does not require an `Adapter` instance,
/// so unit tests can run without a model checkpoint.
pub fn answers_to_json(answers: &[(String, AdaptedAnswer)]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (qid, adapted) in answers {
        map.insert(qid.clone(), adapted_to_json(adapted));
    }
    serde_json::json!({ "answers": map })
}

fn adapted_to_json(answer: &AdaptedAnswer) -> serde_json::Value {
    match answer {
        AdaptedAnswer::Choice(c) => {
            let probs: serde_json::Map<String, serde_json::Value> = c.probabilities.iter()
                .map(|(k, p)| (k.clone(), serde_json::json!(p)))
                .collect();
            let mut out = serde_json::Map::new();
            out.insert("type".into(), serde_json::json!("choice"));
            out.insert("probabilities".into(), serde_json::Value::Object(probs));
            out.insert("choice".into(), serde_json::json!(c.winner));
            out.insert("max_probability".into(), serde_json::json!(
                c.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)
            ));
            serde_json::Value::Object(out)
        }
        AdaptedAnswer::Noul(n) => {
            let mut out = serde_json::Map::new();
            out.insert("type".into(), serde_json::json!("noul"));
            out.insert("noul".into(), serde_json::json!(n.prob_true));
            serde_json::Value::Object(out)
        }
        AdaptedAnswer::Score(s) => {
            let probs: serde_json::Map<String, serde_json::Value> = s.probabilities.iter()
                .map(|(k, p)| (k.clone(), serde_json::json!(p)))
                .collect();
            let mut out = serde_json::Map::new();
            out.insert("type".into(), serde_json::json!("score"));
            out.insert("probabilities".into(), serde_json::Value::Object(probs));
            out.insert("score".into(), serde_json::json!(s.expected_score));
            out.insert("max_probability".into(), serde_json::json!(
                s.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)
            ));
            serde_json::Value::Object(out)
        }
    }
}