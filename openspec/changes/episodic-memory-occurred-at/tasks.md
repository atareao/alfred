# Tasks

> **Invariante**: ninguna aserción de test existente puede cambiar salvo estas dos excepciones, declaradas a propósito y de forma explícita:
> 1. Las que fijan el formato de ficha con fecha (`[{created_at}] {content}`).
> 2. Las que miden el decaimiento contra `created_at`.
>
> Cualquier otro test que se rompa es una regresión, no una actualización.

## 1. Caracterización del comportamiento actual (baseline GREEN)

- [ ] 1.1 Ejecutar `cargo test --lib orchestrator::context_builder && cargo test --lib workers::episodic_memory` y registrar el resultado como baseline. Confirmar GREEN antes de tocar nada.
- [ ] 1.2 Añadir tests de caracterización que documenten el comportamiento actual: el decaimiento mide los `días` desde `memory.created_at`, y `format_memory` precede cada ficha con su fecha derivada de `created_at`. Ejecutar `cargo test --lib db::repos::memory && cargo test --lib orchestrator::context_builder` y confirmar GREEN (caracterización, no comportamiento nuevo).
- [ ] 1.3 Documentar en el test de caracterización las dos únicas excepciones a la invariante: la aserción de formato `[{created_at}] {content}` y las que miden el decaimiento contra `created_at`. Ejecutar `cargo test --lib db::repos::memory && cargo test --lib orchestrator::context_builder && cargo test --lib workers::episodic_memory` y confirmar GREEN.

## 2. `first_message_at` y `last_message_at` en el worker (RED → GREEN)

- [ ] 2.1 Escribir el test del happy path: un lote de mensajes de origen con `created_at` de `2026-09-29T17:13:00Z` a `2026-09-30T17:37:00Z` produce una `metadata` con `first_message_at = 2026-09-29T17:13:00Z` y `last_message_at = 2026-09-30T17:37:00Z`. Ejecutar `cargo test --lib workers::episodic_memory` y confirmar RED.
- [ ] 2.2 Implementar en `EpisodicMemoryWorker::persist` (y su construcción de `metadata`) la derivación del `created_at` más antiguo y más reciente a partir de `metadata.primary_message_ids`, escribiendo ambas claves en la `metadata`. Ejecutar `cargo test --lib workers::episodic_memory` y confirmar RED → GREEN.
- [ ] 2.3 Escribir el test del caso borde: al añadir `first_message_at` y `last_message_at` no se pierde ni se renombra nada de la `metadata`; `source`, `primary_message_ids` y `date_context` siguen escribiéndose con los mismos valores. Ejecutar `cargo test --lib workers::episodic_memory` y confirmar GREEN.
- [ ] 2.4 Verificar que no cambia el esquema: `memory` sigue con las columnas `id, content, tokens_count, created_at, metadata` y `metadata` sigue siendo TEXT con JSON. Ejecutar `cargo test --lib db::schema` y confirmar GREEN.

## 3. El decaimiento contra `last_message_at` con fallback a `created_at` (RED → GREEN)

- [ ] 3.1 Escribir el test: dos candidatas con la misma similitud y distinto `metadata.last_message_at` ordenan por `final`, y la del `last_message_at` más reciente va antes. Ejecutar `cargo test --lib db::repos::memory` y confirmar RED.
- [ ] 3.2 Cambiar en `MemoryRepo::search_by_vector` (`src/db/repos/memory.rs`) el cálculo de los `días` del decaimiento para leerlos de `metadata.last_message_at`. Ejecutar `cargo test --lib db::repos::memory` y confirmar RED → GREEN.
- [ ] 3.3 Escribir el test del fallback: una candidata cuya `metadata` no contiene `last_message_at` mide su antigüedad desde `memory.created_at`, sin fallar ni quedar descartada. Ejecutar `cargo test --lib db::repos::memory` y confirmar RED → GREEN.
- [ ] 3.4 Verificar que la fórmula y la regla de orden no excluyente se conservan en `MemoryRepo::search_by_vector`: `final = similitud × exp(-ln2 × días / MEMORY_HALF_LIFE_DAYS)` con `f64::exp()`, orden por `final` descendente, y el umbral sigue aplicándose a la similitud y nunca al resultado final. Ejecutar `cargo test --lib db::repos::memory` y confirmar GREEN. El presupuesto con `break` NO se toca: eso es del constructor y no entra en este cambio.

## 4. Quitar la fecha del prompt (RED → GREEN)

- [ ] 4.1 Escribir el test: una ficha con `created_at = "2026-09-29T10:00:00Z"` y `metadata` con `last_message_at` se formatea **sin ninguna fecha**; el texto es su contenido tal cual. Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar RED.
- [ ] 4.2 Reescribir `format_memory()` para devolver el contenido tal cual, sin prefijo de fecha. Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar RED → GREEN.
- [ ] 4.3 Actualizar la aserción de formato que fijaba `[{created_at}] {content}` (excepción 1 de la invariante). Ejecutar `cargo test --lib orchestrator::context_builder` y confirmar GREEN.
- [ ] 4.4 Verificar en `src/orchestrator/agent.rs` que el bloque inyectado conserva el orden por relevancia final descendente y que ni la composición del bloque ni `format_memory` vuelven a preceder la ficha con una fecha. Ejecutar `cargo test --lib orchestrator::agent` y confirmar GREEN.

## 5. Verificación final

- [ ] 5.1 Ejecutar `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test`; confirmar todo verde.
- [ ] 5.2 Ejecutar `just check-spec` y confirmar que el change proposal está activo y aprobado.
- [ ] 5.3 Ejecutar `openspec validate episodic-memory-occurred-at --strict`; confirmar válido.
- [ ] 5.4 Ejecutar `git status --porcelain`; confirmar que solo aparecen el directorio `openspec/changes/episodic-memory-occurred-at/` y los ficheros de código del change, si ya se ha implementado.
