# Proposal: Estimador determinista de tokens JSON

## Why

La Capa C mide su tamaño con `estimate_markdown_tokens_heuristic`, pensado para prosa: cuenta
palabras y símbolos sueltos y subestima las cadenas largas (una URL o un blob sin espacios
cuenta como una sola «palabra»). El presupuesto de tokens y su techo absoluto se apoyan en esa
medida, así que una subestimación compromete la única cota dura de la sección inyectada. No se
quiere un tokenizer real: pesa, arrastra vocabulario y depende de red; y como Valet habla con
varios proveedores, ninguna medida sería exacta para todos.

## What Changes

- **D1 — Estimador propio y determinista.** Nueva función pura
  `estimate_json_tokens(json: &str) -> usize` que recorre el JSON como JSON: símbolos
  estructurales (`{}[]:,`), cadenas (con ratio distinta para ASCII y no-ASCII), literales
  (`true`/`false`/`null`) y números. Sin dependencias, sin red y sin I/O.
- **D2 — La Capa C mide con él.** `payload_token_count` deja de usar el heurístico de markdown
  y usa `estimate_json_tokens` sobre el JSON minificado. La medición de los mensajes de chat NO
  se toca.
- **D3 — Calibración con datos reales.** Comparar la estimación con los tokens reales que ya
  registran las stats del proveedor (`llm_requests`) sobre muestras representativas, y ajustar
  los ratios si la desviación es grande. Procedimiento documentado, sin dependencias.
- **D4 — Recalibración del presupuesto.** Comprobar si el default `PERSISTENT_MEMORY_BUDGET_TOKENS`
  (500) sigue siendo razonable con el nuevo medidor y ajustarlo si procede.

### Fuera de alcance

- Meter un tokenizer real (`tiktoken-rs` u otro): descartado por peso, vocabulario y porque
  Valet usa varios proveedores.
- Cambiar el medidor de los mensajes de chat.

## Capabilities

### New Capabilities

- `json-token-estimator`: la estimación determinista de tokens de una cadena JSON.

### Modified Capabilities

- (ninguna)

## Impact

- **Specs**: `json-token-estimator` (nueva).
- **Código**: nuevo módulo del estimador; `src/persistent_memory.rs` (`payload_token_count`).
- **Sin migración**, sin cambios en la API ni en `docker-compose.prod.yml`.
