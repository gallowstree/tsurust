use std::time::Duration;
use tokio::net::TcpListener;
use tracing::info;

/// Default grace period before a room with no connected clients is removed.
const DEFAULT_ROOM_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Leveled logging, filterable per module via RUST_LOG (default: info)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Read configuration from environment variables with defaults
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    info!(%addr, "Tsurust WebSocket server listening");

    let idle_timeout = std::env::var("ROOM_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_ROOM_IDLE_TIMEOUT);

    tsurust_server::serve(listener, idle_timeout).await;

    Ok(())
}
