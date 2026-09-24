# Models Spec Delta — f2-api-core

## ADDED: Conversation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,           // UUID v4
    pub title: String,
    pub created_at: String,   // ISO 8601
    pub updated_at: String,   // ISO 8601
}

#[derive(Debug, Deserialize)]
pub struct CreateConversation {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConversation {
    pub title: Option<String>,
}
```

## ADDED: Message

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Value>,   // serde_json::Value
    pub tool_results: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "system" => Some(Self::System),
            "tool" => Some(Self::Tool),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_results: Option<Value>,
}
```

## ADDED: Profile

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub preferences: Value,    // JSONB
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfile {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub preferences: Option<Value>,
}
```

## ADDED: Memory

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub profile_id: String,
    pub content: String,
    pub category: String,
    pub source: String,
    pub embedding_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemory {
    pub profile_id: String,
    pub content: String,
    pub category: Option<String>,
    pub source: Option<String>,
}
```

## ADDED: Tool

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}
```

## ADDED: Pagination

```rust
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub total: Option<i64>,  // solo para offset-based
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
```

## ADDED: API Error

```rust
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub details: Option<String>,
}
```

## Scenarios (BDD)

### Scenario: Create conversation with default title
- **Given** a valid POST request to `/api/conversations` with no title
- **When** the handler processes it
- **Then** returns 201 with a conversation that has a UUID id and empty title

### Scenario: Create conversation with custom title
- **Given** a valid POST request to `/api/conversations` with `{"title": "Test"}`
- **When** the handler processes it
- **Then** returns 201 with title "Test"

### Scenario: List conversations returns paginated results
- **Given** there are 15 conversations in the database
- **When** GET `/api/conversations?limit=10` is called
- **Then** returns 10 conversations with a next_cursor

### Scenario: Get conversation by id
- **Given** a conversation exists
- **When** GET `/api/conversations/:id` is called
- **Then** returns 200 with the conversation

### Scenario: Get non-existent conversation returns 404
- **Given** no conversation with that id exists
- **When** GET `/api/conversations/:id` is called
- **Then** returns 404 with error message

### Scenario: Update conversation title
- **Given** a conversation exists
- **When** PUT `/api/conversations/:id` with `{"title": "New Title"}`
- **Then** returns 200 with updated title

### Scenario: Delete conversation
- **Given** a conversation exists
- **When** DELETE `/api/conversations/:id`
- **Then** returns 204 and the conversation is gone

### Scenario: Create message in conversation
- **Given** a conversation exists
- **When** POST `/api/conversations/:id/messages` with valid body
- **Then** returns 201 with the created message

### Scenario: List messages with cursor pagination
- **Given** a conversation has 25 messages
- **When** GET `/api/conversations/:id/messages?limit=10`
- **Then** returns 10 messages with next_cursor

### Scenario: Get profile returns default
- **Given** no profile has been created
- **When** GET `/api/profile`
- **Then** returns 200 with a default profile

### Scenario: Update profile
- **Given** a profile exists
- **When** PUT `/api/profile` with valid body
- **Then** returns 200 with updated fields

### Scenario: Create memory
- **Given** a valid POST to `/api/memories`
- **When** the handler processes it
- **Then** returns 201 with the created memory

### Scenario: List memories with pagination
- **Given** there are 20 memories
- **When** GET `/api/memories?limit=5`
- **Then** returns 5 memories

### Scenario: Delete memory
- **Given** a memory exists
- **When** DELETE `/api/memories/:id`
- **Then** returns 204

### Scenario: List tools
- **Given** tools exist in the database
- **When** GET `/api/tools`
- **Then** returns 200 with a list of tools

### Scenario: Toggle tool enabled
- **Given** a tool exists with enabled=true
- **When** PUT `/api/tools/:id/toggle`
- **Then** returns 200 with enabled=false