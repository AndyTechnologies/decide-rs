/// Thin typed adapter between `julia1::Answers` and decide-rs structured results.
///
/// The adapter wraps the engine, calls `predict_typed` once, and translates the
/// raw softmax answers into typed [`AdaptedAnswer`] variants that carry the
/// caller's criterion IDs or rubric levels alongside the probabilities.
use crate::{load_engine, run_decisions};
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
    pub fn new(model_path: &Path) -> anyhow::Result<Self> {
        Ok(Self { engine: load_engine(model_path)? })
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
        let max = probs.iter().cloned().fold(0.0f64, |a, b| a.max(b));
        let n_f = n as f64;
        (n_f * max - 1.0) / (n_f - 1.0)
    }

    /// Validate the request and run inference, returning adapted answers.
    pub fn run(&self, state: &str, questions: &Value) -> anyhow::Result<Vec<(String, AdaptedAnswer)>> {
        let request = crate::schema::parse_request(&Value::String(state.to_owned()), questions).map_err(|errors| {
            let details: Vec<String> = errors.iter()
                .map(|e| format!("  {}: {}", e.path, e.message))
                .collect();
            anyhow::anyhow!("request rejected before inference:\n{}", details.join("\n"))
        })?;
        let answers = run_decisions(&self.engine, state, questions)?;
        Ok(self.evaluate(&request, &answers))
    }

    /// Translate raw [`Answers`] into typed [`AdaptedAnswer`]s using schema context.
    pub fn evaluate(&self, request: &Request, answers: &Answers) -> Vec<(String, AdaptedAnswer)> {
        answers.iter().map(|(qid, answer)| {
            let question = request.questions.iter()
                .find(|(id, _)| id == qid)
                .expect("answer key without matching question");
            let adapted = match &question.1 {
                Question { qtype: QuestionType::Choice, criteria: Criteria::Choice(criterion_ids), .. } => {
                    AdaptedAnswer::Choice(Adapter::evaluate_choice(answer, criterion_ids))
                }
                Question { qtype: QuestionType::Noul, .. } => AdaptedAnswer::Noul(self.evaluate_noul(answer)),
                Question { qtype: QuestionType::Score, criteria: Criteria::Score(levels), .. } => {
                    AdaptedAnswer::Score(Adapter::evaluate_score(answer, levels))
                }
                _ => unreachable!("schema validation should prevent type mismatch"),
            };
            (qid.clone(), adapted)
        }).collect()
    }

    pub fn evaluate_choice(answer: &Answer, criterion_ids: &[(String, String)]) -> ChoiceResult {
        let mut best_idx = 0usize;
        let mut best_prob = answer.probabilities[0];
        for (i, &prob) in answer.probabilities.iter().enumerate().skip(1) {
            match prob.partial_cmp(&best_prob).unwrap() {
                Ordering::Greater => {
                    best_idx = i;
                    best_prob = prob;
                }
                Ordering::Equal => {
                    // tie: choose lower index (i.e., keep best_idx if it's lower)
                    if i < best_idx {
                        best_idx = i;
                        best_prob = prob;
                    }
                }
                Ordering::Less => {}
            }
        }
        let probabilities: Vec<(String, f64)> = answer.probabilities.iter().enumerate().map(|(i, &p)| {
            (criterion_ids[i].0.clone(), p)
        }).collect();
        let winner = criterion_ids[best_idx].0.clone();
        let confidence = Self::compute_confidence(&answer.probabilities);
        ChoiceResult { winner, probabilities, confidence }
    }

    fn evaluate_noul(&self, answer: &Answer) -> NoulResult {
        let true_idx = answer.keys.iter().position(|k| k == "true").unwrap_or(1);
        NoulResult { prob_true: answer.probabilities[true_idx] }
    }

    pub fn evaluate_score(answer: &Answer, levels: &[String]) -> ScoreResult {
        let probabilities: Vec<(String, f64)> = levels.iter().map(|l| l.clone()).zip(answer.probabilities.iter().copied())
            .collect();
        let expected_score: f64 = answer.probabilities.iter().enumerate()
            .map(|(i, &p)| (i + 1) as f64 * p)
            .sum();
        let confidence = Self::compute_confidence(&answer.probabilities);
        ScoreResult { probabilities, expected_score, confidence }
    }

    /// Serialize adapted answers to JSON (mirrors `julia1::answers_to_json` structure).
    pub fn answers_to_json(&self, answers: &[(String, AdaptedAnswer)]) -> Value {
        let mut map = Map::new();
        for (qid, adapted) in answers {
            map.insert(qid.clone(), json!(adapted_to_json(adapted)));
        }
        json!({ "answers": map })
    }
}

fn adapted_to_json(answer: &AdaptedAnswer) -> Value {
    match answer {
        AdaptedAnswer::Choice(c) => {
            let probs: Map<String, Value> = c.probabilities.iter().map(|(k, p)| (k.clone(), json!(p))).collect();
            let mut out = Map::new();
            out.insert("type".into(), json!("choice"));
            out.insert("probabilities".into(), Value::Object(probs));
            out.insert("choice".into(), json!(c.winner));
            out.insert("max_probability".into(), json!(c.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)));
            Value::Object(out)
        }
        AdaptedAnswer::Noul(n) => {
            let mut out = Map::new();
            out.insert("type".into(), json!("noul"));
            out.insert("noul".into(), json!(n.prob_true));
            Value::Object(out)
        }
        AdaptedAnswer::Score(s) => {
            let probs: Map<String, Value> = s.probabilities.iter().map(|(k, p)| (k.clone(), json!(p))).collect();
            let mut out = Map::new();
            out.insert("type".into(), json!("score"));
            out.insert("probabilities".into(), Value::Object(probs));
            out.insert("score".into(), json!(s.expected_score));
            out.insert("max_probability".into(), json!(s.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)));
            Value::Object(out)
        }
    }
}
