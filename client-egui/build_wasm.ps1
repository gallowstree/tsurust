# Build script for WASM deployment (Windows PowerShell)
# Usage: .\build_wasm.ps1

Write-Host "Building Tsurust for WebAssembly..." -ForegroundColor Green

# Check if wasm32-unknown-unknown target is installed
$hasTarget = rustup target list --installed | Select-String "wasm32-unknown-unknown"
if (-not $hasTarget) {
    Write-Host "Installing wasm32-unknown-unknown target..." -ForegroundColor Yellow
    rustup target add wasm32-unknown-unknown
}

# Check if wasm-bindgen-cli is installed
$hasWasmBindgen = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
if (-not $hasWasmBindgen) {
    Write-Host "Installing wasm-bindgen-cli..." -ForegroundColor Yellow
    cargo install wasm-bindgen-cli
}

# Build in release mode for optimal performance
Write-Host "Building WASM binary..." -ForegroundColor Cyan
cargo build --release --lib --target wasm32-unknown-unknown

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Generate JavaScript bindings
Write-Host "Generating JavaScript bindings..." -ForegroundColor Cyan
wasm-bindgen `
    --out-dir web `
    --target web `
    --no-typescript `
    ../target/wasm32-unknown-unknown/release/client_egui.wasm

if ($LASTEXITCODE -ne 0) {
    Write-Host "wasm-bindgen failed!" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Build complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To test locally, run a web server in the client-egui directory:" -ForegroundColor Yellow
Write-Host "  cd client-egui"
Write-Host "  python -m http.server 8000"
Write-Host ""
Write-Host "Or use: npx http-server -p 8000" -ForegroundColor Yellow
Write-Host ""
Write-Host "Then open http://localhost:8000/web/" -ForegroundColor Cyan
