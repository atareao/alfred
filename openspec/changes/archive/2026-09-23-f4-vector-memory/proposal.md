# Change Proposal: f4-vector-memory

## Why
Alfred necesita recordar. No solo búsqueda textual (FTS5 ya existe en el esquema), sino **búsqueda semántica**: entender el significado de lo que el usuario dice para encontrar información relevante aunque no use las mismas palabras. Esto es la base del sistema de memoria de 3 capas (Fase 5a).

## What Changes
Se implementa el pipeline completo de memoria vectorial: generación de embeddings (Ollama local / OpenRouter remoto), indexado automático de mensajes y memorias, búsqueda híbrida (vector + FTS5 con RRF), y endpoints de búsqueda.

## Scope

### Incluye
- Integración de sqlite-vec como extensión de SQLite
- Tablas vectoriales vec0 para mensajes y memorias
- Configuración FTS5 completa (ya parcialmente en schema)
- Proveedor de embeddings abstracto (trait + Ollama + OpenRouter)
- EmbeddingWorker: genera embedding al crear mensajes
- MemoryConsolidator: extrae hechos de conversaciones → memorias semánticas
- Búsqueda híbrida (RRF: Reciprocal Rank Fusion de vector + FTS5)
- Endpoint `GET /api/search?q=&type=message|memory&limit=`
- Tests de búsqueda y embeddings

### Excluye
- Integración con el orquestador (Fase 5a)
- Frontend de búsqueda (MemoryExplorer, Fase 5a)
- Workers asíncronos en background (Fase 6)

## Impacto
- Añade sqlite-vec como dependencia (feature-flag opcional)
- Crea ~8-10 nuevos archivos Rust
- Modifica migrations para incluir tablas vec0
- Los tests existentes deben seguir pasando

## Risk Assessment
- **sqlite-vec puede no compilar en todas las plataformas**: feature flag con fallback a solo FTS5
- **Embeddings lentos localmente**: timeout configurable, modelo mini por defecto (all-MiniLM-L6-v2)
- **OpenRouter tiene coste**: usar Ollama por defecto, OpenRouter como fallback