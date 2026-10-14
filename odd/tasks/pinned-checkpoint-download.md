# Feature: Descarga fijada por revisión + verificación sha256 (ADR 0001 D1)

- **Estado**: en curso
- **Branch**: `feat/pinned-checkpoint-download`
- **Límite**: <300 líneas escritas a mano (`Cargo.lock` excluido).
- **Alcance**:
  1. `src/checkpoint.rs`: constantes `HF_REPO`, `REVISION=a85b127…`, `FILES` (5), `WEIGHTS_SHA256=df853bf7…`; `download_url()` fijada por revisión (nunca `resolve/main`); `download(dir)` con `.part` + rename atómico y check de Content-Length; `verify_weights_hash(expected, path)`.
  2. Deps: `ureq` + `sha2` fijadas exactas.
  3. CLI: subcomando `fetch <dir>` (download + verify), manteniendo `predict`.
  4. Unit tests sin red: URL fijada, FILES, verify acepta/rechaza con fixtures en tempdir.
- **Criterio de aceptación**: descarga y verificación deterministas por revisión fijada; unit tests en verde sin red; sin referencias flotantes.
- **Fuera de alcance**: confidence, reshaping Jev, esquema tipado (Fase 2), MCP.

## Tareas

- [x] 1. Delegar implementación a gentle-ai-worker — 247 líneas cambiadas (checkpoint.rs 128, checkpoint_unit.rs 86, main.rs +23, lib.rs +8, Cargo.toml +2); deps fijadas `ureq = "=2.12.1"`, `sha2 = "=0.10.9"`
- [x] 2. Verificación independiente (gentle-ai-verify): 10/10 PASS — fmt, build 0 warnings, 7 passed/0 failed/3 ignored, 247 < 300, sin referencias flotantes, `.part`+rename con Content-Length, hash streaming, CLI `fetch` exit 2/1 consistentes

## Evidencia / commits

- (pendiente)
