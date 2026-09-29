# Tasks

## 1. Validación de specs OpenSpec

- [x] 1.1 Añadir `## Purpose` a `openspec/specs/infra/spec.md`, `openspec/specs/llm/openrouter/spec.md` y `openspec/specs/tools/geo-weather/spec.md`; añadir `## Requirements` a `openspec/specs/frontend-serving/spec.md`. Verificar con `openspec validate <spec> --type spec`.
- [x] 1.2 Añadir SHALL/MUST al cuerpo de los requisitos que solo lo tienen en el header o no lo tienen, y añadir al menos un `#### Scenario:` a los requisitos sin escenarios, en `db/repos`, `events-api`, `frontend`, `orchestrator/agent` y `tools/geo`. No cambiar el significado de los requisitos.
- [x] 1.3 Ejecutar `openspec validate --specs` y confirmar 0 errores (los warnings de RFC 2119 deben desaparecer también).

## 2. Lint del frontend

- [x] 2.1 Añadir `eslint`, `typescript-eslint`, `eslint-plugin-react-hooks` y `eslint-plugin-react-refresh` a `frontend/devDependencies`, crear `frontend/eslint.config.js` (flat config) y ejecutar `npm install`. Verificar que `npx eslint --version` resuelve la versión local.
- [x] 2.2 Ejecutar `cd frontend && npm run lint` y confirmar que termina sin errores (0 errores, 11 warnings preexistentes de `react-hooks` documentados).

## 3. Test flaky de episodic_memory

- [x] 3.1 Aislar el estado global `LAST_LLM_ATTEMPT` en `src/workers/episodic_memory.rs` (reset en el setup de `test_db()`) para que `test_evaluate_persists_memory_embedding_and_updates` no sea flaky.
- [x] 3.2 Ejecutar `cargo test --lib workers::episodic_memory` varias veces (5) y confirmar que no falla de forma intermitente.

## 4. Verificación final

- [x] 4.1 Ejecutar `openspec validate --specs` y confirmar 0 errores.
- [x] 4.2 Ejecutar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`; confirmar todo verde.
- [x] 4.3 Ejecutar `cd frontend && npx tsc --noEmit && npx vitest run && npm run lint`; confirmar todo verde.