# Change Proposal: f1-scaffolding

## Intent
Crear el esqueleto completo del proyecto Alfred: backend Rust + Axum, frontend Vite + React + Antd, base de datos SQLite + sqlite-vec, y tooling de desarrollo (justfile, docker-compose dev). El objetivo es que el proyecto compile, se ejecute, tenga un health check funcional, y el frontend renderice.

## Scope

### Incluye
- Inicialización del proyecto Rust con Cargo (workspace opcional, dependencias clave)
- Servidor Axum con rutas base y endpoint `GET /api/health`
- Conexión SQLite con rusqlite y extensión sqlite-vec registrada
- Esquema inicial de base de datos (conversations, messages, profiles, memories, tools + tablas vec0 + FTS5)
- Migración automática al arrancar
- Frontend Vite + React 18 + TypeScript + Antd 5
- Tema Antd oscuro/claro configurable
- Tooling: justfile, rustfmt, ESLint, Prettier
- Dockerización dev (backend + frontend)
- `cargo build` y `npm run build` correctos

### Excluye
- Endpoints REST funcionales (Fase 2)
- Modelos de dominio completos (solo esquema DB)
- Tests de integración (Fase 2)
- Autenticación PocketID (Fase 5a)
- Caché, rate limiting, logging tracing (Fase 6)

## Impacto
- Crea todo el scaffolding inicial del proyecto
- No hay código legacy que modificar (proyecto vacío)
- Establece la estructura final de directorios del proyecto
- Define las dependencias base que evolucionarán en fases posteriores

## Spec Deltas
- `specs/backend/spec.md`: Estructura Rust, Cargo.toml, esquema DB
- `specs/frontend/spec.md`: Proyecto Vite, tema, componente base
- `specs/docker/spec.md`: Dockerfiles dev, docker-compose

## Risk Assessment
- **sqlite-vec puede no compilar**: Mitigación: feature flag con fallback a solo FTS5
- **Versiones de dependencias incompatibles**: Mitigación: pinning en Cargo.lock y package-lock
- **Antd tree-shaking**: Mitigación: importaciones por módulo, no global CSS