### CalendarTool.get_events: duration opcional

```rust
// Antes (requerido):
let duration = args
    .get("duration")
    .and_then(|v| v.as_i64())
    .ok_or_else(|| ToolError::InvalidArguments("Missing duration".into()))?;

// Después (opcional, default 1440):
let duration = args
    .get("duration")
    .and_then(|v| v.as_i64())
    .unwrap_or(1440);
```

### Scenarios

**Happy path — con duration explícito**
- Given: `get_events` con `start: "2026-09-27T00:00:00Z"` y `duration: 60`
- When: se ejecuta
- Then: devuelve eventos entre las 00:00 y 01:00 del 2026-09-27

**Happy path — sin duration (default 24h)**
- Given: `get_events` con `start: "2026-09-27T00:00:00+02:00"` sin `duration`
- When: se ejecuta
- Then: devuelve eventos con default duration=1440 (24h desde start)

**Edge — start con timezone offset**
- Given: `get_events` con `start: "2026-09-27T00:00:00+02:00"` sin `duration`
- When: se ejecuta
- Then: parsea correctamente el offset con `DateTime::parse_from_rfc3339` y aplica default 1440