## ADDED Requirements

### Requirement: SettingsDialog SHALL display Perfil tab

SettingsDialog SHALL display a "Perfil" tab with a form to edit user profile (name and avatar URL).

**Given** el SettingsDialog está abierto en la tab "Perfil"
**When** se renderiza
**Then** muestra un formulario con campos "Nombre" (Input) y "Avatar URL" (Input)
**And** los valores iniciales se cargan desde `useProfile()`

#### Scenario: Perfil se guarda correctamente
**Given** el usuario ha modificado "Nombre" a "Juan"
**When** hace clic en "Guardar"
**Then** se llama a `profile.updateProfile({ name: "Juan" })`
**And** se muestra mensaje "Perfil actualizado"
**And** el diálogo se cierra

#### Scenario: Error al guardar perfil
**Given** `updateProfile` lanza un error
**When** el usuario guarda
**Then** se muestra mensaje "Error al actualizar perfil"
**And** el diálogo permanece abierto

### Requirement: SettingsDialog SHALL display Interfaz tab

SettingsDialog SHALL display an "Interfaz" tab with font size, context window, and page size controls.

**Given** el SettingsDialog está abierto en la tab "Interfaz"
**When** se renderiza
**Then** muestra:
- "Tamaño de fuente" (InputNumber, min 12, max 24)
- "Ventana de contexto (tokens)" (InputNumber, min 1000, max 100000)
- "Tamaño de página" (InputNumber, min 10, max 100)

#### Scenario: Interfaz se guarda correctamente
**Given** el usuario cambia font_size a 18
**When** hace clic en "Guardar"
**Then** se llama a `updateSettings` con los valores actualizados
**And** se muestra mensaje "Ajustes guardados"
**And** el diálogo se cierra

### Requirement: SettingsDialog SHALL display Prompt tab

SettingsDialog SHALL display a "Prompt" tab with a textarea for the custom system prompt.

**Given** el SettingsDialog está abierto en la tab "Prompt"
**When** se renderiza
**Then** muestra un TextArea de 10 filas con el system_prompt actual
**And** un placeholder con el prompt por defecto si está vacío

#### Scenario: Prompt se guarda
**Given** el usuario escribe "Eres un asistente útil" en el TextArea
**When** hace clic en "Guardar"
**Then** `updateSettings` se llama con `{ system_prompt: "Eres un asistente útil", ... }`

### Requirement: SettingsDialog SHALL display API Keys tab

SettingsDialog SHALL display an "API Keys" tab with password fields for API keys.

**Given** el SettingsDialog está abierto en la tab "API Keys"
**When** se renderiza
**Then** muestra tres campos Input.Password:
- "OpenWeatherMap API Key"
- "Google Places API Key"
- "Brave Search API Key"

#### Scenario: API Keys se guardan
**Given** el usuario introduce una API key de OpenWeatherMap
**When** guarda
**Then** `updateSettings` se llama con la API key incluida

### Requirement: SettingsDialog SHALL unify profile and settings buttons

SettingsDialog SHALL open when clicking the settings icon in the header, replacing the separate profile and settings drawers.

**Given** el header tiene un botón con icono `SettingOutlined`
**When** el usuario hace clic en él
**Then** se abre un Modal titulado "⚙️ Settings" con tabs: Perfil, Interfaz, Prompt, API Keys
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

### Requirement: SettingsDialog SHALL handle loading and error states

#### Scenario: Restaurar valores por defecto
**Given** el usuario ha modificado valores en Interfaz
**When** hace clic en "Restaurar valores por defecto"
**Then** todos los campos vuelven a sus valores default
**And** se muestra mensaje "Valores por defecto restaurados"