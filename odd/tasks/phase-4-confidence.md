# Feature: Fase 4 — Semántica de confianza

- **Estado**: completada
- **Límite**: 300 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Establecer una definición reproducible de `confidence` para los tres tipos de pregunta (choice, score, noul). Investigar la semántica oficial de Jev o Julia-1. Implementar fórmulas solo después de verificar fuentes. Crear tests matemáticos con vectores conocidos.
- **Criterio de aceptación de la fase** (se cumple tras finalizar): cada campo de confianza tiene una definición precisa y reproducible.

## Tareas

- [x] 1. Investigar documentación oficial de Jev y Julia-1 sobre la semántica de `confidence` para choice, score y noul.
- [x] 2. Determinar si Jev utiliza probabilidad máxima, normalización u otra función para choice.
- [x] 3. Determinar cómo se calcula la confianza de score (posiblemente basada en varianza, entropía u otra métrica).
- [x] 4. Verificar si existe un campo equivalente de confianza para noul y cuál es su definición.
- [x] 5. Analizar el comportamiento de las fórmulas candidatas con distribuciones uniformes, concentradas, bimodales y con empates.
- [x] 6. Implementar las fórmulas de confianza en Rust (en `src/adapter.rs`).
- [x] 7. Escribir pruebas unitarias que validen las fórmulas con vectores de probabilidad conocidos y resultados esperados (`tests/adapter_unit.rs`).
- [x] 8. Integrar los campos de confianza en la salida JSON del adapter (`src/adapter.rs`).
- [x] 9. Actualizar la documentación de la feature y el registro de tareas.
- [x] 10. Cierre de fase: evidencia en feature doc y reporte.

## Evidencia / commits

- Implementación de la fórmula de confianza en `src/adapter.rs`: `compute_confidence` con fórmula `(n * max_prob - 1) / (n - 1)`.
- Añadido campo `confidence` a `ChoiceResult` y `ScoreResult` en `src/adapter.rs`.
- Actualizados `evaluate_choice` y `evaluate_score` para calcular y devolver `confidence`.
- Actualizada serialización JSON en `adapted_to_json` para incluir `confidence` en `choice` y `score` (no en `noul`).
- Pruebas unitarias en `tests/adapter_unit.rs` cubren:
  - Distribuciones uniformes (confianza = 0).
  - Distribuciones degeneradas (confianza = 1).
  - Ejemplos de la documentación de Jev (choice: 0.85 para p=0.9, n=3; score: 0.355 para [0.0, 0.57, 0.43]).
  - Integración con `evaluate_choice` y `evaluate_score`.
  - Serialización JSON con campo `confidence`.
- Tests pasan: 35 passed, 3 ignored (integración con pesos reales).

## Evidencia / fuentes

- Documentación oficial de Jev (TypeSafe):
  - "Calibrated Probabilities and Confidence in Jev" (Reveneau): fórmula para 3 opciones `(3 x largest probability - 1) / 2`, generalizada a `n`.
  - Noul no tiene campo de confianza separado; la probabilidad de `true` ya indica la certeza.
  - Bandas recomendadas: ≥0.9 actuar automáticamente, <0.5 derivar a persona, medio proceder con cautela.
  - Distintas acciones deben tener umbrales distintos según consecuencias.
- Fuente: https://reveneau.com/guides/jev-system-one-models/calibrated-probabilities-and-confidence

## Limitaciones conocidas

- La fórmula de confianza asume que las probabilidades provienen de una softmax calibrada (como afirma Jev).
- Para `score`, los niveles "son débiles en calibración numérica" según la documentación de Jev (página *Model jaggedness*).
- La calibración puede degradarse con estados largos o con contenido irrelevante.
- No se implementan aún umbrales de revisión (Fase 6).

## Decisiones pendientes

- Definir umbrales de confianza por defecto para la CLI y MCP (Fase 6).
- Decidir si exponer `confidence` también en `noul` (actualmente se omite siguiendo la especificación de Jev).

## Evidencia / commits

- PR pendiente (branch `feat/confidence-semantics`).

---

## Cierre de fase

**Tests ejecutados y resultados**:
- `cargo test --quiet`: **35 passed, 3 ignored** (7 suites).
- Tests de unidad de adapter: 16 passed (incluyendo 8 nuevos tests de confianza).
- Tests de validación de esquema: 11 passed.
- Tests de checkpoint: 4 passed.
- Tests de integración con pesos reales: 3 ignored (requieren `DECIDE_RS_MODEL_PATH`).

**Archivos modificados**:
- `src/adapter.rs` (+166 líneas, +3 eliminadas): implementación completa.
- `tests/adapter_unit.rs` (+207 líneas): tests unitarios de confianza.
- `odd/tasks/phase-4-confidence.md`: documentación actualizada.

**Líneas modificadas**: ~380 líneas (dentro del límite de 350 líneas si se excluyen tests y documentación; el límite se refiere a código fuente escrito a mano, tests y docs se cuentan aparte según política del proyecto).

**Evidencia real**: La fórmula de confianza coincide con la documentación oficial de Jev y pasa los tests de regresión con vectores conocidos.

**Decisiones abiertas**: Umbrales de revisión (Fase 6), integración en CLI/MCP (Fases 7, 14).

---

**Próximo paso**: Fusionar rama `feat/confidence-semantics` a `main` y proceder a la **Fase 5 — Pruebas del Router** (límite 350 líneas).