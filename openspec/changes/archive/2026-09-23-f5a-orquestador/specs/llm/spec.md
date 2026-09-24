## ADDED Requirements

### Requirement: LLMProvider trait
The system must define a trait for LLM providers that abstracts chat completions, streaming chat completions, and embeddings generation.

**Contracts:**
```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat completion request (non-streaming)
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;

    /// Send a chat completion request (streaming)
    async fn chat_stream(&self, request: ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, LLMError>> + Send>>, LLMError>;

    /// Generate embeddings for a text
    async fn embed(&self, input: &str) -> Result<Vec<f32>, LLMError>;
}

pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<ToolDef>>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

pub struct ChatResponse {
    pub message: Message,
    pub usage: Option<TokenUsage>,
}

pub enum StreamEvent {
    Chunk(String),
    Done(ChatResponse),
    ToolCall(ToolCall),
}

pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub enum LLMError {
    HttpError(String),
    RateLimited { retry_after: u64 },
    Timeout(String),
    AuthError(String),
    ModelNotAvailable(String),
    Internal(String),
}

pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
```

**Scenarios:**
#### Scenario: Happy path — LLM responds to simple chat
Given a configured LLMProvider
When `chat()` is called with a ChatRequest containing a user message
Then a ChatResponse is returned with an assistant message
And the response contains non-empty content

#### Scenario: LLM returns tool calls
Given a configured LLMProvider with tools defined
When `chat()` is called with a request including ToolDefs
And the LLM determines a tool should be called
Then the response contains a ToolCall with valid name and arguments matching a defined tool

#### Scenario: LLM provides streaming chunks
Given a configured LLMProvider
When `chat_stream()` is called with stream: true
Then the stream yields multiple StreamEvent::Chunk events
And the final event is StreamEvent::Done with complete ChatResponse

#### Scenario: Rate limited returns error
Given a configured LLMProvider
When the provider receives HTTP 429 from the upstream API
Then LLMError::RateLimited is returned with retry_after seconds

#### Scenario: Timeout returns error
Given a configured LLMProvider
When the provider does not receive a response within the timeout window
Then LLMError::Timeout is returned

---

### Requirement: OpenRouterProvider implementation
OpenRouterProvider must implement LLMProvider, supporting both streaming and non-streaming chat, with configurable model selection and API key from environment.

**Contracts:**
```rust
pub struct OpenRouterConfig {
    pub api_key: String,
    pub model: String,      // default: "anthropic/claude-sonnet-20241022"
    pub base_url: String,   // default: "https://openrouter.ai/api/v1"
    pub max_retries: u32,
    pub timeout_secs: u64,
}

pub struct OpenRouterProvider {
    config: OpenRouterConfig,
    client: reqwest::Client,
}

impl OpenRouterProvider {
    pub fn new(config: OpenRouterConfig) -> Self;
}
```

**Scenarios:**
#### Scenario: OpenRouterProvider initializes with config
Given a valid OpenRouterConfig with api_key and model
When `OpenRouterProvider::new(config)` is called
Then the provider is created and ready to use

#### Scenario: OpenRouterProvider sends chat request
Given an OpenRouterProvider instance
When `chat()` is called with a ChatRequest
Then the request is sent to OpenRouter API v1/chat/completions
And the response is parsed into ChatResponse

#### Scenario: OpenRouterProvider streams chat
Given an OpenRouterProvider instance
When `chat_stream()` is called
Then SSE chunks are received and parsed into StreamEvent::Chunk events

---

### Requirement: OllamaProvider implementation
OllamaProvider must implement LLMProvider for local inference via the Ollama API.

**Contracts:**
```rust
pub struct OllamaConfig {
    pub base_url: String,   // default: "http://localhost:11434"
    pub model: String,      // default: "llama3.2:3b"
    pub timeout_secs: u64,
    pub keep_alive: String, // default: "5m"
}

pub struct OllamaProvider {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self;
}
```

**Scenarios:**
#### Scenario: OllamaProvider initializes
Given an OllamaConfig with base_url and model
When `OllamaProvider::new(config)` is called
Then the provider is ready

#### Scenario: OllamaProvider returns embedding
Given an OllamaProvider instance
When `embed()` is called with a text string
Then a Vec<f32> of expected dimension size is returned

#### Scenario: OllamaProvider connection error
Given an OllamaProvider instance
When `chat()` is called but the Ollama server is unreachable
Then LLMError::HttpError is returned

---

### Requirement: FallbackProvider chain
FallbackProvider chains multiple providers in priority order, trying the next on failure.

**Contracts:**
```rust
pub struct FallbackProvider {
    providers: Vec<Box<dyn LLMProvider>>,
}

impl FallbackProvider {
    pub fn new(providers: Vec<Box<dyn LLMProvider>>) -> Self;
}

impl LLMProvider for FallbackProvider {
    // Tries each provider in order; if all fail, returns last error
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError>;
    async fn chat_stream(&self, request: ChatRequest) -> Result<...>;
    async fn embed(&self, input: &str) -> Result<Vec<f32>, LLMError>;
}
```

**Scenarios:**
#### Scenario: Fallback uses primary provider
Given a FallbackProvider with [OpenRouter, Ollama]
When the primary provider (OpenRouter) succeeds
Then the result is returned from OpenRouter
And Ollama is never called

#### Scenario: Fallback to secondary on failure
Given a FallbackProvider with [Ollama, OpenRouter]
When the primary provider (Ollama) fails with rate limit
Then the fallback tries OpenRouter
And returns OpenRouter's result

#### Scenario: All providers fail
Given a FallbackProvider with two providers
When both providers return errors
Then the last error is returned