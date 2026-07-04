mod handler;
mod room;
mod server;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod room_tests;
#[cfg(test)]
mod server_tests;

use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

use crate::handler::handle_connection;
use crate::server::GameServer;

/// How often the background reaper checks for idle rooms.
const ROOM_REAP_INTERVAL: Duration = Duration::from_secs(60);
/// Default grace period before a room with no connected clients is removed.
const DEFAULT_ROOM_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read configuration from environment variables with defaults
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    println!("Tsurust WebSocket server listening on {}", addr);

    let game_server = Arc::new(GameServer::new());

    // Reap rooms whose clients are all gone (crashed, or orphaned the room by
    // creating/joining another one) after an idle grace period.
    let idle_timeout = std::env::var("ROOM_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_ROOM_IDLE_TIMEOUT);
    let reaper_server = Arc::clone(&game_server);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(ROOM_REAP_INTERVAL);
        loop {
            interval.tick().await;
            reaper_server.reap_idle_rooms(idle_timeout).await;
        }
    });

    while let Ok((stream, peer_addr)) = listener.accept().await {
        println!("Incoming TCP connection from {}", peer_addr);

        let game_server = Arc::clone(&game_server);
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    let connection_id = game_server.next_connection_id().await;
                    println!("New WebSocket connection: {}", connection_id);
                    handle_connection(ws_stream, connection_id, game_server).await;
                }
                Err(e) => {
                    eprintln!("Failed to accept WebSocket connection: {}", e);
                }
            }
        });
    }

    Ok(())
}
