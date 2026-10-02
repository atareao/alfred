# Spec Delta: db/repos

## REMOVED Requirements

### Requirement: search_by_vector SHALL warn on embedding dimension mismatch

## ADDED Requirements

### Requirement: search_by_vector SHALL resolver la búsqueda KNN dentro de SQLite con vec0

`MemoryRepo::search_by_vector` SHALL resolver la búsqueda dentro de SQLite con `MATCH … AND k = MEMORY_KNN_CANDIDATES ORDER BY distance` sobre la tabla virtual `vec_memory`, haciendo JOIN con `memory` para recuperar la ficha (`memory m JOIN vec_memory v ON m.id = v.id`). SHALL NOT cargar todas las filas ni parsear el embedding como JSON ni calcular el coseno en Rust. `memory` SHALL seguir siendo la fuente de verdad.

**Given** una tabla virtual `vec_memory` y una consulta con su embedding  
**When** se ejecuta `search_by_vector`  
**Then** SHALL usar `MATCH … AND k = MEMORY_KNN_CANDIDATES ORDER BY distance`  
**And** SHALL recuperar los campos de `memory` mediante el JOIN por id  
**And** SHALL NOT parsear el embedding almacenado como JSON  
**And** SHALL NOT calcular el coseno en Rust

#### Scenario: La búsqueda se resuelve en SQLite
**Given** filas en `memory` y sus vectores en `vec_memory`  
**When** se ejecuta `search_by_vector`  
**Then** las candidatas vuelven ordenadas por `distance` ascendente  
**And** cada resultado incluye el `id`, `content` y `tokens_count` de `memory` junto con su `distance`

#### Scenario: La deriva de dimensión es imposible
**Given** `vec_memory USING vec0(id TEXT PRIMARY KEY, embedding float[1024] distance_metric=cosine)`  
**When** se intenta almacenar un vector de 1536 dims  
**Then** la operación falla por el esquema `vec0`  
**And** no queda ningún embedding inbuscable

### Requirement: Los mandos de memoria SHALL vivir en settings y leerse en cada consulta

Los cuatro mandos de la memoria episódica SHALL vivir en la tabla `settings` (como los prompts) y SHALL leerse en cada consulta, de modo que cambiarlos surta efecto sin reiniciar. Los defaults iniciales SHALL ser `MEMORY_HALF_LIFE_DAYS = 90`, `SIMILARITY_THRESHOLD = 0.5` (provisional), `RAG_BUDGET_TOKENS = 800` y `MEMORY_KNN_CANDIDATES = 20`. `MEMORY_KNN_CANDIDATES` SHALL acotar el número de candidatas de la KNN, no lo que entra al prompt (eso lo determina `RAG_BUDGET_TOKENS`), y SHALL ser mayor que lo que el presupuesto admite para dar margen al reordenado por antigüedad.

| clave | qué hace | inicial |
|---|---|---|
| `MEMORY_HALF_LIFE_DAYS` | cuánto pesa la antigüedad | 90 |
| `SIMILARITY_THRESHOLD` | qué se descarta por irrelevante | 0,5 (provisional) |
| `RAG_BUDGET_TOKENS` | cuánto ocupa la memoria en el prompt | 800 |
| `MEMORY_KNN_CANDIDATES` | cuántas candidatas se traen antes de filtrar | 20 |

**Given** una base de datos migrada  
**When** se leen los mandos de memoria  
**Then** `settings` SHALL contener `MEMORY_HALF_LIFE_DAYS`, `SIMILARITY_THRESHOLD`, `RAG_BUDGET_TOKENS` y `MEMORY_KNN_CANDIDATES` con sus valores iniciales  
**And** SHALL respetarse cualquier valor no vacío ya existente  
**And** los mandos SHALL leerse en cada consulta, no solo al arrancar

#### Scenario: Los cuatro mandos se siembran con sus defaults
**Given** una base de datos recién migrada  
**When** se consulta `settings`  
**Then** `MEMORY_HALF_LIFE_DAYS = 90`, `SIMILARITY_THRESHOLD = 0.5`, `RAG_BUDGET_TOKENS = 800` y `MEMORY_KNN_CANDIDATES = 20`

#### Scenario: Un cambio en caliente surte efecto sin reiniciar
**Given** una consulta que devuelve resultado con `SIMILARITY_THRESHOLD = 0.5`  
**When** se actualiza `settings.SIMILARITY_THRESHOLD` a un valor más alto y se repite la consulta sin reiniciar  
**Then** el resultado refleja el nuevo umbral

#### Scenario: La personalización existente se respeta
**Given** `settings.RAG_BUDGET_TOKENS = 1200` antes de migrar  
**When** se ejecuta la migración  
**Then** `RAG_BUDGET_TOKENS` sigue siendo `1200`
