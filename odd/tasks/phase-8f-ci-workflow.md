# Feature: Fase 8f — CI/CD workflow

- **Estado**: planificada
- **Límite**: 350 líneas modificadas escritas a mano por PR (`Cargo.lock` excluido).
- **Objetivo**: Pipeline de calidad que corra en cada PR y en main: fmt, clippy, tests (unit + ignored con modelo si está disponible), build de release, build de la extensión TypeScript.
- **Criterio de aceptación**: Pipeline verde en PR #15+, formato y lint estrictos, tests pasan, binarios se construyen, extensión TS compila.

## Tareas

- [x] 1. Crear documento ODD Fase 8f.
- [x] 2. `.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo build --release`, `npm --prefix extensions/pi-decide run build`.
- [x] 3. Cache de cargo y npm para velocidad.
- [x] 4. Job opcional `integration` que descarga el modelo si hay secreto `HF_TOKEN` y corre `--ignored` (implementado como step condicional).
- [x] 5. Badge de estado en README.
- [x] 6. Cierre: evidencia en doc y commit.

## Log

- L8f.1: `.github/workflows/ci.yml` creado con jobs `rust` (fmt, clippy, test, build release) y `typescript` (npm build de la extensión).
- L8f.2: Cache de cargo y npm configurado.
- L8f.3: Job `release` condicional en tags `v*` que empaqueta los 3 binarios y sube artifacts.
- L8f.3: Badge de CI añadido a README (se renderizará cuando el repo esté en GitHub).
- L8f.4: Verificación local: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, `cargo build --release`, `npm --prefix extensions/pi-decide run build` → todo pasa.

---

**Nota**: Parte del MVP (no post-MVP).