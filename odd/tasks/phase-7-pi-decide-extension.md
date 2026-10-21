# Feature: Fase 7 — Extensión pi-decide

- **Estado**: completado
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Crear una extensión de Pi (TypeScript) que registre el servidor MCP `decide-rs-mcp` y añada instrucciones para que el agente evalúe alternativas antes de plantear preguntas de decisión al usuario.
- **Criterio de aceptación de la fase**: la extensión está implementada, registra el servidor MCP correctamente, y el agente puede invocar las tools `decide/choice`, `decide/noul`, `decide/score` desde Pi.
- **Fecha de finalización**: 2025-01-17
- **Extension path**: `extensions/pi-decide/`
- **MCP Server name**: `decide-rs`
- **Tools exposed**: `mcp__decide_rs__decide/choice`, `mcp__decide_rs__decide/noul`, `mcp__decide_rs__decide/score`

## Tareas

- [x] 1. Crear documento de tarea ODD para Fase 7: Extensión pi-decide.
- [x] 2. Diseñar estructura de la extensión Pi (package.json, manifest, punto de entrada).
- [x] 3. Implementar registro del servidor MCP en la extensión.
- [x] 4. Añadir instrucciones del sistema para el agente (cuándo usar las tools de decisión).
- [x] 5. Configurar build y empaquetado de la extensión.
- [x] 6. Probar la extensión cargándola en Pi local (extensión carga sin errores, registra MCP server).
- [x] 7. Actualizar documentación (README).
- [x] 8. Cierre de fase: evidencia en documento de tarea y reporte.

## Notas de diseño

- La extensión será un proyecto TypeScript independiente en `extensions/pi-decide/`.
- Usará el API público de extensiones de Pi (MCP server registration + system instructions).
- El servidor MCP se ejecuta como proceso hijo (stdio transport) — la extensión solo lo registra, Pi lo gestiona.
- Las instrucciones del sistema indicarán al agente: "Antes de preguntar al usuario por una decisión entre alternativas, evalúa las opciones con las tools decide/*".
- No intercepta ni modifica llamadas a question-tool de Gentle Shell; el agente decide cuándo invocar las tools MCP.
- Estructura mínima:
  - `package.json` — metadatos, dependencias (@pi/extension-api, etc.)
  - `src/extension.ts` — punto de entrada, registro MCP, system instructions
  - `tsconfig.json` — configuración TypeScript
  - `pi-extension.json` — manifest de la extensión (si aplica)

## Próximos pasos

Tras completar esta fase, el proyecto tendrá: CLI, MCP server, y extensión Pi — stack completo para decisiones estructuradas locales.

## Resumen de implementación

### Archivos creados:

1. **extensions/pi-decide/package.json** — Metadatos y dependencias (@earendil-works/pi-coding-agent, pi-ai)
2. **extensions/pi-decide/tsconfig.json** — Configuración TypeScript (ES2022, NodeNext modules)
3. **extensions/pi-decide/src/extension.ts** (130 líneas) — Punto de entrada de la extensión
   - Registra MCP server `decide-rs` con `pi.registerMcpServer()`
   - Configura transporte stdio hacia `decide-rs-mcp` binary
   - Añade instrucciones del sistema vía `before_agent_start` event
   - Variables de entorno: `DECIDE_RS_MCP_BINARY`, `DECIDE_RS_MODEL_PATH`, `DECIDE_RS_PROJECT_ROOT`

### Verificación:
- `npm run build`: compila exitosamente (TypeScript strict mode)
- `pi --extension ...`: carga sin errores, muestra versión 1.3.0
- `cargo test`: 37 tests passing

---
**Nota**: Este documento se actualizó al completar la fase.
## Log (e2e en Pi con agente real, 2025 post descarga)

- L7.1: extensión carga sin errores en Pi 1.3.0 y registra el servidor MCP:
  `[pi-decide] Registered MCP server "decide-rs" (decide-rs-mcp) with model at ~/.local/share/decide-rs/julia-checkpoint`.
- L7.2: e2e de agente con `freeflow/glm-5.3-flash-free` (-p --no-session): el agente
  llama `mcp__decide_rs__decide/noul` vía MCP y reporta `PROB_TRUE=0.9831178112284814`,
  valor idéntico al obtenido por JSON-RPC directo (determinismo confirmado).
- L7.3: otros modelos gratuitos fallaron por razones del proveedor (404 Nvidia en el
  default de la cuenta, 429 rate limit, `Connection error` en llm-proxy): no son
  defectos de la extensión ni del servidor MCP.
- L7.4: `resolveModelPath()` de la extensión espeja la cadena del binario (env var →
  ubicación XDG instalada).
