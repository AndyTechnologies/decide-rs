//! Real-weights integration tests for the Julia-1 decision model.
//!
//! ALL tests here are `#[ignore]`d and require the real Julia-1 checkpoint,
//! configured through the `DECIDE_RS_MODEL_PATH` environment variable pointing
//! at a local checkpoint directory. They assert structural expectations only
//! (answer keys exist, probabilities sum to ~1.0, values in valid ranges) and
//! make no accuracy, calibration or quality claims.
//!
//! These tests are NOT claimed to pass in this PR; they need the real weights.
use std::path::PathBuf;
use std::sync::OnceLock;

fn model_path() -> PathBuf {
    match std::env::var("DECIDE_RS_MODEL_PATH") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => panic!(
            "DECIDE_RS_MODEL_PATH is not set; set it to a local Julia-1 \
             checkpoint directory to run these real-weights tests"
        ),
    }
}

fn engine() -> &'static julia1::Engine {
    static ENGINE: OnceLock<julia1::Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        decide_rs::load_engine(&model_path()).expect("failed to load Julia-1 engine")
    })
}

fn run() -> julia1::Answers {
    decide_rs::run_decisions(
        engine(),
        decide_rs::sample_state(),
        &decide_rs::sample_questions(),
    )
    .expect("run_decisions failed")
}

fn answer<'a>(answers: &'a julia1::Answers, qid: &str) -> &'a julia1::Answer {
    &(answers
        .iter()
        .find(|(key, _)| key == qid)
        .unwrap_or_else(|| panic!("missing answer key {qid}"))
        .1)
}

fn assert_probabilities_sum_to_one(probabilities: &[f64]) {
    let sum: f64 = probabilities.iter().sum();
    assert!(
        (sum - 1.0).abs() < 1e-6,
        "probabilities sum to {sum}, expected ~1.0"
    );
}

/// Choice decision against real weights. Requires the real Julia-1 checkpoint;
/// NOT claimed to pass in this PR.
#[test]
#[ignore = "requires the real Julia-1 checkpoint via DECIDE_RS_MODEL_PATH"]
fn choice_is_one_of_the_criteria_ids() {
    let answers = run();
    let choice = answer(&answers, "route");
    let selected = choice
        .choice()
        .expect("choice answer must resolve an option");
    assert!(
        ["billing", "platform", "account"].contains(&selected),
        "choice {selected} is not one of the criteria ids"
    );
    assert_probabilities_sum_to_one(&choice.probabilities);
}

/// Noul decision against real weights. Requires the real Julia-1 checkpoint;
/// NOT claimed to pass in this PR.
#[test]
#[ignore = "requires the real Julia-1 checkpoint via DECIDE_RS_MODEL_PATH"]
fn noul_probability_is_in_unit_interval() {
    let answers = run();
    let noul = answer(&answers, "urgent").noul().expect("noul answer");
    assert!(
        (0.0..=1.0).contains(&noul),
        "noul probability {noul} outside [0, 1]"
    );
}

/// Score decision against real weights. Requires the real Julia-1 checkpoint;
/// NOT claimed to pass in this PR.
#[test]
#[ignore = "requires the real Julia-1 checkpoint via DECIDE_RS_MODEL_PATH"]
fn score_is_within_the_rubric_index_range() {
    let levels = 3; // sample rubric: calm / annoyed / angry
    let answers = run();
    let score = answer(&answers, "frustration")
        .score()
        .expect("score answer");
    assert!(
        (0.0..=levels as f64 - 1.0).contains(&score),
        "score {score} outside [0, {levels}) rubric index range"
    );
}
