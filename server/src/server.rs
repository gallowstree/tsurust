use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tsurust_common::board::{Player, PlayerPos, CellCoord, PlayerID};
use tsurust_common::game::Game;

use crate::protocol::{RoomId, ServerMessage};
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
        let room_id = room_name.clone();

        // Check if room already exists
        {
            let rooms = self.rooms.read().await;
            if rooms.contains_key(&room_id) {
                return Err(format!("Room '{}' already exists", room_id));
            }
        }

        // Create initial player at default starting position
        let player_id = 0;
        let start_pos = PlayerPos {
            cell: CellCoord { row: 0, col: 0 },
            endpoint: 0,
        };
        let player_color = tsurust_common::colors::get_player_color(player_id);
        let player = Player::new_with_name(player_id, creator_name, start_pos, player_color);

        // Create game with single player
        let game = Game::new(vec![player]);
        let room = GameRoom::new(room_id.clone(), game);

        // Store room
        let mut rooms = self.rooms.write().await;
        rooms.insert(room_id.clone(), room);

        Ok((room_id, player_id))
    }

    pub async fn join_room(&self, room_id: RoomId, player_name: String) -> Result<PlayerID, String> {
        let mut rooms = self.rooms.write().await;
        let room = rooms.get_mut(&room_id)
            .ok_or_else(|| format!("Room '{}' not found", room_id))?;

        // Determine next player ID
        let player_id = room.game.players.len();

        // Create player at default starting position (will be customized in lobby)
        let start_pos = PlayerPos {
            cell: CellCoord { row: 0, col: 0 },
            endpoint: 0,
        };
        let player_color = tsurust_common::colors::get_player_color(player_id);
        let player = Player::new_with_name(player_id, player_name.clone(), start_pos, player_color);

        // Add player to game
        room.game.players.push(player);

        // Broadcast player joined
        let join_msg = ServerMessage::PlayerJoined {
            room_id: room_id.clone(),
            player_id,
            player_name,
        };
        room.broadcast(join_msg);

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
