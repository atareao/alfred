# message Specification

## Purpose
TBD - created by archiving change message-schema-enrichment. Update Purpose after archive.

## Requirements

### Requirement: estimate_markdown_tokens_heuristic SHALL compute token estimate via word + markdown heuristic
**Given** a text string  
**When** `estimate_markdown_tokens_heuristic(text)` is called  
**Then** it SHALL return `(word_count * 1.33).trunc() + md_symbol_count`  
**Where** word boundaries are whitespace or punctuation/Markdown symbols, and md_symbols are characters in `# * ` _ [] () > - + ! . , ; : ? " ' / \ = ~ |`

#### Scenario: empty text
**Given** empty text ""  
**When** `estimate_markdown_tokens_heuristic("")` is called  
**Then** it SHALL return 0

#### Scenario: plain text
**Given** text "Hola" (1 word)  
**When** `estimate_markdown_tokens_heuristic("Hola")` is called  
**Then** it SHALL return 1

#### Scenario: markdown heading
**Given** text "# Hello World" (2 words, `#` symbol)  
**When** `estimate_markdown_tokens_heuristic("# Hello World")` is called  
**Then** it SHALL return 3

#### Scenario: markdown bold
**Given** text "Some **bold** text" (3 words, 4 `*` symbols)  
**When** `estimate_markdown_tokens_heuristic("Some **bold** text")` is called  
**Then** it SHALL return 7

#### Scenario: markdown list
**Given** text "- Item one\n- Item two" (4 words, 2 `-` symbols)  
**When** `estimate_markdown_tokens_heuristic(text)` is called  
**Then** it SHALL return 7

#### Scenario: inline code
**Given** text "Use `let x = 1;`" (4 words, 4 md symbols)  
**When** `estimate_markdown_tokens_heuristic(text)` is called  
**Then** it SHALL return 9

### Requirement: Message model SHALL include new fields
**Given** a message stored in the database  
**Then** it SHALL have fields `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`

#### Scenario: Creation computes tokens_count automatically
**Given** content "Hola, ¿cómo estás?" (20 chars)  
**When** saved via `MessagesRepo::create`  
**Then** `msg.tokens_count` SHALL equal `estimate_markdown_tokens_heuristic(content)`  
**And** `msg.collapsed_content` SHALL be `None`  
**And** `msg.collapsed_tokens_count` SHALL be `0`  
**And** `msg.is_indexed` SHALL be `false`

### Requirement: Collapse threshold SHALL be configurable
**Given** `COLLAPSE_THRESHOLD_TOKENS` env var  
**When** `Config::from_env()` is called  
**Then** `collapse_threshold_tokens` SHALL take the env var value or default to 2000

#### Scenario: Short message SHALL NOT trigger collapse
**Given** `collapse_threshold_tokens` = 2000  
**Given** a message with 100 chars  
**When** saved  
**Then** `tokens_count` SHALL be < 2000  
**And** the collapse callback SHALL NOT be invoked

#### Scenario: Long message SHALL trigger collapse
**Given** `collapse_threshold_tokens` = 2000  
**Given** a message with sufficient content to exceed the threshold  
**When** saved  
**Then** `tokens_count` SHALL be >= 2000  
**And** the collapse callback SHALL be invoked with the message `id`

### Requirement: Migration SHALL add new columns idempotently
**Given** a database with an existing `messages` table  
**When** `run_migrations()` is executed  
**Then** the table SHALL have columns `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`  
**And** running migrations twice SHALL NOT fail

#### Scenario: New columns exist after migration
**Given** a fresh in-memory database  
**When** `run_migrations()` is executed  
**Then** `PRAGMA table_info(messages)` SHALL include `tokens_count`, `collapsed_content`, `collapsed_tokens_count`, `is_indexed`, `summary_ref`

#### Scenario: Migration is idempotent
**Given** a database where `run_migrations()` has already been executed
**When** `run_migrations()` is executed again
**Then** it SHALL succeed without error

### Requirement: Message list endpoint SHALL use configurable page size

**Given** una petición GET `/api/conversations/{id}/messages`
**When** no se especifica `limit` en query params
**Then** el handler SHALL leer `message_page_size` de settings y usarlo como límite por defecto
**And** si el setting no existe, SHALL usar 50 como fallback

#### Scenario: GET sin limit usa el setting
**Given** `message_page_size` = 25 en settings
**When** GET `/api/conversations/conv-id/messages`
**Then** devuelve 25 mensajes

#### Scenario: GET con limit explícito sobreescribe el setting
**Given** `message_page_size` = 25 en settings
**When** GET `/api/conversations/conv-id/messages?limit=10`
**Then** devuelve 10 mensajes
