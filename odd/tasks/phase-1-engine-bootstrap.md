# Feature: Fase 1 — Integración mínima con julia1

- **Estado**: en curso
- **Branch**: `feat/phase-1-julia1-integration`
- **Límite**: 300 líneas modificadas escritas a mano (`Cargo.lock` excluido como generado).
- **Alcance**:
  1. Integrar `julia1 = "=0.1.2"` (fijado) cargando el modelo **solo desde ruta local** (`Engine::load`; prohibido `from_pretrained`/download).
  2. Prototipo que ejecuta en una sola llamada una decisión `choice`, una `noul` y una `score`.
  3. Inspección de probabilidades y estructuras de resultado (JSON a stdout).
  4. Ruta de modelo configurable para tests de integración vía env `DECIDE_RS_MODEL_PATH`.
  5. Tests unitarios sin red y sin pesos; tests separados con pesos reales (`#[ignore]`, gating por env).
- **Criterio de aceptación**: el prototipo compila y ejecuta los tres tipos de decisión en Linux x86-64 sin servidor de inferencia externo.
- **Fuera de alcance**: descarga/checkpoint fijado, confidence (ADR D6 pendiente), reshaping Jev, límites Jev, CLI completa, MCP, extensión pi-decide.

## Tareas

- [x] 1. Delegar implementación a gentle-ai-worker (superficies: Cargo.toml, Cargo.lock, src/lib.rs, src/main.rs, tests/integration_real_weights.rs) — 286 líneas escritas a mano
- [x] 2. Verificación independiente (gentle-ai-verify): 8/8 PASS — fmt limpio, build 0 warnings, `cargo test` 4 passed/3 ignored, 286 < 300, deps solo julia1+serde_json+anyhow, sin download ni `from_pretrained`
- [x] 3. Commit convencional, push y `gh pr create`
- [x] 4. Cierre: evidencia en feature doc y reporte

## Evidencia / commits

- `3957c2f` feat: add minimal julia1 integration with choice, noul, and score (branch `feat/phase-1-julia1-integration`)
- PR: https://github.com/AndyTechnologies/decide-rs/pull/1 (base `main`)
- Verificación gentle-ai-verify: 8/8 PASS (fmt, build, test 4 passed + 3 ignored, 286 < 300 líneas, deps fijadas, sin download, gating `DECIDE_RS_MODEL_PATH`)
- Evidencia con pesos reales: **COMPLETADA 2026-10-11** — checkpoint `a85b127…` descargado a `~/.cache/decide-rs/Julia-1` (577,189,056 bytes), sha256 verificado contra `df853bf7…` ✓; `DECIDE_RS_MODEL_PATH=… cargo test --test integration_real_weights -- --ignored` → **3 passed** en 70.25s (CPU, Linux x86-64)
