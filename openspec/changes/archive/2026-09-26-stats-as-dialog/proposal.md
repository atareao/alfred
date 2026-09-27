# Stats como diálogo Modal

## Intento
Convertir la página de estadísticas (`/stats`) en un diálogo Modal,
exactamente igual que Agenda y Tasks. Al hacer clic en el icono de
gráfico de barras del header, se abre un Modal con el dashboard de
stats en lugar de navegar a una ruta separada.

## Alcance
Solo frontend — no cambia backend ni API.

### Archivos a modificar
- `frontend/src/App.tsx` — eliminar ruta `/stats`, eliminar import de StatsDashboard
- `frontend/src/components/AppLayout.tsx` — añadir estado `statsVisible`, cambiar
  `navigate("/stats")` por `setStatsVisible(true)`, añadir Modal con
  `<StatsDashboard />`, importar StatsDashboard

### Archivos a eliminar
- Ninguno

## Impacto
- Se elimina la navegación a `/stats`
- Stats ahora se comporta como diálogo (misma interacción que Agenda/Tasks)
- StatsDashboard se renderiza dentro de un Modal, heredando el fondo oscuro
  del Layout, pero con su propio padding y scroll si es necesario
- El icono de stats sigue en el mismo lugar del header