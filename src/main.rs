//! `decide-rs` CLI: load a local Julia-1 checkpoint and run one mixed
//! choice/noul/score decision, printing the JSON result to stdout.
//! `decide-rs fetch <dest-dir>` downloads the revision-pinned checkpoint.
use std::path::Path;

const USAGE: &str = "usage: decide-rs <model-path> [state-text]\n       decide-rs fetch <dest-dir>";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("fetch") {
        let dest = match (args.get(2), args.len()) {
            (Some(dest), 3) => dest,
            _ => {
                eprintln!("{USAGE}");
                std::process::exit(2);
            }
        };
        if let Err(err) = decide_rs::checkpoint::download(Path::new(dest)) {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
        println!(
            "checkpoint ready at {dest} (revision {})",
            decide_rs::checkpoint::REVISION
        );
        return;
    }
    let model_path = match args.get(1) {
        Some(path) => path,
        None => {
            eprintln!("{USAGE}");
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
    let adapter = decide_rs::adapter::Adapter::new_from_engine(engine);
    let adapted_answers = adapter.run(state, &decide_rs::sample_questions())?;
    let mut result = adapter.answers_to_json(&adapted_answers);
    result["model"] = serde_json::json!("julia-1");
    result["state"] = serde_json::json!(state);
    Ok(result)
}
