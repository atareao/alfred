# Tasks: settings-dialog

## TDD Checklist

### Phase 1 — Create SettingsDialog component
- [ ] RED: Create SettingsDialog.test.tsx with tests for all four tabs
- [ ] GREEN: Implement SettingsDialog.tsx with Tabs + forms for Perfil, Interfaz, Prompt, API Keys
- [ ] REFACTOR: lint + typecheck + all tests pass

### Phase 2 — Update AppLayout
- [ ] RED: Update AppLayout.test.tsx — remove ProfileEditor/SettingsEditor mocks, add SettingsDialog mock
- [ ] GREEN: Replace ProfileEditor + SettingsEditor with SettingsDialog, use single button/state
- [ ] REFACTOR: lint + typecheck + all tests pass

### Phase 3 — Remove old components
- [ ] Delete ProfileEditor.tsx and SettingsEditor.tsx
- [ ] Update test imports if needed
- [ ] Verify all tests pass

### Phase 4 — Final verification
- [ ] `npx vitest run` — all tests green
- [ ] `npx tsc --noEmit` — no type errors
- [ ] `npm run lint` — no lint errors
- [ ] Archive change proposal