# API Spec Delta — f2-api-core

## ADDED: Routes

### Conversations

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/conversations` | `list_conversations` | Lista paginada (cursor) |
| POST | `/api/conversations` | `create_conversation` | Crea nueva conversación |
| GET | `/api/conversations/:id` | `get_conversation` | Obtiene una conversación |
| PUT | `/api/conversations/:id` | `update_conversation` | Actualiza título |
| DELETE | `/api/conversations/:id` | `delete_conversation` | Elimina conversación |

### Messages

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/conversations/:id/messages` | `list_messages` | Lista paginada (cursor) |
| POST | `/api/conversations/:id/messages` | `create_message` | Crea mensaje |
| GET | `/api/conversations/:id/messages/:msg_id` | `get_message` | Obtiene mensaje |

### Profile

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/profile` | `get_profile` | Obtiene perfil (crea default si no existe) |
| PUT | `/api/profile` | `update_profile` | Actualiza perfil |

### Memories

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/memories` | `list_memories` | Lista paginada (offset) |
| POST | `/api/memories` | `create_memory` | Crea memoria |
| DELETE | `/api/memories/:id` | `delete_memory` | Elimina memoria |

### Tools

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/api/tools` | `list_tools` | Lista todas las tools |
| PUT | `/api/tools/:id/toggle` | `toggle_tool` | Activa/desactiva tool |

## ADDED: Handler signatures

```rust
// Conversations
pub async fn list_conversations(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Conversation>>, AppError>;

pub async fn create_conversation(
    State(state): State<AppState>,
    Json(body): Json<CreateConversation>,
) -> Result<(StatusCode, Json<Conversation>), AppError>;

pub async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Conversation>, AppError>;

pub async fn update_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateConversation>,
) -> Result<Json<Conversation>, AppError>;

pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError>;

// Messages
pub async fn list_messages(
    State(state): State<AppState>,
    Path(conv_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Message>>, AppError>;

pub async fn create_message(
    State(state): State<AppState>,
    Path(conv_id): Path<String>,
    Json(body): Json<CreateMessage>,
) -> Result<(StatusCode, Json<Message>), AppError>;

pub async fn get_message(
    State(state): State<AppState>,
    Path((conv_id, msg_id)): Path<(String, String)>,
) -> Result<Json<Message>, AppError>;

// Profile
pub async fn get_profile(
    State(state): State<AppState>,
) -> Result<Json<Profile>, AppError>;

pub async fn update_profile(
    State(state): State<AppState>,
    Json(body): Json<UpdateProfile>,
) -> Result<Json<Profile>, AppError>;

// Memories
pub async fn list_memories(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Memory>>, AppError>;

pub async fn create_memory(
    State(state): State<AppState>,
    Json(body): Json<CreateMemory>,
) -> Result<(StatusCode, Json<Memory>), AppError>;

pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError>;

// Tools
pub async fn list_tools(
    State(state): State<AppState>,
) -> Result<Json<Vec<Tool>>, AppError>;

pub async fn toggle_tool(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Tool>, AppError>;
```

## ADDED: AppError

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(ApiError { error: message, details: None })).into_response()
    }
}
```

## ADDED: Route mounting

```rust
// En src/lib.rs
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
        // Conversations
        .route("/api/conversations", get(list_conversations).post(create_conversation))
        .route("/api/conversations/:id", get(get_conversation).put(update_conversation).delete(delete_conversation))
        // Messages
        .route("/api/conversations/:id/messages", get(list_messages).post(create_message))
        .route("/api/conversations/:id/messages/:msg_id", get(get_message))
        // Profile
        .route("/api/profile", get(get_profile).put(update_profile))
        // Memories
        .route("/api/memories", get(list_memories).post(create_memory))
        .route("/api/memories/:id", delete(delete_memory))
        // Tools
        .route("/api/tools", get(list_tools))
        .route("/api/tools/:id/toggle", put(toggle_tool))
        .with_state(state)
}
```

## Scenarios (BDD)

### Scenario: Full conversation lifecycle
- **Given** the API is running
- **When** creating a conversation, adding messages, updating title, listing, and deleting
- **Then** all operations succeed in sequence

### Scenario: Message belongs to conversation
- **Given** a conversation exists
- **When** creating a message with a non-existent conversation_id
- **Then** returns 404 (foreign key constraint)

### Scenario: Profile is auto-created on first GET
- **Given** no profile exists
- **When** GET /api/profile
- **Then** returns 200 with a default profile

### Scenario: Tools are seeded on first GET
- **Given** tools table is empty
- **When** GET /api/tools
- **Then** returns 200 with default tool list

### Scenario: Delete non-existent returns 404
- **Given** no resource with that id
- **When** DELETE /api/conversations/:id
- **Then** returns 404