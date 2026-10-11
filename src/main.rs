//! `decide-rs` CLI: load a local Julia-1 checkpoint and run one mixed
//! choice/noul/score decision, printing the JSON result to stdout.
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model_path = match args.get(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: decide-rs <model-path> [state-text]");
            std::process::exit(2);
        }
    };
    let state: &str = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or(decide_rs::sample_state());
    match run(model_path, state) {
        Ok(result) => println!("{result}"),
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    }
}

fn run(model_path: &str, state: &str) -> anyhow::Result<serde_json::Value> {
    let engine = decide_rs::load_engine(Path::new(model_path))?;
    let answers = decide_rs::run_decisions(&engine, state, &decide_rs::sample_questions())?;
    let mut result = decide_rs::answers_json(&answers);
    result["model"] = serde_json::json!("julia-1");
    result["state"] = serde_json::json!(state);
    Ok(result)
}
