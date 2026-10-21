# Feature: Fase 6 — Servidor MCP

- **Estado**: completado
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Implementar un servidor MCP (Model Context Protocol) que permita a clientes externos (como la extensión pi-decide) solicitar evaluaciones de decisión sin recargar el modelo en cada llamada. El servidor reutilizará el núcleo del CLI (motor, esquema, adaptador).
- **Criterio de aceptación de la fase** (se cumple tras finalizar): el servidor MCP está implementado, probado y documentado; se puede iniciar y responder a solicitudes básicas de elección, noul y score.
- **Fecha de finalización**: 2025-01-17
- **Binario**: `decide-rs-mcp` (~10.3 MB release)
- **Tests**: 37 tests passing (incluyendo 5 unit tests nuevos para MCP)
- **Transport**: stdio (JSON-RPC 2.0)
- **Métodos MCP**: `decide/choice`, `decide/noul`, `decide/score`

## Tareas

- [x] 1. Crear documento de tarea ODD para Fase 6: Servidor MCP.
- [x] 2. Diseñar API del servidor MCP (endpointes, métodos, JSON-RPC).
- [x] 3. Implementar servidor MCP reutilizando el núcleo del CLI (motor, esquema, adaptador).
- [x] 4. Añadir binario del servidor MCP (ej. `decide-rs-mcp`).
- [x] 5. Escribir pruebas unitarias para el servidor MCP.
- [x] 6. Actualizar documentación (README, AGENTS.md si es necesario).
- [x] 7. Prueba de integración básica (cliente simple que habla con el servidor) - verificado que el binario arranca y reporta error apropiado sin modelo.
- [x] 8. Cierre de fase: evidencia en documento de tarea y reporte.

## Notas de diseño

- El servidor MCP seguirá la especificación básica de MCP: transporte por stdio (para comenzar) y posiblemente por TCP en el futuro.
- Utilizaremos un enfoque sencillo: el servidor leerá solicitudes JSON-RPC de stdin y escribirá respuestas a stdout.
- Los métodos que soportaremos inicialmente:
  - `decide/choice`
  - `decide/noul`
  - `decide/score`
- Cada método tomará los mismos parámetros que la CLI: estado (string) y pregunta (objeto JSON con type, instructions, criteria, etc.).
- El servidor no recargará el modelo: lo cargará una vez al inicio y lo reutilizará.
- Debemos manejar errores y devolver respuestas JSON-RPC válidas.
- Usaremos la versión 3.5.1 del crate `rmcp` (Apache-2.0) como SDK para el servidor MCP.
- El servidor tendrá un modo de ejecución único (por ahora) que procesa una solicitud a la vez desde stdin/stdout.
- Para futuras extensiones, podemos considerar múltiples workers o transporte TCP, pero por ahora nos enfocamos en stdio para simplicidad y compatibilidad con la extensión Pi.
- El servidor MCP se ejecutará como un binario separado (`decide-rs-mcp`) que se puede invocar desde la línea de comandos o mediante la extensión Pi.
- El servidor MCP no tendrá estado entre solicitudes (cada solicitud es independiente) excepto por el motor cargado que se mantiene en memoria.
- Definiremos una estructura de estado del servidor que contenga el motor y opcionalmente configuración.
- El servidor MCP se detendrá cuando reciba una señal de interrupción (Ctrl+C) o cuando se cierre stdin.

## Próximos pasos

Tras completar esta fase, el siguiente paso según el plan sería la **Fase 7 — Extensión pi-decide** (límite: 350 líneas modificadas). Se creará una extensión de Pi que registre el servidor MCP y añada instrucciones para que el agente evalúe alternativas antes de plantear preguntas de decisión al usuario.

## Resumen de implementación

### Archivos creados/modificados:

1. **src/bin/mcp_server.rs** (211 líneas) - Implementación principal del servidor MCP
   - Carga el motor Julia-1 una vez al inicio
   - Implementa `ServerHandler` de rmcp
   - Expone 3 tools: `decide/choice`, `decide/noul`, `decide/score`
   - Transporte stdio con JSON-RPC 2.0
   - Manejo de errores con respuestas JSON-RPC válidas

2. **tests/mcp_server_unit.rs** (184 líneas) - Tests unitarios
   - Test de estructura de schema de tools
   - Test de serialización JSON de respuestas (choice, noul, score)
   - Tests de integración ignorados (requieren modelo real)

3. **Cargo.toml** - Añadida dependencia `tokio` y binario `decide-rs-mcp`

4. **README.md** - Documentación de uso del servidor MCP

### Verificación:
- `cargo build --release`: compila exitosamente
- `cargo test`: 37 tests pasan, 6 ignorados (requieren modelo)
- Binario `decide-rs-mcp` arranca y reporta error claro si no hay modelo
- Responde a `tools/list` correctamente cuando el modelo está presente

---
**Nota**: Este documento se actualizó al completar la fase.
## Log (e2e con pesos reales, 2025 — post descarga)

- L6.1: modelo instalado en carpeta gitignored del proyecto `julia-checkpoint/`
  (verificación opuesta e>=550 MB); enlace en `~/.local/share/decide-rs/julia-checkpoint`.
- L6.2: sha256 de `model.safetensors` verificado contra `WEIGHTS_SHA256`
  (`df853bf...b72`): coincide exactamente; descarga con `HF_HUB_DISABLE_XET=1 hf download` (xet fallaba con CAS error).
- L6.3: tests de integración con pesos reales pasan 6/6 con
  `DECIDE_RS_MODEL_PATH=julia-checkpoint cargo test --release -- --ignored`.
- L6.4: e2e CLI correcto: `choice: platform 87.0%`, `noul true 99.5%`, `score ≈ 1.99 ("Annoyed")`.
- L6.5: e2e servidor MCP (stdio JSON-RPC): initialize 2025-11-25 OK, tools/list OK,
  tools/call de choice/noul/score OK. Logro clave: `serve_server()` resuelve al completar
  `initialize`; se necesitaba `service.waiting().await` para no terminar el proceso.
- L6.6: resolución del modelo (`decide_rs::resolve_model_path`):
  `DECIDE_RS_MODEL_PATH` → `~/.local/share/decide-rs/julia-checkpoint` (XDG) → `./julia-checkpoint`.
  `decide-rs fetch` sin argumento instala en la ubicación estándar. Verificado e2e desde cwd ajeno.
