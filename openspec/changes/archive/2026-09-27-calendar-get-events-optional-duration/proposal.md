# Change proposal: calendar-get-events-optional-duration

## Why
El LLM llama a `get_events` con solo `start` (ej: `"2026-09-27T00:00:00+02:00"`) porque mentalmente eso ya expresa "los eventos de ese día". Pero la herramienta requiere `duration`, lo que provoca el error `Invalid arguments: Missing duration`. El fix hace `duration` opcional con default 1440 minutos (24h).

## What Changes
- `get_events`: `duration` pasa de requerido a opcional, default 1440 minutos
- `parameters()`: descripción de `duration` actualizada indicando que es opcional en `get_events`
- Tests: se añaden casos sin `duration` para verificar el default