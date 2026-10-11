# Feature: Fase 0 — Investigación técnica y decisiones de compatibilidad

- **Estado**: investigación completa (incluye integración Pi/MCP); ADR 0001 propuesto, pendiente de revisión humana
- **Fecha UTC de inicio**: 2026-10-10T22:20:15Z
- **Límite de la fase**: 250 líneas modificadas. Solo documentación; no implementar funcionalidades.
- **Entregable**: `docs/decisions/0001-runtime-and-compatibility.md`
- **Criterio de aceptación**: el documento permite conocer qué versiones utilizar, qué garantías existen y qué limitaciones siguen pendientes. No se implementan fórmulas de confianza o límites de alternativas sin evidencia verificable.

## Tareas

- [x] 1. Verificar estado del repo y disponibilidad de recursos externos (2026-10-10T22:20:15Z; api.typesafe.ai → 451 desde este egress, 405 GET desde red del usuario)
- [x] 2. julia1 0.1.2 (única release, 2026-10-02, Apache-2.0, deps permissivas sin ONNX/Python; Engine/typed/router/download revisados en docs.rs)
- [x] 3. Checkpoint Julia-1 @ a85b127321d580d65176c89ced8273f305745d85 (2026-09-26), Apache-2.0, weights_sha256 df853bf7…, límites 2–20 opciones / 48 tok/opción / 8192 ctx
- [x] 4. Contrato Jev 1.13 reconstruido desde docs oficiales + adaptador Python; fórmulas de confidence verificadas con doble fuente (docs + confidence_metrics.py)
- [x] 5. Licencias: julia1 Apache-2.0, modelo Apache-2.0, base mmBERT-small pendiente de verificación; pesos solo como artefactos de release
- [x] 6. ADR redactado: `docs/decisions/0001-runtime-and-compatibility.md`
- [x] 7. API de extensiones de Pi (Pi 1.1.0 instalado): `pi.registerMcpServer(name, config)` con config stdio `{command, args?, env?, cwd?}` + exposure/description/toolExposure/enabled/timeout; registro en load conecta al iniciar la sesión (antes de la primera llamada al modelo); `before_agent_start` como canal de instrucciones (append de secciones, no reemplazo total); sin hooks de interceptación (no-goal confirmado viable)
- [x] 8. rmcp 3.5.1 (Apache-2.0, spec 2026-07-28, server por defecto, stdio vía `transport-io`) y Gentle Shell (MIT, TypeScript, activo); condición de parada del plan NO activada
- [x] 9. Cierre: reporte al usuario + ofrecer commit/PR

## Evidencia / commits

- Sin commits todavía (archivos sin trackear: AGENTS.md, README.md, docs/, odd/)
- Hallazgos críticos: `Engine::from_pretrained` usa `resolve/main` (flotante) → D1 exige descarga fija + sha256; Pi 1.1.0 permite registrar MCP desde extensión antes de la primera llamada al modelo
- Pendiente de revisión: D6 (probabilidades raw vs display) y decisiones abiertas #1–#8 del ADR
