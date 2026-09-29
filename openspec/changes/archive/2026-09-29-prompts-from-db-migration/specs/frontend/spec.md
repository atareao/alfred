# Spec Delta: frontend

## MODIFIED Requirements

### Requirement: SettingsDialog SHALL display Prompt tab

SettingsDialog SHALL display a "Prompts" tab with three sub-tabs, one per editable prompt: System, Archivist and Collapse.

**Given** el SettingsDialog está abierto en la tab "Prompts"
**When** se renderiza
**Then** muestra tres sub-pestañas: "System", "Archivist" y "Collapse"
**And** cada sub-pestaña muestra un TextArea de 10 filas con el valor actual de `system_prompt`, `archivist_prompt` y `collapse_prompt` respectivamente
**And** los valores se cargan desde `GET /settings`
**And** un botón "Guardar" persiste los tres valores vía `PUT /settings`

#### Scenario: Las tres sub-pestañas están presentes
**Given** el SettingsDialog está abierto en la tab "Prompts"
**When** se renderiza
**Then** existen las sub-pestañas "System", "Archivist" y "Collapse"
**And** al hacer clic en cada una se muestra su TextArea correspondiente

#### Scenario: System prompt se carga desde la BD
**Given** `GET /settings` devuelve `system_prompt = "Eres Valet"`
**When** se abre la sub-pestaña "System"
**Then** el TextArea muestra "Eres Valet"

#### Scenario: Archivist prompt se carga desde la BD
**Given** `GET /settings` devuelve `archivist_prompt = "Eres un archivista"`
**When** se abre la sub-pestaña "Archivist"
**Then** el TextArea muestra "Eres un archivista"

#### Scenario: Collapse prompt se carga desde la BD
**Given** `GET /settings` devuelve `collapse_prompt = "Resume el texto"`
**When** se abre la sub-pestaña "Collapse"
**Then** el TextArea muestra "Resume el texto"

#### Scenario: Los tres prompts se guardan
**Given** el usuario edita los tres TextAreas
**When** hace clic en "Guardar"
**Then** `updateSettings` se llama con `{ system_prompt, archivist_prompt, collapse_prompt }`
**And** se muestra mensaje "Ajustes guardados"

#### Scenario: Prompt se guarda
**Given** el usuario escribe "Eres un asistente útil" en el TextArea "System"
**When** hace clic en "Guardar"
**Then** `updateSettings` se llama con `{ system_prompt: "Eres un asistente útil", ... }`

#### Scenario: Editar un prompt no borra los otros
**Given** el usuario modifica solo el TextArea "Archivist"
**When** hace clic en "Guardar"
**Then** `system_prompt` y `collapse_prompt` se envían con sus valores actuales sin cambios

### Requirement: SettingsDialog SHALL unify profile and settings buttons

SettingsDialog SHALL open when clicking the settings icon in the header, replacing the separate profile and settings drawers.

**Given** el header tiene un botón con icono `SettingOutlined`
**When** el usuario hace clic en él
**Then** se abre un Modal titulado "⚙️ Settings" con tabs: Perfil, Interfaz, Prompts, API Keys
**And** el botón `UserOutlined` ya no existe en el header

#### Scenario: Dialog closes
**Given** el SettingsDialog está abierto
**When** el usuario hace clic en X o fuera del modal
**Then** el modal se cierra

#### Scenario: Modal con tabs funcionales
**Given** el SettingsDialog está abierto
**When** el usuario hace clic en la tab "API Keys"
**Then** se muestra el contenido de API Keys
**And** las otras tabs no están visibles