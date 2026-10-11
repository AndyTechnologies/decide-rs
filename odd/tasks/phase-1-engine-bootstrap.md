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
- [ ] 3. Commit convencional, push y `gh pr create`
- [ ] 4. Cierre: evidencia en feature doc y reporte

## Evidencia / commits

- (pendiente)
