# Feature: Fase 3 — Adaptador de inferencia y respuesta base

- **Estado**: sin comenzar
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Implementar el adaptador entre la API pública de la CLI y `julia1::Engine`. Cargar una sola instancia del motor por proceso y reutilizarla. Para `choice`: obtener la distribución de probabilidades, identificar la alternativa ganadora, mantener la correspondencia entre los criterios originales y la salida. Para `noul`: recuperar la probabilidad de verdadero. Para `score`: recuperar la distribución por niveles y reproducir la puntuación esperada de la escala según la semántica documentada. No introducir todavía umbrales de revisión.
- **Criterio de aceptación de la fase** (se cumple tras finalizar): los tres tipos producen resultados tipados consistentes con la implementación de referencia de Julia-1.

## Tareas

- [ ] 1. Crear `src/adapter.rs` con el struct `Adapter` que envuelve `Engine` y métodos para evaluar cada tipo de pregunta.
- [ ] 2. Implementar lógica para `choice`: obtener distribución, ganador (argmax), mapear IDs de criterios a índices de probabilidad.
- [ ] 3. Implementar lógica para `noul`: obtener probabilidad de la clave `"true"`.
- [ ] 4. Implementar lógica para `score`: obtener distribución por niveles, calcular puntuación esperada (Σ i·p_i) según la semántica de Julia-1 (niveles comenzando en 1).
- [ ] 5. Asegurar que el adaptador carga el motor una vez y lo reutiliza (patrón singleton o mediante paso explícito desde main).
- [ ] 6. Escribir pruebas unitarias que usen `julia1::Answer` sintetizadas para verificar el mapeo y cálculos sin necesidad de pesos reales.
- [ ] 7. Añadir pruebas de integración con pesos reales (marcadas `#[ignore]`) que validen la consistencia con la implementación de referencia de Julia-1.
- [ ] 8. Actualizar `src/lib.rs` y `src/main.rs` para usar el adaptador en lugar de llamar directamente a `engine.predict_typed`.
- [ ] 9. Actualizar la documentación de la feature y el registro de tareas.
- [ ] 10. Cierre de fase: evidencia en feature doc y reporte.

## Evidencia / commits

- (pendiente)

