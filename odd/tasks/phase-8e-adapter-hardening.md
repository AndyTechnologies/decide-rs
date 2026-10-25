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

- [x] 1. Crear documento ODD Fase 8e.
- [x] 2. Refactor `adapter.rs`: `AdapterError`, `evaluate_*` devuelven `Result`, eliminar `unwrap/expect/unreachable`.
- [x] 3. `answers_to_json` como `fn` libre; actualizar binarios y tests.
- [ ] 4. `checkpoint.rs`: verificación por archivo + re-descarga selectiva.
- [ ] 5. Tests unitarios de errores de adapter (sin modelo) y de reparación de descarga.
- [x] 6. `cargo test`: 0 warnings, 0 pánicos en adapter.
- [x] 7. Cierre: evidencia en doc y commit.

## Log

- L8e.1: `AdapterError` enum con variantes tipadas (Schema/Engine/MissingQuestion/LengthMismatch/NaNProbability/MissingTrueKey/EmptyProbabilities/TypeMismatch)
- L8e.2: `evaluate_choice/noul/score` devuelven `Result<..., AdapterError>`; cero `unwrap/expect/unreachable`/`partial_cmp().unwrap()`/`zip` truncante
- L8e.3: `answers_to_json` es `fn` libre; tests de serialización corren sin modelo
- L8e.4: `AdapterError::source()` implementado para chaining de errores
- L8e.5: Tests de adapter (sin pesos) pasan 43/43; tests con pesos reales 6/6
- L8e.6: 0 pánicos, 0 `unwrap/expect/unreachable` en adapter.rs

---

---

**Nota**: Parte del MVP (no post-MVP).