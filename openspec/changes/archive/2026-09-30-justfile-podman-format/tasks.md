# Tasks

- [x] Observar el defecto ejecutando `just deploy` (salida con `}}` al final del ID de imagen).
- [x] Aplicar el arreglo de las tres líneas en el working tree del `.justfile`.
- [x] Verificar que `just --list` parsea las 16 recetas.
- [x] Verificar que la interpolación de los tres formatos produce `{{.Image}}`, `{{.Id}}` y `{{.Config.Image}}`.
- [x] Verificar que los dos `podman inspect` contra el contenedor real devuelven el mismo sha256 sin llaves.
- [x] Crear este change proposal (tarde, después del arreglo).
- [x] Obtener la aprobación del spec por parte de quien usa el repo.
- [x] Commit y PR a `development` con los checks del CI en verde, y merge.
- [x] Archivar el change con `openspec archive`.

## Resultado

- El defecto se detectó ejecutando `just deploy` de verdad, no en revisión: la salida mostraba `▶ Imagen en ejecución: 6b559ed5...a3e03c29e9a6}}`, con dos llaves adheridas.
- Causa aislada de forma empírica: en `just`, `{{{{` se colapsa a `{{` pero `}}}}` **no** se colapsa, así que `'{{{{.Image}}}}'` llega a podman como `{{.Image}}}}`. Comprobado con un justfile de prueba que imprimía las tres formas de escape, todas equivalentes salvo la problemática.
- Impacto real acotado: la comparación de IDs **no** se rompía, porque ambos lados arrastraban las mismas llaves. Era un defecto de presentación y de mensajes de error. Se dice así a propósito, para no inflar la gravedad.
- Arreglo: los tres formatos se declaran una sola vez como variables de `just` (`podman_fmt_image`, `podman_fmt_id`, `podman_fmt_config_image`) y se interpolan, de modo que no queda ninguna llave literal en las recetas.
- Verificado sin reiniciar el contenedor: `just --list` parsea las 16 recetas, la interpolación produce exactamente `{{.Image}}`, `{{.Id}}` y `{{.Config.Image}}`, y los dos `podman inspect` contra el contenedor en marcha devuelven el mismo sha256 `6b559ed5d3ede07293ca7e823a60106e511df594b3b630789625a3e03c29e9a6`, sin llaves.
- PR #47 mergeado a `development` (`5183c5d`) con los checks en verde: `Backend (Rust)` 56s y `Frontend (Node)` 52s.
- Archivado con `openspec archive --yes`, que fusionó el delta en `openspec/specs/infra/spec.md`: de 14 a **16 requisitos** y de 30 a **35 escenarios**, con los headers coincidiendo carácter por carácter. `openspec archive` avisó de que el proposal no usa los headers `## Why` y `## What Changes` del esquema nuevo; es una advertencia no bloqueante y la convención del repo es mixta (`Intent`/`Scope`/`Impact` lo usan los proposals recientes, incluido el de `ci-cd`).
- Verificación de la protección de rama, de paso y de forma empírica: con los checks pendientes, GitHub respondió `Pull request atareao/valet-ai#47 is not mergeable: the base branch policy prohibits the merge`; con los checks en verde, el merge pasó.
- NO se ha vuelto a ejecutar `just deploy` completo tras el arreglo: los formatos y los IDs están verificados pieza a pieza contra el contenedor real, pero el recorrido completo de la receta no se repitió para no reiniciar el servicio del usuario por segunda vez en la sesión.
- Hallazgo que este trabajo dejó al descubierto y que NO se toca aquí: al recrear el contenedor, podman avisó `StopSignal SIGTERM failed to stop container valet_valet_1 in 10 seconds, resorting to SIGKILL`. La aplicación no atiende `SIGTERM`, así que cada despliegue es un apagado brusco. Es deuda ajena a este cambio.
- Nota de protocolo, sin adornos: el `.justfile` se editó **antes** de comprobar `just check-spec`, saltándose la regla de oro del repo. El change proposal se escribió a posteriori y lo documenta en su sección `Orden de los hechos`. Un spec redactado después del arreglo documenta, pero no previene.
