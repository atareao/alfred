# Tasks

- [x] Observar el defecto ejecutando `just deploy` (salida con `}}` al final del ID de imagen).
- [x] Aplicar el arreglo de las tres líneas en el working tree del `.justfile`.
- [x] Verificar que `just --list` parsea las 16 recetas.
- [x] Verificar que la interpolación de los tres formatos produce `{{.Image}}`, `{{.Id}}` y `{{.Config.Image}}`.
- [x] Verificar que los dos `podman inspect` contra el contenedor real devuelven el mismo sha256 sin llaves.
- [x] Crear este change proposal (tarde, después del arreglo).
- [ ] Obtener la aprobación del spec por parte de quien usa el repo.
- [ ] Commit y PR a `development` con los checks del CI en verde, y merge.
- [ ] Archivar el change con `openspec archive`.

## Resultado

Pendiente de completar al terminar el ciclo (aprobación, PR y archivado).
