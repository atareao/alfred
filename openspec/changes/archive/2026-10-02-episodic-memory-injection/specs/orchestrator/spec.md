# Spec Delta: orchestrator

## MODIFIED Requirements

### Requirement: RAG SHALL NOT inject placeholder memories

`ContextBuilder` SHALL devolver `Vec::new()` cuando falte el pool, falte el provider o falle la búsqueda, logueando un warning. SHALL NOT devolver memorias hardcodeadas.

**Given** un `ContextBuilder` sin pool o sin provider, o una búsqueda vectorial que falla  
**When** se construye el contexto  
**Then** `rag_memories` SHALL ser vacío  
**And** SHALL NOT contener valores hardcodeados como `"memory1"` o `"memory2"`  
**And** SHALL loguearse un warning

#### Scenario: Sin pool devuelve vacío
**Given** un `ContextBuilder` con `pool: None`  
**When** se construye el contexto  
**Then** `rag_memories` es vacío

#### Scenario: Sin provider devuelve vacío
**Given** un `ContextBuilder` con `pool: Some` y `provider: None`  
**When** se construye el contexto  
**Then** `rag_memories` es vacío

#### Scenario: Error de búsqueda devuelve vacío con warning
**Given** un `ContextBuilder` con pool y provider, y una búsqueda que falla  
**When** se construye el contexto  
**Then** `rag_memories` es vacío  
**And** se loguea un warning

#### Scenario: Búsqueda real devuelve memorias formateadas
**Given** un `ContextBuilder` con pool y provider, y una fila en `memory` + `vec_memory` cuya similitud supera el umbral  
**When** se construye el contexto  
**Then** `rag_memories` contiene la ficha con su fecha derivada de `memory.created_at` y su contenido  
**And** SHALL NOT contener el prefijo `[{tags}]`  
**And** SHALL NOT contener corchetes de etiquetas vacíos (`[]`)

## ADDED Requirements

### Requirement: El umbral de parecido SHALL aplicarse a la similitud, nunca al resultado final

El constructor SHALL descartar las fichas cuya **similitud** (`1 - distance` con `distance_metric=cosine`) sea inferior a `SIMILARITY_THRESHOLD`. SHALL NOT aplicar el corte al `final` ya multiplicado por el decaimiento, porque eso hundiría las fichas antiguas por debajo del umbral y volvería inalcanzable la memoria antigua. La antigüedad SHALL decidir el orden, no la pertenencia. Como la KNN de `vec0` ordena por distancia ascendente y `similitud = 1 - distancia`, el corte conserva un prefijo: si la primera candidata supera el umbral hay resultado.

**Given** una búsqueda vectorial con candidatas ordenadas por similitud descendente  
**When** el constructor filtra por `SIMILARITY_THRESHOLD`  
**Then** SHALL conservar solo las fichas cuya similitud sea `>= SIMILARITY_THRESHOLD`  
**And** una ficha antigua con similitud por encima del umbral SHALL seguir presente aunque su `final` sea bajo  
**And** si ninguna candidata supera el umbral, `rag_memories` SHALL ser `vec![]`

#### Scenario: El umbral descarta por similitud baja
**Given** un `SIMILARITY_THRESHOLD = 0.5` y dos candidatas con similitud 0.7 y 0.2  
**When** se construye el contexto  
**Then** `rag_memories` contiene solo la candidata con similitud 0.7

#### Scenario: Una ficha antigua relevante no cae por el decaimiento
**Given** una candidata antigua con similitud 0.9 y muchos días de antigüedad (su `final` es menor que el de otra candidata reciente)  
**When** se construye el contexto  
**Then** la candidata antigua sigue presente porque su similitud supera el umbral  
**And** el decaimiento solo afecta a su posición en el orden

#### Scenario: Sin candidatas por encima del umbral el resultado es vacío
**Given** un `SIMILARITY_THRESHOLD` alto y todas las candidatas con similitud inferior  
**When** se construye el contexto  
**Then** `rag_memories` es `vec![]`

### Requirement: El decaimiento temporal SHALL calcularse en Rust y ordenar los resultados

El constructor SHALL calcular `final = similitud × exp(-λ × días)`, con `λ = ln(2) / MEMORY_HALF_LIFE_DAYS`, **en Rust** (`f64::exp()`), SHALL NOT intentar calcularlo en SQL porque la SQLite que enlaza `sqlx` no expone funciones matemáticas (`exp`, `pow` y `ln` fallan con `no such function`). El constructor SHALL ordenar las fichas por `final` descendente antes de aplicar el presupuesto.

**Given** candidatas que superan el umbral, con distinta antigüedad  
**When** el constructor calcula la relevancia final  
**Then** SHALL calcular `final = similitud × exp(-ln2 × días / MEMORY_HALF_LIFE_DAYS)` con `f64::exp()`  
**And** SHALL ordenar por `final` descendente  
**And** SHALL NOT ejecutar `exp`/`pow`/`ln` en SQL

#### Scenario: Una ficha más reciente adelanta a una más antigua de igual similitud
**Given** dos candidatas con la misma similitud y distinta antigüedad  
**When** se ordenan por `final`  
**Then** la más reciente aparece antes que la más antigua

#### Scenario: El cálculo no usa funciones matemáticas de SQLite
**Given** la base de datos que enlaza `sqlx`  
**When** se comprueba la disponibilidad de `exp` en SQL  
**Then** `SELECT exp(-0.7)` falla con `no such function: exp`  
**And** el decaimiento se resuelve con `f64::exp()` en Rust

### Requirement: El presupuesto de tokens SHALL cortar con `break`

El constructor SHALL acumular `tokens_count` hasta `RAG_BUDGET_TOKENS` y, cuando la siguiente ficha no quepa, SHALL detener la acumulación con `break`, SHALL NOT continuar buscando una ficha posterior que sí quepa. Continuar metería una ficha menos relevante por ser más pequeña.

**Given** candidatas ordenadas por relevancia final descendente y un `RAG_BUDGET_TOKENS`   
**When** se acumulan `tokens_count`  
**Then** SHALL incluir fichas mientras la suma no supere el presupuesto  
**And** al encontrar la primera ficha que no cabe, SHALL detenerse (`break`), sin evaluar las siguientes

#### Scenario: El presupuesto corta y no salta a la siguiente
**Given** `RAG_BUDGET_TOKENS = 800` y fichas ordenadas de 347, 307, 248, 245, 210, 164 y 138 tokens  
**When** se acumulan tokens  
**Then** `rag_memories` contiene 2 fichas (347 + 307)  
**And** SHALL NOT contener la ficha de 138 tokens aunque quepa tras saltarse la de 248

### Requirement: La estrategia de contexto SHALL NOT gobernar la memoria episódica

La recuperación de memoria episódica SHALL ser ortogonal a `ContextStrategy`: las estrategias `SlidingWindow` y `Historical` SHALL inyectar memoria del mismo modo cuando existan fichas que superen el umbral.

**Given** un `ContextBuilder` con pool y provider, y fichas que superan el umbral  
**When** se construye el contexto con cualquier `ContextStrategy`  
**Then** `rag_memories` SHALL contener las mismas fichas en `SlidingWindow` y `Historical`  
**And** la estrategia SHALL NOT habilitar ni deshabilitar la recuperación

#### Scenario: Un mensaje normal sin override recibe memoria si supera el umbral
**Given** un mensaje que `ContextClassifier::classify()` resuelve como `Override::None` / `ContextStrategy::SlidingWindow`  
**And** fichas cuya similitud supera `SIMILARITY_THRESHOLD`  
**When** se construye el contexto  
**Then** `rag_memories` NO SHALL ser vacío

#### Scenario: Misma memoria en `SlidingWindow` y `Historical`
**Given** un `ContextBuilder` con pool, provider y las mismas fichas sobre el umbral  
**When** se construye el contexto con `SlidingWindow` y con `Historical`  
**Then** ambos SHALL devolver las mismas fichas

### Requirement: MEMORY_KNN_CANDIDATES SHALL acotar el número de candidatas

`MEMORY_KNN_CANDIDATES` SHALL acotar el número de candidatas que trae la KNN de `vec0`, no lo que entra al prompt (eso SHALL hacerlo `RAG_BUDGET_TOKENS`). SHALL ser mayor que el número de fichas que el presupuesto admite, para dar margen al reordenado por antigüedad. Que el resultado no quede vacío depende del corte por umbral, no de este mando (véase «El umbral de parecido SHALL aplicarse a la similitud, nunca al resultado final»).

**Given** un `MEMORY_KNN_CANDIDATES` configurado  
**When** se ejecuta la búsqueda KNN  
**Then** SHALL traer como máximo `MEMORY_KNN_CANDIDATES` candidatas ordenadas por distancia ascendente  
**And** el número de fichas que finalmente entran al prompt SHALL depender de `RAG_BUDGET_TOKENS`

#### Scenario: La KNN acota las candidatas
**Given** 50 fichas en `vec_memory` y `MEMORY_KNN_CANDIDATES = 20`  
**When** se ejecuta la búsqueda  
**Then** SHALL evaluarse como máximo 20 candidatas

#### Scenario: Una candidata sobre el umbral impide el vacío
**Given** `MEMORY_KNN_CANDIDATES = 20` y la primera candidata por distancia supera la similitud mínima  
**When** se construye el contexto  
**Then** `rag_memories` NO SHALL ser vacío
