#!/bin/sh
set -e

# Create a config.js file with environment variables
cat > /usr/share/nginx/html/config.js <<CONFIGEOF
// Auto-generated configuration from environment variables
window.TSURUST_CONFIG = {
    wsServerUrl: "${WS_SERVER_URL:-ws://localhost:8080}"
};
CONFIGEOF

echo "WebSocket server URL configured as: ${WS_SERVER_URL:-ws://localhost:8080}"

# Execute the CMD
exec "$@"
