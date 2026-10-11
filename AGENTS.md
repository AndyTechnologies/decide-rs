# AGENTS.md

## Mandatory execution rules

### PR size and contents

- Every PR must modify fewer than 400 lines of hand-written source, tests, configuration, and documentation. Target 100–300 lines.
- Tool-generated files (Cargo.lock, model weights, binary artifacts) are excluded from that count, but must still be reviewed, versioned, or published as appropriate.
- If a phase needs 399 or more lines within budget, split it into independent PRs before implementing.
- Do not combine two independent concerns, or features from different phases, in one PR.
- Do not implement a phase if the previous phase's tests do not pass.
- Each PR includes: objective and scope; changes made; tests run and their results; evidence of any real-model check; known limitations; decisions still open.

### Reproducibility

- Before implementing, investigate the latest published versions of: `julia1` on crates.io; Julia-1 and its checkpoint in the official model repo; Pi and Gentle Shell/Gentle Pi; the public Pi extension API of the target version; the official Rust MCP SDK (`rmcp`); and the official adapters or repositories that document Jev confidence semantics.
- Record exact versions and revisions, the UTC verification date, and the sources consulted.
- Do not use floating references such as `main`, `latest`, or mutable tags as final model identifiers. The version used in a release must be pinned and documented.

### Compatibility and honesty

- Do not declare Jev compatibility for a feature that only has partial compatibility.
- Do not invent model properties, usage metrics, probabilities, or calibration guarantees.
- Do not generate free-form justifications attributed to Julia-1 that the model did not produce.
- When a check requires the real model weights, create an explicit integration test. Do not replace it with a mocked unit test and claim that the real test passed.

### Architecture

- The engine does not depend on Pi, MCP, or the installation system.
- The MCP server reuses the same core as the CLI.
- The Pi extension is a small, independent TypeScript project that communicates with the server through MCP.
- Do not introduce ONNX Runtime, Python, Ollama, ChooseKit, or another inference server as runtime dependencies.
- Do not load the model once per question or tool call.
- Pin exact dependency versions and commit `Cargo.lock`; do not use floating references to download the model.
- Do not commit model weights to Git. They must be distributed as release artifacts.
