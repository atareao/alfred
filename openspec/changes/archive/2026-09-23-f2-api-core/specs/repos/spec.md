# Repos Spec Delta — f2-api-core

## ADDED: ConversationsRepo

```rust
pub struct ConversationsRepo;

impl ConversationsRepo {
    pub fn create(conn: &Connection, title: &str) -> Result<Conversation>;
    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Conversation>>;
    pub fn list(conn: &Connection, limit: i64, cursor: Option<&str>) -> Result<(Vec<Conversation>, Option<String>)>;
    pub fn update(conn: &Connection, id: &str, title: Option<&str>) -> Result<Option<Conversation>>;
    pub fn delete(conn: &Connection, id: &str) -> Result<bool>;
}
```

## ADDED: MessagesRepo

```rust
pub struct MessagesRepo;

impl MessagesRepo {
    pub fn create(conn: &Connection, conversation_id: &str, role: &str, content: &str, tool_calls: Option<&Value>, tool_results: Option<&Value>) -> Result<Message>;
    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Message>>;
    pub fn list_by_conversation(conn: &Connection, conversation_id: &str, limit: i64, cursor: Option<&str>) -> Result<(Vec<Message>, Option<String>)>;
    pub fn delete_by_conversation(conn: &Connection, conversation_id: &str) -> Result<usize>;
}
```

## ADDED: ProfilesRepo

```rust
pub struct ProfilesRepo;

impl ProfilesRepo {
    pub fn get_or_create(conn: &Connection) -> Result<Profile>;
    pub fn update(conn: &Connection, name: Option<&str>, avatar_url: Option<&str>, preferences: Option<&Value>) -> Result<Profile>;
}
```

## ADDED: MemoriesRepo

```rust
pub struct MemoriesRepo;

impl MemoriesRepo {
    pub fn create(conn: &Connection, profile_id: &str, content: &str, category: &str, source: &str) -> Result<Memory>;
    pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Memory>>;
    pub fn list(conn: &Connection, limit: i64, offset: i64) -> Result<(Vec<Memory>, i64)>;
    pub fn delete(conn: &Connection, id: &str) -> Result<bool>;
}
```

## ADDED: ToolsRepo

```rust
pub struct ToolsRepo;

impl ToolsRepo {
    pub fn list(conn: &Connection) -> Result<Vec<Tool>>;
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tool>>;
    pub fn toggle_enabled(conn: &Connection, id: &str) -> Result<Option<Tool>>;
    pub fn seed_defaults(conn: &Connection) -> Result<()>;
}
```

## Scenarios (BDD)

### Scenario: ConversationsRepo create generates UUID
- **Given** a database connection
- **When** create() is called with a title
- **Then** returns a Conversation with a non-empty UUID id

### Scenario: ConversationsRepo list returns cursor
- **Given** 12 conversations exist
- **When** list(limit=5) is called
- **Then** returns 5 conversations and a non-null next_cursor

### Scenario: ConversationsRepo list with cursor paginates
- **Given** 12 conversations exist
- **When** list(limit=5) is called, then list(limit=5, cursor=next_cursor)
- **Then** second call returns different conversations

### Scenario: MessagesRepo create stores role correctly
- **Given** a conversation exists
- **When** create() is called with role="user"
- **Then** the stored message has role="user"

### Scenario: MessagesRepo list returns messages ordered by created_at
- **Given** a conversation has 3 messages
- **When** list() is called
- **Then** messages are returned in ascending created_at order

### Scenario: ProfilesRepo get_or_create returns existing
- **Given** a profile already exists
- **When** get_or_create() is called
- **Then** returns the existing profile (not a new one)

### Scenario: ProfilesRepo update changes fields
- **Given** a profile exists
- **When** update() is called with new name
- **Then** the profile's name is updated

### Scenario: MemoriesRepo create stores category
- **Given** a profile exists
- **When** create() is called with category="fact"
- **Then** the memory has category="fact"

### Scenario: MemoriesRepo delete returns false for non-existent
- **Given** no memory with that id exists
- **When** delete() is called
- **Then** returns false

### Scenario: ToolsRepo seed_defaults creates tools
- **Given** an empty tools table
- **When** seed_defaults() is called
- **Then** creates rows for calendar, tasks, weather, geo, meals, habits, knowledge, contacts, reminders

### Scenario: ToolsRepo toggle_enabled flips value
- **Given** a tool with enabled=true
- **When** toggle_enabled() is called
- **Then** returns tool with enabled=false