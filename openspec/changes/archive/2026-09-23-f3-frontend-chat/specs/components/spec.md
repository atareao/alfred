# Components Spec — f3-frontend-chat

## ADDED: Sidebar layout

```
┌──────────────┬──────────────────────────────┐
│  Sider       │  Content                     │
│              │                              │
│  ┌────────┐  │  ┌─────────────────────────┐ │
│  │ 💬     │  │  │    ChatView             │ │
│  │ Alfred │  │  │                         │ │
│  ├────────┤  │  │   MessageBubble (user)  │ │
│  │ 🌤️     │  │  │   MessageBubble (asst)  │ │
│  │ Tiempo │  │  │   MessageBubble (system)│ │
│  │ ...    │  │  │                         │ │
│  ├────────┤  │  ├─────────────────────────┤ │
│  │ ⚙️     │  │  │   MessageInput          │ │
│  │ Perfil │  │  └─────────────────────────┘ │
│  └────────┘  │                              │
└──────────────┴──────────────────────────────┘
```

## ADDED: AppLayout (modificado)

El layout debe:
- Sidebar fijo a la izquierda (280px, colapsable a 80px)
- **Primer item**: Main Chat "💬 Alfred" — siempre visible, no se puede cerrar
- **Items siguientes**: Chats efímeros "🌤️ Tiempo en Nardó" — con X para cerrar
- **Al final**: Perfil "⚙️" — abre ProfileEditor
- El chat activo se resalta en el menú
- Al hacer clic en un chat efímero, el ChatView muestra sus mensajes
- Al hacer clic en "Alfred", vuelve al main chat

## ADDED: ChatView

```typescript
interface ChatViewProps {
  messages: Message[];
  loading: boolean;
  hasMore: boolean;
  onLoadMore: () => void;
  onSendMessage: (content: string) => void;
  title: string;           // "Alfred" o título del chat efímero
}
```

- Header con título del chat activo
- Lista de MessageBubble con scroll automático al fondo
- Load more al hacer scroll arriba (cargar histórico)
- Estado vacío cuando no hay mensajes
- MessageInput fijo al fondo

## ADDED: MessageBubble

```typescript
interface MessageBubbleProps {
  message: Message;
}
```

- **user**: Alineación derecha, color primario azul
- **assistant**: Alineación izquierda, color gris claro
- **system**: Alineación centrada, texto itálico
- **tool**: Alineación izquierda, fondo monospace (código)
- Timestamp relativo ("hace 2min")

## ADDED: MessageInput

```typescript
interface MessageInputProps {
  onSend: (content: string) => void;
  disabled: boolean;
}
```

- Antd Input.TextArea con auto-size (2-6 filas)
- Enter para enviar, Shift+Enter para nueva línea
- Botón de envío con SendOutlined
- Deshabilitado mientras se procesa

## ADDED: ProfileEditor

```typescript
interface ProfileEditorProps {
  profile: Profile | null;
  onUpdate: (data: UpdateProfile) => Promise<void>;
  visible: boolean;
  onClose: () => void;
}
```

- Antd Drawer desde la derecha
- Formulario con: name, avatar_url
- JSON editor para preferences (textarea con validación JSON)
- Botón guardar con loading

## ADDED: Dependencies

Añadir a package.json:
```json
{
  "devDependencies": {
    "vitest": "^2.1",
    "@testing-library/react": "^16.0",
    "@testing-library/jest-dom": "^6.5",
    "@testing-library/user-event": "^14.5",
    "jsdom": "^25.0"
  }
}
```

## ADDED: vitest.config.ts

```typescript
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: './src/test/setup.ts',
  },
})
```

## Scenarios (BDD)

### Scenario: Main chat is always visible
- **Given** the app loads
- **When** the sidebar renders
- **Then** "💬 Alfred" is the first item and cannot be closed

### Scenario: Ephemeral chat appears in sidebar
- **Given** an ephemeral chat is created
- **When** it's created via the UI
- **Then** it appears as a new item in the sidebar with an X close button

### Scenario: Close ephemeral chat
- **Given** an ephemeral chat is active
- **When** the user clicks X on it
- **Then** the chat is removed from the sidebar and the main chat becomes active

### Scenario: MessageInput sends on Enter
- **Given** MessageInput is rendered
- **When** user types text and presses Enter (no Shift)
- **Then** onSend is called with the text and input clears