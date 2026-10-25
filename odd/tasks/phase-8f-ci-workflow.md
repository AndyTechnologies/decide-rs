# Feature: Fase 8f — CI/CD workflow

- **Estado**: planificada
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Pipeline de calidad que corra en cada PR y en main: fmt, clippy, tests (unit + ignored con modelo si está disponible), build de release, build de la extensión TypeScript.
- **Criterio de aceptación**: Pipeline verde en PR #15+, formato y lint estrictos, tests pasan, binarios se construyen, extensión TS compila.

## Tareas

- [ ] 1. Crear documento ODD Fase 8f.
- [ ] 2. `.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo build --release`, `npm --prefix extensions/pi-decide run build`.
- [ ] 3. Cache de cargo y npm para velocidad.
- [ ] 6. Job opcional `integration` que descarga el modelo si hay secreto `HF_TOKEN` y corre `--ignored`.
- [ ] 7. Badge de estado en README.
- [ ] 8. Cierre: evidencia en doc y commit.

---

**Nota**: Parte del MVP (no post-MVP).