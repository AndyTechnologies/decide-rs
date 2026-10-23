# Feature: Fase 8 — refinamiento y operación

- **Estado**: en progreso
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido). Cuatro sub-PRs independientes, uno por sub-fase.
- **Objetivo**: cerrar las decisiones abiertas del PR #7 y preparar operación: MCP refinado, transporte no-stdio, CLI enriquecida y distribución.
- **Criterio de aceptación de la fase** (se cumple tras finalizar): las cuatro sub-fases están implementadas, probadas y documentadas, cada una en su PR.

## Sub-fases (orden de implementación)

S1. **8a — MCP refinado** (`feat/mcp-structured-output`): MERGEADA (PR #10).
S2. **8c — Transporte TCP** (`feat/mcp-tcp-transport`): implementado y PARACADO por decisión del usuario; no se mergea para el MVP (revisión Astra-6.1 lo ubica después del MVP).
S3. **8d — Contrato de herramientas + CLI enriquecida** (dos PRs):
    S3a `feat/mcp-contract-tooltypes`: renombrar tools (sin `/`) + especialización por tipo de pregunta + serialización canónica de `state` (3 tipos) + corrección de instrucciones de extensión (clasificador ≠ razonamiento del agente).
    S3b `feat/cli-enriched`: subcomandos, entrada JSON por archivo/stdin, `--model`, `--help`, códigos de salida, JSONL.
S4. **8e — Endurecimiento del núcleo** (`feat/adapter-typed-errors`): error tipado en adapter (sin panic/zip truncado), `answers_to_json` sin instancia (tests sin pesos siempre aserciones), reparación de descargas corruptas.
S5. **8f — CI** (`feat/ci-workflow`): fmt/clippy/test + build extensión TS en cada PR.
S6. **8b — Distribución** (`feat/distribution-packaging`): posterior al MVP; incluye definir licencia.

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

- L8.2 (S3a, 2026-10-23): decisiones del usuario tras la revisión de Astra-6.1:
  1) renombrar tools de MCP quitando `/` (decide_choice/noul/score) Y especializar
     cada tool a su tipo de pregunta (rechazo por-path de mezclas);
  2) contrato de `state`: mantener los 3 tipos con serialización canónica
     (JSON compacto con claves recursivamente ordenadas) definida en el núcleo.
  Verificación por cable: mezcla rechazada en `questions.<id>.type`; nombre viejo
  devuelve unknown tool; objeto de estado canónico entra al motor.
  Corrección adicional de honestidad (AGENTS.md): las instrucciones de Pi ya no
  atribuyen razonamiento a Julia-1 (predicción del clasificador vs interpretación del agente).
