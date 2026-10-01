# ui-feedback Specification

## Purpose
TBD - created by archiving change antd-message-context. Update Purpose after archive.

## Requirements

### Requirement: La raíz provee el contexto de avisos de antd

`src/App.tsx` SHALL envolver el árbol de la aplicación con el componente `<App>` de antd, dentro de
`ConfigProvider`, de modo que `App.useApp()` esté disponible en cualquier componente descendiente.

**Given** la aplicación montada
**When** un componente descendiente invoca `App.useApp()`
**Then** SHALL recibir una implementación funcional de `message`
**And** NO SHALL emitirse el aviso de antd sobre funciones estáticas sin contexto

#### Scenario: Un descendiente obtiene el contexto

**Given** un componente consumidor montado dentro de `<App>`
**When** invoca `App.useApp()`
**Then** la instancia de `message` SHALL ser funcional
**And** SHALL poder mostrar un aviso sin errores

### Requirement: Los avisos se muestran mediante la API contextual

Ningún módulo de `src/` SHALL usar la API estática de antd (`message.*`, `notification.*`).
`SettingsDialog`, `RetentionConfig`, `EventModal` y `TaskModal` SHALL obtener `message` de
`App.useApp()`.

**Given** el código fuente de `src/`
**When** se buscan llamadas a la API estática de antd
**Then** NO SHALL existir ninguna

#### Scenario: `SettingsDialog` avisa por el contexto

**Given** `SettingsDialog` montado dentro de `<App>`
**When** se guarda el perfil con éxito
**Then** SHALL mostrarse el aviso "Perfil actualizado"
**And** SHALL mostrarse el error "Error al actualizar perfil" cuando el guardado falla

#### Scenario: `RetentionConfig` avisa por el contexto

**Given** `RetentionConfig` montado dentro de `<App>`
**When** la carga de la configuración falla
**Then** SHALL mostrarse el aviso "Failed to load retention config"
**And** SHALL mostrarse "Retention config updated" cuando el guardado tiene éxito

#### Scenario: `EventModal` avisa por el contexto

**Given** `EventModal` montado dentro de `<App>` y la creación fallando
**When** se envía el formulario
**Then** SHALL mostrarse el texto del error en el DOM
**And** el módulo NO SHALL importar la API estática de antd (lo verifica el lint)

#### Scenario: `TaskModal` avisa por el contexto

**Given** `TaskModal` montado dentro de `<App>` y la creación fallando
**When** se envía el formulario
**Then** SHALL mostrarse el texto del error en el DOM
**And** SHALL mostrarse "Task created" cuando la creación tiene éxito
**And** el módulo NO SHALL importar la API estática de antd

### Requirement: Un solo idioma para mostrar avisos

SHALL NOT quedar en `src/` ningún uso del patrón por componente `message.useMessage()` con su
`contextHolder`: todos los avisos SHALL obtenerse de `App.useApp()`. Así la importación de `message`
desde `antd` deja de ser necesaria en cualquier fichero y la regla de lint puede prohibirla **sin
excepciones**, que es lo que la convierte en una garantía y no en una recomendación.

#### Scenario: Sin `useMessage()` ni `contextHolder`

**Given** el código de `src/`
**When** se buscan `useMessage()` y `contextHolder`
**Then** NO SHALL existir ninguna coincidencia

#### Scenario: La regla no necesita excepciones

**Given** la regla `no-restricted-imports` activa sobre todos los `ts/tsx`
**When** se ejecuta `npx eslint .` sobre el árbol real
**Then** NO SHALL haber ningún error de `no-restricted-imports`

### Requirement: Los textos de los avisos no cambian

Los avisos SHALL conservar exactamente los mismos textos que antes del change: "Perfil actualizado",
"Error al actualizar perfil", "Ajustes guardados", "Error al guardar ajustes", "Valores por defecto
restaurados", "Error al restaurar valores", "Failed to load retention config", "Retention config
updated", "Failed to save retention config" y el mensaje dinámico de error de `EventModal`.

#### Scenario: Los textos se mantienen

**Given** el diff del change
**When** se comparan las cadenas de los avisos con las anteriores
**Then** NO SHALL haber cambiado ninguna

### Requirement: Los avisos se limpian al desmontar

Los temporizadores de los avisos SHALL quedar limpios al desmontar el componente que los originó. Al
terminar la suite completa NO SHALL quedar ningún temporizador de 1 segundo o más vivo.

#### Scenario: Sin temporizadores huérfanos

**Given** la suite completa ejecutada con el instrumento de recuento de temporizadores
**When** la ejecución termina
**Then** SHALL ser 0 el número de temporizadores largos vivos en todos los ficheros

### Requirement: Las pruebas no espían la API estática de antd

Ningún fichero de prueba SHALL espiar ni neutralizar `message` ni `notification` de antd. Las
aserciones sobre avisos SHALL hacerse sobre el DOM.

Prohibir por lint la importación de la API estática hace imposible mantener el espía: para espiar
`message.error` hay que importarlo, e importarlo es un error. La garantía deja de ser una aserción
en tiempo de ejecución y pasa a ser estática, que es más fuerte: si alguien vuelve a la API estática,
el lint lo tumba antes de que las pruebas lleguen a ejecutarse.

#### Scenario: Sin espías sobre la API de antd

**Given** los ficheros de prueba tras el change
**When** se busca `spyOn(message` en ellos
**Then** NO SHALL existir ninguna coincidencia
**And** las aserciones sobre avisos SHALL consultar el texto en el DOM

#### Scenario: El espía deja de ser posible

**Given** la regla `no-restricted-imports` activa
**When** un fichero de prueba intenta importar `message` desde `antd`
**Then** SHALL ser un error de lint

### Requirement: El lint impide reintroducir la API estática

La configuración de ESLint SHALL restringir, en cualquier fichero y pruebas incluidas: la importación
de `message` y `notification` desde `antd` (por nombre o con alias), su reexportación, las
importaciones profundas (`antd/es/*`, `antd/lib/*`) y el uso de los estáticos de `Modal`
(`Modal.confirm`, `Modal.info`, `Modal.success`, `Modal.error`, `Modal.warning`, `Modal.destroy`),
que tienen la misma fuga de temporizador que motivó este change.

**Límite conocido y aceptado.** La regla no cubre la importación de espacio de nombres ni la
importación dinámica (`import * as antd from "antd"`, `await import("antd")`, `require("antd")`):
`no-restricted-imports` solo visita declaraciones de importación y exportación. Esquivarla por esa
vía exige escribir código deliberadamente ofuscado, no es el camino natural de una regresión, y no
se persigue. El objetivo no es que la vuelta atrás sea imposible en el sentido matemático, sino que
**no ocurra por descuido** — que es como ocurriría de verdad.

#### Scenario: Reimportar la API estática rompe el lint

**Given** un fichero que importa `message` desde `antd`
**When** se ejecuta `npx eslint .`
**Then** SHALL reportarse un error de `no-restricted-imports`
**And** el job `Frontend (Node)` SHALL fallar

#### Scenario: Los estáticos de `Modal` rompen el lint

**Given** un fichero que llama a `Modal.confirm(...)`
**When** se ejecuta `npx eslint .`
**Then** SHALL reportarse un error de `no-restricted-syntax`
**And** el uso contextual (`App.useApp().modal.confirm`) SHALL seguir permitido

#### Scenario: Las importaciones profundas rompen el lint

**Given** un fichero que importa desde `antd/es/*` o `antd/lib/*`
**When** se ejecuta `npx eslint .`
**Then** SHALL reportarse un error de `no-restricted-imports`

#### Scenario: El árbol real pasa

**Given** el código tras el change
**When** se ejecuta `npx eslint .`
**Then** NO SHALL haber errores de `no-restricted-imports`
