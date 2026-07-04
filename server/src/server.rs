use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use tsurust_common::board::PlayerID;
use tsurust_common::lobby::{next_lobby_id, LobbyEvent};

use tsurust_common::protocol::{RoomId, ServerMessage};

use crate::room::{GameRoom, RoomPhase};

pub type ConnectionId = usize;

pub struct GameServer {
    pub rooms: Arc<RwLock<HashMap<RoomId, GameRoom>>>,
    #[allow(dead_code)]
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

    pub async fn create_room(
        &self,
        room_name: String,
        creator_name: String,
    ) -> Result<(RoomId, PlayerID), String> {
        // Hold the write lock across generate-check-insert so two concurrent
        // creates can't race into the same room ID
        let mut rooms = self.rooms.write().await;
        let room_id = loop {
            let id = next_lobby_id();
            if !rooms.contains_key(&id) {
                break id;
            }
        };

        let player_id = 1;
        let mut room = GameRoom::new(room_id.clone(), room_name);
        if let RoomPhase::Lobby(lobby) = &mut room.phase {
            lobby
                .handle_event(LobbyEvent::PlayerJoined {
                    player_id,
                    player_name: creator_name,
                })
                .map_err(|e| format!("Failed to add creator to lobby: {}", e))?;
        }

        rooms.insert(room_id.clone(), room);

        Ok((room_id, player_id))
    }

    pub async fn join_room(
        &self,
        room_id: RoomId,
        player_name: String,
    ) -> Result<PlayerID, String> {
        let mut rooms = self.rooms.write().await;
        let room = rooms
            .get_mut(&room_id)
            .ok_or_else(|| format!("Room '{}' not found", room_id))?;

        // A started game has no lobby; joining it would add a ghost player with
        // no hand or stats that wedges the turn rotation
        room.touch();
        let RoomPhase::Lobby(lobby) = &mut room.phase else {
            return Err(format!("Room '{}' has already started its game", room_id));
        };

        // Determine next player ID (start from 1, not 0)
        let player_id = lobby.players.keys().max().copied().unwrap_or(0) + 1;

        lobby
            .handle_event(LobbyEvent::PlayerJoined {
                player_id,
                player_name: player_name.clone(),
            })
            .map_err(|e| format!("Failed to join room '{}': {}", room_id, e))?;

        let lobby_snapshot = lobby.clone();

        // Broadcast player joined
        let join_msg = ServerMessage::PlayerJoined {
            room_id: room_id.clone(),
            player_id,
            player_name,
        };
        room.broadcast(join_msg);

        // Broadcast updated lobby state
        let lobby_update = ServerMessage::LobbyStateUpdate {
            room_id: room_id.clone(),
            lobby: lobby_snapshot,
        };
        room.broadcast(lobby_update);

        Ok(player_id)
    }

    /// Remove rooms that have no connected clients and have been idle past the
    /// timeout. A room with any live broadcast subscriber is never reaped, so a
    /// game waiting on a slow player stays alive indefinitely; the timeout is a
    /// grace period for rooms whose last connection is gone (or was orphaned by
    /// a client that created/joined another room). Returns how many were reaped.
    pub async fn reap_idle_rooms(&self, idle_timeout: Duration) -> usize {
        let mut rooms = self.rooms.write().await;
        let before = rooms.len();
        rooms.retain(|id, room| {
            let reap = room.update_tx.receiver_count() == 0
                && room.last_activity.elapsed() >= idle_timeout;
            if reap {
                tracing::info!(room_id = %id, "reaping idle room with no connected clients");
            }
            !reap
        });
        before - rooms.len()
    }

    pub async fn leave_room(&self, room_id: RoomId, player_id: PlayerID) -> Result<(), String> {
        self.handle_disconnect(room_id, player_id).await;
        Ok(())
    }

    pub async fn handle_disconnect(&self, room_id: RoomId, player_id: PlayerID) {
        let mut rooms = self.rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_id) {
            let should_remove = room.handle_disconnect(player_id);
            if should_remove {
                tracing::info!(%room_id, "room empty, removing");
                rooms.remove(&room_id);
            }
        }
    }
}
