# Proposal: Arreglar validación de specs, lint del frontend y test flaky

## Why

Tres problemas preexistentes de salud del proyecto, no relacionados con cambios de comportamiento:

1. **Validación OpenSpec**: `openspec validate --specs` reporta **9 specs con 18 errores** (requisitos sin escenarios, sin SHALL/MUST, o specs sin `## Purpose`/`## Requirements`). Esto bloquea `openspec archive` de futuros changes que toquen esas capacidades.
2. **Lint del frontend**: `npm run lint` (`eslint .`) no es ejecutable. `eslint` no está en `devDependencies` y no existe `eslint.config.*`; el eslint global (v10) exige flat config.
3. **Test flaky**: `test_evaluate_persists_memory_embedding_and_updates` (`src/workers/episodic_memory.rs`) comparte el `static LAST_LLM_ATTEMPT: AtomicI64` global con otros tests; en ejecución paralela el cooldown puede saltarse la llamada al LLM y hacer fallar el test de forma intermitente.

## What Changes

- **Specs**: corregir las 9 specs para que `openspec validate --specs` pase sin errores:
  - Añadir `## Purpose` a `infra`, `llm/openrouter`, `tools/geo-weather`.
  - Añadir `## Requirements` a `frontend-serving`.
  - Añadir SHALL/MUST al cuerpo de los requisitos que solo lo tienen en el header o no lo tienen.
  - Añadir al menos un `#### Scenario:` a los requisitos que carecen de él.
  - No se cambia el significado de ningún requisito; solo se normaliza el formato.
- **Lint**: añadir `eslint`, `typescript-eslint`, `eslint-plugin-react-hooks` y `eslint-plugin-react-refresh` a `frontend/devDependencies`, crear `frontend/eslint.config.js` (flat config) y verificar `npm run lint`.
- **Test flaky**: aislar el estado global `LAST_LLM_ATTEMPT` para que los tests de `episodic_memory` no interfieran entre sí (serializar los tests afectados o resetear el estado en el setup).

## Capabilities

### New Capabilities

<!-- Ninguna: no hay cambios de comportamiento. -->

### Modified Capabilities

<!-- Ninguna: solo se normaliza el formato de specs existentes. -->

## Impact

- **Specs**: `openspec/specs/{db/repos,events-api,frontend,frontend-serving,infra,llm/openrouter,orchestrator/agent,tools/geo,tools/geo-weather}/spec.md`.
- **Frontend**: `frontend/package.json`, `frontend/eslint.config.js` (nuevo), `frontend/package-lock.json`.
- **Backend**: `src/workers/episodic_memory.rs` (solo tests).
- **Sin cambios de comportamiento** en producción.