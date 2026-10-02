# Design: Inyección automática de memoria episódica

## Context

Ver `proposal.md` — Why. Estado actual relevante y verificado:

- `ContextBuilder::build(strategy, profile_id, user_message)` devuelve `BuiltContext { system_prompt, messages, token_estimate, rag_memories, session_summary }`. Solo la rama `ContextStrategy::RAG` llama a `build_rag_memories()`; `Historical` y `SlidingWindow` devuelven `rag_memories: vec![]` fijo.
- `agent.rs` inyecta cada memoria como mensaje `system` con el literal `format!("[Memory context] {}", memory)` (líneas 343 y 742, `process_message` y `process_message_stream`).
- `format_memory()` produce `"[{tags}] {content}"`. El worker guarda `metadata = {"source","primary_message_ids","date_context"}`; nunca `tags`. En producción: 7 fichas, 0 con `tags`.
- `MemoryRepo::search_by_vector(pool, query_embedding, limit, budget_tokens)` carga todas las filas de `vec_memory`, parsea el JSON, calcula coseno en Rust, ordena descendente, corta en `limit` (10) y filtra por `budget_tokens` con `continue`. **No hay umbral de similitud.**
- `vec_memory(id TEXT PRIMARY KEY, embedding TEXT)` guarda el vector como array JSON. No hay tabla virtual `vec0` ni extensión cargada. `sqlite-vec` está en `Cargo.toml` como dependencia **opcional** con feature `vec0` que nadie activa.
- `Config.embedding_dimension` (`EMBEDDING_DIMENSION`, opcional) ya existe; `search_by_vector` compara cada embedding contra la dimensión de la consulta y descarta los que no coinciden con `tracing::warn!`.
- `session_summary` es `None` en las tres ramas: el bloque `[Session summary]` es código muerto.

## Spike verificado (evidencia empírica)

Se ejecutó un crate desechable (ya borrado) que enlazaba `sqlite-vec 0.1.9` como dependencia directa y registraba la extensión sobre `libsqlite3-sys 0.37`. Salida literal:

```
[1] sqlite3_auto_extension called on libsqlite3-sys 0.37
[3] SELECT vec_version() -> v0.1.9          ← desde una conexión del pool de sqlx
[6] KNN (k=2): id=aaa distance=0.141421377658844 ; id=bbb distance=1.2727922201156616
[7] JOIN memory m JOIN vec_items v ON m.id = v.id  → devuelve id, content, tokens_count, distance
[8] fresh connection SELECT vec_version() -> v0.1.9
```

Hechos confirmados por el spike:

- **`sqlite3_auto_extension` sobre `libsqlite3-sys 0.37` declarado como dependencia directa afecta a las conexiones abiertas por `sqlx`.** El registro se aplica a cada conexión al abrirse, así que el pool queda cubierto sin tocarlo (probado con `vec_version()` desde el pool y desde una conexión nueva).
- **`sqlite-vec 0.1.9` no depende de `libsqlite3-sys`**: su `lib.rs` es solo `#[link(name = "sqlite_vec0")] extern "C" { pub fn sqlite3_vec_init(); }` más un build script con `cc` que compila el C y lo enlaza estáticamente. Por eso Cargo unifica sin conflicto de versiones (`cargo tree -i libsqlite3-sys` muestra una única instancia `0.37.0` compartida por el crate del spike y `sqlx-sqlite`).
- `MATCH ... AND k = ... ORDER BY distance` funciona y devuelve los vecinos en orden ascendente de distancia.
- **`vec0` requiere declarar explícitamente la columna de id textual**: `CREATE VIRTUAL TABLE vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[1024])`. Sin esa columna, la tabla solo tiene el `rowid` implícito y no se puede hacer el JOIN por id con `memory`; el error es `table vec_items has no column named id`.
- Con `distance_metric=cosine`, la distancia es `1 - similitud`: idénticos → 0, ortogonales → 1.
- `vec0` almacena los vectores en binario y particiona en chunks de 1024 por defecto (máximo 4096).

## Goals / Non-Goals

**Goals:**

- Que la memoria episódica archivada se consulte e inyecte en **cada** mensaje, no solo con `!doc`.
- Acotar la inyección con cuatro mandos editables en caliente: umbral de similitud, presupuesto de tokens, vida media del decaimiento y número de candidatas KNN.
- Ordenar por relevancia con decaimiento temporal calculado en Rust.
- Aislar estructuralmente el bloque (`<episodic_memory>` + instrucción de que son antecedentes) y anclar temporalmente cada ficha.
- Sustituir el backend vectorial por `vec0`, con la extensión registrada de forma fiable en todas las conexiones del pool y fail-fast si no carga.
- Impedir estructuralmente la deriva de dimensión de embeddings.
- Retirar la estrategia RAG y `!doc`, y el código muerto asociado.

**Non-Goals:**

- No se introduce un índice ANN real (HNSW/IVF) ni un motor vectorial externo: a esta escala la fuerza bruta de `vec0` es la elección correcta. `vec0` **no es ANN**; su búsqueda es un escaneo lineal.
- No se introduce cuantización (int8/binaria).
- No se introduce filtrado por `profile_id`: la memoria sigue siendo global.
- No se toca el pipeline de escritura del `EpisodicMemoryWorker` (lote, prompts, marca `is_indexed`), salvo el formato del vector persistido.
- No hay backfill de los datos actuales: se reconstruyen desde la fuente.
- No hay tarea de calibración: los cuatro mandos son ajustables en caliente, así que los valores provisionales se afinan usándolos.

## Pipeline de recuperación (orden exacto)

```
1. Embeber el mensaje del usuario
2. vec0 KNN: MATCH ... AND k = MEMORY_KNN_CANDIDATES ORDER BY distance
      → candidatas por distancia ascendente
3. similitud = 1 - distance        (con distance_metric=cosine; idéntico→0, ortogonal→1)
4. descartar similitud < SIMILARITY_THRESHOLD
      (conserva un PREFIJO, porque el orden de la KNN ya es el de similitud)
5. final = similitud × exp(-ln2 × días / MEMORY_HALF_LIFE_DAYS)     ← en Rust
6. ordenar por final descendente
7. acumular tokens hasta RAG_BUDGET_TOKENS, con `break`, NO `continue`
```

El paso 7 corrige un bug existente: el filtro con `continue` se salta la ficha que no cabe y sigue buscando hueco, lo que mete una ficha **menos relevante por ser más pequeña**. Con datos reales (presupuesto 800; fichas 347/307/248/245/210/164/138), `continue` da 3 fichas (347+307+138) y `break` da 2 (347+307).

## Decisions

### D1. Backend vectorial: sqlite-vec `vec0` por fuerza bruta

`MemoryRepo::search_by_vector` es el **único** punto de entrada de búsqueda vectorial, lo que hace el cambio de backend reversible a coste cero. Coste medido del escaneo actual (JSON + coseno en Rust): ~0,1 MB por consulta con 7 fichas. Pero el crecimiento medido es **3,49 fichas/día**: 500 fichas (~6 meses) → **5,4 MB** por consulta; 1000 fichas (~1 año) → **10,7 MB** por consulta. Al pasar a inyección por mensaje, la búsqueda deja de ser un caso raro y pasa a ocurrir siempre.

**Corrección de un error previo:** una versión anterior de este documento describía sqlite-vec como «ANN» y le atribuía búsqueda sublineal. **Eso era falso.** La documentación oficial de sqlite-vec dice literalmente:

> *"It is critical to note that this benchmark exclusively tests brute force linear scans and does not evaluate approximate nearest neighbors (ANN) implementations."* — en palabras llanas: sqlite-vec **no es ANN**.

sqlite-vec **es fuerza bruta**; **no es ANN**. El argumento correcto no es que `vec0` sea sublineal, sino que **a esta escala la fuerza bruta es la elección correcta**: un índice ANN solo empieza a compensar a partir de 10^5–10^6 vectores; por debajo, su sobrecarga de construcción y navegación gana a la fuerza bruta. Con 3,49 fichas/día harían falta **más de 50 años** para llegar a ese orden de magnitud. Entre las implementaciones de fuerza bruta, `vec0` es la mejor disponible en este stack: búsqueda en C con SIMD, vectores binarios, `ORDER BY distance LIMIT k` resuelto dentro de SQLite (sin materializar la tabla en Rust) y almacenamiento por chunks.

Escalera de escalado (reencuadrada):

1. **JSON + coseno en Rust** (histórico, ya no se usa). Coste de parseo de ~11 KB por vector y cálculo del coseno en Rust.
2. **`vec0` por fuerza bruta en C** (este change). Vectores binarios, `MATCH`/`k`/`distance` resueltos en SQLite. **Sin ANN.**
3. **Índice ANN real (HNSW/IVF) en otro motor** si algún día hacen falta 10^5+ vectores. No es una simple activación de feature en este stack: exigiría un motor vectorial externo y se aplaza.

Tabla de coste:

| | JSON + coseno en Rust (histórico) | `vec0` fuerza bruta (este change) |
|---|---|---|
| Parseo por consulta | JSON de ~11 KB/vector (12.810 caracteres) | Ninguno (vector binario, lo lee el C de la extensión) |
| Distancia | Coseno calculado en Rust | En C con SIMD, dentro de SQLite |
| Materialización | Todas las filas cargadas en Rust | `ORDER BY distance LIMIT k` resuelto en SQLite |
| I/O a ~1 año (1000 fichas) | ~10,7 MB | Sin materializar la tabla en Rust |

### D2. El bloque se compone en código, no como placeholder en `settings.system_prompt`

`system_prompt` es editable por el usuario desde la pestaña Prompts del `SettingsDialog`. Un placeholder borrado desactivaría la memoria **en silencio** y volvería el comportamiento dependiente de contenido editable. La composición en código da control sobre etiquetas, instrucción y ubicación en el array de mensajes.
- Alternativa descartada: placeholder `{{ EPISODIC_MEMORY }}` en `system_prompt` → frágil ante ediciones de la UI y difícil de testear.

### D3. El umbral se aplica a la similitud, nunca al resultado final

Si el corte se aplicara al `final` (similitud ya multiplicada por el decaimiento), una ficha antigua con similitud alta pero cientos de días de antigüedad caería por debajo del umbral y la memoria antigua se volvería inalcanzable, justo lo contrario de tener memoria. Por eso el orden del pipeline es: primero se descarta por similitud (paso 4), después se aplica el decaimiento solo para **ordenar** (pasos 5–6). La antigüedad decide **quién va primero**, no **quién existe**.

El umbral se expresa sobre la **similitud** (`1 - distance`), no sobre la distancia: `SIMILARITY_THRESHOLD = 0.5` provisional.

### D4. Decaimiento temporal calculado en Rust

`final = similitud × exp(-λ × días)`, con `λ = ln(2) / MEMORY_HALF_LIFE_DAYS`. Se calcula en Rust, no en SQL, porque la SQLite que enlaza `sqlx` **no tiene funciones matemáticas**: verificado, `SELECT exp(-0.7)` falla con `no such function: exp` (y `pow`/`ln` tampoco). `julianday('now') - julianday(created_at)` sí devuelve días, pero el producto se resuelve con `f64::exp()` en Rust, que sí existe.
- Alternativa descartada: registrar/implementar `exp` en SQL → complejidad innecesaria teniendo `f64::exp()` al lado.

### D5. `MEMORY_KNN_CANDIDATES` acota candidatas, no resultado

La KNN trae hasta `MEMORY_KNN_CANDIDATES` (20) fichas ordenadas por distancia ascendente. Este número **no limita lo que entra al prompt** (eso lo hace `RAG_BUDGET_TOKENS`) sino el **margen de maniobra del reordenado por antigüedad**: debe ser mayor que lo que el presupuesto admite, para que el decaimiento pueda reordenar candidatas que el presupuesto sí aceptaría. Como la KNN ordena por distancia ascendente y `similitud = 1 - distancia`, el corte del paso 4 conserva un prefijo: si la primera candidata supera el umbral, hay resultado. `MEMORY_KNN_CANDIDATES` **no puede vaciar el resultado** si existe al menos una ficha por encima del umbral.

### D6. Presupuesto de tokens por defecto 800 y `break`

Ficha media 237 tokens (rango 138–347). Con 800 caben ~3 fichas, un contexto razonable sin desplazar el historial. Hoy el presupuesto es 2000 **sin umbral**, así que nunca actuaba como filtro real. El corte debe ser `break`: una vez que la ficha en curso no cabe, las siguientes (peor ordenadas) tampoco deben colarse por ser más pequeñas.

### D7. Cuatro mandos editables en caliente, sin ritual de calibración

Los cuatro viven en la tabla `settings` (como los prompts) y se leen en cada consulta, así que cambiarlos surte efecto **sin reiniciar**:

| clave | qué hace | inicial |
|---|---|---|
| `MEMORY_HALF_LIFE_DAYS` | cuánto pesa la antigüedad | 90 |
| `SIMILARITY_THRESHOLD` | qué se descarta por irrelevante | 0,5 (provisional) |
| `RAG_BUDGET_TOKENS` | cuánto ocupa la memoria en el prompt | 800 |
| `MEMORY_KNN_CANDIDATES` | cuántas candidatas se traen antes de filtrar | 20 |

Al ser ajustables en caliente, no hay tarea de calibración: los valores provisionales se afinan usándolos.

### D8. `session_summary` y la rama `[tags]` se eliminan

`session_summary` es `None` en las tres ramas y el bloque `[Session summary]` en `agent.rs` nunca se ejecuta. Se retira de `BuiltContext` y de los dos puntos de inyección. La rama `[tags]` de `format_memory` es inalcanzable en la práctica: el worker nunca escribe `tags` (7 fichas, 0 con `tags`), así que el corchete sale siempre vacío `[]`.

### D9. Esquema: `memory` es la fuente de verdad y `vec_memory` es el índice `vec0`

`memory` se conserva como fuente de verdad y `vec_memory` pasa a ser la tabla virtual `vec0`, **manteniendo el JOIN** `memory m JOIN vec_memory v ON m.id = v.id`. Se descarta usar columnas auxiliares (`+content`) o metadata de `vec0` para evitar el JOIN porque duplicarían el contenido y sacarían a `memory` del modelo mental actual. El coste del JOIN es trivial.

- Esquema: `CREATE VIRTUAL TABLE vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[1024] distance_metric=cosine)`. La columna `id` textual **se declara explícitamente**; sin ella la tabla solo expone el `rowid` implícito y el JOIN por id falla (`table vec_items has no column named id`).

### D10. Dimensión declarada en `vec0` y comprobación al arrancar

La migración fija la dimensión en `1024`. Si algún día cambia `EMBEDDING_DIMENSION`, el esquema de la tabla virtual quedaría desalineado. Se **comprueba al arrancar** que la dimensión declarada por `vec_memory` coincide con `EMBEDDING_DIMENSION` y se falla de forma explícita indicando que hay que reconstruir el índice. La deriva de dimensión **desaparece estructuralmente**: `vec0` no admite almacenar un vector de otra dimensión, así que el requisito `search_by_vector SHALL warn on embedding dimension mismatch` deja de poder cumplirse y se retira.
- Alternativa descartada: mantener el conteo de vectores inbuscables → ya no puede haberlos.

### D11. Fail-fast si la extensión `sqlite-vec` no carga

Se prueba `SELECT vec_version()` al arrancar y, si falla, **no se arranca**, con un error explícito que diga qué falta (extensión no registrada / `vec0` no disponible). Un asistente que arranca aparentemente bien pero sin memoria es exactamente la clase de fallo silencioso que este proyecto ha decidido no tolerar.
- Alternativa descartada: arrancar degradado con un `warn` → silencioso; contradice el principio del proyecto.

### D12. Cuantización (int8/binaria) no se añade ahora

A esta escala (decenas de fichas) cuantizar es pagar pérdida de precisión y un re-ranking posterior a cambio de nada: el escaneo en C ya es suficientemente rápido y el ahorro de memoria es irrelevante. Se documenta como posible escalón futuro solo si el volumen lo justifica.

### D13. Reconstrucción desde la fuente, sin backfill

Los datos actuales son de prueba y se pueden perder. Los mensajes son el original y las fichas son derivados, así que es más robusto resetear `messages.is_indexed = 0` y `summary_ref = NULL`, vaciar `memory` y `vec_memory`, y dejar que el `EpisodicMemoryWorker` rearchive todo con el backend nuevo, en lugar de re-embeder contenido ya resumido por un LLM.

## Invariante de tests

**Ninguna aserción de test existente cambia** salvo estas tres excepciones, declaradas a propósito y de forma explícita:

1. Las que fijan el formato `[{tags}] {content}`.
2. Las que asumen que `vec_memory` guarda JSON.
3. Los tests de `!doc` (`test_doc_override`, `test_classify_with_doc`), que desaparecen con la funcionalidad.

Cualquier otro test que se rompa es una regresión, no una actualización.

## Risks / Trade-offs

- **Umbral provisional** → `0.5` es un punto de partida, no un valor calibrado. Mitigación: es editable en caliente y se afina usándolo; no hay una tarea de calibración bloqueante.
- **Coste de escaneo por mensaje** → `vec0` elimina el parseo JSON y el coseno en Rust y resuelve el top-k dentro de SQLite; a la escala actual el coste es despreciable. La escalera `vec0` (sin ANN) → índice ANN real (otro motor) queda documentada para cuando el crecimiento lo justifique.
- **La extensión no carga en runtime** → mitigación: comprobar `vec_version()` al arrancar y abortar con error explícito (D11).
- **Tests que fijan el formato `[{tags}] {content}`** → se actualizan a propósito (excepción 1).
- **Tests que asumen JSON en `vec_memory`** → se actualizan a propósito al vector binario de `vec0` (excepción 2).
- **Tests de `!doc`** → desaparecen con la estrategia RAG (excepción 3).
- **Vector huérfano de 1536 dims** → **desaparece estructuralmente**: `vec0` declara la dimensión en el esquema y no admite un vector de otra dimensión. La reconstrucción desde la fuente lo elimina sin reindexado manual.
- **Cambio de `EMBEDDING_DIMENSION` a futuro** → el esquema de `vec0` no admite `ALTER` de dimensión: exige reconstruir el índice. La comprobación al arrancar lo hace explícito en lugar de silencioso.

## Migration Plan

1. Desplegar el código: activar `vec0`, añadir `libsqlite3-sys`, registrar `sqlite3_auto_extension` al arrancar y migrar `vec_memory` a la tabla virtual `vec0`.
2. **Reconstruir el índice desde la fuente (sin backfill).** Los datos actuales son de prueba y se pueden perder:
   - `UPDATE messages SET is_indexed = 0, summary_ref = NULL;`
   - vaciar `memory` y `vec_memory`;
   - dejar que el `EpisodicMemoryWorker` vuelva a archivar todo con el backend nuevo.
3. Verificar que el conteo final de fichas coincide con el esperado y que `memory` y `vec_memory` están alineados por id.
4. Rollback: revertir el código y restaurar el esquema JSON de `vec_memory` desde una copia. Como no hay backfill, la BD original no se destruye hasta que la reconstrucción se valida.

## Open Questions

<!-- Ninguna: todas las decisiones están cerradas. -->
