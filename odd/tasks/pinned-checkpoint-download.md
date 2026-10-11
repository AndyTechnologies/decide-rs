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

- [ ] 1. Delegar implementación a gentle-ai-worker
- [ ] 2. Verificación independiente + PR (objetivo, cambios, tests, limitaciones, decisiones abiertas)

## Evidencia / commits

- (pendiente)
