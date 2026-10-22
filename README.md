# decide-rs

> ⚠️ **Status: under active development.** This project is a work in progress and is not yet ready for production use. APIs, behavior, and documentation may change without notice.

A local CLI for structured decisions, written in Rust.

`decide-rs` implements a decision-making command-line interface that runs entirely on the user's machine, powered directly by the Rust [`julia1`](https://crates.io) library and the Julia-1 model. It aims to be a faithful Rust counterpart of the public API of **Jev / TypeSafe System One** — matching its input types, result semantics, and JSON schema as closely as possible.

No Python, ONNX Runtime, Ollama, or inference server is required: everything runs locally from a single executable.

## Planned components

- **CLI and decision library** — the inference core, with `choice`, `noul`, and `score` operations compatible in structure and semantics with the public Jev API.
- **MCP server** — a persistent interface so agents and applications can request evaluations without reloading the model on every call; it reuses the CLI core. The server runs over stdio transport and exposes three tools:
  - `decide/choice` — categorical choice decisions
  - `decide/noul` — yes/no (noul) decisions
  - `decide/score` — score/rubric decisions
- **pi-decide extension** — a small, independent TypeScript Pi extension that registers the MCP server and adds instructions so the agent evaluates alternatives before posing decision questions to the user when appropriate. It does **not** intercept or modify Gentle Shell question-tool calls; the agent decides when to call the MCP tools and what to do with the result.

## Objective

- Provide a local CLI for structured decisions in Rust, using the `julia1` library and the Julia-1 model directly.
- Approximate the public API of Jev / TypeSafe System One as closely as possible in input types, result semantics, and JSON schema.
- Run locally without requiring the user to install Python, ONNX Runtime, Ollama, or any inference server.
- Target platform for the first version: **Linux x86-64**.

## Planned distributions

| Distribution | Contents | Network requirement |
| --- | --- | --- |
| `full-offline` | Executable + all model resources bundled | Works fully offline after installation |
| `standard` | Executable + runtime included | Model is downloaded automatically on first use |

## Development

Implementation is incremental: each phase is developed in an independent PR, with tests and documentation scoped to that phase.

- **Language:** Rust (edition 2024)
- **First supported platform:** Linux x86-64

## Running the MCP Server

The MCP server binary is `decide-rs-mcp`. It requires a local Julia-1 checkpoint directory.

```bash
# Model resolution order: DECIDE_RS_MODEL_PATH, then
# ~/.local/share/decide-rs/julia-checkpoint, then ./julia-checkpoint
export DECIDE_RS_MODEL_PATH=/path/to/julia-checkpoint

# Run the server (communicates over stdio)
decide-rs-mcp
```

The server loads the model once at startup and serves requests over stdio using the Model Context Protocol (MCP). It exposes three tools:

- `decide/choice` — parameters: `state` (string), `questions` (object with choice-type question)
- `decide/noul` — parameters: `state` (string), `questions` (object with noul-type question)
- `decide/score` — parameters: `state` (string), `questions` (object with score-type question)

Each tool returns a JSON result with the decision outcome.

## License

TBD.
