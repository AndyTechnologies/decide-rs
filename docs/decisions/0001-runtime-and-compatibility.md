# ADR 0001 — Runtime and compatibility decisions

- **Status:** Proposed — awaiting review. No implementation of these decisions starts before approval.
- **Verification date (UTC):** 2026-10-10, 22:20–22:45Z. URLs below are starting points, not immutable references; re-verify at each phase start.
- **Scope:** Phase 0 research only (docs, no code), including Pi/MCP integration feasibility (§7).

## 1. Verified versions

| Resource | Exact version / revision | Notes |
| --- | --- | --- |
| `julia1` crate | **0.1.2** (only release; published 2026-10-02) | Apache-2.0, edition 2024, lib + `julia1` bin; features: default (CPU), `cuda` (optional `cudarc`) [S1] |
| Julia-1 checkpoint | HF repo sha **`a85b127321d580d65176c89ced8273f305745d85`** (last modified 2026-09-26T22:03:07Z) | Apache-2.0; base model `jhu-clsp/mmBERT-small`; 144,292,870 F32 params [S4] |
| Jev (TypeSafe) | `jev-latest` → **`jev-1.13.0`** in published examples | Jaggedness doc last reviewed 2026-10-02 [S5][S7] |
| `system-one-adapter-python` | main tree sha `e1d4cc938204b22fc5a3c3aca7044072fe3f712d` | Confidence formula cross-check [S8] |
| Pi (target host) | **1.1.0** (installed) | `pi.registerMcpServer` + `before_agent_start` available [S10] |
| `rmcp` crate | **3.5.1** (Apache-2.0) | Official MCP Rust SDK; spec 2026-07-28 [S11] |
| Gentle Shell | MIT, TypeScript, active (last push 2026-10-11) | pi-decide host harness [S12] |
| Canonical endpoint | `POST https://api.typesafe.ai/v1/systemone` | From this egress: HTTP **451** region block (verified 2026-10-10); from a non-blocked network: GET → **405** (POST-only, live) [S9] |

## 2. Checkpoint revision and download

- Runtime files (from `julia1::download::FILES`): `julia_config.json`, `encoder/config.json`, `tokenizer/tokenizer.json`, `tokenizer/tokenizer_config.json`, `model.safetensors`. Total download ≈ 577 MB [S2].
- **Pinned download URL pattern:** `https://huggingface.co/SupersonicLabs/Julia-1/resolve/a85b127321d580d65176c89ced8273f305745d85/<file>`.
- Integrity: `provenance.json` and `inference-policy.json` both declare `weights_sha256 = df853bf7fe424420011f3d0c47a05d7341aa9eefa7fb9f203ea4aada4ad95b72` for `model.safetensors`; variant `posttrained-candidate`, training step 500 [S4].
- **D1 (runtime):** do **not** use `Engine::from_pretrained()` / `julia1::download::download()` in shipping code: it fetches `resolve/main/...`, a floating reference, which violates the pinning rule [S2]. The CLI downloads the five files from the pinned revision above, verifies `weights_sha256`, then calls `Engine::load(dir, options)`.
- **D2 (engine options):** `EngineOptions { strict_encoding: true, head_length: 512, .. }`. `512` comes from `inference-policy.json` (the Rust default is `256`); `strict_encoding: true` is also policy-mandated. `max_length` defaults to the checkpoint's `max_position_embeddings` (policy: 8192) [S2][S4].

## 3. Compatible input and output fields

**Input (Jev contract [S5] vs `julia1::typed` [S2][S4]):** `state` (string | object | array; Julia-1 accepts str/dict/list and JSON-renders non-strings) and `questions: map<string, {type, instructions, criteria}>`. Gaps:

- `instructions`: Jev allows string | object | array; Julia-1's `validate_row` requires a **string** question. **D3:** v1 accepts string instructions only; structured instructions are rejected with a clear error (no partial-compatibility claim).
- Question ids are caller-side only; not sent to the model [S5].

**Output mapping (Jev answer → decide-rs):**

| Jev field | Julia-1 native | decide-rs v1 |
| --- | --- | --- |
| `type` | `type` | passthrough |
| `choice` / `noul` | `choice` / `noul` | passthrough |
| `score` | `score` (expected level index) | passthrough |
| `probabilities` | raw softmax map | passthrough (see D6) |
| `confidence` (choice, score) | **absent** (only `max_probability`) | computed client-side (D5) |
| `legend` (score) | **absent** | built from `criteria` |
| `max_probability` | present (choice, score) | omitted in Jev mode; exposed only via `--raw-julia` |
| `model` | absent | emit `julia-1` — never claim `jev-*` |
| `usage` | absent | omitted in v1 (would require inventing token counts) |

**D4 (schema):** v1 emits the Jev answer schema exactly as above. Noul answers carry no `probabilities` and no `confidence`, matching Jev [S5][S7].

## 4. Limits: Julia-1 vs Jev contract

| Constraint | Julia-1 (model/library) | Jev contract | decide-rs v1 |
| --- | --- | --- | --- |
| Choice options | **2–20** per call (`validate_row`); `Router` allows up to 4096 via hierarchical reranking, but final probabilities are **conditional on candidates**, never global [S2][S4] | ≤ 255 options, global probabilities [S5] | enforce **2–20**; larger sets rejected. Hierarchical mode (future) must be documented as partial compatibility (probability scope differs) |
| Score levels | 2–20 | ≥ 2; API accepts up to 10 [S5] | enforce **2–10** (Jev contract wins) |
| Noul | exactly 2 ordered `[false, true]`; optional `criteria` must map exactly `false`+`true` | `criteria` optional `{true, false}` | match Jev; validate per Julia-1 |
| Option encoding | ≤ 48 tokens per option, question head within head budget, no MASK marker (strict) [S4] | not published | enforce via `strict_encoding: true`; surface Julia-1 errors |
| State/context | ≤ 8192 tokens minus head; strict mode errors on overflow (non-strict truncates) [S2][S4] | not published | strict mode: overflow is an error |
| Questions per request | not published | not published | no artificial cap; documented as unknown |

## 5. Confidence semantics

Julia-1 does not compute confidence: `typed.py` and `typed.rs` return `max_probability` for choice/score only [S2][S4]. Jev returns `confidence` for Choice and Score, "derived from the answer's probability distribution", and none for Noul [S5][S6]. Provenance-verified formulas (two independent official sources, algebraically equivalent):

- **Choice:** normalize `p`; `uniform = 1/n`; `confidence = (max(p) − uniform) / (1 − uniform)`, clamped to [0,1]; `n = 1 → 1.0`. Source: `confidence_metrics.py::choice_confidence` [S8]; equivalent form `(n·peak − 1)/(n − 1)` in the official Confidence page's interactive code [S6].
- **Score:** normalize `p`; `mode = argmax(p)`; `distance = Σ pᵢ·|i − mode|`; `even = Σ|i − (n−1)/2| / n`; `confidence = max(0, 1 − distance/even)`. Same two sources [S6][S8].

**D5:** decide-rs computes `confidence` as deterministic post-processing of the probability vector using exactly these formulas. It is a derived presentation value, **not** a model property. No calibration claim is made for Julia-1: `inference-policy.json` sets `calibration: null` and the model card makes no calibration guarantee [S4].

**D6 (pending review):** compute confidence from the **raw softmax** vector. Julia-1 also ships a display rule (`display_probabilities`: winner > 0.95 with all others < 0.045 → hard one-hot; else floor at 0.01 and renormalize) [S4]. Jev's published examples show display-like zeros [S7], but Jev's internal order of operations is not public. v1 emits raw probabilities by default and offers display-rounded output behind a flag; this difference is documented as semantic (not schema) partial compatibility.

## 6. Limitations and pending decisions

1. **Live endpoint contract unverifiable from this region** (451 here, 405-on-GET elsewhere). Reconstructed from official docs, quickstart examples, and the published adapter. Pending: one live POST with an API key to confirm error bodies and `usage` semantics.
2. **Jev confidence internals** (raw vs displayed probabilities) are not public → D6 stays pending review.
3. **Maximum questions per request:** unpublished on both sides; no cap asserted.
4. **Base model license** (`jhu-clsp/mmBERT-small`): verify at packaging time before shipping weights.
5. **`julia1` 0.1.2 is brand new** (12 downloads at check time), single maintainer. Mitigation: pin 0.1.2 in `Cargo.lock`, keep parity tests against the HF reference implementation (`tests/test_typed_api.py`, `scripts/reproduce_typed.py` are available in the model repo) [S4].
6. **CUDA** stays out of v1 (Linux x86-64 CPU only); the `cuda` feature is evaluated later.
7. **Error mapping:** the CLI needs a Jev-analog error shape for validation failures (the API's 422 body details the offending field [S5]); exact CLI error JSON is a pending design decision.
8. **Benchmarks** (typed decisions 73.15% vs Jev reference 72.70%, measured 2026-09-24 on H200 BF16 [S4]) are SupersonicLabs' published numbers with supplied Jev comparison references — cite, never restate as our own measurements.

## 7. Pi / MCP integration (pi-decide)

Target host: **Pi 1.1.0** (installed locally; verified 2026-10-10 UTC), running under Gentle Shell.

- **`pi.registerMcpServer(name: string, config: McpServerConfig): void` exists in 1.1.0** (`ExtensionAPI`); `unregisterMcpServer(name)` removes it. Stdio config: `{ type?: "stdio", command, args?, env?, cwd? }` plus `exposure?`, `description?`, `toolExposure?`, `enabled?`, `timeout?` [S10].
- **Registration before the first model call: supported.** Servers registered while the extension factory runs connect when the session starts (the async factory is awaited before startup continues); later registrations connect immediately. Registrations are session-scoped and not persisted — register on every load [S10].
- **`before_agent_start`** exposes the prompt and `systemPromptOptions`; prefer returning appended prompt sections/guidelines (transcript delta) over replacing the whole system prompt. This is the pi-decide instructions channel: the agent decides when to evaluate alternatives [S10].
- **No interception hooks required.** Pi's `tool_call` event can mutate or block tool calls, but pi-decide will not register it — the agent decides when to call the MCP tools and what to do with results (plan non-goal) [S10].
- **MCP server SDK:** `rmcp` **3.5.1** (Apache-2.0, spec 2026-07-28); server functionality is a default feature; server-side stdio transport via `transport-io` (`rmcp::transport::io::stdio()`) [S11]. Pin the exact version.
- **Gentle Shell** (`Gentleman-Programming/gentle-shell`): public, MIT, TypeScript, active. pi-decide integrates only through the documented Pi extension API; no Gentle Shell internals are required [S12].
- **Plan stop-condition:** not triggered — Pi 1.1.0 supports session-start MCP registration from an extension, so the integration design proceeds to review as planned.

## 8. Sources

- [S1] https://crates.io/api/v1/crates/julia1 and `.../0.1.2/dependencies` (all deps permissive: anyhow, clap, gemm, half, libm, lru, memmap2, rayon, serde, serde_json, tokenizers, ureq; optional cudarc; no ONNX Runtime/Python)
- [S2] https://docs.rs/julia1/latest/julia1/ + sources `typed.rs`, `engine.rs`, `router.rs`, `download.rs`
- [S3] https://github.com/apiplant/julia1-rs (crate repository)
- [S4] https://huggingface.co/SupersonicLabs/Julia-1 @ `a85b127…`: `README.md`, `inference-policy.json`, `julia_config.json`, `config.json`, `provenance.json`, `julia/typed.py`, `julia/data.py`, `julia/probabilities.py`
- [S5] https://docs.typesafe.ai/api.md (request/response/limits/errors)
- [S6] https://docs.typesafe.ai/confidence.md
- [S7] https://docs.typesafe.ai/introduction/quickstart.md and https://docs.typesafe.ai/model-jaggedness/jev-1.13.md
- [S8] https://github.com/typesafe-ai/system-one-adapter-python `src/system_one_adapter/_utils/confidence_metrics.py`
- [S9] https://api.typesafe.ai/v1/systemone — 451 from this egress; 405 on GET from an unblocked network (2026-10-10 UTC)
- [S10] Local Pi 1.1.0 install (`/home/andy/.pi/agent/install/releases/1.1.0/node_modules/@earendil-works/pi-coding-agent/`): `docs/extensions.md`, `dist/core/extensions/types.d.ts`, `dist/core/mcp-servers.d.ts`
- [S11] https://docs.rs/rmcp/latest/rmcp/ and https://crates.io/api/v1/crates/rmcp
- [S12] https://github.com/Gentleman-Programming/gentle-shell (repo metadata verified 2026-10-10 UTC)
