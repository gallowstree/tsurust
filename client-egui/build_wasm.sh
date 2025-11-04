#!/bin/bash
set -eu

# Build script for WASM deployment
# Usage: ./build_wasm.sh

echo "Building Tsurust for WebAssembly..."

# Check if wasm32-unknown-unknown target is installed
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Check if wasm-bindgen-cli is installed
if ! command -v wasm-bindgen &> /dev/null; then
    echo "Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli
fi

# Build in release mode for optimal performance
echo "Building WASM binary..."
cargo build --release --lib --target wasm32-unknown-unknown

# Generate JavaScript bindings
echo "Generating JavaScript bindings..."
wasm-bindgen \
    --out-dir web \
    --target web \
    --no-typescript \
    ../target/wasm32-unknown-unknown/release/client_egui.wasm

echo ""
echo "Build complete!"
echo ""
echo "To test locally, run a web server in the client-egui directory:"
echo "  cd client-egui"
echo "  python3 -m http.server 8000"
echo ""
echo "Then open http://localhost:8000/web/"
