# Fix tools_used Persistence

## Intent
Alinear el proceso de tools_used con el de location: resolverlo y persistirlo en BD antes de enviarlo al frontend, tanto en el flujo del orquestador como en la API REST.

## Problema actual
- CreateMessage struct no tiene campo tools_used, por lo que la API REST POST /api/messages siempre lo ignora (pasa None)
- La construccion de tools_used en el orquestador es inline en lugar de tener un metodo dedicado como resolve_location()

## Scope
- src/models/message.rs: anadir tools_used a CreateMessage
- src/handlers/messages.rs: pasar tools_used a MessagesRepo::create
- frontend/src/types/index.ts: anadir tools_used a CreateMessage TS interface

## Impacto
- La API REST ahora aceptara tools_used opcionalmente
- El orquestador mantiene su logica actual (ya funciona correctamente segun las pruebas)
- Frontend puede crear mensajes con tools_used via API