pub mod handler;
pub mod room;
pub mod server;

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
use tracing::{debug, info, warn};

use crate::handler::handle_connection;
use crate::server::GameServer;

/// How often the background reaper checks for idle rooms.
const ROOM_REAP_INTERVAL: Duration = Duration::from_secs(60);

/// Accept WebSocket connections on `listener` until the future is dropped.
/// Rooms whose clients are all gone (crashed, or orphaned the room by
/// creating/joining another one) are reaped after `room_idle_timeout`.
///
/// This is the whole server behind `main`'s config parsing; tests embed it
/// in-process by binding a listener on an ephemeral port.
pub async fn serve(listener: TcpListener, room_idle_timeout: Duration) {
    let game_server = Arc::new(GameServer::new());

    let reaper_server = Arc::clone(&game_server);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(ROOM_REAP_INTERVAL);
        loop {
            interval.tick().await;
            reaper_server.reap_idle_rooms(room_idle_timeout).await;
        }
    });

    while let Ok((stream, peer_addr)) = listener.accept().await {
        debug!(%peer_addr, "incoming TCP connection");

        let game_server = Arc::clone(&game_server);
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    let connection_id = game_server.next_connection_id().await;
                    info!(connection_id, "new WebSocket connection");
                    handle_connection(ws_stream, connection_id, game_server).await;
                }
                Err(e) => {
                    warn!(error = %e, "failed to accept WebSocket connection");
                }
            }
        });
    }
}
