# Feature: Fase 8e — Endurecimiento del adapter

- **Estado**: planificada
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Eliminar pánicos y comportamientos indefinidos en el adapter; tipar los errores; hacer que `answers_to_json` no requiera instancia; reparar descargas corruptas.
- **Criterio de aceptación**: Cero `unwrap()/expect()/unreachable!` en adapter.rs; errores tipados propagados; tests de serialización pasan sin modelo; descarga reintenta archivos corruptos.

## Specs (basado en revisión Astra-6.1)

- S1. **Adapter sin pánicos**: `evaluate_choice` no usa `probabilities[0].unwrap()`, `partial_cmp().unwrap()`, ni `unreachable!()`. Devuelve `Result` con error tipado.
- S2. **Adapter sin truncamiento silencioso**: `evaluate_score` verifica `levels.len() == probabilities.len()` antes de `zip`.
- S3. **Errores tipados**: `AdapterError` enum cubre `Schema`, `Engine`, `Internal`, `CorruptedWeights`.
- S4. **`answers_to_json` como función libre** (no método de `Adapter`): tests de serialización corren sin checkpoint.
- S5. **Descarga robusta**: `checkpoint::download` verifica hash por archivo; si falla, re-baja solo el corrupto.

## Tareas

- [ ] 1. Crear documento ODD Fase 8e.
- [ ] 2. Refactor `adapter.rs`: `AdapterError`, `evaluate_*` devuelven `Result`, eliminar `unwrap/expect/unreachable`.
- [ ] 3. `answers_to_json` como `fn` libre en `lib.rs`; actualizar binarios y tests.
- [ ] 4. `checkpoint.rs`: verificación por archivo + re-descarga selectiva.
- [ ] 5. Tests unitarios de errores de adapter (sin modelo) y de reparación de descarga.
- [ ] 6. `cargo test`: 0 warnings, 0 pánicos en adapter.
- [ ] 7. Cierre: evidencia en doc y commit.

---

**Nota**: Parte del MVP (no post-MVP).