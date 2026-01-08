#!/usr/bin/env pwsh
# Test script for Tsurust WebSocket Server Container
# Run with: .\test-server-container.ps1

Write-Host "=== Tsurust Server Container Test Script ===" -ForegroundColor Cyan
Write-Host ""

# Configuration
$SERVER_PORT = 8080
$CONTAINER_NAME = "tsurust-server"
$TIMEOUT_SECONDS = 60

# Color-coded output functions
function Write-Success { param($msg) Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Error-Msg { param($msg) Write-Host "[FAIL] $msg" -ForegroundColor Red }
function Write-Info { param($msg) Write-Host "  -> $msg" -ForegroundColor Yellow }
function Write-Step { param($msg) Write-Host "`n[$msg]" -ForegroundColor Cyan }

# Test results tracking
$testsPassed = 0
$testsFailed = 0

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

# Step 1: Check Docker is available
Write-Step "Checking Prerequisites"
Test-Step "Docker is installed" {
    $null = docker --version 2>&1
    $LASTEXITCODE -eq 0
}

Test-Step "Docker Compose is installed" {
    $null = docker-compose --version 2>&1
    $LASTEXITCODE -eq 0
}

# Step 2: Stop any existing container
Write-Step "Cleaning Up Existing Containers"
Write-Info "Stopping any running server containers..."
docker-compose stop server 2>&1 | Out-Null
docker-compose rm -f server 2>&1 | Out-Null
Start-Sleep -Seconds 2

# Step 3: Build the container
Write-Step "Building Server Container"
Write-Info "Building Docker image (this may take a few minutes)..."
$buildOutput = docker-compose build server 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error-Msg "Build failed!"
    Write-Host $buildOutput
    exit 1
}
Write-Success "Container built successfully"

# Step 4: Start the container
Write-Step "Starting Server Container"
Write-Info "Starting server in detached mode..."
docker-compose up -d server 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Error-Msg "Failed to start container"
    docker-compose logs server
    exit 1
}
Write-Success "Container started"

# Step 5: Wait for container to be healthy
Write-Step "Waiting for Container Health Check"
Write-Info "Waiting up to $TIMEOUT_SECONDS seconds for container to become healthy..."
$elapsed = 0
$healthy = $false

while ($elapsed -lt $TIMEOUT_SECONDS) {
    $health = docker inspect --format='{{.State.Health.Status}}' $CONTAINER_NAME 2>$null

    if ($health -eq "healthy") {
        $healthy = $true
        break
    }

    Write-Host "." -NoNewline
    Start-Sleep -Seconds 2
    $elapsed += 2
}

Write-Host ""
if ($healthy) {
    Write-Success "Container is healthy (took $elapsed seconds)"
    $testsPassed++
} else {
    Write-Error-Msg "Container did not become healthy within timeout"
    $testsFailed++
    Write-Info "Showing container logs:"
    docker-compose logs --tail=50 server
}

# Step 6: Check container is running
Write-Step "Verifying Container Status"
Test-Step "Container is running" {
    $status = docker inspect --format='{{.State.Running}}' $CONTAINER_NAME 2>$null
    $status -eq "true"
}

# Step 7: Check port is exposed
Write-Step "Checking Network Connectivity"
Test-Step "Port $SERVER_PORT is listening" {
    $listening = Get-NetTCPConnection -LocalPort $SERVER_PORT -ErrorAction SilentlyContinue
    $null -ne $listening
}

Test-Step "Port is accessible from host" {
    $tcpClient = New-Object System.Net.Sockets.TcpClient
    try {
        $tcpClient.Connect("localhost", $SERVER_PORT)
        $connected = $tcpClient.Connected
        $tcpClient.Close()
        return $connected
    } catch {
        return $false
    }
}

# Step 8: Test WebSocket connection
Write-Step "Testing WebSocket Connection"
Test-Step "WebSocket handshake succeeds" {
    try {
        $ws = New-Object System.Net.WebSockets.ClientWebSocket
        $uri = New-Object System.Uri("ws://localhost:$SERVER_PORT")
        $cts = New-Object System.Threading.CancellationTokenSource
        $cts.CancelAfter(5000) # 5 second timeout

        $connectTask = $ws.ConnectAsync($uri, $cts.Token)
        $connectTask.Wait()

        if ($ws.State -eq [System.Net.WebSockets.WebSocketState]::Open) {
            $ws.CloseAsync(1000, "Test complete", [System.Threading.CancellationToken]::None).Wait()
            return $true
        }
        return $false
    } catch {
        Write-Host "  (WebSocket test failed: $_)" -ForegroundColor DarkGray
        return $false
    }
}

# Step 9: Check logs for errors
Write-Step "Checking Container Logs"
Write-Info "Fetching recent logs..."
$logs = docker-compose logs --tail=20 server 2>&1
Write-Host $logs

$hasErrors = $logs -match "error|panic|fatal"
if (-not $hasErrors) {
    Write-Success "No errors found in logs"
    $testsPassed++
} else {
    Write-Error-Msg "Found errors in logs"
    $testsFailed++
}

# Step 10: Check resource usage
Write-Step "Checking Resource Usage"
$stats = docker stats $CONTAINER_NAME --no-stream --format "{{.CPUPerc}},{{.MemUsage}}" 2>$null
if ($stats) {
    $cpu, $mem = $stats -split ','
    Write-Info "CPU Usage: $cpu"
    Write-Info "Memory Usage: $mem"
    Write-Success "Resource stats retrieved"
    $testsPassed++
} else {
    Write-Error-Msg "Could not retrieve resource stats"
    $testsFailed++
}

# Final Summary
Write-Host ""
Write-Host "================================" -ForegroundColor Cyan
Write-Host "Test Summary" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host "Tests Passed: $testsPassed" -ForegroundColor Green
Write-Host "Tests Failed: $testsFailed" -ForegroundColor $(if ($testsFailed -eq 0) { "Green" } else { "Red" })
Write-Host ""

if ($testsFailed -eq 0) {
    Write-Host "SUCCESS! All tests passed! Server container is working correctly." -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  * View logs: docker-compose logs -f server"
    Write-Host "  * Stop server: docker-compose stop server"
    Write-Host "  * Connect client to: ws://localhost:$SERVER_PORT"
    exit 0
} else {
    Write-Host "FAILURE: Some tests failed. Check the output above for details." -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  * Check logs: docker-compose logs server"
    Write-Host "  * Restart: docker-compose restart server"
    Write-Host "  * Rebuild: docker-compose build --no-cache server"
    exit 1
}
