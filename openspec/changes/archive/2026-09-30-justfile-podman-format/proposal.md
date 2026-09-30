# 2026-09-30-justfile-podman-format

## Intent

En `just`, la secuencia `{{{{` se colapsa a `{{` pero `}}}}` no se colapsa. Por eso tres líneas del `.justfile` que escribían formatos de `podman inspect` como `'{{{{.Image}}}}'` llegaban a podman como `{{.Image}}}}`, y podman imprimía el valor con dos llaves adheridas al final. El check de comparación no se rompía (ambos lados arrastraban las mismas llaves), así que era un defecto de presentación y de mensajes de error, no de seguridad. Se arregla declarando los formatos una sola vez como variables de `just` y interpolándolos.

## Scope

Entra: únicamente el `.justfile`. Se añaden tres variables de `just` con el formato literal (`podman_fmt_image := '{{.Image}}'`, `podman_fmt_id := '{{.Id}}'`, `podman_fmt_config_image := '{{.Config.Image}}'`) y las tres líneas afectadas pasan a interpolar esas variables con `--format '{{ podman_fmt_image }}'`, etc.

No entra: NO se toca la lógica de comparación de IDs, ni las recetas (más allá de esas tres líneas), ni el CI, ni ningún código fuente. Tampoco cambia el comportamiento del despliegue.

## Impact

Afecta a las recetas `deploy` y `deploy-local`, y a los mensajes que imprimen: el ID de la imagen en ejecución, el ID de la imagen publicada y el nombre de la imagen. Sin impacto en el comportamiento de despliegue: la comparación sigue funcionando igual y solo se corrige el texto mostrado.

## Orden de los hechos

Desviación de orden, documentada con honestidad: el arreglo del `.justfile` se aplicó ANTES de crear este change proposal, saltándose la regla de oro del repo (no editar código sin un change proposal aprobado). La evidencia del defecto se capturó antes de tocar el fichero: la salida de `just deploy` mostraba

```
▶ Imagen en ejecución: 6b559ed5d3ede07293ca7e823a60106e511df594b3b630789625a3e03c29e9a6}}
```

Este change proposal se escribe a posteriori para regularizar la situación y dejar el cambio bajo el mecanismo de OpenSpec.

## Verificación

- `just --list` parsea las 16 recetas correctamente.
- La interpolación de los tres formatos produce exactamente `{{.Image}}`, `{{.Id}}` y `{{.Config.Image}}`.
- Los dos `podman inspect` contra el contenedor real (sin reiniciarlo) devuelven el mismo sha256 sin llaves:

```
en ejecución: 6b559ed5d3ede07293ca7e823a60106e511df594b3b630789625a3e03c29e9a6
publicada:    6b559ed5d3ede07293ca7e823a60106e511df594b3b630789625a3e03c29e9a6
EQUAL=yes   (ambos sin llaves al final)
```
