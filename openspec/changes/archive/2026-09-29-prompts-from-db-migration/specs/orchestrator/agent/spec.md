# Spec Delta: orchestrator/agent

## MODIFIED Requirements

### Requirement: System prompt template uses Markdown and emojis
**Given** una base de datos migrada con el prompt sembrado en `settings.system_prompt`
**When** el orquestador construye la petición al LLM
**Then** SHALL leer `system_prompt` de la tabla `settings`
**And** SHALL usar ese valor como mensaje de sistema
**And** SHALL NOT usar ningún template hardcodeado en `OrchestratorConfig`
**And** el prompt SHALL contener instrucciones de Markdown, "Emojis" y formato rico
**And** si `system_prompt` está ausente o vacío, SHALL usar un fallback mínimo genérico y loguear un warning

#### Scenario: System prompt incluye personaje de mayordomo
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "asistente personal británico"
**And** contiene "usted"
**And** contiene "caballero"

#### Scenario: System prompt tiene modo conciso y expandido
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "Modo Conciso (Predeterminado)"
**And** contiene "Expandido"

#### Scenario: System prompt permite Markdown completo y emojis
**Given** la base de datos migrada
**When** se lee `settings.system_prompt`
**Then** contiene "Markdown"
**And** contiene "Emojis"

#### Scenario: process_message usa el prompt de la BD
**Given** un orquestador con `settings.system_prompt = "Prompt de prueba"`
**When** se llama `process_message()`
**Then** el mensaje de sistema enviado al LLM es "Prompt de prueba"

#### Scenario: process_message_stream usa el prompt de la BD
**Given** un orquestador con `settings.system_prompt = "Prompt de prueba"`
**When** se llama `process_message_stream()`
**Then** el mensaje de sistema enviado al LLM es "Prompt de prueba"

#### Scenario: Fallback mínimo si falta el prompt
**Given** un orquestador cuya tabla `settings` no tiene `system_prompt`
**When** se construye la petición al LLM
**Then** se usa un fallback mínimo genérico no vacío
**And** se loguea un warning indicando que falta el prompt