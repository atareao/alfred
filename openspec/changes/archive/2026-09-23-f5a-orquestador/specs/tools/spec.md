## ADDED Requirements

### Requirement: Tool trait
Every tool must implement the Tool trait, which defines its metadata, input schema, and execution logic.

**Contracts:**
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g., "get_events", "add_task")
    fn name(&self) -> &'static str;

    /// Human-readable description for the LLM
    fn description(&self) -> &'static str;

    /// JSON Schema for tool parameters
    fn parameters(&self) -> serde_json::Value;

    /// Permission level required
    fn permission(&self) -> Permission;

    /// Execute the tool with given arguments
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult, ToolError>;
}

pub struct ToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub message: Option<String>,
}

pub enum ToolError {
    InvalidArguments(String),
    ExecutionError(String),
    PermissionDenied(String),
    NotFound(String),
}

pub enum Permission {
    /// Execute silently, no user notification
    NoConfirm,
    /// Execute and notify user
    Notify,
    /// Require explicit user approval before executing
    ExplicitApproval,
}
```

**Scenarios:**
#### Scenario: Tool name is unique and descriptive
Given a Tool implementation
When `name()` is called
Then it returns a non-empty kebab-case string

#### Scenario: Tool returns valid JSON Schema
Given a Tool implementation
When `parameters()` is called
Then it returns a valid JSON Schema object with at least a `type: "object"` property

#### Scenario: Tool executes successfully
Given a Tool implementation with valid arguments
When `execute()` is called with matching JSON arguments
Then ToolResult with success: true is returned

#### Scenario: Tool rejects invalid arguments
Given a Tool implementation
When `execute()` is called with missing or wrong type arguments
Then ToolError::InvalidArguments is returned

---

### Requirement: Tool Registry
The registry manages registration and discovery of all available tools.

**Contracts:**
```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self;

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>);

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool>;

    /// Get all registered tool definitions (for LLM function calling)
    pub fn definitions(&self) -> Vec<ToolDef>;

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: serde_json::Value) -> Result<ToolResult, ToolError>;

    /// Get the permission level for a tool
    pub fn permission(&self, name: &str) -> Option<Permission>;
}
```

**Scenarios:**
#### Scenario: Register and retrieve tool
Given an empty ToolRegistry
When a tool is registered and then retrieved by name
Then the tool is returned

#### Scenario: Get unknown tool returns None
Given a ToolRegistry with no tools
When `get("nonexistent")` is called
Then None is returned

#### Scenario: Definitions returns all tool schemas
Given a ToolRegistry with 3 registered tools
When `definitions()` is called
Then a Vec of 3 ToolDef structs is returned, each with name, description, and parameters

---

### Requirement: Permission levels
Each tool has a permission level that determines human-in-the-loop requirements.

**Contracts:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    NoConfirm,
    Notify,
    ExplicitApproval,
}

pub struct GuardrailCheck {
    pub tool: String,
    pub permission: Permission,
    pub reason: String,
    pub requires_approval: bool,
    pub approved: Option<bool>,
}
```

**Scenarios:**
#### Scenario: Read-only tools are NoConfirm
Given a tool defined with Permission::NoConfirm
When the permission is checked
Then the tool can execute without user interaction

#### Scenario: Reversible tools are Notify
Given a tool defined with Permission::Notify
When the tool executes
Then the result is returned to the user with a notification

#### Scenario: Destructive tools require ExplicitApproval
Given a tool defined with Permission::ExplicitApproval
When the tool is requested by the LLM
Then execution pauses and the user must approve before execution