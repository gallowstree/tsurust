# Multi-stage build for Tsurust WASM Client
# Stage 1: Build the WASM application
FROM rust:1.83-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install wasm32 target and wasm-bindgen-cli (version must match Cargo.toml)
RUN rustup target add wasm32-unknown-unknown && \
    cargo install wasm-bindgen-cli --version 0.2.92

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY client-egui ./client-egui

# Create stubs for server to satisfy workspace requirements
RUN mkdir -p server/src && \
    echo '// Stub binary for Docker build' > server/src/main.rs
COPY server/Cargo.toml ./server/Cargo.toml

WORKDIR /app/client-egui

# Build WASM in release mode
RUN cargo build --release --lib --target wasm32-unknown-unknown

# Generate JavaScript bindings
RUN wasm-bindgen \
    --out-dir web \
    --target web \
    --no-typescript \
    /app/target/wasm32-unknown-unknown/release/client_egui.wasm

# Stage 2: Serve with nginx
FROM nginx:alpine

# Copy WASM files to nginx html directory
COPY --from=builder /app/client-egui/web /usr/share/nginx/html/

# Copy custom nginx configuration
COPY client-egui/nginx.conf /etc/nginx/conf.d/default.conf

# Environment variable for WebSocket server URL
ENV WS_SERVER_URL=ws://localhost:8080

# Expose HTTP port
EXPOSE 80

# Use custom entrypoint to inject environment variables into config.js
COPY client-egui/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
