# Changelog
## [0.5.0] - 2026-09-25

### Features

- Consolidate all pending changes
- Remove multi-conversation complexity (chat único e infinito)
- Real SSE streaming implementation
- CollapseWorker with configurable model
- Web search tool (Brave Search API)
- Google Places tool
- Configurable message page size via settings

### Styling

- UI Polish: scrollbar oscura, tipografía mobile-first, tamaño de fuente ajustable en Ajustes

### Bug Fixes

- Tool call error handling improvements
- Tool max retries with exponential backoff
- Geo timeout fix
- Docker-compose missing BRAVE_SEARCH_API_KEY and GOOGLE_PLACES_API_KEY

### Refactor

- Remove multi-conversation: backend (conversations table, routes, handlers, repos), frontend (sidebar, ephemeral chat)
- Unify message migrations into single initial schema
- Orchestrator agent refactor with improved context building
- Database repos simplification and cleanup
- Seed script restructured

### Miscellaneous Tasks

- Update OpenSpec documentation across all modules
- Add OpenRouter provider spec
- Add geo-weather tool spec
- Add web_search tool spec