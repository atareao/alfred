# Change Proposal: Collapse Worker

## Why
Cuando un mensaje supera el umbral de tokens configurable, su contenido debe ser resumido (colapsado) usando un LLM económico en background. Esto permite mantener la ventana de contexto del asistente manejable sin perder información importante.

## What Changes
1. Nuevo worker `CollapseWorker` que escucha un canal `mpsc` con IDs de mensajes a colapsar
2. Nueva configuración `collapse_model` (env `COLLAPSE_MODEL`, default `mistralai/mistral-small`)
3. Nueva setting `collapse_prompt` en DB para el prompt de resumen (configurable)
4. Conexión del callback `on_collapse_needed` en el handler de creación de mensajes
5. Integración del worker en `WorkerPool`
6. Actualización de `AppState` para incluir el canal de collapse

## Impacto
- No rompe API existente
- Los mensajes largos se colapsan asíncronamente en background
- El prompt de collapse es configurable via settings
- El modelo de collapse es configurable via env var