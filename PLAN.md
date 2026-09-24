# Alfred — Implementation Plan

## Visión General

Alfred es un **Life Operating System**: un asistente ejecutivo y de vida personal para el hogar. Diseñado para una **cuenta de hogar con dos perfiles** (pareja, convivientes), Alfred gestiona el tiempo, la productividad, la alimentación, las relaciones y la rutina diaria de las personas que comparten una casa.

Alfred opera con un **orquestador central** que gestiona un ciclo agente (ReAct), dispone de un **sistema de memoria** de 3 capas, ejecuta **herramientas de productividad** (calendario, tareas, email, viajes) de forma autónoma, respeta **guardrails de seguridad** con human-in-the-loop, y ejecuta **tareas proactivas en segundo plano** (briefing matutino, detección de conflictos, preparación de viajes).

**Stack:** Rust + Axum | TypeScript + React + Antd | SQLite + sqlite-vec (híbrido vectorial)

**Problema que resuelve:** Un asistente AI personal que recuerda tus gustos, conoce tus horarios, gestiona tu agenda, redacta correos, planifica viajes y se anticipa a tus necesidades — todo privado (datos locales).

---

## Arquitectura del Sistema

```
                                  ┌───────────────────────────┐
                                  │   INTERFACES / FRONTENDS  │
                                  │   (PWA / Web / Matrix)    │
                                  └─────────────┬─────────────┘
                                                │ HTTP/SSE
                                                ▼
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│                            RUST SERVER (Axum)                                                │
│                                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────────────────────┐   │
│  │  ORQUESTADOR CENTRAL                                                                  │   │
│  │                                                                                       │   │
│  │  ┌─────────────────────────┐  ┌──────────────────────────┐  ┌──────────────────────┐  │   │
│  │  │ GUARDRAILS & PERMISOS   │  │ CONTEXT CLASSIFIER       │  │ ANALIZADOR SEGUNDO   │  │   │
│  │  │ (Human-in-the-loop)     │  │ (Intent Routing)         │  │ PLANO (Reflexión)    │  │   │
│  │  └────────────┬────────────┘  └───────────┬──────────────┘  └──────────┬───────────┘  │   │
│  │               │                           │                            │               │   │
│  │  ┌────────────▼───────────────────────────▼────────────────────────────▼───────────┐  │   │
│  │  │  CONTEXT BUILDER + SLIDING WINDOW                                                │  │   │
│  │  │  (System + Perfil + Resumen Sesión + RAG + Ventana 8-12)                         │  │   │
│  │  └───────────────────────────────────────┬──────────────────────────────────────────┘  │   │
│  │                                          │                                             │   │
│  │  ┌──────────────────────────────────────▼──────────────────────────────────────────┐  │   │
│  │  │  REACT LOOP (max 10 iteraciones)                                                │  │   │
│  │  │  Think → Act (tool calls) → Observe → tool_choice según nivel de autonomía      │  │   │
│  │  └─────────────────────────────────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                              │
│  ┌────────────────────────┐  ┌────────────────────────┐  ┌──────────────────────────────┐   │
│  │  CAPA DE MEMORIA       │  │  MOTOR DE HERRAMIENTAS │  │  TRABAJADORES EN SEGUNDO     │   │
│  │  (SQLite + sqlite-vec) │  │  (APIs externas)       │  │  PLANO (tokio::spawn)        │   │
│  ├────────────────────────┤  ├────────────────────────┤  ├──────────────────────────────┤   │
│  │ • Sesión (Sliding Win) │  │ • Agenda y Tiempo      │  │ • Briefing matutino          │   │
│  │ • Perfil JSONB         │  │ • Productividad/Tareas │  │ • Alertas de agenda          │   │
│  │ • Índice sqlite-vec    │  │ • Comunicaciones/Email │  │ • Detección conflictos       │   │
│  │ • FTS5                 │  │ • Viajes y Ocio        │  │ • Consolidación nocturna     │   │
│  └────────────────────────┘  └────────────────────────┘  └──────────────────────────────┘   │
│                                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────────────────────┐   │
│  │  LLM PROVIDER (OpenRouter / Ollama / Fallback)                                       │   │
│  └──────────────────────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Flujo completo del Orquestador

```
1. Usuario envía mensaje → POST /api/conversations/:id/messages
2. ORQUESTADOR RECIBE EL MENSAJE
3. GUARDRAILS: validar nivel de autonomía de la acción solicitada
   └── Si requiere aprobación explícita → pausar, preguntar al usuario
4. CLASIFICACIÓN DE CONTEXTO:
   ├── Override (!historico, !doc, !reset)? → forzar estrategia
   ├── ¿Cotidiana?  → ESTRATEGIA A (Sliding Window: 8-12 msgs + resumen + RAG)
   ├── ¿Histórico?  → ESTRATEGIA B (consulta masiva SQL)
   └── ¿RAG?        → ESTRATEGIA C (vector search + hilo completo)
5. CONSTRUCCIÓN DE CONTEXTO:
   ├── System Prompt + Perfil JSONB (~300-500t)
   ├── Resumen de Sesión (~150-300t)
   ├── Memoria RAG (sqlite-vec, ~200-400t)
   └── Ventana Deslizante (últimos 8-12 mensajes)
6. CICLO ReAct (max 10 iteraciones):
   ├── Enviar contexto + tools al LLM
   ├── ¿tool_calls? → validar permiso → ejecutar (con nivel autonomía) → repetir
   └── ¿stop? → salir del ciclo
7. ANALIZADOR SEGUNDO PLANO (Reflexión):
   ├── ¿La respuesta es correcta y útil?
   └── ¿Hace falta preguntar algo más al usuario?
8. Persistir mensajes
9. Disparar workers:
   ├── EmbeddingWorker (indexar mensaje)
   ├── MemoryConsolidator (extraer hechos)
   └── SessionWindow (compactar si corresponde)
10. Responder al frontend vía SSE streaming
```

---

## Conceptos Arquitectónicos Clave

### 1. Sistema de Memoria (3 capas)

Alfred organiza su memoria en **3 capas**:

| Capa | Contenido | Implementación |
|------|-----------|----------------|
| **Sesión** | Conversación actual + tarea en curso | Ventana Deslizante (8-12 msgs) + Resumen de Sesión (~150-300t) |
| **Índice (Semántica)** | Conocimiento personal: preferencias, hechos, notas, conversaciones pasadas | `sqlite-vec` + Búsqueda Híbrida (vectorial + FTS5) |
| **Perfil/Reglas** | JSONB de perfil: dieta, horarios, viajes, preferencias | Inyección en System Prompt desde DB + configuración |

#### 1.1 Capas de Contexto — Sliding Window + Summarization

La **Memoria de Sesión** usa una **Ventana Deslizante (Sliding Window)** combinada con **Resumen de Sesión**. Se mantienen 8-12 mensajes recientes intactos y se resume automáticamente lo que queda atrás.

```
[ Contexto Antiguo ] ──> [ LLM los resume en 3 frases ] ──> Pasa a "Resumen de Sesión"
                                                                  │
                                                                  ▼
[ Ventana Deslizante ] ──> Mensajes recientes (8-12)             │
                                                                  │
                                                                  ▼
[ PROMPT FINAL ] System + Perfil + Resumen + RAG + Ventana + Actual
```

**Estructura del prompt final:**

| Elemento | Tamaño | Contenido |
|----------|--------|-----------|
| **System + Perfil** | ~300-500t | Instrucciones + JSON de perfil del usuario |
| **Resumen Sesión** | ~150-300t | Acuerdos/temas de más de 10 mensajes atrás |
| **Memoria RAG** | ~200-400t | 2-3 recuerdos relevantes de sqlite-vec |
| **Ventana** | 8-12 msgs | Historial reciente intacto |
| **Mensaje Actual** | — | Entrada del usuario |

**Compactación:** al llegar al mensaje 11, se resumen los 4 más antiguos con un LLM ligero y se fusionan con el `session_summary` existente. El prompt siempre mantiene tamaño fijo.

**Excepción Tool Calls:** no cortar en medio de un flujo `Tool Request → Tool Result`.

**Archivo:** `src/orchestrator/session_window.rs`

---

### 2. Sistema de Tools — Las 4 Dimensiones del Asistente Personal

Alfred expone herramientas al LLM vía el formato OpenAI tool_calling, organizadas en **dimensiones de la vida cotidiana**. Los datos de agenda y tareas son **propios (SQLite)**, sin depender de APIs externas.

#### Dimensión 1: Agenda (SQLite)

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `get_events({ date?, range? })` | Leer eventos del día o rango | 🔓 |
| `check_availability({ date, duration })` | Buscar huecos libres en el calendario | 🔓 |
| `create_event({ title, start, end, location?, description? })` | Crear evento en agenda | 🔔 |
| `update_event({ id, changes })` | Mover, modificar o cancelar evento | 🔒 |

#### Dimensión 2: Tareas (SQLite)

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `list_tasks({ project?, status?, due_date? })` | Listar tareas pendientes | 🔓 |
| `add_task({ content, due_date?, priority?, project? })` | Añadir tarea | 🔔 |
| `update_task({ id, changes })` | Renombrar, repriorizar, recategorizar | 🔔 |
| `complete_task({ id })` | Marcar tarea completada | 🔔 |

#### Geolocalización — Pilar Transversal

La geolocalización **no es una tool más**, es un pilar que atraviesa todo el sistema:

- **Perfil del usuario:** `home_city`, `home_coordinates`, `work_coordinates`
- **Frontend PWA:** envía posición actual (`navigator.geolocation`) en cada solicitud
- **Búsqueda de lugares:** toda query usa coordenadas + radio
- **Clima:** localización precisa, no solo nombre de ciudad
- **Memoria:** *"cuando estuve en Barcelona"* se asocia a coordenadas

```
Frontend PWA                    Backend
    │                               │
    ├─ getUserPosition() ──────────►│
    │  (navigator.geolocation)      │ ¿Tiene permiso?
    │                               │ ├─ Sí → usar coords actuales
    │                               │ └─ No → usar coords del perfil
    │                               │
    │ "¿Dónde puedo comer?" ───────►│ search_places(query, coords, radius)
    │                               │     └──► OpenStreetMap / Google Places
    │◄─── [Restaurantes cercanos] ──│
```

#### Dimensión 3: Clima + Geolocalización + Lugares

| Tool | Descripción | API | Nivel |
|------|-------------|-----|-------|
| `get_weather({ latitude, longitude, date? })` | Clima en coordenadas precisas | OpenWeather / WeatherAPI | 🔓 |
| `geocode({ query })` | Dirección → { lat, lng } (directa) | Nominatim (OSM, gratis) | 🔓 |
| `reverse_geocode({ latitude, longitude })` | Coordenadas → dirección (inversa) | Nominatim (OSM, gratis) | 🔓 |
| `search_places({ query, latitude, longitude, radius? })` | Qué hay alrededor (restaurantes, sitios, etc.) | Google Places / Overpass (OSM) | 🔓 |

#### Dimensión 4: Comidas y Lista de la Compra

Alfred planifica los **menús semanales** según las restricciones y preferencias del perfil, y genera automáticamente la **lista de la compra** con los ingredientes necesarios, organizada por categorías.

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `plan_week_meals({ preferences?, days? })` | Generar menú semanal según perfil dietético | 🔔 |
| `generate_shopping_list()` | Convertir menú semanal en lista de la compra | 🔔 |
| `add_to_shopping_list({ item, quantity?, category? })` | Añadir item manual a la lista | 🔓 |
| `list_shopping_list({ category? })` | Ver lista de la compra actual | 🔓 |
| `check_off_item({ item })` | Marcar item como comprado | 🔓 |

**Flujo semanal típico:**
```
Domingo: "Alfred, planifica la semana"
    │
    ▼
Alfred consulta perfil: sin lactosa, mediterránea
    │
    ▼
Genera menú: Lun-Vie (comida+cena) + finde (especial)
    │
    ▼
"¿Te parece bien este menú?" → Usuario ajusta → Alfred guarda
    │
    ▼
"Genera la lista de la compra"
    │
    ▼
Lista: Verduras | Lácteos | Despensa | Carnes | Congelados
```

**Integración con perfil:**
```json
"diet_and_health": {
  "restrictions": ["sin_lactosa"],
  "preferences": ["comida_mediterranea"],
  "weekly_budget": 60,
  "meal_plan_days": 5
}
```

**Almacenamiento:** Menú semanal → tabla `meal_plans` (JSON). Lista de la compra → tabla `shopping_list`.

**Archivos:** `src/tools/meals.rs`, `src/db/repos/meal_plans.rs`, `src/db/repos/shopping_list.rs`

#### Dimensión 5: Recordatorios con Alarma

No son tareas — son avisos que **suenan en el momento exacto**. Se integran con Web Push API para notificaciones push al navegador.

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `set_reminder({ text, datetime })` | Crear recordatorio con alarma | 🔔 |
| `list_reminders({ status?, date? })` | Ver recordatorios pendientes | 🔓 |
| `dismiss_reminder({ id })` | Descartar recordatorio | 🔓 |
| `snooze_reminder({ id, minutes })` | Posponer | 🔓 |

**Tabla:** `reminders` (id, text, datetime, status, created_at)

**Archivo:** `src/tools/reminders.rs`

#### Dimensión 6: Conocimiento y Bloc (Unificado)

Notas rápidas, entradas de diario y hechos importantes se guardan en un **mismo sistema** con categorías. Alfred decide internamente si indexa en FTS5, sqlite-vec, o ambos según la categoría.

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `create_note({ content, category?, tags? })` | Guardar nota rápida (`category: "idea"`, `"journal"`, `"fact"`, `"todo"`) | 🔓 |
| `list_notes({ category?, date? })` | Ver notas por categoría o fecha | 🔓 |
| `delete_note({ id })` | Eliminar nota | 🔓 |
| `unified_search({ query, dimensions? })` | **Buscar en TODAS las dimensiones** (notas, eventos, tareas, contactos, mensajes, memorias) vía FTS5 + sqlite-vec | 🔓 |

**Tabla única:** `notes` (id, user_id, content, category, tags, created_at, updated_at). Indexada en FTS5 + sqlite-vec.

**Archivos:** `src/tools/knowledge.rs`, `src/tools/unified_search.rs`

#### Dimensión 7: Contactos y Relaciones

Agenda personal para consultar y gestionar relaciones rápido.

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `search_contacts({ query })` | Buscar contacto por nombre/teléfono | 🔓 |
| `add_contact({ name, phone?, email?, notes? })` | Añadir contacto | 🔓 |
| `update_contact({ id, changes })` | Actualizar contacto | 🔓 |

**Tabla:** `contacts` (id, user_id, name, phone, email, notes, created_at, updated_at).

**Archivo:** `src/tools/contacts.rs`

#### Dimensión 8: Seguimiento de Hábitos

Alfred te ayuda a mantener rutinas. Lleva la cuenta de rachas y frecuencias.

| Tool | Descripción | Nivel |
|------|-------------|-------|
| `create_habit({ name, frequency, target? })` | Crear hábito (diario/semanal) | 🔔 |
| `log_habit({ habit_id })` | Registrar que hiciste el hábito hoy | 🔓 |
| `habit_streaks()` | Ver rachas actuales de todos los hábitos | 🔓 |
| `habit_stats({ habit_id, period? })` | Estadísticas de cumplimiento | 🔓 |

**Tabla:** `habits` (id, user_id, name, frequency, target). `habit_logs` (habit_id, user_id, date, completed).

**Integración con perfil:**
```json
"habits": {
  "targets": {
    "exercise": {"frequency": "daily", "streak_target": 30},
    "reading": {"frequency": "daily", "minutes": 20}
  }
}
```

**Archivo:** `src/tools/habits.rs`

#### Niveles de Autonomía (Human-in-the-loop)

| Nivel | Acción | Ejemplo |
|-------|--------|---------|
| 🔓 Sin confirmación | Lectura / organización interna | Consultar agenda, buscar sitios, geocodificar |
| 🔔 Con notificación | Acciones reversibles | Crear evento, añadir tarea, guardar memoria |
| 🔒 Aprobación explícita | Acciones externas / compromisos | Modificar evento existente |

**Implementación:** Cada tool se define con schema JSON, handler Rust, y `Permission` enum. El orquestador consulta el nivel antes de ejecutar.

**Archivos:** `src/tools/mod.rs`, `src/tools/registry.rs`, `src/tools/trait.rs`, `src/tools/permission.rs`, `src/tools/calendar.rs`, `src/tools/tasks.rs`, `src/tools/weather.rs`, `src/tools/geo.rs`, `src/tools/meals.rs`, `src/tools/reminders.rs`, `src/tools/knowledge.rs`, `src/tools/unified_search.rs`, `src/tools/contacts.rs`, `src/tools/habits.rs`

---

### 3. Perfil del Usuario (JSONB)

El perfil se almacena en la tabla `profiles` y se inyecta en el System Prompt en cada interacción:

```json
{
  "personal": {
    "name": "...",
    "city": "Madrid",
    "language": "es",
    "timezone": "Europe/Madrid",
    "home_coordinates": { "lat": 40.4168, "lng": -3.7038 },
    "work_coordinates": { "lat": 40.4522, "lng": -3.6881 }
  },
  "diet_and_health": {
    "restrictions": ["sin_lactosa"],
    "preferences": ["comida_mediterranea"],
    "favorite_cafes": ["Café de la Plaza"]
  },
  "travel": {
    "preferred_transport": ["tren"],
    "accommodation_style": "boutique_centrico",
    "loyalty_programs": ["Iberia Plus", "Renfe"],
    "home_airport": "MAD"
  },
  "productivity": {
    "working_hours": "09:00-18:00",
    "preferred_break_time": "14:00",
    "morning_briefing_time": "08:15",
    "focus_blocks": []
  },
  "contacts": {
    "emergency": "...",
    "frequent": []
  }
}
```

**Archivo:** `src/models/profile.rs`, `src/db/repos/profiles.rs`

### 3.5 Cuenta de Hogar — Dos Perfiles

Alfred usa **PocketID** como proveedor OIDC self-hosted. Cada miembro del hogar tiene su propio perfil, pero comparten el mismo Alfred.

**Modelo de datos:**
```
cuenta_de_hogar (PocketID)
     ├── user_id_1: Ana
     │     └── profile: { diet, habits, contacts, ... }
     └── user_id_2: Luis
           └── profile: { diet, habits, contacts, ... }
```

**Scope de datos:**
| Dato | Scope | Ejemplo |
|------|-------|---------|
| Eventos | `shared` o `personal` | "Cena en casa" vs "Fisioterapia Luis" |
| Tareas | `shared` o `personal` | "Comprar leche" vs "Llamar al seguro" |
| Menú semanal | `shared` | Lo comen juntos |
| Lista compra | `shared` | Para la casa |
| Notas | `personal` | Cada uno sus ideas |
| Contactos | `personal` | Cada uno sus contactos |
| Hábitos | `personal` | Cada uno sus rutinas |
| Recordatorios | `shared` | "Saca la pizza" |

**PocketID como sidecar en Docker:**
```yaml
services:
  alfred:
    ...
  pocketid:
    image: ghcr.io/pocket-id/pocket-id
    environment:
      - PUBLIC_APP_URL=https://alfred.tudominio.com
    volumes:
      - pocketid_data:/app/data
```

**Flujo:** Browser → PocketID (login) → redirect a Alfred con JWT → Alfred extrae `user_id` → perfiles cargados.

**Archivo:** `src/auth.rs`

---

### 4. Control de Interacción — Clasificación y Routing de Contexto

El orquestador clasifica la intención del mensaje y selecciona la estrategia de contexto antes de llamar al LLM.

#### Estrategia A: Cotidiana (Default) — Sliding Window

| Aspecto | Detalle |
|---------|--------|
| **Cuándo** | *"Añade leche a la compra"*, *"¿A qué hora salía el tren?"*, *"Reserva mesa para cenar"* |
| **Qué carga** | System + Perfil + Resumen Sesión + RAG + Ventana (8-12 msgs) |
| **Resultado** | Prompt de ~1000-1500 tokens. Latencia mínima |

#### Estrategia B: Análisis Histórico

| Aspecto | Detalle |
|---------|--------|
| **Cuándo** | *"¿Qué planes hicimos para el puente?"*, *"Resumen de la semana"* |
| **Qué carga** | Consulta SQL masiva + resumen de sesión largo |
| **Resultado** | Bloque de hasta 150K tokens |

#### Estrategia C: RAG On-Demand

| Aspecto | Detalle |
|---------|--------|
| **Cuándo** | *"¿Qué recomendación de hotel me dio Juan?"* |
| **Qué carga** | Búsqueda vectorial + hilo completo de ese día + ventana reciente |
| **Resultado** | Contexto preciso sin cargar todo el histórico |

#### Override Manual

| Comando | Efecto |
|---------|--------|
| `!historico [pregunta]` | Fuerza Estrategia B |
| `!doc [pregunta]` | Fuerza Estrategia C |
| `!reset` | Limpia ventana de sesión |

**Archivos:** `src/orchestrator/context_classifier.rs`, `src/orchestrator/context_builder.rs`

---

### 5. Guardrails y Niveles de Autonomía

Cada acción pasa por una matriz de seguridad antes de ejecutarse:

```rust
enum Permission {
    /// Lectura/organización — se ejecuta sin preguntar
    NoConfirm,
    /// Acción reversible — se ejecuta y notifica
    Notify,
    /// Acción con compromiso — requiere ok del usuario
    ExplicitApproval,
}

struct GuardrailCheck {
    tool: String,
    permission: Permission,
    reason: String,        // Por qué requiere este nivel
    user_notified: bool,
    user_approved: bool,
}
```

**Flujo:**
1. El LLM solicita ejecutar una tool
2. El orquestador consulta el `Permission` de esa tool
3. Si es `NoConfirm` → ejecuta directamente
4. Si es `Notify` → ejecuta + envía notificación al frontend
5. Si es `ExplicitApproval` → pausa el loop, pregunta al usuario, espera respuesta

**Archivo:** `src/orchestrator/guardrails.rs`

---

### 6. Analizador de Segundo Plano (Reflexión)

Tras el ciclo ReAct, un analizador ligero evalúa la respuesta antes de enviarla al usuario:

```rust
struct Reflection {
    is_coherent: bool,
    is_complete: bool,
    needs_clarification: Option<String>,
    suggested_followup: Option<String>,
}
```

Propósitos:
- Detectar si la respuesta requiere acción del usuario
- Sugerir preguntas de seguimiento
- Verificar que las tools se ejecutaron correctamente

**Archivo:** `src/orchestrator/reflection.rs`

---

### 7. Tareas Proactivas en Segundo Plano (Cron Jobs)

Workers programados que trabajan en segundo plano para anticiparse al usuario:

| Tarea | Horario | Qué hace |
|-------|---------|----------|
| **Briefing Matutino** | 08:15 (configurable) | Lee calendario del día, tareas pendientes, clima → envía resumen |
| **Detección Conflictos** | Cada evento nuevo | Calcula tiempo de desplazamiento entre citas consecutivas; alerta si hay solapamiento |
| **Preparación Viajes** | N días antes del viaje | Compila clima destino, sugiere restaurantes según perfil, propone itinerario |
| **Consolidación Nocturna** | 23:00 | Compacta ventana de sesión, consolida memoria, limpia summaries viejos |

**Implementación:** `tokio::spawn` con `tokio::time::interval` para tareas periódicas, disparadores por eventos para detectores de conflictos.

**Archivos:** `src/workers/mod.rs`, `src/workers/briefing.rs`, `src/workers/conflict_detector.rs`, `src/workers/travel_prep.rs`, `src/workers/memory_worker.rs`

---

### 8. Orquestador y Ciclo ReAct

El orquestador central integra todos los componentes en un flujo secuencial:

```
1. Recibir mensaje del usuario
2. Guardrails: validar permisos de la acción implícita
3. Clasificar intención → seleccionar estrategia de contexto
4. Construir prompt: System + Perfil + Resumen + RAG + Ventana + Msg
5. CICLO ReAct (max 10 iter):
   a. Enviar al LLM con tools disponibles
   b. Si tool_calls:
      - Validar permiso de cada tool (Guardrails)
      - Ejecutar según nivel de autonomía
      - Añadir resultado al contexto
      - Volver a 5a
   c. Si stop → salir
6. Analizador Segundo Plano: reflexión sobre la respuesta
7. Persistir mensajes (user + assistant + tool_calls)
8. Disparar workers asíncronos
9. Responder vía SSE streaming
```

**Archivo:** `src/orchestrator/agent.rs`

---

## Fases de Desarrollo

### Fase 1: Fundación — Scaffolding del proyecto

**Objetivo:** Proyecto Rust compilable, frontend renderizable, SQLite con sqlite-vec operativo.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 1.1 | Inicializar proyecto Rust con Cargo + dependencias | `Cargo.toml`, `src/main.rs`, `src/lib.rs` | `rust-writer` |
| 1.2 | Configurar Axum con rutas base y health check | `src/main.rs`, `src/routes/mod.rs`, `src/routes/health.rs` | `rust-writer` |
| 1.3 | Configurar SQLite + sqlite-vec con rusqlite | `src/db/mod.rs`, `src/db/migrations.rs` | `sqlite-expert` |
| 1.4 | Crear esquema inicial (conversations, messages, profiles, message_embeddings vec0, memories, memory_embeddings vec0, tools) + FTS5 | `src/db/schema.rs` | `sqlite-expert` |
| 1.5 | Inicializar frontend Vite + React + TypeScript + Antd | `frontend/package.json`, `frontend/vite.config.ts` | `frontend-dev` |
| 1.6 | Configurar tema Antd oscuro/claro | `frontend/src/theme.ts`, `frontend/src/App.tsx` | `frontend-dev` |
| 1.7 | Configurar tooling (ESLint, Prettier, justfile, rustfmt) | `justfile`, `.rustfmt.toml`, `frontend/.eslintrc.cjs` | `rust-writer`, `frontend-dev` |
| 1.8 | Dockerizar backend + frontend (dev) | `docker-compose.yml`, `Dockerfile.backend`, `Dockerfile.frontend` | `docker-expert` |
| 1.9 | Probar: `cargo build`, `cargo test`, `npm run build`, `npx tsc` | — | `rust-writer`, `frontend-dev` |

**Criterio de éxito:** `cargo build` y `npm run build` pasan. Health check responde 200. sqlite-vec registrado y funcional.

---

### Fase 2: API Core — Endpoints REST + Modelos de datos

**Objetivo:** API REST funcional con CRUD de conversaciones, mensajes, perfil, tools.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 2.1 | Definir modelos: Conversation, Message, Memory, Profile, Tool | `src/models/mod.rs`, `src/models/*.rs` | `rust-writer` |
| 2.2 | Implementar repositorios DB | `src/db/repos/*.rs` | `sqlite-expert` |
| 2.3 | Crear sistema de errores unificado con `thiserror` | `src/errors.rs` | `rust-writer` |
| 2.4 | Implementar AppState con pool + config | `src/state.rs` | `rust-writer` |
| 2.5 | Endpoints CRUD conversaciones | `src/routes/conversations.rs`, `src/handlers/conversations.rs` | `rust-writer` |
| 2.6 | Endpoints CRUD mensajes | `src/routes/messages.rs`, `src/handlers/messages.rs` | `rust-writer` |
| 2.7 | Endpoint POST /api/conversations/:id/messages (enviar) | `src/handlers/messages.rs` | `rust-writer` |
| 2.8 | Endpoints CRUD perfil de usuario | `src/routes/profile.rs`, `src/handlers/profile.rs` | `rust-writer` |
| 2.9 | Endpoints CRUD memorias | `src/routes/memories.rs`, `src/handlers/memories.rs` | `rust-writer` |
| 2.10| Endpoints CRUD tools/config | `src/routes/tools.rs`, `src/handlers/tools.rs` | `rust-writer` |
| 2.11| Paginación cursor-based | `src/models/pagination.rs` | `rust-writer` |
| 2.12| Tests de integración | `tests/api/*.rs` | `rust-writer` |
| 2.13| `cargo test` + `cargo clippy` | — | `rust-writer` |

**Criterio de éxito:** Todos los endpoints responden correctamente. Tests en verde.

---

### Fase 3: Frontend Chat — Interfaz conversacional

**Objetivo:** UI de chat completa con Antd, conectada a la API real.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 3.1 | Definir tipos TypeScript | `frontend/src/types/index.ts` | `frontend-dev` |
| 3.2 | Implementar API client tipado | `frontend/src/api/client.ts` | `frontend-dev` |
| 3.3 | Hook `useConversations` | `frontend/src/hooks/useConversations.ts` | `frontend-dev` |
| 3.4 | Hook `useMessages` con streaming SSE | `frontend/src/hooks/useMessages.ts` | `frontend-dev` |
| 3.5 | Hook `useSSE` | `frontend/src/hooks/useSSE.ts` | `frontend-dev` |
| 3.6 | Hook `useProfile` | `frontend/src/hooks/useProfile.ts` | `frontend-dev` |
| 3.7 | Hook `useMemories` | `frontend/src/hooks/useMemories.ts` | `frontend-dev` |
| 3.8 | AppLayout + Sidebar + ConversationList | `AppLayout.tsx`, `Sidebar.tsx`, `ConversationList.tsx` | `frontend-dev` |
| 3.9 | ChatView + MessageBubble + MessageInput | `ChatView.tsx`, `MessageBubble.tsx`, `MessageInput.tsx` | `frontend-dev` |
| 3.10| ProfileEditor (editar perfil JSONB) | `ProfileEditor.tsx` | `frontend-dev` |
| 3.11| MemoryExplorer + SearchDialog | `MemoryExplorer.tsx`, `SearchDialog.tsx` | `frontend-dev` |
| 3.12| Tests componentes | `*.test.tsx` | `frontend-dev` |
| 3.13| `npx tsc --noEmit`, `npx vitest run` | — | `frontend-dev` |

**Criterio de éxito:** UI renderiza, chat funcional, perfil editable, búsqueda operativa.

---

### Fase 4: Memoria Vectorial — sqlite-vec + embeddings

**Objetivo:** Búsqueda semántica híbrida sobre mensajes y memorias.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 4.1 | Integrar sqlite-vec (registro extensión Rust) | `Cargo.toml`, `src/db/vector.rs` | `sqlite-expert` |
| 4.2 | Crear tablas virtuales vec0 + migraciones | `src/db/migrations.rs` | `sqlite-expert` |
| 4.3 | Configurar FTS5 para búsqueda textual | `src/db/fts.rs` | `sqlite-expert` |
| 4.4 | Implementar generación de embeddings (Ollama/OpenRouter) | `src/embeddings/mod.rs`, `src/embeddings/provider.rs` | `rust-writer` |
| 4.5 | EmbeddingWorker: generar embedding al crear mensaje | `src/services/embedding_service.rs` | `rust-writer` |
| 4.6 | MemoryConsolidator: extraer hechos → memoria semántica | `src/services/memory_service.rs` | `rust-writer` |
| 4.7 | Búsqueda híbrida (RRF: vector + FTS5) | `src/services/search_service.rs` | `sqlite-expert` |
| 4.8 | Endpoint `GET /api/search?q=&type=message|memory` | `src/routes/search.rs`, `src/handlers/search.rs` | `rust-writer` |
| 4.9 | Tests de búsqueda | `tests/api/search.rs` | `rust-writer` |
| 4.10| `cargo test`, `cargo clippy` | — | `rust-writer` |

**Criterio de éxito:** Búsqueda semántica devuelve resultados relevantes. Embeddings automáticos.

---

### Fase 5a: Núcleo del Orquestador

**Objetivo:** LLM providers, orquestador, guardrails, sliding window, contexto.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 5a.1 | Definir trait `LLMProvider` (chat, chat_stream, embeddings) | `src/llm/mod.rs`, `src/llm/provider.rs` | `rust-writer` |
| 5a.2 | Implementar `OpenRouterProvider` | `src/llm/openrouter.rs` | `rust-writer` |
| 5a.3 | Implementar `OllamaProvider` | `src/llm/ollama.rs` | `rust-writer` |
| 5a.4 | Implementar fallback chain | `src/llm/fallback.rs` | `rust-writer` |
| 5a.5 | Sistema de Tools: trait + registry + permission | `src/tools/mod.rs`, `src/tools/trait.rs`, `src/tools/registry.rs`, `src/tools/permission.rs` | `rust-writer` |
| 5a.6 | Guardrails: permisos + human-in-the-loop | `src/orchestrator/guardrails.rs` | `rust-writer` |
| 5a.7 | Clasificador de Contexto | `src/orchestrator/context_classifier.rs` | `rust-writer` |
| 5a.8 | Constructor de Contexto (3 estrategias) | `src/orchestrator/context_builder.rs` | `rust-writer` |
| 5a.9 | Ventana Deslizante + Resumen de Sesión | `src/orchestrator/session_window.rs` | `rust-writer` |
| 5a.10| Orquestador (ciclo ReAct completo) | `src/orchestrator/mod.rs`, `src/orchestrator/agent.rs` | `rust-writer` |
| 5a.11| Analizador Segundo Plano (Reflexión) | `src/orchestrator/reflection.rs` | `rust-writer` |
| 5a.12| Streaming SSE desde el orquestador | `src/routes/stream.rs`, `src/handlers/stream.rs` | `rust-writer` |
| 5a.13| Endpoint POST /messages orquestado | `src/handlers/messages.rs` | `rust-writer` |
| 5a.14| Auth PocketID + JWT + user_id en handlers | `src/auth.rs` | `rust-writer` |
| 5a.15| Conectar frontend con SSE real | `frontend/src/hooks/useSSE.ts` | `frontend-dev` |
| 5a.16| Tests de integración del orquestador | `tests/api/chat.rs`, `tests/api/guardrails.rs`, `tests/api/context.rs` | `rust-writer` |

**Criterio:** Chat funcional: usuario escribe → Alfred clasifica contexto → responde con streaming.

---

### Fase 5b: Tools Core (Agenda, Tareas, Recordatorios, Notas, Contactos)

**Objetivo:** Tools esenciales del día a día con datos en SQLite.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 5b.1 | Tool: Agenda (CRUD eventos SQLite con scope shared/personal) | `src/tools/calendar.rs` | `rust-writer` |
| 5b.2 | Tool: Tareas (CRUD SQLite con scope) | `src/tools/tasks.rs` | `rust-writer` |
| 5b.3 | Tool: Recordatorios con alarma + Web Push | `src/tools/reminders.rs` | `rust-writer` |
| 5b.4 | Tool: Conocimiento y Bloc (create_note unificado + FTS5 + vec) | `src/tools/knowledge.rs` | `rust-writer` |
| 5b.5 | Tool: Contactos (CRUD SQLite, scope personal) | `src/tools/contacts.rs` | `rust-writer` |
| 5b.6 | Tool: Búsqueda unificada (FTS5 + sqlite-vec en todas las tablas) | `src/tools/unified_search.rs` | `rust-writer` |
| 5b.7 | Tests de todas las tools core | `tests/api/tools/` | `rust-writer` |

**Criterio:** CRUD funcional de agenda, tareas, notas, contactos y recordatorios.

---

### Fase 5c: Tools de Valor (Geo, Clima, Comidas, Hábitos)

**Objetivo:** Tools contextuales que requieren API externa o lógica compleja.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 5c.1 | Tool: Clima con coordenadas precisas | `src/tools/weather.rs` | `rust-writer` |
| 5c.2 | Tool: Geocoding directo e inverso (Nominatim) | `src/tools/geo.rs` | `rust-writer` |
| 5c.3 | Tool: Búsqueda de lugares con radio (OSM/Google) | `src/tools/geo.rs` | `rust-writer` |
| 5c.4 | Tool: Menús semanales y lista de la compra | `src/tools/meals.rs` | `rust-writer` |
| 5c.5 | Tool: Seguimiento de hábitos y rachas | `src/tools/habits.rs` | `rust-writer` |
| 5c.6 | Perfil multi-usuario (modelo + repositorio con scope) | `src/models/profile.rs`, `src/db/repos/profiles.rs` | `rust-writer` |
| 5c.7 | Tests integración tools de valor | `tests/api/tools/` | `rust-writer` |
| 5c.8 | `cargo test`, `cargo clippy` | — | `rust-writer` |

**Criterio:** Todas las tools funcionales con tests pasando. Alfred responde con tools según permisos.

---

### Fase 6: Tareas Proactivas + Pulido + DX

**Objetivo:** Workers proactivos funcionando, proyecto documentado y configurable.

| # | Tarea | Archivos | Subagente |
|---|-------|----------|-----------|
| 6.1 | WorkerPool con canal mpsc | `src/workers/mod.rs`, `src/workers/pool.rs` | `rust-writer` |
| 6.2 | Briefing Matutino (cron diario) | `src/workers/briefing.rs` | `rust-writer` |
| 6.3 | Detección de Conflictos de Agenda | `src/workers/conflict_detector.rs` | `rust-writer` |
| 6.4 | Preparación de Viajes | `src/workers/travel_prep.rs` | `rust-writer` |
| 6.5 | Consolidación Nocturna | `src/workers/memory_worker.rs` | `rust-writer` |
| 6.6 | PerfilEditor en frontend | `frontend/src/components/ProfileEditor.tsx` | `frontend-dev` |
| 6.7 | Logging estructurado con tracing | `src/telemetry.rs` | `rust-writer` |
| 6.8 | Config desde entorno (config.rs + .env) | `src/config.rs`, `.env.example` | `rust-writer` |
| 6.9 | CORS middleware | `src/main.rs` | `rust-writer` |
| 6.10| Seed data para desarrollo | `src/bin/seed.rs` | `sqlite-expert` |
| 6.11| Export de datos: `GET /api/export` (JSON completo del hogar) | `src/routes/export.rs`, `src/handlers/export.rs` | `rust-writer` |
| 6.12| Docker compose producción (multi-stage + PocketID sidecar) | `docker-compose.prod.yml` | `docker-expert` |
| 6.13| README.md completo | `README.md` | — |
| 6.14| Auditoría final: `cargo clippy`, `cargo test`, `npx tsc`, `npx vitest` | — | `rust-writer`, `frontend-dev` |

**Criterio de éxito:** Briefing matutino se envía automáticamente. Conflictos se detectan. `just check-all` pasa.

---

## Hitos y Entregables

| Hito | Fase | Entregable | Criterio |
|------|------|------------|----------|
| **M1: Fundación** | F1 | Backend + frontend compilables, SQLite+vec | `cargo build` + `npm run build` OK |
| **M2: API REST** | F2 | API REST con perfiles multi-usuario | Tests integración pasan |
| **M3: UI conversacional** | F3 | Chat React + perfil + búsqueda | Flujo crear→chatear funciona |
| **M4: Memoria vectorial** | F4 | sqlite-vec + embeddings + búsqueda híbrida | Búsqueda semántica devuelve resultados |
| **M5a: Orquestador** | F5a | LLM + Guardrails + Contexto + Reflexión + PocketID | Chat real con streaming |
| **M5b: Tools Core** | F5b | Agenda, tareas, notas, contactos, recordatorios | CRUD completo con scope shared/personal |
| **M5c: Tools Valor** | F5c | Clima, geo, comidas, hábitos | Tools contextuales funcionales |
| **M6: Proactivo** | F6 | Briefing, conflictos, export, Docker prod, README | `just check-all` pasa |

---

## Riesgos y Mitigaciones

| Riesgo | Impacto | Probabilidad | Mitigación |
|--------|---------|-------------|------------|
| **APIs externas (Google, Todoist) requieren OAuth** | Alto | Alta | Usar OAuth 2.0 con refresh tokens; almacenar tokens cifrados en DB |
| **Rate limits de APIs externas** | Medio | Alta | Cachear respuestas; cola de requests con backoff |
| **Human-in-the-loop rompe UX del chat** | Medio | Media | Notificación asíncrona; el usuario puede responder "sí/no" en el chat |
| **sqlite-vec no compila** | Alto | Media | Fallback a solo FTS5 |
| **Embeddings locales lentos** | Medio | Alta | Embeddings configurables: Ollama local o remoto |
| **Perfil JSONB muy grande** | Bajo | Baja | Cachear en memoria con versionado |

---

## Criterios de Éxito Global

1. **Chat funcional:** Usuario chatea con Alfred, las conversaciones persisten, respuestas streamean
2. **Tools de productividad:** Alfred puede leer agenda, crear tareas, consultar clima, buscar sitios
3. **Guardrails:** Acciones destructivas requieren aprobación explícita; el usuario controla la autonomía
4. **Proactividad:** Briefing matutino, detección de conflictos, preparación de viajes
5. **Perfil persistente:** Alfred conoce preferencias del usuario (dieta, viajes, horarios)
6. **Calidad:** `cargo clippy -- -D warnings`, `cargo test`, `npx tsc --noEmit`, `npx vitest run` pasan

---

## Estructura Final del Proyecto

```
/data/rust/alfred/
├── Cargo.toml
├── justfile
├── .env.example
├── docker-compose.yml
├── Dockerfile.backend
├── Dockerfile.frontend
├── docker-compose.prod.yml
├── README.md
├── PLAN.md
├── AGENTS.md
├── openspec/
│   ├── specs/
│   └── changes/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── errors.rs
│   ├── state.rs
│   ├── telemetry.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── conversation.rs
│   │   ├── message.rs
│   │   ├── memory.rs
│   │   ├── profile.rs
│   │   ├── tool.rs
│   │   └── pagination.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── migrations.rs
│   │   ├── schema.rs
│   │   ├── vector.rs
│   │   ├── fts.rs
│   │   └── repos/
│   │       ├── mod.rs
│   │       ├── conversations.rs
│   │       ├── messages.rs
│   │       ├── profiles.rs
│   │       ├── memories.rs
│   │       └── tools.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── health.rs
│   │   ├── conversations.rs
│   │   ├── messages.rs
│   │   ├── profile.rs
│   │   ├── memories.rs
│   │   ├── tools.rs
│   │   ├── search.rs
│   │   └── stream.rs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── conversations.rs
│   │   ├── messages.rs
│   │   ├── profile.rs
│   │   ├── memories.rs
│   │   ├── tools.rs
│   │   ├── search.rs
│   │   └── stream.rs
│   ├── orchestrator/
│   │   ├── mod.rs
│   │   ├── agent.rs              # ReAct loop
│   │   ├── session_window.rs      # Sliding Window + Summarization
│   │   ├── context_classifier.rs  # Intent routing
│   │   ├── context_builder.rs     # 3 estrategias de contexto
│   │   ├── guardrails.rs          # Permisos + human-in-the-loop
│   │   └── reflection.rs          # Analizador segundo plano
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── provider.rs
│   │   ├── openrouter.rs
│   │   ├── ollama.rs
│   │   └── fallback.rs
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── trait.rs
│   │   ├── registry.rs
│   │   ├── permission.rs
│   │   ├── calendar.rs
│   │   ├── tasks.rs
│   │   ├── email.rs
│   │   ├── travel.rs
│   │   └── memory_tools.rs
│   ├── embeddings/
│   │   ├── mod.rs
│   │   └── provider.rs
│   ├── workers/
│   │   ├── mod.rs
│   │   ├── pool.rs
│   │   ├── briefing.rs
│   │   ├── conflict_detector.rs
│   │   ├── travel_prep.rs
│   │   └── memory_worker.rs
│   ├── middleware/
│   │   └── rate_limit.rs
│   └── bin/
│       └── seed.rs
├── tests/
│   ├── api/
│   │   ├── conversations.rs
│   │   ├── messages.rs
│   │   ├── profile.rs
│   │   ├── memories.rs
│   │   ├── tools.rs
│   │   ├── search.rs
│   │   ├── guardrails.rs
│   │   ├── context.rs
│   │   └── chat.rs
│   └── load/
│       └── chat.rs
└── frontend/
    ├── package.json
    ├── vite.config.ts
    ├── tsconfig.json
    ├── .eslintrc.cjs
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── theme.ts
        ├── types/
        │   └── index.ts
        ├── api/
        │   └── client.ts
        ├── hooks/
        │   ├── useConversations.ts
        │   ├── useMessages.ts
        │   ├── useSSE.ts
        │   ├── useProfile.ts
        │   └── useMemories.ts
        └── components/
            ├── AppLayout.tsx
            ├── Sidebar.tsx
            ├── ConversationList.tsx
            ├── ChatView.tsx
            ├── MessageBubble.tsx
            ├── MessageInput.tsx
            ├── ProfileEditor.tsx
            ├── MemoryExplorer.tsx
            ├── MemoryCard.tsx
            └── SearchDialog.tsx
```

---

## Flujo de Trabajo (OpenSpec + TDD)

Cada fase sigue el protocolo de `AGENTS.md`:

1. **Crear change proposal:** `openspec new change <feature-name>`
2. **Esperar aprobación del usuario** antes de escribir código
3. **RED:** Escribir tests que fallen
4. **GREEN:** Implementación mínima para que pasen
5. **REFACTOR:** Limpiar, clippy, fmt
6. **Archive:** `openspec archive <feature-name>`

**Comandos justfile necesarios:**

```justfile
dev:        # Levanta backend + frontend en modo desarrollo
check-all:  # Ejecuta todos los checks (test, clippy, tsc, lint, vitest)
check-spec: # Verifica que existe un change proposal activo
```