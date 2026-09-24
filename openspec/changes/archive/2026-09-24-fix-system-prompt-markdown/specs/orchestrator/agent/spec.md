# Orchestrator Agent: System Prompt

## ADDED Requirements

### Requirement: System prompt template uses Markdown and emojis
**Given** la configuración por defecto del orquestador  
**When** se carga `OrchestratorConfig::default()`  
**Then** el `system_prompt_template` contiene instrucciones de Markdown  
**And** contiene "negritas", "cursivas", "Emojis" y "Listas"

#### Scenario: System prompt incluye formato Markdown
**Given** la configuración por defecto  
**When** se accede a `system_prompt_template`  
**Then** contiene "**negritas**"  
**And** contiene "*cursivas*"  
**And** contiene "Emojis"  
**And** contiene "Listas con viñetas"