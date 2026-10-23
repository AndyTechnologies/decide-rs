# decide-rs

> ⚠️ **Status: under active development.** This project is a work in progress and is not yet ready for production use. APIs, behavior, and documentation may change without notice.

A local CLI for structured decisions, written in Rust.

`decide-rs` implements a decision-making command-line interface that runs entirely on the user's machine, powered directly by the Rust [`julia1`](https://crates.io) library and the Julia-1 model. It aims to be a faithful Rust counterpart of the public API of **Jev / TypeSafe System One** — matching its input types, result semantics, and JSON schema as closely as possible.

No Python, ONNX Runtime, Ollama, or inference server is required: everything runs locally from a single executable.

## Planned components

- **CLI and decision library** — the inference core, with `choice`, `noul`, and `score` operations compatible in structure and semantics with the public Jev API.
- **MCP server** — a persistent interface so agents and applications can request evaluations without reloading the model on every call; it reuses the CLI core.
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

## pi-decide Extension (Pi)

The `pi-decide` extension registers the MCP server with Pi and adds system instructions so the agent evaluates alternatives before posing decision questions to the user.

### Installation

```bash
export DECIDE_RS_MCP_BINARY=/path/to/decide-rs-mcp  # or add to PATH
export DECIDE_RS_MODEL_PATH=/path/to/julia-checkpoint

pi --extension /path/to/decide-rs/extensions/pi-decide/src/extension.ts
```

### What it does

1. **Registers the MCP server** `decide-rs` with Pi (stdio transport, uses `decide-rs-mcp` binary)
2. **Adds system prompt guidance** so the agent knows when and how to use the decision tools
3. **Exposes three tools** in the `mcp__decide_rs` namespace:
   - `mcp__decide_rs__decide_choice` — categorical choices
   - `mcp__decide_rs__decide_noul` — yes/no decisions
   - `mcp__decide_rs__decide_score` — rubric scoring

### Usage in Pi

Once loaded, the agent can call the tools directly:

```
> Should I route this ticket to billing or platform team?

Based on the situation (checkout payments failing, revenue impact), the model recommends **platform** team with 72% probability...
```

### Configuration

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `DECIDE_RS_MCP_BINARY` | Path to `decide-rs-mcp` binary | `decide-rs-mcp` (from PATH) |
| `DECIDE_RS_MODEL_PATH` | Path to Julia-1 checkpoint | same resolution as the binary |
| `DECIDE_RS_PROJECT_ROOT` | Project root (for dev binary) | unset |

## Running the MCP Server

The MCP server binary is `decide-rs-mcp`. It requires a local Julia-1 checkpoint directory.

```bash
# Model resolution order: DECIDE_RS_MODEL_PATH, then
# ~/.local/share/decide-rs/julia-checkpoint, then ./julia-checkpoint
export DECIDE_RS_MODEL_PATH=/path/to/julia-checkpoint

# Run the server (communicates over stdio)
decide-rs-mcp
```

The server loads the model once at startup and serves requests using the Model Context Protocol (MCP). Valid calls carry the answers JSON both as text content and `structuredContent`; schema violations return `-32602` with `data.errors[]` of `{path, message}`.

## Tool contract

Tool names follow the MCP pattern recommendation (`decide_choice`, `decide_noul`, `decide_score`, no `/`), and each tool validates that every question it receives matches its own type: mixed-type requests are rejected with `-32602` and a per-question path (`questions.<id>.type`).

`state` accepts a string, object, or array. Objects and arrays are serialized canonically before inference (compact JSON, keys recursively sorted), so equal states always produce equal engine input.

## License

TBD.
