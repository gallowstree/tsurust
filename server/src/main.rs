mod room;
mod server;
mod handler;

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_websockets::ServerBuilder;

use crate::server::GameServer;
use crate::handler::handle_connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await
        .expect("Failed to bind to address");

    println!("Tsurust WebSocket server listening on {}", addr);

    let game_server = Arc::new(GameServer::new());

    while let Ok((stream, peer_addr)) = listener.accept().await {
        println!("Incoming TCP connection from {}", peer_addr);

        let game_server = Arc::clone(&game_server);
        tokio::spawn(async move {
            match ServerBuilder::new().accept(stream).await {
                Ok(ws_stream) => {
                    let connection_id = game_server.next_connection_id().await;
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
