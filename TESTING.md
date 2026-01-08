# Server Testing Guide

This guide explains how to test the Tsurust WebSocket server, both locally and in a container.

## Quick Start

### Local Testing (No Docker Required)

Test the server directly on your machine:

**Windows (PowerShell):**
```powershell
.\test-server-local.ps1
```

**Manual:**
```bash
# Start the server
cargo run --bin server

# In another terminal, test connection
Test-NetConnection localhost -Port 8080
```

### Container Testing (Docker Required)

**Windows (PowerShell):**
```powershell
.\test-server-container.ps1
```

**Linux/Mac/Git Bash:**
```bash
bash test-server-container.sh
```

## What the Test Scripts Do

### Local Test Script (`test-server-local.ps1`)

Tests the server without Docker:

1. **Prerequisites Check**
   - Verifies Rust toolchain is installed
   - Checks project files exist

2. **Port Availability**
   - Ensures port 8080 is not in use
   - Offers to kill conflicting processes

3. **Build & Start**
   - Builds the server with `cargo build`
   - Starts the server process
   - Monitors for startup completion

4. **Connection Tests**
   - TCP connection verification
   - WebSocket handshake test

5. **Health Monitoring**
   - Checks process is alive
   - Reviews logs for errors
   - Reports memory and CPU usage

6. **Cleanup**
   - Automatically stops server on completion
   - Removes temporary log files

### Container Test Script (`test-server-container.ps1`)

Tests the containerized server:

1. **Prerequisites Check**
   - Verifies Docker is installed
   - Verifies Docker Compose is installed

2. **Container Lifecycle**
   - Stops any existing server containers
   - Builds the server Docker image
   - Starts the container in detached mode

3. **Health Verification**
   - Waits for container health check to pass (up to 60 seconds)
   - Verifies container is running
   - Checks container resource usage

4. **Network Tests**
   - Confirms port 8080 is listening
   - Tests TCP connectivity to the server
   - Attempts WebSocket handshake

5. **Log Analysis**
   - Fetches recent container logs
   - Scans for errors, panics, or fatal messages

## Manual Testing

If you prefer to test manually:

### 1. Build and Start
```bash
docker-compose up --build server
```

### 2. Check Status
```bash
# View container status
docker-compose ps

# Check logs
docker-compose logs -f server
```

### 3. Test WebSocket Connection

**Using wscat (Node.js):**
```bash
npm install -g wscat
wscat -c ws://localhost:8080
```

**Using PowerShell:**
```powershell
$ws = New-Object System.Net.WebSockets.ClientWebSocket
$uri = [Uri]'ws://localhost:8080'
$ws.ConnectAsync($uri, [Threading.CancellationToken]::None).Wait()
Write-Host "Connected: $($ws.State)"
$ws.CloseAsync(1000, 'Test', [Threading.CancellationToken]::None).Wait()
```

**Using curl (if available):**
```bash
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://localhost:8080/
```

### 4. Health Check
```bash
# Check container health status
docker inspect --format='{{.State.Health.Status}}' tsurust-server

# Manual TCP test
timeout 2 bash -c '</dev/tcp/localhost/8080'
# Or in PowerShell:
Test-NetConnection localhost -Port 8080
```

### 5. View Resource Usage
```bash
docker stats tsurust-server
```

## Troubleshooting

### Container Won't Start
```bash
# View build logs
docker-compose build --no-cache server

# Check for errors
docker-compose logs server
```

### Health Check Failing
```bash
# Enter the container
docker exec -it tsurust-server sh

# Manually test the port from inside
timeout 2 bash -c '</dev/tcp/localhost/8080'

# Check if process is running
ps aux | grep server
```

### Port Already in Use
```bash
# Windows (find process using port 8080)
netstat -ano | findstr :8080

# Kill the process (replace PID)
taskkill /PID <PID> /F

# Or change the port in docker-compose.yml
```

### WebSocket Connection Fails
- Ensure firewall isn't blocking port 8080
- Check if the server is listening: `netstat -an | findstr 8080`
- Verify the container is healthy: `docker-compose ps`

## Integration Testing

To test the full stack (server + client):

```bash
# Start both services
docker-compose up --build

# Access the client at http://localhost
# The client should automatically connect to ws://localhost:8080
```

## Cleanup

```bash
# Stop all containers
docker-compose down

# Remove volumes and images
docker-compose down -v --rmi all
```

## Expected Behavior

When working correctly:
- Container builds without errors
- Health check passes within 10-15 seconds
- Server listens on `0.0.0.0:8080`
- WebSocket connections are accepted
- No error/panic messages in logs
- CPU and memory usage are reasonable (<10% CPU, <100MB RAM)
