# Infraestructura: Docker y Despliegue

## Contratos

### Docker Compose (docker-compose.yml)

```yaml
services:
  alfred:
    build:
      context: .
      dockerfile: Dockerfile
    ports: ["3000:3000"]
    volumes:
      - alfred_data:/app/data
    environment:
      - DATABASE_URL
      - RUST_LOG
      - OPENROUTER_API_KEY
      - OPENWEATHER_API_KEY
      - AUTH_ENABLED
```

### Dockerfile (multi-stage)

```dockerfile
# Stage 1: Backend (Rust)
FROM docker.io/library/rust:alpine3.21 AS backend-builder
RUN apk add --no-cache build-base musl-dev pkgconfig
WORKDIR /build
RUN cargo init --bin --name alfred . && echo "pub fn dummy() {}" > src/lib.rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release && strip target/release/alfred

# Stage 2: Frontend (Node)
FROM docker.io/library/node:22-alpine AS frontend-builder
WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN CI=true npm run build

# Stage 3: Runtime (Alpine)
FROM alpine:3.21
RUN apk add --no-cache ca-certificates && rm -rf /var/cache/apk/*
WORKDIR /app
COPY --from=backend-builder /build/target/release/alfred /app/alfred
COPY --from=frontend-builder /build/dist /app/static
EXPOSE 3000
CMD ["/app/alfred"]
```

Tamaño final de imagen: ~30 MB (binario musl ~8 MB + assets frontend compilados).

### TLS stack (rustls)

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
# → rustls puro (100% Rust, sin dependencias C nativas)
# → NO depende de openssl-sys, native-tls, ni libssl-dev
```

## Escenarios

### Docker build produce imagen con frontend embebido
**Given** el directorio del proyecto con src/, Cargo.toml, Cargo.lock y frontend/  
**When** se ejecuta `docker build -t alfred .`  
**Then** la imagen se construye sin errores  
**And** el directorio /app/static/ contiene index.html y assets JS/CSS compilados  
**And** el binario compilado con musl responde en el puerto 3000  
**And** la imagen pesa ~30 MB

### Docker compose levanta un único servicio
**Given** docker-compose.yml con las variables de entorno necesarias  
**When** se ejecuta `docker compose up -d`  
**Then** el servicio alfred responde en localhost:3000/api/health  
**And** el servicio alfred sirve el frontend SPA en localhost:3000/  
**And** no existe el servicio frontend en el compose

### reqwest usa rustls-tls (sin openssl)
**Given** Cargo.toml con `default-features = false, features = ["json", "rustls-tls"]`  
**When** se ejecuta `cargo tree -i openssl-sys`  
**Then** el comando devuelve "package ID specification openssl-sys matched no packages"  
**And** la compilación no requiere openssl-dev ni libssl-dev  
**And** las peticiones HTTPS funcionan correctamente