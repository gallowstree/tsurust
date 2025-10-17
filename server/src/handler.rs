use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_websockets::{Message, WebSocketStream};

use tsurust_common::protocol::{ClientMessage, RoomId, ServerMessage};

use crate::server::{ConnectionId, GameServer};

pub async fn handle_connection(
    mut ws: WebSocketStream<tokio::net::TcpStream>,
    connection_id: ConnectionId,
    server: Arc<GameServer>,
) {
    println!("New WebSocket connection: {}", connection_id);

    let mut update_rx: Option<broadcast::Receiver<ServerMessage>> = None;
    let mut current_room: Option<RoomId> = None;

    loop {
        tokio::select! {
            // Receive messages from client
            msg_result = ws.next() => {
                match msg_result {
                    Some(Ok(msg)) => {
                        if let Err(e) = handle_client_message(
                            &mut ws,
                            msg,
                            &server,
                            &mut update_rx,
                            &mut current_room,
                        ).await {
                            eprintln!("Error handling client message: {}", e);
                            let error_msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = ws.send(Message::text(json)).await;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error for connection {}: {}", connection_id, e);
                        break;
                    }
                    None => {
                        println!("WebSocket connection {} closed by client", connection_id);
                        break;
                    }
                }
            }

            // Forward room updates to this client
            update = async {
                match &mut update_rx {
                    Some(rx) => rx.recv().await.ok(),
                    None => None,
                }
            } => {
                if let Some(update) = update {
                    if let Ok(json) = serde_json::to_string(&update) {
                        if let Err(e) = ws.send(Message::text(json)).await {
                            eprintln!("Failed to send update to connection {}: {}", connection_id, e);
                            break;
                        }
                    }
                }
            }
        }
    }

    // Cleanup on disconnect
    if let Some(room_id) = current_room {
        println!("Connection {} disconnected from room {}", connection_id, room_id);
        // TODO: Handle player disconnect (leave room, notify others, etc.)
    }
}

async fn handle_client_message(
    ws: &mut WebSocketStream<tokio::net::TcpStream>,
    msg: Message,
    server: &Arc<GameServer>,
    update_rx: &mut Option<broadcast::Receiver<ServerMessage>>,
    current_room: &mut Option<RoomId>,
) -> Result<(), String> {
    if !msg.is_text() {
        return Ok(());
    }

    let text = msg.as_text().ok_or("Failed to get text from message")?;
    let client_msg: ClientMessage = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse client message: {}", e))?;

    match client_msg {
        ClientMessage::CreateRoom { room_name, creator_name } => {
            let (room_id, player_id) = server.create_room(room_name, creator_name).await?;

            // Subscribe to room updates
            let rooms = server.rooms.read().await;
            if let Some(room) = rooms.get(&room_id) {
                *update_rx = Some(room.update_tx.subscribe());
                *current_room = Some(room_id.clone());
            }
            drop(rooms);

            // Send response
            let response = ServerMessage::RoomCreated {
                room_id,
                player_id,
            };
            let json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            ws.send(Message::text(json)).await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::JoinRoom { room_id, player_name } => {
            let player_id = server.join_room(room_id.clone(), player_name.clone()).await?;

            // Subscribe to room updates
            let rooms = server.rooms.read().await;
            if let Some(room) = rooms.get(&room_id) {
                *update_rx = Some(room.update_tx.subscribe());
                *current_room = Some(room_id.clone());
            }
            drop(rooms);

            // Send direct confirmation to the joining player
            let response = ServerMessage::PlayerJoined {
                room_id,
                player_id,
                player_name,
            };
            let json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            ws.send(Message::text(json)).await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::LeaveRoom { room_id, player_id } => {
            server.leave_room(room_id, player_id).await?;
            *update_rx = None;
            *current_room = None;
        }

        ClientMessage::PlaceTile { room_id, player_id, mov } => {
            let mut rooms = server.rooms.write().await;
            let room = rooms.get_mut(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            room.place_tile(player_id, mov)?;
            // Updates are broadcast automatically by place_tile
        }

        ClientMessage::GetGameState { room_id } => {
            let rooms = server.rooms.read().await;
            let room = rooms.get(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            let response = ServerMessage::GameStateUpdate {
                room_id: room_id.clone(),
                state: room.game.clone(),
            };
            let json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            ws.send(Message::text(json)).await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::PlacePawn { room_id, player_id, position } => {
            let mut rooms = server.rooms.write().await;
            let room = rooms.get_mut(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            room.place_pawn(player_id, position)?;
            // Updates are broadcast automatically by place_pawn
        }
    }

    Ok(())
}
