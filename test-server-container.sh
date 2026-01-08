#!/bin/bash
# Test script for Tsurust WebSocket Server Container
# Run with: bash test-server-container.sh

set -e

# Configuration
SERVER_PORT=8080
CONTAINER_NAME="tsurust-server"
TIMEOUT_SECONDS=60

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test counters
tests_passed=0
tests_failed=0

# Output functions
success() { echo -e "${GREEN}✓ $1${NC}"; ((tests_passed++)); }
error() { echo -e "${RED}✗ $1${NC}"; ((tests_failed++)); }
info() { echo -e "${YELLOW}→ $1${NC}"; }
step() { echo -e "\n${CYAN}[$1]${NC}"; }

test_step() {
    local name="$1"
    local command="$2"
    info "Testing: $name"
    if eval "$command"; then
        success "$name"
        return 0
    else
        error "$name"
        return 1
    fi
}

echo -e "${CYAN}=== Tsurust Server Container Test Script ===${NC}\n"

# Step 1: Check prerequisites
step "Checking Prerequisites"
test_step "Docker is installed" "command -v docker &> /dev/null"
test_step "Docker Compose is installed" "command -v docker-compose &> /dev/null"

# Step 2: Clean up
step "Cleaning Up Existing Containers"
info "Stopping any running server containers..."
docker-compose stop server 2>&1 > /dev/null || true
docker-compose rm -f server 2>&1 > /dev/null || true
sleep 2

# Step 3: Build
step "Building Server Container"
info "Building Docker image (this may take a few minutes)..."
if docker-compose build server; then
    success "Container built successfully"
else
    error "Build failed!"
    exit 1
fi

# Step 4: Start
step "Starting Server Container"
info "Starting server in detached mode..."
if docker-compose up -d server; then
    success "Container started"
else
    error "Failed to start container"
    docker-compose logs server
    exit 1
fi

# Step 5: Wait for health
step "Waiting for Container Health Check"
info "Waiting up to $TIMEOUT_SECONDS seconds for container to become healthy..."
elapsed=0
healthy=false

while [ $elapsed -lt $TIMEOUT_SECONDS ]; do
    health=$(docker inspect --format='{{.State.Health.Status}}' $CONTAINER_NAME 2>/dev/null || echo "none")

    if [ "$health" = "healthy" ]; then
        healthy=true
        break
    fi

    echo -n "."
    sleep 2
    ((elapsed+=2))
done

echo ""
if [ "$healthy" = true ]; then
    success "Container is healthy (took $elapsed seconds)"
else
    error "Container did not become healthy within timeout"
    info "Showing container logs:"
    docker-compose logs --tail=50 server
fi

# Step 6: Check running
step "Verifying Container Status"
test_step "Container is running" \
    "[ \"\$(docker inspect --format='{{.State.Running}}' $CONTAINER_NAME 2>/dev/null)\" = \"true\" ]"

# Step 7: Check port
step "Checking Network Connectivity"
test_step "Port $SERVER_PORT is listening" \
    "timeout 2 bash -c '</dev/tcp/localhost/$SERVER_PORT' 2>/dev/null"

# Step 8: Test WebSocket (basic TCP connection test)
step "Testing WebSocket Connection"
info "Testing: WebSocket port is accessible"
if timeout 2 bash -c "exec 3<>/dev/tcp/localhost/$SERVER_PORT" 2>/dev/null; then
    success "WebSocket port is accessible"
else
    error "WebSocket port is not accessible"
fi

# Step 9: Check logs
step "Checking Container Logs"
info "Fetching recent logs..."
logs=$(docker-compose logs --tail=20 server 2>&1)
echo "$logs"

if echo "$logs" | grep -iqE "error|panic|fatal"; then
    error "Found errors in logs"
else
    success "No errors found in logs"
fi

# Step 10: Resource usage
step "Checking Resource Usage"
stats=$(docker stats $CONTAINER_NAME --no-stream --format "CPU: {{.CPUPerc}}, Memory: {{.MemUsage}}" 2>/dev/null)
if [ -n "$stats" ]; then
    info "$stats"
    success "Resource stats retrieved"
else
    error "Could not retrieve resource stats"
fi

# Summary
echo ""
echo -e "${CYAN}================================${NC}"
echo -e "${CYAN}Test Summary${NC}"
echo -e "${CYAN}================================${NC}"
echo -e "${GREEN}Tests Passed: $tests_passed${NC}"
if [ $tests_failed -eq 0 ]; then
    echo -e "${GREEN}Tests Failed: $tests_failed${NC}"
else
    echo -e "${RED}Tests Failed: $tests_failed${NC}"
fi
echo ""

if [ $tests_failed -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed! Server container is working correctly.${NC}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "  • View logs: docker-compose logs -f server"
    echo "  • Stop server: docker-compose stop server"
    echo "  • Connect client to: ws://localhost:$SERVER_PORT"
    exit 0
else
    echo -e "${RED}❌ Some tests failed. Check the output above for details.${NC}"
    echo ""
    echo -e "${YELLOW}Troubleshooting:${NC}"
    echo "  • Check logs: docker-compose logs server"
    echo "  • Restart: docker-compose restart server"
    echo "  • Rebuild: docker-compose build --no-cache server"
    exit 1
fi
