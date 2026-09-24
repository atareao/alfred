# 🧠 Alfred — Life Operating System

Un asistente ejecutivo y de vida personal auto-hospedado para el hogar. Diseñado para parejas o convivientes, Alfred gestiona el tiempo, la productividad, la alimentación, las relaciones y la rutina diaria.

## Características

- **💬 Chat con IA**: Orquestador con ciclo ReAct, memoria de 3 capas (sesión, vectorial, perfil)
- **📅 Agenda y Tareas**: Gestión de eventos, tareas y recordatorios con scope shared/personal
- **🌤️ Clima y Geo**: Clima por coordenadas, geocoding (Nominatim), búsqueda de lugares (Overpass OSM)
- **🍽️ Comidas**: Planificación semanal de menús y lista de la compra
- **🎯 Hábitos**: Seguimiento de rachas diarias/semanales
- **🔍 Búsqueda Unificada**: FTS5 en todas las dimensiones
- **🔒 Privado**: Datos locales en SQLite, auto-hospedado con PocketID
- **🔔 Proactivo**: Briefing matutino, detección de conflictos, preparación de viajes
- **📱 PWA**: Frontend React + Ant Design, responsive

## Stack

| Capa | Tecnología |
|------|-----------|
| Backend | Rust + Axum |
| Frontend | TypeScript + React + Antd + Vite |
| Base de datos | SQLite + FTS5 + sqlite-vec |
| Auth | PocketID (OIDC self-hosted) |
| LLM | OpenRouter / Ollama (fallback) |
| Contenedores | Docker + Docker Compose |

## Requisitos

- Rust 1.82+
- Node.js 22+
- SQLite 3.45+ (con FTS5)
- Docker + Docker Compose (opcional)

## Inicio rápido

```bash
# 1. Clonar
git clone https://github.com/tu-usuario/alfred.git
cd alfred

# 2. Backend
cp .env.example .env
cargo run

# 3. Frontend (otra terminal)
cd frontend
npm install
npm run dev
```

## Configuración

Variables de entorno principales (ver `.env.example`):

| Variable | Descripción | Default |
|----------|-------------|---------|
| `DATABASE_URL` | Ruta a la BD SQLite | `alfred.db` |
| `OPENROUTER_API_KEY` | API key de OpenRouter | — |
| `OPENWEATHER_API_KEY` | API key de OpenWeather | — |
| `AUTH_ENABLED` | Habilitar autenticación | `false` |
| `LOG_LEVEL` | Nivel de log | `info` |
| `BRIEFING_TIME` | Hora del briefing | `08:15` |

## Producción

```bash
# Variables requeridas
export OPENROUTER_API_KEY="sk-..."
export JWT_SECRET="cambiar-en-produccion"

# Levantar
docker compose -f docker-compose.prod.yml up -d
```

## Arquitectura

```
┌─────────────────────────────────────────────────┐
│                   Frontend PWA                   │
│            (React + Antd + Vite)                 │
└─────────────────────┬───────────────────────────┘
                      │ HTTP/SSE
┌─────────────────────▼───────────────────────────┐
│               Rust Server (Axum)                 │
│                                                  │
│  ┌─────────────┐  ┌──────────┐  ┌────────────┐  │
│  │ Orquestador │  │ Memoria  │  │  Tools     │  │
│  │ (ReAct)     │  │ (3 capas)│  │  (8 dim.)  │  │
│  └─────────────┘  └──────────┘  └────────────┘  │
│                                                  │
│  ┌──────────────────────────────────────────┐    │
│  │         SQLite + FTS5 + sqlite-vec       │    │
│  └──────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

## Tests

```bash
# Backend
cargo test
cargo clippy -- -D warnings

# Frontend
cd frontend && npx tsc --noEmit && npx vitest run
```

## Licencia

MIT
