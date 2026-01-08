#!/usr/bin/env pwsh
# Local Server Test Script (No Docker Required)
# Run with: .\test-server-local.ps1

Write-Host "=== Tsurust Local Server Test Script ===" -ForegroundColor Cyan
Write-Host ""

# Configuration
$SERVER_PORT = 8080
$BUILD_TIMEOUT = 300000  # 5 minutes for build
$STARTUP_TIMEOUT = 30    # 30 seconds for server startup

# Color-coded output functions
function Write-Success { param($msg) Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Error-Msg { param($msg) Write-Host "[FAIL] $msg" -ForegroundColor Red }
function Write-Info { param($msg) Write-Host "  -> $msg" -ForegroundColor Yellow }
function Write-Step { param($msg) Write-Host "`n[$msg]" -ForegroundColor Cyan }

# Test results tracking
$testsPassed = 0
$testsFailed = 0
$serverProcess = $null

function Test-Step {
    param($name, $scriptBlock)
    Write-Info "Testing: $name"
    try {
        $result = & $scriptBlock
        if ($result) {
            Write-Success $name
            $script:testsPassed++
            return $true
        } else {
            Write-Error-Msg "$name - Test returned false"
            $script:testsFailed++
            return $false
        }
    } catch {
        Write-Error-Msg "$name - $_"
        $script:testsFailed++
        return $false
    }
}

# Cleanup function
function Cleanup {
    Write-Step "Cleaning Up"
    if ($script:serverProcess -and !$script:serverProcess.HasExited) {
        Write-Info "Stopping server process (PID: $($script:serverProcess.Id))..."
        Stop-Process -Id $script:serverProcess.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
        Write-Success "Server stopped"
    }

    # Kill any remaining processes on port 8080
    $portProcesses = Get-NetTCPConnection -LocalPort $SERVER_PORT -ErrorAction SilentlyContinue |
                     Select-Object -ExpandProperty OwningProcess -Unique

    foreach ($processId in $portProcesses) {
        Write-Info "Killing process on port ${SERVER_PORT}: PID $processId"
        Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
    }
}

# Register cleanup on exit
Register-EngineEvent PowerShell.Exiting -Action { Cleanup } | Out-Null
trap { Cleanup; break }

# Step 1: Check prerequisites
Write-Step "Checking Prerequisites"
Test-Step "Rust toolchain is installed" {
    $null = cargo --version 2>&1
    $LASTEXITCODE -eq 0
}

Test-Step "Project directory exists" {
    Test-Path "server/Cargo.toml"
}

# Step 2: Check if port is available
Write-Step "Checking Port Availability"
$portInUse = Get-NetTCPConnection -LocalPort $SERVER_PORT -ErrorAction SilentlyContinue |
             Where-Object { $_.State -ne "TimeWait" }

if ($portInUse) {
    Write-Error-Msg "Port $SERVER_PORT is already in use"
    Write-Info "Processes using port ${SERVER_PORT}:"
    $portInUse | ForEach-Object {
        $proc = Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue
        if ($proc) {
            Write-Host "  - PID $($_.OwningProcess): $($proc.ProcessName) (State: $($_.State))"
        }
    }

    $response = Read-Host "Kill these processes? (y/N)"
    if ($response -eq 'y' -or $response -eq 'Y') {
        Cleanup
        Start-Sleep -Seconds 2
    } else {
        Write-Error-Msg "Cannot continue with port in use"
        exit 1
    }
}

Write-Success "Port $SERVER_PORT is available"
$testsPassed++

# Step 3: Build the server
Write-Step "Building Server"
Write-Info "Running: cargo build --bin server"
Write-Info "(This may take a few minutes on first run...)"

$buildOutput = cargo build --bin server 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error-Msg "Build failed!"
    Write-Host $buildOutput
    exit 1
}
Write-Success "Server built successfully"
$testsPassed++

# Step 4: Start the server
Write-Step "Starting Server"
Write-Info "Launching server on port $SERVER_PORT..."

$env:PORT = $SERVER_PORT
$env:HOST = "127.0.0.1"

try {
    $script:serverProcess = Start-Process -FilePath "cargo" `
                                          -ArgumentList "run", "--bin", "server" `
                                          -PassThru `
                                          -NoNewWindow `
                                          -RedirectStandardOutput "server_output.log" `
                                          -RedirectStandardError "server_error.log"

    Write-Success "Server process started (PID: $($script:serverProcess.Id))"
    $testsPassed++
} catch {
    Write-Error-Msg "Failed to start server: $_"
    $testsFailed++
    exit 1
}

# Step 5: Wait for server to be ready
Write-Step "Waiting for Server to Start"
Write-Info "Waiting up to $STARTUP_TIMEOUT seconds for server to accept connections..."

$elapsed = 0
$serverReady = $false

while ($elapsed -lt $STARTUP_TIMEOUT) {
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $tcpClient.Connect("127.0.0.1", $SERVER_PORT)
        if ($tcpClient.Connected) {
            $serverReady = $true
            $tcpClient.Close()
            break
        }
    } catch {
        # Server not ready yet
    }

    if ($script:serverProcess.HasExited) {
        Write-Error-Msg "Server process exited unexpectedly (Exit Code: $($script:serverProcess.ExitCode))"
        Write-Info "Server error log:"
        if (Test-Path "server_error.log") {
            Get-Content "server_error.log"
        }
        $testsFailed++
        exit 1
    }

    Write-Host "." -NoNewline
    Start-Sleep -Seconds 1
    $elapsed++
}

Write-Host ""
if ($serverReady) {
    Write-Success "Server is accepting connections (took $elapsed seconds)"
    $testsPassed++
} else {
    Write-Error-Msg "Server did not start within timeout"
    $testsFailed++
}

# Step 6: Test TCP connection
Write-Step "Testing Network Connectivity"
Test-Step "TCP connection succeeds" {
    try {
        $tcpClient = New-Object System.Net.Sockets.TcpClient
        $tcpClient.Connect("127.0.0.1", $SERVER_PORT)
        $connected = $tcpClient.Connected
        $tcpClient.Close()
        return $connected
    } catch {
        return $false
    }
}

# Step 7: Test WebSocket connection
Write-Step "Testing WebSocket Connection"
Test-Step "WebSocket handshake succeeds" {
    try {
        $ws = New-Object System.Net.WebSockets.ClientWebSocket
        $uri = New-Object System.Uri("ws://127.0.0.1:$SERVER_PORT")
        $cts = New-Object System.Threading.CancellationTokenSource
        $cts.CancelAfter(5000)

        $connectTask = $ws.ConnectAsync($uri, $cts.Token)
        $connectTask.Wait()

        if ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
            Write-Info "WebSocket state: $($ws.State)"
            $ws.CloseAsync(1000, "Test complete", [System.Threading.CancellationToken]::None).Wait()
            return $true
        }
        return $false
    } catch {
        Write-Host "  (Error: $_)" -ForegroundColor DarkGray
        return $false
    }
}

# Step 8: Check server logs
Write-Step "Checking Server Logs"
Start-Sleep -Seconds 2

if (Test-Path "server_output.log") {
    $output = Get-Content "server_output.log" -Raw
    if ($output) {
        Write-Info "Server output:"
        Write-Host $output -ForegroundColor DarkGray
    }
}

if (Test-Path "server_error.log") {
    $errors = Get-Content "server_error.log" -Raw
    if ($errors -and $errors.Trim().Length -gt 0) {
        # Filter out expected warnings and test-related errors
        $filteredErrors = $errors -split "`n" | Where-Object {
            $_ -notmatch "warning:" -and
            $_ -notmatch "^$" -and
            $_ -notmatch "Finished .* profile" -and
            $_ -notmatch "Running .*target" -and
            $_ -notmatch "unexpected end of file" -and  # Expected from test connections
            $_ -notmatch "package:" -and
            $_ -notmatch "workspace:" -and
            $_ -notmatch "^\s*-->" -and
            $_ -notmatch "^\s*\|" -and
            $_ -notmatch "^\s*=" -and
            $_ -notmatch "pub struct" -and
            $_ -notmatch "field in this struct" -and
            $_ -notmatch "\^\^\^" -and
            $_ -notmatch "^\s*\d+\s*\|" -and  # Warning line numbers
            $_ -notmatch "Arc<RwLock" -and
            $_.Trim().Length -gt 0
        }

        if ($filteredErrors) {
            Write-Error-Msg "Server errors detected:"
            $filteredErrors | ForEach-Object { Write-Host $_ -ForegroundColor Red }
            $testsFailed++
        } else {
            Write-Success "No critical errors (warnings are OK)"
            $testsPassed++
        }
    } else {
        Write-Success "No error output"
        $testsPassed++
    }
}

# Step 9: Check process is alive
Write-Step "Verifying Server Process"
if ($script:serverProcess.HasExited) {
    Write-Error-Msg "Server process has exited (Exit Code: $($script:serverProcess.ExitCode))"
    $testsFailed++
} else {
    Write-Success "Server process is running"
    Write-Info "PID: $($script:serverProcess.Id)"
    Write-Info "CPU Time: $($script:serverProcess.TotalProcessorTime)"
    Write-Info "Memory: $([math]::Round($script:serverProcess.WorkingSet64 / 1MB, 2)) MB"
    $testsPassed++
}

# Final Summary
Write-Host ""
Write-Host "================================" -ForegroundColor Cyan
Write-Host "Test Summary" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host "Tests Passed: $testsPassed" -ForegroundColor Green
Write-Host "Tests Failed: $testsFailed" -ForegroundColor $(if ($testsFailed -eq 0) { "Green" } else { "Red" })
Write-Host ""

# Cleanup
Cleanup

# Clean up log files
Remove-Item "server_output.log" -ErrorAction SilentlyContinue
Remove-Item "server_error.log" -ErrorAction SilentlyContinue

if ($testsFailed -eq 0) {
    Write-Host "SUCCESS! All tests passed! Server is working correctly." -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  * Run server: cargo run --bin server"
    Write-Host "  * Connect client to: ws://localhost:$SERVER_PORT"
    Write-Host "  * View server code: server/src/main.rs"
    exit 0
} else {
    Write-Host "FAILURE: Some tests failed. Check the output above for details." -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  * Check server code: server/src/main.rs"
    Write-Host "  * Run manually: cargo run --bin server"
    Write-Host "  * Check for compile errors: cargo check --bin server"
    exit 1
}
