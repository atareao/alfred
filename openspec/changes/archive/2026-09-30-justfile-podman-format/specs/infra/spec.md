## ADDED Requirements

### Requirement: Formato de inspección de imágenes sin caracteres sobrantes

Las recetas de despliegue DEBEN obtener el ID de la imagen en ejecución, el ID de la imagen publicada y el nombre de la imagen mediante formatos de `podman inspect` que se resuelvan exactamente a `{{.Image}}`, `{{.Id}}` y `{{.Config.Image}}`, sin ninguna secuencia de llaves sobrante.

La causa del defecto y su razón de ser: en `just`, `{{{{` se colapsa a `{{` pero `}}}}` no se colapsa, así que un formato escrito como `'{{{{.Image}}}}'` llega a podman como `{{.Image}}}}` y podman imprime el valor con dos llaves adheridas. El formato DEBE declararse una sola vez como variable de `just` e interpolarse, para que no haya llaves literales en las recetas.

#### Scenario: La interpolación produce el formato limpio

- **Given** una variable de `just` declarada como `podman_fmt_image := '{{.Image}}'`
- **When** una receta la interpola con `--format '{{ podman_fmt_image }}'`
- **Then** el argumento que recibe `podman` es exactamente `{{.Image}}`, sin llaves añadidas

#### Scenario: El ID se imprime sin llaves adheridas

- **Given** un contenedor en ejecución con una imagen publicada
- **When** la receta imprime el ID de la imagen en ejecución
- **Then** la salida es un `sha256` limpio, sin ninguna llave al final

#### Scenario: El `.justfile` no contiene llaves de cierre duplicadas en las recetas

- **Given** el `.justfile` completo
- **When** se inspeccionan las líneas de las recetas
- **Then** ninguna línea de receta contiene la secuencia de cuatro llaves de cierre

### Requirement: La comprobación de imagen sigue detectando discordancias

La receta `deploy` DEBE seguir fallando cuando el contenedor en ejecución no corresponde a la imagen publicada, y DEBE seguir pasando cuando coinciden. El arreglo del formato NO DEBE alterar esta lógica.

#### Scenario: El contenedor usa la imagen publicada

- **Given** un contenedor recreado a partir de la imagen publicada
- **When** se comparan ambos IDs
- **Then** coinciden y la receta continúa hacia la verificación de salud

#### Scenario: El contenedor usa otra imagen

- **Given** un contenedor cuya imagen en ejecución no es la publicada
- **When** se comparan ambos IDs
- **Then** la receta falla, muestra ambos IDs limpios y no continúa
