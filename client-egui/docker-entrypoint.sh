#!/bin/sh
set -e

# Generate config.js with environment variables
cat > /usr/share/nginx/html/config.js <<EOF
// Auto-generated configuration from environment variables
window.TSURUST_CONFIG = {
    wsServerUrl: "${WS_SERVER_URL:-ws://localhost:8080}"
};
EOF

echo "Generated config.js with WS_SERVER_URL=${WS_SERVER_URL:-ws://localhost:8080}"

# Execute the main command
exec "$@"
