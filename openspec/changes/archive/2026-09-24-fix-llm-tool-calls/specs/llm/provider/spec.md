# LLM Providers: Tool Calls

## ADDED Requirements

### Requirement: Parse tool_calls from OpenRouter responses
**Given** una respuesta de OpenRouter con `choices[0].message.tool_calls`  
**When** `OpenRouterProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls` conteniendo los tool calls  
**And** cada tool call tiene `id`, `name` y `arguments` como `Value`  
**And** `content` puede ser `""` (string vacío) si el LLM devolvió `null`

#### Scenario: OpenRouter tool_calls se parsean correctamente
**Given** una respuesta de OpenRouter con `choices[0].message.tool_calls`  
**When** `OpenRouterProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls` conteniendo los tool calls  
**And** cada tool call tiene `id`, `name` y `arguments` como `Value`  
**And** `content` puede ser `""` (string vacío) si el LLM devolvió `null`

#### Scenario: OpenRouter respuesta sin tool_calls
**Given** una respuesta de OpenRouter sin `tool_calls`  
**When** `OpenRouterProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls == None`  
**And** `message.content` contiene el texto de la respuesta

#### Scenario: OpenRouter arguments es string JSON válido
**Given** una respuesta de OpenRouter donde `function.arguments` es un string JSON  
**When** se parsea el tool call  
**Then** el string se parsea a `serde_json::Value`  
**And** si el parseo falla, se usa el string original como `Value::String`

### Requirement: Parse tool_calls from Ollama responses
**Given** una respuesta de Ollama con `message.tool_calls`  
**When** `OllamaProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls` conteniendo los tool calls  
**And** cada tool call tiene `id` (generado si no viene), `name` y `arguments` como `Value`

#### Scenario: Ollama tool_calls se parsean correctamente
**Given** una respuesta de Ollama con `message.tool_calls`  
**When** `OllamaProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls` conteniendo los tool calls  
**And** cada tool call tiene `id` (generado si no viene), `name` y `arguments` como `Value`

#### Scenario: Ollama respuesta sin tool_calls
**Given** una respuesta de Ollama sin `tool_calls`  
**When** `OllamaProvider::chat()` procesa la respuesta  
**Then** devuelve `ChatResponse` con `message.tool_calls == None`  
**And** `message.content` contiene el texto de la respuesta

#### Scenario: Ollama arguments es objeto JSON directamente
**Given** una respuesta de Ollama donde `function.arguments` es un objeto JSON  
**When** se parsea el tool call  
**Then** se usa directamente como `Value` sin parseo adicional