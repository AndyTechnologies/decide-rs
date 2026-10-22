# Feature: Fase 8 — refinamiento y operación

- **Estado**: en progreso
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido). Cuatro sub-PRs independientes, uno por sub-fase.
- **Objetivo**: cerrar las decisiones abiertas del PR #7 y preparar operación: MCP refinado, transporte no-stdio, CLI enriquecida y distribución.
- **Criterio de aceptación de la fase** (se cumple tras finalizar): las cuatro sub-fases están implementadas, probadas y documentadas, cada una en su PR.

## Sub-fases (orden de implementación)

S1. **8a — MCP refinado** (`feat/mcp-structured-output`):
    - `structuredContent` en las respuestas de `tools/call` (además del text content).
    - Errores de esquema como `INVALID_PARAMS` con `data.errors` por-path (hoy todo cae en `internal_error`).
    - Lógica del tool extraída a `decide_rs::mcp` (lib), testeable por integration tests sin bins.
S2. **8c — Transporte TCP/streamable-HTTP** (`feat/mcp-tcp-transport`).
S3. **8d — CLI enriquecida** (`feat/cli-enriched`): preguntas desde archivo JSON, salida enriquecida, alias por proyecto.
S4. **8b — Distribución** (`feat/distribution-packaging`): empaquetado `full-offline`/`standard`, extensión para `pi install`.

## Specs

- S1 (usuario, PR #7 decisiones abiertas): "structuredContent: tool results currently return JSON as text content; decide whether to also emit structuredContent."
- S1b (usuario, PR #7): "Error mapping: expose per-path schema errors through JSON-RPC data."
- S2-S4: según decisiones del usuario (todas aprobadas en la elección de fase 8).

## Tareas

- [x] 1. Crear documento de tarea ODD de Fase 8 con split en sub-PRs.
- [x] 2. S1 8a: MCP refinado (structuredContent + errores por-path + helper en lib).
- [ ] 3. S2 8c: transporte TCP/streamable-HTTP.
- [ ] 4. S3 8d: CLI enriquecida.
- [ ] 5. S4 8b: distribución/empaquetado.
- [ ] 6. Cierre de fase: evidencia y PRs.

---
**Nota**: Este documento se actualizará a medida que avance la fase.
## Log

- L8.1 (S1): PR #10 (commit dc56146, issue #9). Verificado en cable:
  `structuredContent` espeja el JSON en llamadas válidas; rechazos OK esquema
  vuelven `-32602` con `data.errors[{path,message}]` (`questions.route.criteria`,
  `arguments.questions`). Tests 41 pass + 6/6 pesos reales.
