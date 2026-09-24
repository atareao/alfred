## ADDED Requirements

### Requirement: SSE streaming endpoint
The streaming endpoint sends orchestrated responses to the frontend via Server-Sent Events.

**Contracts:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SSEEvent {
    #[serde(rename = "chunk")]
    Chunk { content: String },
    #[serde(rename = "tool_call")]
    ToolCall { name: String, args: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { name: String, success: bool },
    #[serde(rename = "done")]
    Done { message_id: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "approval_required")]
    ApprovalRequired { request_id: String, tool_name: String, reason: String },
    #[serde(rename = "approval_result")]
    ApprovalResult { request_id: String, approved: bool },
}

pub struct StreamHandler {
    orchestrator: Arc<Orchestrator>,
}

// POST /api/conversations/:id/messages with Accept: text/event-stream
// Returns SSE stream of SSEEvent messages
// POST /api/approval/:request_id with body { approved: bool }
// Resolves a pending approval request
```

**Scenarios:**
#### Scenario: SSE stream returns chunks
Given a POST to /api/conversations/:id/messages with text/event-stream
When the orquestador processes the message
Then the response is an SSE stream
And events include multiple chunk events
And a final done event

#### Scenario: SSE stream includes tool calls
Given a POST that triggers a tool call
When the orchestrator calls a tool
Then a tool_call SSE event is sent before execution
And a tool_result SSE event is sent after

#### Scenario: Approval required pauses stream
Given a POST that triggers an ExplicitApproval tool
When the guardrail check requires approval
Then an approval_required SSE event is sent
And the stream pauses until approval is resolved

#### Scenario: Error during streaming
Given a POST with an invalid conversation_id
When the orchestrator fails to process
Then an error SSE event is sent
And the stream closes

---

### Requirement: Approval resolution endpoint
Users can approve or deny tool execution via an API endpoint.

**Contracts:**
```
POST /api/approval/:request_id
Body: { "approved": true | false }
Response: 200 { "status": "resolved", "approved": true }
          404 { "error": "Request not found" }
          409 { "error": "Already resolved" }
```

**Scenarios:**
#### Scenario: Approve a pending request
Given a pending approval request with id "req-1"
When POST /api/approval/req-1 with { approved: true }
Then 200 OK is returned
And the guardrail allows execution

#### Scenario: Deny a pending request
Given a pending approval request with id "req-1"
When POST /api/approval/req-1 with { approved: false }
Then 200 OK is returned
And the guardrail denies execution

#### Scenario: Unknown request returns 404
When POST /api/approval/nonexistent
Then 404 Not Found is returned

#### Scenario: Already resolved returns 409
Given an already resolved approval request
When POST /api/approval/req-1 again
Then 409 Conflict is returned