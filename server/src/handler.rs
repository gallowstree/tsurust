use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use tsurust_common::board::PlayerID;
use tsurust_common::protocol::{ClientMessage, RoomId, ServerMessage};

use crate::server::{ConnectionId, GameServer};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PING_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn handle_connection(
    mut ws: WebSocketStream<tokio::net::TcpStream>,
    connection_id: ConnectionId,
    server: Arc<GameServer>,
) {
    let mut update_rx: Option<broadcast::Receiver<ServerMessage>> = None;
    let mut current_room: Option<RoomId> = None;
    let mut current_player_id: Option<PlayerID> = None;
    // Heartbeat: ping every PING_INTERVAL; after a ping, the deadline shortens
    // to PING_TIMEOUT and a pong stretches it back to PING_INTERVAL
    let mut ping_deadline = time::Instant::now() + PING_INTERVAL;
    let mut waiting_for_pong = false;

    loop {
        tokio::select! {
            // Receive messages from client
            msg_result = ws.next() => {
                match msg_result {
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to client-initiated pings
                        let _ = ws.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        waiting_for_pong = false;
                        ping_deadline = time::Instant::now() + PING_INTERVAL;
                    }
                    Some(Ok(msg)) => {
                        if let Err(e) = handle_client_message(
                            &mut ws,
                            msg,
                            &server,
                            &mut update_rx,
                            &mut current_room,
                            &mut current_player_id,
                        ).await {
                            eprintln!("Error handling client message: {}", e);
                            let error_msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = ws.send(Message::Text(json.into())).await;
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
                    // pending() never resolves — prevents a busy-spin when no room is joined
                    None => std::future::pending().await,
                }
            } => {
                if let Some(update) = update {
                    match serde_json::to_string(&update) {
                        Ok(json) => {
                            if let Err(e) = ws.send(Message::Text(json.into())).await {
                                eprintln!("Failed to send update to connection {}: {}", connection_id, e);
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to serialize broadcast message for connection {}: {}", connection_id, e);
                        }
                    }
                }
            }

            // Send periodic pings; close connection if pong not received
            _ = time::sleep_until(ping_deadline) => {
                if waiting_for_pong {
                    eprintln!(
                        "Connection {} timed out (no pong in {}s), closing",
                        connection_id, PING_TIMEOUT.as_secs()
                    );
                    break;
                }
                if let Err(e) = ws.send(Message::Ping(vec![].into())).await {
                    eprintln!("Failed to send ping to connection {}: {}", connection_id, e);
                    break;
                }
                waiting_for_pong = true;
                ping_deadline = time::Instant::now() + PING_TIMEOUT;
            }
        }
    }

    // Cleanup on disconnect
    if let (Some(room_id), Some(player_id)) = (current_room, current_player_id) {
        println!(
            "Connection {} (player {}) disconnected from room {}",
            connection_id, player_id, room_id
        );
        server.handle_disconnect(room_id, player_id).await;
    }
}

/// Reject messages whose claimed identity doesn't match what this connection was
/// assigned when it created or joined a room. Without this, any client can act
/// as any player (or on any room) just by writing a different id into the message.
fn verify_sender(
    claimed_room: &RoomId,
    claimed_player: Option<PlayerID>,
    current_room: &Option<RoomId>,
    current_player_id: &Option<PlayerID>,
) -> Result<(), String> {
    match current_room {
        Some(room) if room == claimed_room => {}
        _ => return Err(format!("This connection is not in room '{}'", claimed_room)),
    }
    if let Some(claimed) = claimed_player {
        match current_player_id {
            Some(own) if *own == claimed => {}
            _ => return Err(format!("This connection cannot act as player {}", claimed)),
        }
    }
    Ok(())
}

async fn handle_client_message(
    ws: &mut WebSocketStream<tokio::net::TcpStream>,
    msg: Message,
    server: &Arc<GameServer>,
    update_rx: &mut Option<broadcast::Receiver<ServerMessage>>,
    current_room: &mut Option<RoomId>,
    current_player_id: &mut Option<PlayerID>,
) -> Result<(), String> {
    let text = match msg {
        Message::Text(text) => text,
        _ => return Ok(()), // Ignore non-text messages
    };
    let client_msg: ClientMessage = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse client message: {}", e))?;

    println!("[SERVER] Received from client: {:?}", client_msg);

    match client_msg {
        ClientMessage::CreateRoom {
            room_name,
            creator_name,
        } => {
            let (room_id, player_id) = server.create_room(room_name, creator_name).await?;

            // Subscribe to room updates and send initial lobby state
            let rooms = server.rooms.read().await;
            if let Some(room) = rooms.get(&room_id) {
                *update_rx = Some(room.update_tx.subscribe());
                *current_room = Some(room_id.clone());
                *current_player_id = Some(player_id);

                // Send current lobby state to the creator
                if let Some(lobby) = room.lobby() {
                    let lobby_state = ServerMessage::LobbyStateUpdate {
                        room_id: room_id.clone(),
                        lobby: lobby.clone(),
                    };
                    let json = serde_json::to_string(&lobby_state)
                        .map_err(|e| format!("Failed to serialize lobby state: {}", e))?;
                    ws.send(Message::Text(json.into()))
                        .await
                        .map_err(|e| format!("Failed to send lobby state: {}", e))?;
                }
            }
            drop(rooms);

            // Send response
            let response = ServerMessage::RoomCreated { room_id, player_id };
            let json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            ws.send(Message::Text(json.into()))
                .await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::JoinRoom {
            room_id,
            player_name,
        } => {
            let player_id = server
                .join_room(room_id.clone(), player_name.clone())
                .await?;

            // Subscribe to room updates and send current lobby state
            let rooms = server.rooms.read().await;
            if let Some(room) = rooms.get(&room_id) {
                *update_rx = Some(room.update_tx.subscribe());
                *current_room = Some(room_id.clone());
                *current_player_id = Some(player_id);

                // Send current lobby state directly to the joining player
                if let Some(lobby) = room.lobby() {
                    let lobby_state = ServerMessage::LobbyStateUpdate {
                        room_id: room_id.clone(),
                        lobby: lobby.clone(),
                    };
                    let json = serde_json::to_string(&lobby_state)
                        .map_err(|e| format!("Failed to serialize lobby state: {}", e))?;
                    ws.send(Message::Text(json.into()))
                        .await
                        .map_err(|e| format!("Failed to send lobby state: {}", e))?;
                }
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
            ws.send(Message::Text(json.into()))
                .await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::LeaveRoom { room_id, player_id } => {
            verify_sender(&room_id, Some(player_id), current_room, current_player_id)?;
            server.leave_room(room_id, player_id).await?;
            *update_rx = None;
            *current_room = None;
            *current_player_id = None;
        }

        ClientMessage::PlaceTile {
            room_id,
            player_id,
            mov,
        } => {
            verify_sender(&room_id, Some(player_id), current_room, current_player_id)?;
            let mut rooms = server.rooms.write().await;
            let room = rooms
                .get_mut(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            match room.place_tile(player_id, mov) {
                Ok(result) => {
                    println!("[SERVER] PlaceTile success: {:?}", result);
                }
                Err(e) => {
                    println!("[SERVER] PlaceTile error: {}", e);
                    return Err(e);
                }
            }
            // Updates are broadcast automatically by place_tile
            drop(rooms); // Explicitly drop the lock to allow broadcast messages to be received
        }

        ClientMessage::GetGameState { room_id } => {
            let rooms = server.rooms.read().await;
            let room = rooms
                .get(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;
            let game = room
                .game()
                .ok_or_else(|| format!("Room '{}' has not started its game", room_id))?;

            let response = ServerMessage::GameStateUpdate {
                room_id: room_id.clone(),
                state: game.clone(),
            };
            let json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            ws.send(Message::Text(json.into()))
                .await
                .map_err(|e| format!("Failed to send response: {}", e))?;
        }

        ClientMessage::PlacePawn {
            room_id,
            player_id,
            position,
        } => {
            verify_sender(&room_id, Some(player_id), current_room, current_player_id)?;
            let mut rooms = server.rooms.write().await;
            let room = rooms
                .get_mut(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            room.place_pawn(player_id, position)?;
            // Updates are broadcast automatically by place_pawn
        }

        ClientMessage::StartGame { room_id } => {
            verify_sender(&room_id, None, current_room, current_player_id)?;
            let mut rooms = server.rooms.write().await;
            let room = rooms
                .get_mut(&room_id)
                .ok_or_else(|| format!("Room '{}' not found", room_id))?;

            room.start_game()?;
            // GameStarted message is broadcast automatically by start_game
        }
    }

    Ok(())
}
