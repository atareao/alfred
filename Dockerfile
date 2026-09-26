# ═══════════════════════════════════════════════════════════════
# Stage 1: Backend (Rust)
# ═══════════════════════════════════════════════════════════════
FROM docker.io/library/rust:alpine3.21 AS backend-builder

RUN apk add --no-cache --update \
    build-base \
    musl-dev \
    pkgconfig

WORKDIR /build

# Cache dependencies (avoid recompiling every time)
RUN cargo init --bin --name alfred . && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/seed.rs

COPY Cargo.toml Cargo.lock ./
RUN cargo build --release && \
    rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && \
    cargo build --release && \
    strip target/release/alfred

# ═══════════════════════════════════════════════════════════════
# Stage 2: Frontend (Node)
# ═══════════════════════════════════════════════════════════════
FROM docker.io/library/node:22-alpine AS frontend-builder

WORKDIR /build
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
ENV CI=true
RUN npm run build

# ═══════════════════════════════════════════════════════════════
# Stage 3: Runtime
# ═══════════════════════════════════════════════════════════════
FROM alpine:3.21

ENV RUST_LOG=info

RUN apk add --no-cache --update \
    ca-certificates && \
    rm -rf /var/cache/apk/*

WORKDIR /app
COPY --from=backend-builder /build/target/release/alfred /app/alfred
COPY --from=frontend-builder /build/dist /app/static
COPY migrations ./migrations

EXPOSE 3000
CMD ["/app/alfred"]
