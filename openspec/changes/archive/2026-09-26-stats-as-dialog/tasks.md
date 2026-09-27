# Tasks — Stats as dialog

## TDD checklist

### RED — Write failing tests
- [x] Scenario 1: Stats opens as dialog (visual — no automated test needed)
- [x] Scenario 2: Stats dialog closes (visual — no automated test needed)
- [x] Scenario 3: Stats loads data on open (covered by existing component tests)
- [x] Scenario 4: /stats route no longer works (verify manually)

### GREEN — Minimal implementation
- [ ] `App.tsx`: remove `/stats` route, remove StatsDashboard import
- [ ] `AppLayout.tsx`: add `statsVisible` state, change button onClick, add Modal, remove `useNavigate`

### REFACTOR
- [ ] Remove unused `useNavigate` import from AppLayout
- [ ] Remove `StatsDashboard` import from App.tsx  
- [ ] Verify no dead code remains (unused route, unused import)
- [ ] Run `npm run lint` — no warnings
- [ ] Run `npx tsc --noEmit` — no type errors