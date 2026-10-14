#!/usr/bin/env rust
//! Unit tests for the inference adapter using synthetic `julia1::Answer` values.
//! No real model weights required — tests exercise pure mapping and arithmetic.

use decide_rs::adapter::{AdaptedAnswer, ChoiceResult, NoulResult, ScoreResult, Adapter};
use julia1::{Answer, Answers, QType};
use serde_json::json;

/// Build synthetic [`Answers`] from raw probability vectors.
fn synth_answers(questions: &[(&str, QType, Vec<String>, Vec<f64>)]) -> Answers {
    questions.iter().map(|(qid, qtype, keys, probs)| {
        (qid.to_string(), Answer {
            qtype: *qtype,
            keys: keys.clone(),
            probabilities: probs.clone(),
        })
    }).collect()
}

// ---- Choice mapping tests ----

#[test]
fn choice_argmax_selects_highest_probability() {
    let probs = vec![0.1, 0.7, 0.2];
    let ids = vec!["billing", "platform", "account"];
    let idx = probs.iter().enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i).unwrap();
    assert_eq!(ids[idx], "platform");
}

#[test]
fn choice_tie_goes_to_first_index() {
    use std::cmp::Ordering;
    let probs = vec![0.4, 0.4, 0.2];
    let idx = probs.iter().enumerate()
        .max_by(|(i_a, p_a), (i_b, p_b)| {
            match p_a.partial_cmp(p_b).unwrap() {
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
                Ordering::Equal => i_a.cmp(i_b).reverse(),
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    assert_eq!(idx, 0, "tie should pick first index");
}

#[test]
fn choice_probabilities_preserve_order() {
    let criterion_ids: Vec<(String, String)> = vec![("billing".into(), "Payments".into()), ("platform".into(), "Checkout".into()), ("account".into(), "Identity".into())];
    let probs = vec![0.1, 0.7, 0.2];
    let mapped: Vec<(String, f64)> = criterion_ids.iter().enumerate().map(|(i, (id, _))| {
        (id.clone(), probs[i])
    }).collect();
    assert_eq!(mapped[0], ("billing".into(), 0.1f64));
    assert_eq!(mapped[1], ("platform".into(), 0.7f64));
    assert_eq!(mapped[2], ("account".into(), 0.2f64));
}

// ---- Noul tests ----

#[test]
fn noul_true_index_is_second_element() {
    let answer = Answer {
        qtype: QType::Noul,
        keys: vec!["false".into(), "true".into()],
        probabilities: vec![0.3, 0.7],
    };
    let true_idx = answer.keys.iter().position(|k| k == "true").unwrap_or(1);
    assert_eq!(true_idx, 1);
    assert!((answer.probabilities[true_idx] - 0.7).abs() < 1e-6);
}

// ---- Score tests (one-indexed expected value) ----

#[test]
fn score_expected_value_is_one_indexed() {
    // P(Calm)=0.2, P(Annoyed)=0.5, P(Angry)=0.3
    // Expected = 1*0.2 + 2*0.5 + 3*0.3 = 2.1
    let probs = vec![0.2, 0.5, 0.3];
    let expected: f64 = probs.iter().enumerate().map(|(i, &p)| (i + 1) as f64 * p).sum();
    assert!((expected - 2.1).abs() < 1e-6, "expected 2.1, got {expected}");
}

#[test]
fn score_two_levels_expected_value() {
    // Binary rubric: P(Low)=0.8, P(High)=0.2
    // Expected = 1*0.8 + 2*0.2 = 1.2
    let probs = vec![0.8, 0.2];
    let expected: f64 = probs.iter().enumerate().map(|(i, &p)| (i + 1) as f64 * p).sum();
    assert!((expected - 1.2).abs() < 1e-6);
}

#[test]
fn score_probabilities_sum_invariant() {
    let probs = vec![0.33, 0.34, 0.33];
    let sum: f64 = probs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-4);
}

// ---- Confidence tests (formula validation) ----

#[test]
fn confidence_uniform_distribution() {
    // For n options, uniform probability = 1/n each -> confidence = 0
    let probs = vec![0.5, 0.5]; // n=2
    assert!((Adapter::compute_confidence(&probs) - 0.0).abs() < 1e-6);
    let probs = vec![1.0/3.0, 1.0/3.0, 1.0/3.0]; // n=3
    assert!((Adapter::compute_confidence(&probs) - 0.0).abs() < 1e-6);
    let probs = vec![0.25, 0.25, 0.25, 0.25]; // n=4
    assert!((Adapter::compute_confidence(&probs) - 0.0).abs() < 1e-6);
}

#[test]
fn confidence_degenerate_distribution() {
    // One option has probability 1.0 -> confidence = 1
    let probs = vec![1.0, 0.0];
    assert!((Adapter::compute_confidence(&probs) - 1.0).abs() < 1e-6);
    let probs = vec![0.0, 1.0, 0.0];
    assert!((Adapter::compute_confidence(&probs) - 1.0).abs() < 1e-6);
}

#[test]
fn confidence_three_choice_example_from_docs() {
    // From Jev docs: largest probability 0.9 -> confidence 0.85
    let probs = vec![0.9, 0.05, 0.05];
    assert!((Adapter::compute_confidence(&probs) - 0.85).abs() < 1e-6);
    // largest 0.5 -> confidence 0.25
    let probs = vec![0.5, 0.25, 0.25];
    assert!((Adapter::compute_confidence(&probs) - 0.25).abs() < 1e-6);
    // largest 0.34 -> confidence ~0.01
    let probs = vec![0.34, 0.33, 0.33];
    assert!((Adapter::compute_confidence(&probs) - 0.01).abs() < 1e-6);
}

#[test]
fn confidence_score_example_from_docs() {
    // Score example: probabilities [0.0, 0.57, 0.43] -> confidence 0.355
    let probs = vec![0.0, 0.57, 0.43];
    assert!((Adapter::compute_confidence(&probs) - 0.355).abs() < 1e-6);
}

#[test]
fn confidence_choice_result_includes_confidence() {
    let criterion_ids = vec![("billing".into(), "Payments".into()), ("platform".into(), "Checkout".into())];
    let answer = Answer {
        qtype: QType::Choice,
        keys: vec!["billing".into(), "platform".into()],
        probabilities: vec![0.2, 0.8],
    };
    let adapted = Adapter::evaluate_choice(&answer, &criterion_ids);
    assert!((adapted.confidence - 0.6).abs() < 1e-6); // (2*0.8 -1)/(2-1) = 0.6
    assert_eq!(adapted.winner, "platform");
}

#[test]
fn confidence_score_result_includes_confidence() {
    let levels = vec!["Low".into(), "Medium".into(), "High".into()];
    let answer = Answer {
        qtype: QType::Score,
        keys: vec![],
        probabilities: vec![0.1, 0.7, 0.2],
    };
    let adapted = Adapter::evaluate_score(&answer, &levels);
    // max=0.7, n=3 => (3*0.7 -1)/2 = 0.55
    assert!((adapted.confidence - 0.55).abs() < 1e-6);
}

// ---- JSON serialization tests (pure AdaptedAnswer → JSON) ----

fn answers_json(answers: &[(String, AdaptedAnswer)]) -> serde_json::Value {
    use serde_json::Map;
    let mut map = Map::new();
    for (qid, adapted) in answers {
        map.insert(qid.clone(), adapted_to_json(adapted));
    }
    json!({ "answers": map })
}

fn adapted_to_json(answer: &AdaptedAnswer) -> serde_json::Value {
    match answer {
        AdaptedAnswer::Choice(c) => {
            let probs: serde_json::Map<String, serde_json::Value> =
                c.probabilities.iter().map(|(k, p)| (k.clone(), json!(p))).collect();
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("choice"));
            out.insert("probabilities".into(), serde_json::Value::Object(probs));
            out.insert("choice".into(), json!(c.winner));
            out.insert("confidence".into(), json!(c.confidence));
            out.insert("max_probability".into(), json!(c.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)));
            serde_json::Value::Object(out)
        }
        AdaptedAnswer::Noul(n) => {
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("noul"));
            out.insert("noul".into(), json!(n.prob_true));
            serde_json::Value::Object(out)
        }
        AdaptedAnswer::Score(s) => {
            let probs: serde_json::Map<String, serde_json::Value> =
                s.probabilities.iter().map(|(k, p)| (k.clone(), json!(p))).collect();
            let mut out = serde_json::Map::new();
            out.insert("type".into(), json!("score"));
            out.insert("probabilities".into(), serde_json::Value::Object(probs));
            out.insert("score".into(), json!(s.expected_score));
            out.insert("confidence".into(), json!(s.confidence));
            out.insert("max_probability".into(), json!(s.probabilities.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max)));
            serde_json::Value::Object(out)
        }
    }
}

#[test]
fn choice_json_has_winner_and_probabilities() {
    let results = &[(
        "route".into(),
        AdaptedAnswer::Choice(ChoiceResult {
            winner: "platform".into(),
            probabilities: vec![
                ("billing".into(), 0.1),
                ("platform".into(), 0.7),
                ("account".into(), 0.2),
            ],
            confidence: 0.55, // (3*0.7 -1)/2 = 0.55
        }),
    )];
    let json = answers_json(results);
    let route = &json["answers"]["route"];
    assert_eq!(route["type"].as_str(), Some("choice"));
    assert_eq!(route["choice"].as_str(), Some("platform"));
    assert!((route["probabilities"]["platform"].as_f64().unwrap() - 0.7).abs() < 1e-6);
    assert!((route["confidence"].as_f64().unwrap() - 0.55).abs() < 1e-6);
    assert!(route["max_probability"].is_number());
}

#[test]
fn noul_json_has_noul_field() {
    let results = &[("urgent".into(), AdaptedAnswer::Noul(NoulResult { prob_true: 0.85 }))];
    let json = answers_json(results);
    let urgent = &json["answers"]["urgent"];
    assert_eq!(urgent["type"].as_str(), Some("noul"));
    assert!((urgent["noul"].as_f64().unwrap() - 0.85).abs() < 1e-6);
}

#[test]
fn score_json_has_expected_score_and_levels() {
    let results = &[(
        "frustration".into(),
        AdaptedAnswer::Score(ScoreResult {
            probabilities: vec![("Calm".into(), 0.1), ("Annoyed".into(), 0.6), ("Angry".into(), 0.3)],
            expected_score: 2.2,
            confidence: 0.4, // (3*0.6 -1)/2 = 0.4
        }),
    )];
    let json = answers_json(results);
    let frustration = &json["answers"]["frustration"];
    assert_eq!(frustration["type"].as_str(), Some("score"));
    assert!((frustration["score"].as_f64().unwrap() - 2.2).abs() < 1e-6);
    assert!((frustration["confidence"].as_f64().unwrap() - 0.4).abs() < 1e-6);
    assert!((frustration["probabilities"]["Annoyed"].as_f64().unwrap() - 0.6).abs() < 1e-6);
}

// ---- Integration with synth_answers + pure logic ----

#[test]
fn synth_answers_structure_matches_julia1() {
    let answers = synth_answers(&[
        ("q1", QType::Choice, vec!["a".into(), "b".into()], vec![0.3, 0.7]),
        ("q2", QType::Noul, vec!["false".into(), "true".into()], vec![0.5, 0.5]),
        ("q3", QType::Score, vec!["0".into(), "1".into(), "2".into()], vec![0.2, 0.5, 0.3]),
    ]);
    assert_eq!(answers.len(), 3);
    assert_eq!(answers[0].0, "q1");
    assert_eq!(answers[1].0, "q2");
    assert_eq!(answers[2].0, "q3");
    // Verify julia1::Answer helper methods agree
    assert_eq!(answers[0].1.choice(), Some("b"));
    assert!((answers[1].1.noul().unwrap() - 0.5).abs() < 1e-6);
    // julia1 score is zero-indexed; adapter uses one-indexed
    assert!((answers[2].1.score().unwrap() - 1.1).abs() < 1e-6); // 0*0.2 + 1*0.5 + 2*0.3 = 1.1
}
