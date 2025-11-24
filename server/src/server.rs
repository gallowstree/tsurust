use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use tsurust_common::board::{CellCoord, Player, PlayerID, PlayerPos};
use tsurust_common::game::Game;
use tsurust_common::lobby::{next_lobby_id, LobbyEvent};

use tsurust_common::protocol::{RoomId, ServerMessage};

use crate::room::GameRoom;

pub type ConnectionId = usize;

pub struct GameServer {
    pub rooms: Arc<RwLock<HashMap<RoomId, GameRoom>>>,
    pub connections: Arc<RwLock<HashMap<ConnectionId, RoomId>>>,
    next_connection_id: Arc<RwLock<ConnectionId>>,
}

impl GameServer {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            next_connection_id: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn next_connection_id(&self) -> ConnectionId {
        let mut id = self.next_connection_id.write().await;
        let current = *id;
        *id += 1;
        current
    }

    pub async fn create_room(&self, room_name: String, creator_name: String) -> Result<(RoomId, PlayerID), String> {
        // Generate a unique short room ID
        let room_id = loop {
            let id = next_lobby_id();
            let rooms = self.rooms.read().await;
            if !rooms.contains_key(&id) {
                break id;
            }
        };

        // Create initial player at default starting position
        let player_id = 1;
        let start_pos = PlayerPos {
            cell: CellCoord { row: 0, col: 0 },
            endpoint: 0,
        };
        let player_color = tsurust_common::colors::get_player_color(player_id);
        let player = Player::new_with_name(player_id, creator_name.clone(), start_pos, player_color);

        // Create game with single player
        let game = Game::new(vec![player]);
        let mut room = GameRoom::new(room_id.clone(), room_name, game);

        // Add creator to lobby
        if let Some(lobby) = &mut room.lobby {
            if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id,
                player_name: creator_name,
            }) {
                eprintln!("Failed to add creator to lobby: {:?}", e);
            }
        }

        // Store room
        let mut rooms = self.rooms.write().await;
        rooms.insert(room_id.clone(), room);

        Ok((room_id, player_id))
    }

    pub async fn join_room(&self, room_id: RoomId, player_name: String) -> Result<PlayerID, String> {
        let mut rooms = self.rooms.write().await;
        let room = rooms.get_mut(&room_id)
            .ok_or_else(|| format!("Room '{}' not found", room_id))?;

        // Determine next player ID (start from 1, not 0)
        let player_id = room.game.players.iter()
            .map(|p| p.id)
            .max()
            .unwrap_or(0) + 1;

        // Create player at default starting position (will be customized in lobby)
        let start_pos = PlayerPos {
            cell: CellCoord { row: 0, col: 0 },
            endpoint: 0,
        };
        let player_color = tsurust_common::colors::get_player_color(player_id);
        let player = Player::new_with_name(player_id, player_name.clone(), start_pos, player_color);

        // Add player to game
        room.game.players.push(player);

        // Add player to lobby if it exists
        if let Some(lobby) = &mut room.lobby {
            if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id,
                player_name: player_name.clone(),
            }) {
                eprintln!("Failed to add player to lobby: {:?}", e);
            }
        }

        // Broadcast player joined
        let join_msg = ServerMessage::PlayerJoined {
            room_id: room_id.clone(),
            player_id,
            player_name,
        };
        room.broadcast(join_msg);

        // Send lobby state update if lobby exists
        if let Some(lobby) = &room.lobby {
            let lobby_update = ServerMessage::LobbyStateUpdate {
                room_id: room_id.clone(),
                lobby: lobby.clone(),
            };
            room.broadcast(lobby_update);
        }

        // Send updated game state
        let state_msg = ServerMessage::GameStateUpdate {
            room_id: room_id.clone(),
            state: room.game.clone(),
        };
        room.broadcast(state_msg);

        Ok(player_id)
    }

    pub async fn leave_room(&self, room_id: RoomId, player_id: PlayerID) -> Result<(), String> {
        let mut rooms = self.rooms.write().await;
        let room = rooms.get_mut(&room_id)
            .ok_or_else(|| format!("Room '{}' not found", room_id))?;

        // Broadcast player left
        let leave_msg = ServerMessage::PlayerLeft {
            room_id: room_id.clone(),
            player_id,
        };
        room.broadcast(leave_msg);

        // TODO: Remove player from game or mark as disconnected
        // For now, just broadcast the message

        Ok(())
    }
}
