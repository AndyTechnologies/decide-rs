# Feature: Fase 2 — Esquema tipado de petición

- **Estado**: ambas partes completas en sus ramas; A en PR #3, B apilado sobre A
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Decisión de split** (regla del plan §2.1: dividir antes de implementar): la fase completa estimada en ~470 líneas post-`rustfmt` excedía las 350, así que se divide en dos PRs independientes:
  - **PR A** (`feat/typed-request-schema`): tipos internos + validación de campos obligatorios, tipos, identificadores (qids y ids de choice no vacíos) y forma de `criteria`; gate de validación en `run_decisions` antes de inferencia; state sin stringify; instructions estructuradas rechazadas (ADR D3).
  - **PR B** (`feat/request-schema-alternatives`, apilado sobre A): `Criteria` tipado (enum), alternativas duplicadas, contenido no vacío de descripciones/niveles, límites (choice 2–20 [modelo], score 2–10 [contrato Jev]), claves fijas `false`/`true` de noul con labels por defecto.
- **Criterio de aceptación de la fase** (se cumple tras B): toda petición se valida antes de ejecutar inferencia y se rechaza de forma determinista si incumple el esquema.

## Tareas

- [x] 1. PR A: schema.rs (205 líneas) + tests (130) + lib.rs (+10/-1) = **346 < 350**; 8 tests de esquema + 4 lib en verde, fmt limpio
- [x] 2. PR B (`feat/request-schema-alternatives`): enum `Criteria`, alternativas duplicadas, descripciones/niveles no vacíos, límites (choice 2–20 [modelo], score 2–10 [contrato Jev]), claves fijas `false`/`true` de noul con labels por defecto y paridad `null` con julia1 → 185 líneas cambiadas, 11 tests de esquema en verde
- [x] 3. Cierre de fase: evidencia en feature doc y reporte

## Evidencia / commits

- PR A: `9336e39` (https://github.com/AndyTechnologies/decide-rs/pull/3, base `main`)
- PR B: (commit pendiente en este branch, base `feat/typed-request-schema`)
- Base del split: versión completa de referencia guardada en /tmp/decide-schema-full/
