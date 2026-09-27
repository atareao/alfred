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

## 🧠 Memoria Episódica

Alfred cuenta con un sistema de memoria episódica de dos capas:

- **Capa A (mensajes en bruto)**: Tabla `messages` con cada interacción usuario↔asistente
- **Capa B (fichas episódicas)**: Tabla `memory` con resúmenes sintéticos generados por un LLM secundario

El worker `EpisodicMemoryWorker` se dispara al insertar un mensaje (o cada 30 min como respaldo):
1. Acumula mensajes sin indexar hasta ~2000 tokens
2. Añade ±2 mensajes de solapamiento para contexto
3. Envía el bloque a un LLM secundario con prompt de archivista
4. Genera una ficha estructurada (FECHA, TEMAS, HECHOS, SÍNTESIS)
5. Almacena la ficha + embedding vectorial en `memory` + `vec_memory`
6. Marca los mensajes como indexados

En el chat, el orquestador usa **RAG**: genera embedding de la consulta del usuario, busca en `vec_memory` por similitud coseno, e inyecta las fichas más relevantes en el contexto (respetando un presupuesto de tokens).

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
| `MEMORY_BATCH_TOKENS` | Tokens acumulados para trigger de ficha episódica | `2000` |
| `MEMORY_INACTIVITY_MINUTES` | Minutos de inactividad para forzar ficha | `30` |
| `MEMORY_OVERLAP` | Mensajes de solapamiento (±) en el bloque | `2` |
| `MEMORY_POLL_INTERVAL_MINUTES` | Intervalo del timer de respaldo | `30` |
| `MEMORY_MODEL` | Modelo LLM para generar fichas | `mistralai/mistral-small` |
| `RAG_BUDGET_TOKENS` | Presupuesto de tokens para RAG en el chat | `2000` |

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

## API — Stats

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/api/stats/memory` | Estadísticas de memoria episódica |

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
