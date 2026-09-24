# SQL Window History — Replace SessionWindow with SQL Window Function

## Intent

Reemplazar el `SessionWindow` (estructura en memoria con `VecDeque<Message>` y
estimación heurística de tokens) por una query SQL con ventana que seleccione
los mensajes del historial usando `tokens_count` reales almacenados en DB.

## Scope

- **Añadir**: `MessagesRepo::list_by_token_budget()` — query SQL con `SUM OVER`
  que selecciona mensajes hasta un presupuesto de tokens.
- **Modificar**: `Orchestrator::process_message()` y
  `Orchestrator::process_message_stream()` — usar `list_by_token_budget()` en
  lugar de `session_window.get_window()`.
- **Eliminar**: `SessionWindow` struct, `session_window.rs`, y todas sus
  referencias en el orquestador.
- **Conservar**: El setting `max_window_tokens` en DB (ahora controla el
  presupuesto de tokens de la query).

## Impact

| Archivo | Cambio |
|---------|--------|
| `src/db/repos/messages.rs` | + método `list_by_token_budget()` |
| `src/orchestrator/agent.rs` | - referencias a `SessionWindow`, + llamada a `list_by_token_budget()` |
| `src/orchestrator/mod.rs` | - `session_window` module export |
| `src/orchestrator/session_window.rs` | **Eliminar** archivo completo |
| `src/lib.rs` o `src/main.rs` | - construcción de `SessionWindow` |
| `tests/api/chat.rs` | Posible ajuste si usa `SessionWindow` |

## Efectos colaterales

- El orquestador ya no necesita `Arc<Mutex<SessionWindow>>` — se simplifica
  el estado compartido.
- La ventana de historial ahora usa tokens reales en lugar de heurística.
- Mensajes colapsados (con `collapsed_tokens_count`) ocupan menos espacio
  automáticamente.
- Se eliminan ~200 líneas de código de gestión de estado en memoria.