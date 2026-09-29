# Changelog
## [0.8.1] - 2026-09-29

### Documentation

- *(openspec)* Archive fix-tools-used-persistence and cleanup-clippy-dead-deps

### Miscellaneous Tasks

- Fix clippy lints and remove unused dev-dependency
## [0.8.0] - 2026-09-29

### Dependencies

- Upgrade axum 0.8, sqlx 0.9 and adapt breaking changes

### Documentation

- Update READMEs and openspec spec for dependency upgrade
## [0.7.0] - 2026-09-29

### Bug Fixes

- *(calendar)* Make duration optional in get_events with default 24h
- Hora local en vez de UTC y ubicación con reverse geocode inline en orchestrator
- Guardar lat/lon inline antes de lanzar orchestrator, reverse geocode en background
- Stats table headers visibles en dark mode + refresco al abrir modal
- DarkAlgorithm + Table/Header tokens para dark mode correcto
- Stats recording con modelo real, coste, cached_tokens y reasoning_tokens desde OpenRouter
- No emitir StreamEvent::Done hasta que el SSE chunk incluya usage
- Nombres de campo correctos en usage de OpenRouter
- Persist tools_used to DB and fix ToolDef serialization for OpenRouter
- Persist tools_used to DB and fix ToolDef serialization for OpenRouter
- Rewrite agent prompt with expanded personality, rules and examples
- Rewrite agent prompt with expanded personality, rules and examples
- Merge agent prompt rewrite from main

### Documentation

- Update OpenRouter spec and remove archived change proposal
- Rewrite README in English, add Spanish version and .env.example

### Features

- *(openrouter)* Add app identification headers to API calls
- *(stats)* Implement stats dashboard with LLM usage, DB sizes, and retention
- *(ui)* Add Stats button to header bar
- *(memory)* Implement episodic memory system
- Implement stats recording, time/location tools, and worker fixes
- Message timestamp, location and date separators
- Add LastApiCall model, endpoint and stats recording in workers
- Add LastApiCallCard UI and Última llamada tab to dashboard
- Add LastApiCall observability endpoint and UI

### Miscellaneous Tasks

- Rename project from Alfred to Valet
- Rename project from Alfred to Valet

### Refactor

- Reverse_geocode inline con cache, eliminar tokio::spawn

### Styling

- Ubicación completa en segunda línea, eliminar extractCity
- StatsDashboard organizado en pestañas (Resumen/Modelos/Sistema)
## [0.6.0] - 2026-09-26

### Bug Fixes

- *(deps)* Update vite to 8.3.1 and vitest to 5.0.2 to fix 7 Dependabot vulnerabilities

### Features

- Consolidate all pending changes
- Consolidate agenda, calendar UI, and orchestrator fixes
- *(tasks)* Implement GTD task management with Kanban and List views (#13)

### Miscellaneous Tasks

- Remove tsbuildinfo from tracking
## [0.5.0] - 2026-09-25

### Documentation

- Update CHANGELOG for v0.5.0

### Features

- Configurable message page size via settings
- Configurable message page size via settings
- Connect real CollapseWorker with configurable model
- Connect real CollapseWorker with configurable model
- Consolidate all pending changes

### Miscellaneous Tasks

- Bump version to 0.5.0
