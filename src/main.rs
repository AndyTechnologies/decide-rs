//! `decide-rs` CLI: run decision requests against a local Julia-1 checkpoint.
//!
//! Subcommands accept the same JSON contract as the MCP tools
//! (`{state, questions}`), validated by `decide_rs::mcp::prepare_call`:
//!
//! - `decide-rs choice|noul|score [FILE|-]` — only the subcommand's question
//!   type; one request object.
//! - `decide-rs run [FILE|-] [--jsonl] [--lenient]` — mixed-type requests;
//!   `--jsonl` processes newline-delimited requests.
//! - `decide-rs demo [state]` — the bundled sample questions (kept for the
//!   historical positional interface).
//! - `decide-rs fetch [dest-dir]` — download the revision-pinned checkpoint.
//!
//! Common flags: `--model PATH` (explicit checkpoint, else the resolution
//! chain), `--pretty` (indented output), `--help`, `--version`.
//!
//! Exit codes: 0 success; 1 runtime error; 2 usage error; 3 rejected request
//! (validation failure; with `--jsonl --lenient` invalid records are reported
//! to stderr and do not stop processing unless a later engine error occurs).
use std::io::Read as _;
use std::path::{Path, PathBuf};

use decide_rs::adapter::Adapter;
use decide_rs::mcp;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const REVISION: &str = decide_rs::checkpoint::REVISION;

const USAGE: &str = "\
usage: decide-rs <command> [options] [FILE|-]

commands:
  choice|noul|score [FILE|-]   run one request; questions must match the command type
  run [FILE|-] [--jsonl] [--lenient]   run one or more mixed-type requests
  demo [state]                 run the bundled sample questions
  fetch [dest-dir]             download the revision-pinned checkpoint

options:
  --model PATH   checkpoint directory (defaults to the shared resolution chain)
  --pretty       indented JSON output
  --help         show this help
  --version      print the version

input (choice|noul|score|run): a JSON object {\"state\": <string|object|array>,
\"questions\": {...}} from FILE or stdin ('-'); with --jsonl, one such object
per line. exit codes: 0 ok, 1 runtime error, 2 usage, 3 rejected request.
";

/// Exhaust the rest of the command line collecting command options.
fn parse_command_tail(
    argv: &mut impl Iterator<Item = String>,
    parsed: &mut Args,
) {
    while let Some(rest) = argv.next() {
        match rest.as_str() {
            "--jsonl" => parsed.jsonl = true,
            "--lenient" => parsed.lenient = true,
            "--pretty" => parsed.pretty = true,
            "--model" => {
                let value = argv
                    .next()
                    .ok_or("--model requires a PATH argument")
                    .ok();
                match value {
                    Some(path) => parsed.model = Some(PathBuf::from(path)),
                    None => {
                        eprintln!("--model requires a PATH argument");
                        std::process::exit(2);
                    }
                }
            }
            positional if positional == "-" || !positional.starts_with('-') => {
                parsed.file = Some(positional.to_owned());
                return;
            }
            other => {
                eprintln!("unknown option or flag \"{other}\"");
                std::process::exit(2);
            }
        }
    }
}

/// Parsed process arguments.
struct Args {
    command: Command,
    model: Option<PathBuf>,
    file: Option<String>,
    pretty: bool,
    jsonl: bool,
    lenient: bool,
    state: Option<String>,
    fetch_dest: Option<String>,
}

enum Command {
    Requests { kind: &'static str },
    Mixed,
    Demo,
    Fetch,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut argv = std::env::args().skip(1);
        let mut parsed = Self {
            command: Command::Mixed,
            model: None,
            file: None,
            pretty: false,
            jsonl: false,
            lenient: false,
            state: None,
            fetch_dest: None,
        };
        // Global flags may appear before or after the command.
        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                "--version" | "-v" => {
                    println!("decide-rs {VERSION}");
                    std::process::exit(0);
                }
                "--model" => {
                    parsed.model = Some(
                        argv.next()
                            .ok_or("--model requires a PATH argument")?
                            .into(),
                    );
                }
                "--pretty" => parsed.pretty = true,
                "--jsonl" => parsed.jsonl = true,
                "--lenient" => parsed.lenient = true,
                "choice" | "noul" | "score" => {
                    let kind = mcp::tool_question_type(&format!("decide_{arg}"))
                        .expect("tool name matches command");
                    parsed.command = Command::Requests { kind };
                    parse_command_tail(&mut argv, &mut parsed);
                    break;
                }
                "run" => {
                    parsed.command = Command::Mixed;
                    parse_command_tail(&mut argv, &mut parsed);
                    break;
                }
                "demo" => {
                    parsed.command = Command::Demo;
                    parsed.state = argv.next();
                    break;
                }
                "fetch" => {
                    parsed.command = Command::Fetch;
                    parsed.fetch_dest = argv.next();
                    break;
                }
                other => return Err(format!("unknown command or option \"{other}\"")),
            }
        }
        Ok(parsed)
    }
}

/// Validate before touching the engine, so rejections use exit code 3
/// even when no checkpoint is available.
fn prepare(kind: &str, args: &serde_json::Value) -> Result<mcp::PreparedCall, i32> {
    mcp::prepare_call(kind, args).map_err(|err| {
        eprintln!("error: {err}");
        3
    })
}

fn load_adapter(model: Option<&Path>) -> Result<Adapter, i32> {
    let path = model.map(Path::to_owned).unwrap_or_else(decide_rs::resolve_model_path);
    Adapter::new(&path).map_err(|err| {
        eprintln!("error: {err:#}");
        1
    })
}

fn print_json(value: &serde_json::Value, pretty: bool) {
    if pretty {
        println!("{}", serde_json::to_string_pretty(value).expect("pretty"));
    } else {
        println!("{}", value);
    }
}

/// One request object rendered for output.
fn result_json(answers: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "answers": answers["answers"],
        "model": "julia-1",
        "revision": REVISION,
    })
}

/// Read the whole input: a file path, stdin for `-`, or raw stdin when no
/// file is given (the engine never reads stdin itself).
fn read_input(file: Option<&str>) -> Result<String, i32> {
    match file {
        Some("-") | None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|err| {
                    eprintln!("error reading stdin: {err}");
                    1
                })?;
            Ok(buf)
        }
        Some(path) => std::fs::read_to_string(path).map_err(|err| {
            eprintln!("error reading {path}: {err}");
            1
        }),
    }
}

/// Run one validated request against the adapter and print its JSON.
fn run_one(adapter: &Adapter, pretty: bool, prepared: &mcp::PreparedCall) -> Result<(), i32> {
    let answers = adapter
        .run(&prepared.state_text, &prepared.questions)
        .map_err(|err| {
            eprintln!("error: {err:#}");
            1
        })?;
    let json = adapter.answers_to_json(&answers);
    print_json(&result_json(&json), pretty);
    Ok(())
}

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            std::process::exit(2);
        }
    };
    if let Err(code) = run(args) {
        std::process::exit(code);
    }
}

fn run(args: Args) -> Result<(), i32> {
    match args.command {
        Command::Fetch => {
            let dest: PathBuf = args
                .fetch_dest
                .map(PathBuf::from)
                .unwrap_or_else(decide_rs::installed_model_path);
            if let Err(err) = decide_rs::checkpoint::download(&dest) {
                eprintln!("error: {err:#}");
                return Err(1);
            }
            println!("checkpoint ready at {} (revision {REVISION})", dest.display());
            Ok(())
        }
        Command::Demo => {
            let adapter = load_adapter(args.model.as_deref())?;
            let state = args
                .state
                .unwrap_or_else(|| decide_rs::sample_state().to_owned());
            let answers = adapter
                .run(&state, &decide_rs::sample_questions())
                .map_err(|err| {
                    eprintln!("error: {err:#}");
                    1
                })?;
            let json = adapter.answers_to_json(&answers);
            let mut result = result_json(&json);
            result["state"] = serde_json::json!(state);
            print_json(&result, args.pretty);
            Ok(())
        }
        Command::Requests { kind } => {
            let tool = format!("decide_{kind}");
            let raw = read_input(args.file.as_deref())?;
            let request = parse_one(&raw)?;
            let prepared = mcp::prepare_call(&tool, &request).map_err(|err| {
                eprintln!("error: {err}");
                3
            })?;
            let adapter = load_adapter(args.model.as_deref())?;
            run_one(&adapter, args.pretty, &prepared)
        }
        Command::Mixed => run_mixed(args),
    }
}

/// Parse a single JSON document.
fn parse_one(raw: &str) -> Result<serde_json::Value, i32> {
    serde_json::from_str(raw.trim()).map_err(|err| {
        eprintln!("error: invalid JSON input: {err}");
        3
    })
}

/// Mixed-type runner: one JSON document, or `--jsonl` with one document per
/// line. Default policy is strict: the first rejected record stops the run
/// with exit code 3 and no partial output. `--lenient` reports rejected
/// records on stderr and keeps going; only engine errors abort the rest.
fn run_mixed(args: Args) -> Result<(), i32> {
    let raw = read_input(args.file.as_deref())?;
    let mut records: Vec<(usize, serde_json::Value)> = Vec::new();
    if args.jsonl {
        for (index, line) in raw.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(line) {
                Ok(record) => records.push((index, record)),
                Err(err) => {
                    eprintln!("error (record {index}): invalid JSON line: {err}");
                    if !args.lenient {
                        return Err(3);
                    }
                }
            }
        }
    } else {
        records.push((0, parse_one(&raw)?));
    }

    if records.is_empty() {
        eprintln!("error: no requests in input");
        return Err(3);
    }

    // Validate every record up front: engine work never starts on a
    // half-processed batch; lenient mode skips rejects.
    let mut prepared: Vec<mcp::PreparedCall> = Vec::with_capacity(records.len());
    for (index, record) in &records {
        match mcp::parse_arguments(record) {
            Ok(call) => prepared.push(call),
            Err(err) => {
                eprintln!("error (record {index}): {err}");
                if !args.lenient {
                    return Err(3);
                }
            }
        }
    }

    let adapter = load_adapter(args.model.as_deref())?;
    for call in &prepared {
        let answers = adapter
            .run(&call.state_text, &call.questions)
            .map_err(|err| {
                eprintln!("error: {err:#}");
                1
            })?;
        let json = adapter.answers_to_json(&answers);
        print_json(&result_json(&json), args.pretty);
    }
    Ok(())
}
