## ADDED Requirements

### Requirement: Context Classifier
The classifier determines the user's intent and selects the appropriate context strategy (A, B, C, or override).

**Contracts:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ContextStrategy {
    /// Sliding window + RAG (default, daily use)
    SlidingWindow,
    /// Full historical analysis (SQL queries)
    Historical,
    /// Vector search + full thread
    RAG,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Override {
    None,
    Historical(String),   // !historico <query>
    Doc(String),          // !doc <query>
    Reset,                // !reset
}

pub struct Classification {
    pub strategy: ContextStrategy,
    pub override_cmd: Override,
    pub confidence: f32,
}

pub struct ContextClassifier;

impl ContextClassifier {
    pub fn new() -> Self;

    /// Classify a user message to determine context strategy
    pub fn classify(&self, message: &str) -> Classification;

    /// Check for manual override commands
    pub fn check_override(&self, message: &str) -> Override;
}
```

**Scenarios:**
#### Scenario: Default message uses SlidingWindow
Given a ContextClassifier
When `classify()` is called with "Añade leche a la compra"
Then a Classification with strategy SlidingWindow is returned

#### Scenario: !historico override forces Historical
Given a ContextClassifier
When `check_override()` receives "!historico ¿qué planes hicimos para el puente?"
Then Override::Historical("¿qué planes hicimos para el puente?") is returned

#### Scenario: !doc override forces RAG
Given a ContextClassifier
When `check_override()` receives "!doc ¿qué recomendación de hotel me dio Juan?"
Then Override::Doc("¿qué recomendación de hotel me dio Juan?") is returned

#### Scenario: !reset returns Reset override
Given a ContextClassifier
When `check_override()` receives "!reset"
Then Override::Reset is returned

#### Scenario: No override returns None
Given a ContextClassifier
When `check_override()` receives a message without "!" prefix
Then Override::None is returned

---

### Requirement: Context Builder
The context builder constructs the final LLM prompt using one of three strategies.

**Contracts:**
```rust
pub struct ContextBuilder {
    session_window: SessionWindow,
    profile_repo: Arc<ProfileRepo>,
    search_service: Arc<SearchService>,
}

impl ContextBuilder {
    pub fn new(session_window: SessionWindow, profile_repo: Arc<ProfileRepo>, search_service: Arc<SearchService>) -> Self;

    /// Build context for a given strategy
    pub async fn build(
        &self,
        strategy: ContextStrategy,
        profile_id: &str,
        user_message: &str,
    ) -> Result<BuiltContext, ContextError>;
}

pub struct BuiltContext {
    pub system_prompt: String,
    pub messages: Vec<Message>,      // Full message list for the LLM
    pub token_estimate: usize,
    pub rag_memories: Vec<Memory>,
    pub session_summary: Option<String>,
}

pub enum ContextError {
    ProfileNotFound,
    SearchError(String),
    WindowError(String),
}
```

**Scenarios:**
#### Scenario: Strategy A builds sliding window context
Given a ContextBuilder with a conversation of 15 messages
When `build(SlidingWindow, ...)` is called
Then the result contains system_prompt + profile + session_summary + last 8 messages
And rag_memories has 2-3 relevant entries
And token_estimate is under 2000

#### Scenario: Strategy B builds historical context
Given a ContextBuilder
When `build(Historical, ...)` is called
Then the result includes a comprehensive historical analysis prompt

#### Scenario: Strategy C builds RAG context
Given a ContextBuilder
When `build(RAG, ...)` is called
Then the result includes vector search results as context

---

### Requirement: Sliding Window + Session Summary
Maintains a window of recent messages and summarizes older ones when the window overflows.

**Contracts:**
```rust
pub struct SessionWindow {
    conversation_id: String,
    recent_messages: VecDeque<Message>,  // Max 12
    summary: Option<String>,             // Compressed summary of older messages
    max_window: usize,                   // Default 12
    compact_threshold: usize,            // When to compact, default 8
}

impl SessionWindow {
    pub fn new(conversation_id: &str) -> Self;

    /// Add a message to the window, auto-compacting if needed
    pub async fn add_message(&mut self, msg: Message) -> Result<(), WindowError>;

    /// Get current window (summary + recent messages)
    pub fn get_window(&self) -> WindowView;

    /// Manually trigger compaction (summarize oldest messages)
    pub async fn compact(&mut self) -> Result<(), WindowError>;

    /// Reset the window (clear messages and summary)
    pub fn reset(&mut self);
}

pub struct WindowView {
    pub summary: Option<String>,
    pub messages: Vec<Message>,
    pub message_count: usize,
}
```

**Scenarios:**
#### Scenario: New window starts empty
Given a new SessionWindow for a conversation
When `get_window()` is called
Then messages is empty
And summary is None

#### Scenario: Adding message under threshold
Given a SessionWindow with 5 messages
When a 6th message is added
Then `get_window()` returns 6 messages
And summary is still None

#### Scenario: Adding message over threshold triggers compact
Given a SessionWindow with 10 messages
When an 11th message is added
Then the 4 oldest messages are summarized
And `get_window()` returns 8 recent messages + a summary

#### Scenario: Reset clears everything
Given a SessionWindow with 10 messages and a summary
When `reset()` is called
Then messages is empty
And summary is None## ADDED Requirements

### Requirement: Guardrails system
The guardrails system validates every tool execution request against the tool's permission level and manages HITL approval flow.

**Contracts:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub reason: String,
    pub status: ApprovalStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct Guardrails {
    registry: Arc<ToolRegistry>,
    pending_approvals: Arc<Mutex<HashMap<String, ApprovalRequest>>>,
}

impl Guardrails {
    pub fn new(registry: Arc<ToolRegistry>) -> Self;

    /// Check if a tool execution is allowed. Returns Ok if NoConfirm or Notify.
    /// Returns Err with ApprovalRequired if ExplicitApproval.
    pub async fn check(&self, tool_name: &str, args: &serde_json::Value) -> Result<GuardrailResult, GuardrailError>;

    /// Create an approval request for explicit approval
    pub fn request_approval(&self, tool_name: &str, args: serde_json::Value, reason: String) -> ApprovalRequest;

    /// Resolve a pending approval
    pub fn resolve_approval(&self, request_id: &str, approved: bool) -> Result<(), GuardrailError>;
}

pub enum GuardrailResult {
    /// Execute immediately
    Allowed { notify: bool },
    /// Pending user approval
    RequiresApproval { request_id: String },
}

pub enum GuardrailError {
    ToolNotFound(String),
    RequestNotFound(String),
    AlreadyResolved(String),
}
```

**Scenarios:**
#### Scenario: NoConfirm tool passes through
Given a Guardrails with a NoConfirm tool
When `check()` is called for that tool
Then GuardrailResult::Allowed { notify: false } is returned

#### Scenario: Notify tool executes with notification
Given a Guardrails with a Notify tool
When `check()` is called for that tool
Then GuardrailResult::Allowed { notify: true } is returned

#### Scenario: ExplicitApproval tool requires approval
Given a Guardrails with an ExplicitApproval tool
When `check()` is called for that tool
Then GuardrailResult::RequiresApproval is returned with a request_id

#### Scenario: Approve a pending request
Given a Guardrails with a pending approval request
When `resolve_approval(request_id, true)` is called
Then the request is marked as Approved
And subsequent check confirms it

#### Scenario: Deny a pending request
Given a Guardrails with a pending approval request
When `resolve_approval(request_id, false)` is called
Then the request is marked as Denied
And subsequent check returns GuardrailError

---

### Requirement: ReAct Agent (Orchestrator)
The orchestrator runs the ReAct loop: think → act (tool calls) → observe → repeat, with max iterations.

**Contracts:**
```rust
pub struct Orchestrator {
    llm: Arc<dyn LLMProvider>,
    registry: Arc<ToolRegistry>,
    guardrails: Arc<Guardrails>,
    context_builder: Arc<ContextBuilder>,
    config: OrchestratorConfig,
}

pub struct OrchestratorConfig {
    pub max_iterations: usize,          // Default 10
    pub max_tokens_per_turn: u32,       // Default 4096
    pub enable_reflection: bool,        // Default true
    pub system_prompt_template: String, // Template with {profile} placeholder
}

impl Orchestrator {
    pub fn new(
        llm: Arc<dyn LLMProvider>,
        registry: Arc<ToolRegistry>,
        guardrails: Arc<Guardrails>,
        context_builder: Arc<ContextBuilder>,
        config: OrchestratorConfig,
    ) -> Self;

    /// Process a user message through the full ReAct loop
    pub async fn process_message(
        &self,
        conversation_id: &str,
        profile_id: &str,
        user_message: &str,
        override_cmd: Option<Override>,
    ) -> Result<AgentResponse, AgentError>;

    /// Stream the response via SSE
    pub async fn process_message_stream(
        &self,
        conversation_id: &str,
        profile_id: &str,
        user_message: &str,
        override_cmd: Option<Override>,
        tx: mpsc::Sender<SSEEvent>,
    ) -> Result<(), AgentError>;

    /// Single ReAct iteration
    async fn iteration(&self, state: &mut AgentState) -> Result<IterationResult, AgentError>;
}

pub struct AgentResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub reflection: Option<Reflection>,
    pub iterations: usize,
}

pub enum IterationResult {
    Complete(String),          // LLM responded with final answer
    ToolRequest(ToolCall),     // LLM wants to call a tool
    MaxIterationsReached,      // Safety limit
}

pub struct AgentState {
    pub messages: Vec<Message>,
    pub iteration_count: usize,
    pub tool_results: Vec<ToolResult>,
}

pub enum AgentError {
    LLMError(LLMError),
    ToolError(ToolError),
    GuardrailError(GuardrailError),
    ContextError(ContextError),
    MaxIterationsExceeded,
    Internal(String),
}
```

**Scenarios:**
#### Scenario: Simple chat — no tool calls
Given an Orchestrator
When `process_message()` is called with "Hola, ¿cómo estás?"
Then AgentResponse is returned with a text message
And tool_calls is empty
And iterations is 1

#### Scenario: Single tool call flow
Given an Orchestrator with a "get_events" tool registered
When `process_message()` is called with "¿Qué tengo mañana?"
Then the ReAct loop runs:
  - LLM returns tool_call (get_events)
  - Tool is executed
  - Result is fed back to LLM
  - LLM returns final answer
And tool_calls contains 1 entry

#### Scenario: Multi-tool flow
Given an Orchestrator with multiple tools
When `process_message()` is called with a query requiring 2 tools
Then the ReAct loop runs 3+ iterations (tool1 → result → tool2 → result → final)

#### Scenario: Max iterations reached
Given an Orchestrator configured with max_iterations: 3
When the LLM keeps calling tools for 3+ iterations
Then AgentError::MaxIterationsExceeded is returned

---

### Requirement: Reflection (Second-order analysis)
After the ReAct loop, the reflection module evaluates the response quality.

**Contracts:**
```rust
pub struct Reflection {
    pub is_coherent: bool,
    pub is_complete: bool,
    pub needs_clarification: Option<String>,
    pub suggested_followup: Option<String>,
}

pub struct ReflectionAnalyzer {
    llm: Arc<dyn LLMProvider>,
}

impl ReflectionAnalyzer {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self;

    /// Analyze the conversation so far and the final response
    pub async fn analyze(
        &self,
        conversation: &[Message],
        response: &str,
    ) -> Result<Reflection, AgentError>;
}
```

**Scenarios:**
#### Scenario: Coherent and complete response
Given a ReflectionAnalyzer
When `analyze()` is called with a clear question and good answer
Then is_coherent is true
And is_complete is true
And needs_clarification is None

#### Scenario: Ambiguous response triggers clarification
Given a ReflectionAnalyzer
When `analyze()` detects the answer is vague or incomplete
Then needs_clarification contains a follow-up question string