# Docker Spec Delta — f1-scaffolding

### ADDED: Dockerfile.backend

Multi-stage Rust dev image based on `rust:1.80-slim-bookworm` with cargo-watch for hot-reload.

### ADDED: Dockerfile.frontend

Node.js dev image based on `node:20-alpine` with Vite HMR.

### ADDED: docker-compose.yml

Two services (backend on :3000, frontend on :5173) with bind mounts for hot-reload and named volume for cargo cache.

### ADDED: .dockerignore

Excludes target/, node_modules/, data/, .git/, *.md from Docker build context.