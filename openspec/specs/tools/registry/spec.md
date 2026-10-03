# tools/registry Specification

## Purpose
El catálogo de herramientas que el orquestador de Valet ofrece al LLM: qué herramientas existen,
cuáles se exponen en cada petición y en qué condiciones se permite su ejecución.

## Requirements

### Requirement: El registry SHALL registrar las herramientas integradas notes y unified_search

La construcción del registry de producción SHALL registrar `notes` y `unified_search` además del
resto de herramientas integradas, garantizando nombres únicos.

#### Scenario: El registry de producción incluye las 12 herramientas

**Given** la aplicación Valet construida con su registro de herramientas de producción
**When** se consulta el catálogo de herramientas registradas
**Then** `notes` y `unified_search` figuran entre ellas
**And** no hay dos herramientas con el mismo nombre

### Requirement: El registry SHALL omitir las herramientas deshabilitadas de las definiciones

`definitions()` SHALL devolver únicamente las herramientas cuyo estado sea habilitado.

#### Scenario: Una herramienta deshabilitada no se ofrece al LLM

**Given** un registry con las herramientas `weather` y `tasks`, y `weather` deshabilitada
**When** se solicitan las definiciones de herramientas
**Then** la lista contiene `tasks`
**And** la lista NO contiene `weather`

### Requirement: El registry SHALL rechazar la ejecución de herramientas deshabilitadas

`execute(nombre, args)` SHALL devolver un error para una herramienta deshabilitada, aunque esté
registrada, sin invocarla.

#### Scenario: La ejecución de una herramienta deshabilitada se rechaza

**Given** un registry con `weather` deshabilitada
**When** se invoca `execute("weather", args)`
**Then** se devuelve `Err(ToolError)`
**And** la herramienta NO se ejecuta

### Requirement: El estado habilitado SHALL proceder de la tabla tools y actualizarse en caliente

El registry SHALL cargar el estado habilitado desde la tabla `tools` al arrancar y SHALL reflejar de
inmediato las habilitaciones y deshabilitaciones disparadas desde la API de administración.

#### Scenario: Deshabilitar una herramienta surte efecto sin reiniciar

**Given** un registry con `weather` habilitada
**When** se deshabilita `weather` a través de la API de administración
**Then** las definiciones posteriores omiten `weather`
**And** `execute("weather", args)` devuelve error

#### Scenario: Habilitar de nuevo una herramienta la reexpone

**Given** un registry con `weather` deshabilitada
**When** se habilita `weather` a través de la API de administración
**Then** las definiciones posteriores vuelven a contener `weather`
**And** `execute("weather", args)` vuelve a invocarla

### Requirement: La interfaz Tool SHALL declarar el permiso en función de los argumentos

`Tool::permission(&self, args: &Value) -> Permission` SHALL recibir los argumentos de la llamada para
que las tools multioperación distingan la operación. El registry SHALL exponer
`permission(name, args)`. Las tools que no distinguen por operación SHALL ignorar `args` y devolver
siempre el mismo permiso.

**Contracts:**

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Value;
    fn permission(&self, args: &Value) -> Permission; // antes: sin argumentos
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError>;
}

impl ToolRegistry {
    pub fn permission(&self, name: &str, args: &Value) -> Option<Permission>;
}
```

#### Scenario: Una operación de lectura no requiere confirmación

**Given** un registry con una tool multioperación
**When** se consulta su permiso con unos args que seleccionan una operación de lectura
**Then** el permiso es `NoConfirm`

#### Scenario: Una operación de borrado requiere confirmación

**Given** un registry con una tool multioperación
**When** se consulta su permiso con unos args que seleccionan una operación de borrado
**Then** el permiso es `ExplicitApproval`

#### Scenario: Una tool sin operaciones mantiene su permiso

**Given** un registry con `web_search`, que no distingue operaciones
**When** se consulta su permiso con args cualesquiera
**Then** el permiso es el mismo que declaraba, con independencia de los args

### Requirement: Las operaciones destructivas SHALL requerir aprobación explícita

Toda operación de borrado de una tool multioperación SHALL declarar `ExplicitApproval`. En concreto,
la tool `tasks` SHALL devolver `ExplicitApproval` para `delete_task` y `NoConfirm` para el resto de
sus operaciones.

#### Scenario: delete_task requiere aprobación explícita

**Given** un registry con la tool `tasks`
**When** se consulta su permiso con `{"operation": "delete_task", "id": "..."}`
**Then** el permiso es `ExplicitApproval`

#### Scenario: list_tasks no requiere aprobación

**Given** un registry con la tool `tasks`
**When** se consulta su permiso con `{"operation": "list_tasks"}`
**Then** el permiso es `NoConfirm`
