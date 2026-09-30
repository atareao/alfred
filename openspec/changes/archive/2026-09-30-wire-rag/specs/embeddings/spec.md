# Spec Delta: embeddings

## ADDED Requirements

### Requirement: Embedding provider SHALL be configured via environment variables

`Config::from_env()` SHALL leer `EMBEDDING_PROVIDER` (`"ollama"` | `"openrouter"`), `EMBEDDING_MODEL` y `EMBEDDING_DIMENSION` (opcional), exponiéndolos como `embedding_provider: Option<String>`, `embedding_model: Option<String>` y `embedding_dimension: Option<usize>`.

**Given** las variables de entorno `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL`  
**When** se llama `Config::from_env()`  
**Then** `embedding_provider` y `embedding_model` SHALL tomar esos valores  
**And** SHALL ser `None` cuando no estén definidas  
**And** NO SHALL existir un modelo de embedding por defecto hardcodeado

#### Scenario: Provider y modelo configurados
**Given** `EMBEDDING_PROVIDER=openrouter` y `EMBEDDING_MODEL=openai/text-embedding-3-small`  
**When** se llama `Config::from_env()`  
**Then** `embedding_provider` es `Some("openrouter")`  
**And** `embedding_model` es `Some("openai/text-embedding-3-small")`

#### Scenario: Sin configuración de embeddings
**Given** `EMBEDDING_PROVIDER` y `EMBEDDING_MODEL` sin definir  
**When** se llama `Config::from_env()`  
**Then** `embedding_provider` es `None`  
**And** `embedding_model` es `None`

#### Scenario: Dimensión opcional
**Given** `EMBEDDING_DIMENSION=1536`  
**When** se llama `Config::from_env()`  
**Then** `embedding_dimension` es `Some(1536)`

### Requirement: create_provider SHALL return None when embeddings are not configured

La construcción del provider de embeddings SHALL devolver `None` cuando falte `EMBEDDING_PROVIDER` o `EMBEDDING_MODEL`, logueando un warning. Con ambos definidos SHALL devolver el provider correspondiente.

**Given** una `Config` sin `embedding_provider` o sin `embedding_model`  
**When** se construye el provider de embeddings  
**Then** el resultado SHALL ser `None`  
**And** SHALL loguearse un warning indicando que el RAG queda deshabilitado

#### Scenario: Sin configuración devuelve None
**Given** `embedding_provider = None`  
**When** se construye el provider  
**Then** el resultado es `None`  
**And** se loguea un warning

#### Scenario: OpenRouter configurado
**Given** `embedding_provider = Some("openrouter")`, `embedding_model = Some("openai/text-embedding-3-small")` y `OPENROUTER_API_KEY` definida  
**When** se construye el provider  
**Then** el resultado es `Some(provider)` que llama a OpenRouter

#### Scenario: Ollama configurado
**Given** `embedding_provider = Some("ollama")` y `embedding_model = Some("all-minilm")`  
**When** se construye el provider  
**Then** el resultado es `Some(provider)` que llama a Ollama

### Requirement: Embedding dimension SHALL be validated

Cuando `EMBEDDING_DIMENSION` esté definida, el re-indexado SHALL validar que cada embedding generado tenga esa longitud y SHALL fallar con un error si no coincide.

**Given** `EMBEDDING_DIMENSION = 1536`  
**When** el re-indexado genera un embedding de longitud distinta  
**Then** SHALL devolver un error indicando la dimensión esperada y la obtenida

#### Scenario: Dimensión correcta
**Given** `EMBEDDING_DIMENSION = 3` y un provider que devuelve vectores de longitud 3  
**When** se re-indexa  
**Then** la operación termina sin error

#### Scenario: Dimensión incorrecta
**Given** `EMBEDDING_DIMENSION = 1536` y un provider que devuelve vectores de longitud 384  
**When** se re-indexa  
**Then** la operación falla con un error de dimensión

### Requirement: Reindex command SHALL regenerate all memory embeddings

`reindex_all(pool, provider, dimension)` SHALL recorrer todas las filas de `memory`, regenerar su embedding con el provider configurado y hacer upsert en `vec_memory`, devolviendo un informe con el número de filas actualizadas y fallidas.

**Given** N filas en la tabla `memory` y un provider de embeddings configurado  
**When** se ejecuta `reindex_all(pool, provider, dimension)`  
**Then** SHALL regenerar el embedding de cada fila  
**And** SHALL hacer upsert en `vec_memory` (`ON CONFLICT(id) DO UPDATE`)  
**And** SHALL devolver `ReindexReport { total, updated, failed }`

#### Scenario: Re-indexa todas las filas
**Given** 3 filas en `memory` y un provider mock  
**When** se ejecuta `reindex_all`  
**Then** `vec_memory` contiene 3 embeddings regenerados  
**And** el informe indica `updated = 3`

#### Scenario: Idempotente
**Given** un re-indexado ya ejecutado  
**When** se ejecuta de nuevo  
**Then** `vec_memory` sigue conteniendo el mismo número de filas  
**And** los embeddings se sobrescriben sin duplicar

#### Scenario: Fallo en una fila no aborta el resto
**Given** un provider que falla para una fila concreta  
**When** se ejecuta `reindex_all`  
**Then** las demás filas se re-indexan  
**And** el informe indica `failed = 1`
