# Design: Arreglar validación de specs, lint y test flaky

## Context

Ver `proposal.md` — Why. Tres frentes independientes: formato de specs OpenSpec, configuración de ESLint en el frontend, y aislamiento de un test con estado global.

## Goals / Non-Goals

**Goals:**
- `openspec validate --specs` sin errores ni warnings.
- `npm run lint` ejecutable y verde.
- `test_evaluate_persists_memory_embedding_and_updates` determinista.

**Non-Goals:**
- No cambiar el comportamiento de ningún requisito ni de producción.
- No reescribir la organización de las specs.
- No añadir reglas de lint agresivas que fuercen refactors masivos.

## Decisions

### D1. Normalizar specs sin cambiar semántica
Se añaden `## Purpose`/`## Requirements` donde faltan, y SHALL/MUST + escenarios donde faltan. Se conserva el texto original de cada requisito; solo se inserta la frase normativa y escenarios derivados del contenido existente.
- Alternativa descartada: reescribir specs enteras → riesgo de perder detalle y salirse del alcance.

### D2. ESLint flat config con dependencias locales
Se añade `eslint` + `typescript-eslint` + `eslint-plugin-react-hooks` + `eslint-plugin-react-refresh` a `devDependencies` y se crea `eslint.config.js` (flat config, formato ESLint 9/10). Se usa `typescript-eslint` con reglas recomendadas y `react-hooks` para hooks.
- Alternativa descartada: depender del eslint global → no reproducible y sin plugins.
- Alternativa descartada: migrar a `.eslintrc` legacy → ESLint 10 ya no lo soporta.

### D3. Aislar el estado global del test flaky
`LAST_LLM_ATTEMPT` es un `static AtomicI64` compartido. Se serializan los tests que dependen de él con `#[serial]` (ya está `serial_test` en dev-dependencies) y/o se resetea el estado en el setup del test. Se prefiere `#[serial]` sobre refactor de producción para no tocar la lógica del worker.
- Alternativa descartada: inyectar el reloj/estado por parámetro → cambio de API de producción fuera de alcance.

## Risks / Trade-offs

- **Añadir SHALL/MUST puede alterar el matiz de un requisito** → revisar que la frase normativa refleje el comportamiento ya descrito; no inventar requisitos nuevos.
- **`npm install` requiere red** → si falla, documentar y dejar el `eslint.config.js` listo para cuando haya red.
- **`#[serial]` reduce paralelismo** → solo en los tests afectados; impacto mínimo.
- **Reglas de lint pueden destapar muchos warnings preexistentes** → empezar con reglas recomendadas y, si hay ruido excesivo, degradar a warnings sin bloquear.

## Migration Plan

No aplica (sin cambios de datos ni despliegue).

## Open Questions

Ninguna.