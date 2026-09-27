# Convert Profile + Settings drawers into unified Settings Modal with tabs

## Why

Actualmente hay dos drawers separados: "Editar Perfil" (Drawer) y "Ajustes" (Drawer). Son dos
modalidades distintas de configuración que el usuario accede desde la barra superior, pero no hay
razón para que estén separados. Un único diálogo Settings con pestañas organiza mejor la
información, reduce la complejidad del header (dos botones → uno) y sigue el mismo patrón que
los modales de Tasks, Agenda y Stats.

## What Changes

1. **Unificar** `ProfileEditor` y `SettingsEditor` en un único componente `SettingsDialog` con tabs.
2. **Convertir de Drawer a Modal** (antd Tabs + Modal) para consistencia con Tasks/Agenda/Stats.
3. **Cuatro pestañas**:
   - **Perfil**: nombre, avatar URL (contenido actual de ProfileEditor)
   - **Interfaz**: font_size, max_window_tokens, message_page_size (subset de SettingsEditor)
   - **Prompt**: system_prompt (de SettingsEditor)
   - **API Keys**: openweather, google_places, brave_search (de SettingsEditor)
4. **Simplificar header**: un solo botón `SettingOutlined` que abre el dialog. Se elimina `UserOutlined`.
5. **Eliminar** los componentes `ProfileEditor.tsx` y `SettingsEditor.tsx` (reemplazados por `SettingsDialog.tsx`).

## Impact

- **Frontend only**: no toca backend ni BD
- **AppLayout.tsx**: reemplazar dos estados (`profileVisible`, `settingsVisible`) por uno (`settingsVisible`)
- **AppLayout.tsx**: reemplazar dos botones por uno, reemplazar `<ProfileEditor>` + `<SettingsEditor>` por `<SettingsDialog>`
- **Tests AppLayout.test.tsx**: actualizar mocks, eliminar tests de drawers individuales, añadir tests del dialog
- **Tests SettingsDialog**: nuevos tests unitarios para el diálogo con tabs
- **Specs**: modificar `openspec/specs/frontend/spec.md` para reflejar el nuevo componente