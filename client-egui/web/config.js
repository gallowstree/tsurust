// Configuration for Tsurust WASM Client
// This file can be overwritten by docker-entrypoint.sh in containerized deployments

window.TSURUST_CONFIG = {
    // WebSocket server URL
    // For local development: ws://localhost:8080
    // For production: wss://your-domain.com (use secure WebSocket)
    wsServerUrl: "ws://127.0.0.1:8080"
};
