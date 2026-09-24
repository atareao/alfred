# Alfred - Justfile

# ── Desarrollo (Podman por defecto) ─────────────────────────────
# El proyecto usa Podman para desarrollo local. Ajusta el binario
# en /usr/bin/podman si tu sistema lo tiene en otra ruta.

# Levanta el servidor con frontend embebido (Podman)
dev:
    podman compose up -d --build
    @echo "Alfred: http://localhost:3000"

# Levanta el servidor con frontend embebido (Docker)
dev-docker:
    docker compose up -d
    @echo "Alfred: http://localhost:3000"

# Ejecuta todos los checks
check-all: test clippy fmt frontend-check

# Tests Rust
test:
    cargo test

# Clippy (zero warnings)
clippy:
    cargo clippy -- -D warnings

# Formato
fmt:
    cargo fmt --check

# Frontend checks (desarrollo standalone con Vite)
frontend-check:
    cd frontend && npx tsc --noEmit && npm run build

# Check que existe un change proposal activo en openspec
check-spec:
    @ls openspec/changes/*/proposal.md 2>/dev/null || (echo "❌ No active change proposal found. Run: openspec new change <feature>" && exit 1)
    @echo "✅ Active change proposal found."

# Limpia todo
clean:
    cargo clean
    rm -rf frontend/dist frontend/node_modules

# Ayuda
help:
    @echo "Comandos disponibles:"
    @echo "  just dev         - Levanta Alfred con Podman (reconstruye imagen)"
    @echo "  just dev-docker  - Levanta Alfred con Docker"
    @echo "  just check-all   - Ejecuta todos los checks"
    @echo "  just test        - cargo test"
    @echo "  just clippy      - cargo clippy"
    @echo "  just fmt         - cargo fmt --check"
    @echo "  just check-spec  - Verifica change proposal activo"
