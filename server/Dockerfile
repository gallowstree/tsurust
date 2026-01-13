# Multi-stage build for Tsurust WebSocket Server
# Stage 1: Build the Rust application with musl for static binary
FROM rust:1.83-alpine as builder

WORKDIR /app

# Install build dependencies (musl-dev for static linking)
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# Add musl target for the current architecture
RUN rustup target add $(uname -m)-unknown-linux-musl

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY server ./server

# Create stubs for client-egui to satisfy workspace requirements
RUN mkdir -p client-egui/src && \
    echo '// Stub library for Docker build' > client-egui/src/lib.rs && \
    echo 'fn main() {}' > client-egui/src/main.rs
COPY client-egui/Cargo.toml ./client-egui/Cargo.toml

# Build the server as a static musl binary
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN TARGET_ARCH=$(uname -m)-unknown-linux-musl && \
    cargo build --release --target $TARGET_ARCH --bin server

# Stage 2: Create minimal runtime image
FROM alpine:3.19

# Install minimal runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    bash

# Create non-root user (Alpine uses adduser)
RUN adduser -D -u 1001 tsurust

WORKDIR /app

# Copy the static binary from builder
COPY --from=builder /app/target/*/release/server /app/server

# Change ownership to non-root user
RUN chown -R tsurust:tsurust /app

USER tsurust

# Environment variables for configuration
ENV HOST=0.0.0.0
ENV PORT=8080

# Expose WebSocket port (default 8080, configurable via PORT env var)
EXPOSE ${PORT}

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD timeout 2 bash -c "</dev/tcp/localhost/${PORT}" || exit 1

# Run the server
CMD ["/app/server"]
