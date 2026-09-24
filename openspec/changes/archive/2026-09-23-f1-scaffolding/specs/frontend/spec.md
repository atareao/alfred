# Frontend Spec Delta — f1-scaffolding

### ADDED: Vite + React + Antd project

Frontend initialized with Vite 5, React 18, TypeScript 5.6, Antd 5.21.

### ADDED: package.json dependencies

```json
{
  "dependencies": {
    "react": "^18.3",
    "react-dom": "^18.3",
    "antd": "^5.21",
    "@ant-design/icons": "^5.5"
  },
  "devDependencies": {
    "typescript": "^5.6",
    "vite": "^5.4",
    "@vitejs/plugin-react": "^4.3"
  }
}
```

### ADDED: Vite proxy configuration

`/api` requests proxied to `http://localhost:3000` in dev mode.

### ADDED: Antd dark theme

Theme with `colorPrimary: '#1677ff'`, dark backgrounds, layout with sidebar.

### ADDED: AppLayout component

Antd Layout with collapsible Sider (280px) and Content area.

### ADDED: Scenario: Frontend builds successfully

- **Given** the frontend project is configured
- **When** `npm run build` is executed
- **Then** the build completes without errors

### ADDED: Scenario: TypeScript compiles

- **Given** the frontend project is configured
- **When** `npx tsc --noEmit` is executed
- **Then** no type errors are reported