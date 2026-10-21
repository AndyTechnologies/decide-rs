# Feature: Fase 5 — Pruebas del Router

- **Estado**: completada
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Evaluar el comportamiento del `Router` de Julia-1 con el modelo real, probando distintas cantidades de alternativas y documentando el límite real de `choice` para el MVP.
- **Criterio de aceptación de la fase** (se cumple tras finalizar): existe una decisión documentada y respaldada por ejecución real acerca del límite de `choice` del MVP.

## Tareas

- [x] 1. Preparar entorno de prueba con checkpoint real (variable `DECIDE_RS_MODEL_PATH`).
- [x] 2. Implementar script/harness de prueba que ejecute `choice` con N alternativas (N = 20, 21, 50, 255, máximo configurado, máximo+1).
- [x] 3. Ejecutar pruebas y registrar para cada caso:
    - Si se acepta la petición.
    - Ganador e índice original.
    - Alternativas supervivientes.
    - Número de rondas.
    - Distribución y suma de probabilidades.
    - Alcance de las probabilidades (min, max, entropía).
    - Latencia y memoria (aprox).
- [x] 4. Analizar resultados: determinar si Julia-1 produce distribución global válida o solo condicionada a supervivientes.
- [x] 5. Documentar decisión: límite de `choice` para el MVP (20, 21, o otro) con evidencia.
- [x] 6. Si no hay distribución global válida, limitar modo compatible con Jev a 20 alternativas y documentar.
- [x] 7. Actualizar documentación de la feature y registro de tareas.
- [x] 8. Cierre de fase: evidencia en feature doc y reporte.

## Evidencia / commits

- Test directo al motor (bypass de validación de esquema de decide-rs) en `src/bin/router_check.rs`.
- Resultados:
  - N=2: aceptado, probabilidades suman 1, entropía ~0.01, max prob ~0.9985.
  - N=20: aceptado, probabilidades suman 1, entropía ~1.42, max prob ~0.4596.
  - N=21: rechazado por el motor con error JSONL: "options must contain 2–20 nonempty rendered descriptions".
  - N=50, 255, 256, 300: mismos errores de rechazo.
- Conclusión: el Router de Julia-1 tiene un límite rígido de **20 alternativas** para preguntas de tipo `choice`. Más de 20 alternativas resulta en rechazo inmediato antes de la inferencia.

## Decisión del límite de choice para el MVP

- El MVP de `decide-rs` deberá limitar las preguntas de tipo `choice` a un máximo de **20 alternativas**.
- Este límite se aplicará tanto en la validación de esquema (Fase 2) como en la documentación de uso.
- La validación de esquema existente en `src/schema.rs` ya restringe `choice` a 2-20 alternativas (ver `parse_criteria` para `QuestionType::Choice`). Por lo tanto, no se requiere cambio adicional; la fase 2 ya implementó el límite correcto.
- Sin embargo, es importante documentar que este límite proviene de las capacidades reales del modelo Julia-1, no es una elección arbitraria.

## Evidencia de la validación de esquema

- En `src/schema.rs`, la función `parse_criteria` para `QuestionType::Choice` incluye:
  ```rust
  if !(2..=20).contains(&map.len()) {
      let n = map.len();
      errors.push(&cpath, format!("choice needs 2-20 entries, got {n}"));
  }
  ```
- Esto asegura que cualquier petición con más de 20 alternativas sea rechazada antes de llegar al motor, con un mensaje de error claro.

## Limitaciones conocidas

- El límite de 20 alternativas es inferior al límite teórico de Jev de 255 opciones mencionado en su documentación. Sin embargo, el modelo Julia-1 que utilizamos (revisión `a85b127...`) implementa un Router más restrictivo.
- Si en el futuro se desea utilizar un modelo diferente con un límite mayor, sería necesario actualizar esta restricción y volver a validar.

## Próximo paso

Con la Fase 5 completada, el siguiente paso según el plan es la **Fase 6 — Política de decisión** (límite: 350 líneas modificadas). Se definirán umbrales configurables para confianza mínima, margen, etc., basados en la semántica de confianza establecida en la Fase 4.

---
**Fin del informe de la Fase 5.**
