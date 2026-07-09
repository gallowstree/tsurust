use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use tsurust_common::board::PlayerID;
use tsurust_common::lobby::{next_lobby_id, LobbyEvent, Visibility};

use tsurust_common::protocol::{LobbyListing, RoomId, ServerMessage};

use crate::room::{GameRoom, RoomPhase};

pub type ConnectionId = usize;

pub struct GameServer {
    pub rooms: Arc<RwLock<HashMap<RoomId, GameRoom>>>,
    #[allow(dead_code)]
    pub connections: Arc<RwLock<HashMap<ConnectionId, RoomId>>>,
    next_connection_id: Arc<RwLock<ConnectionId>>,
}

impl Default for GameServer {
    fn default() -> Self {
        Self::new()
    }
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
        visibility: Visibility,
        turn_timer_secs: Option<u64>,
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
        let mut room = GameRoom::new(room_id.clone(), room_name, visibility, turn_timer_secs);
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

    /// The public lobby directory: every public room, joinable ones first.
    /// Private rooms never appear here — they are reachable only by code.
    pub async fn list_public_rooms(&self) -> Vec<LobbyListing> {
        let rooms = self.rooms.read().await;
        let mut listings: Vec<LobbyListing> = rooms
            .values()
            .filter(|room| room.visibility == Visibility::Public)
            .map(|room| {
                let (player_count, max_players, in_progress) = match &room.phase {
                    RoomPhase::Lobby(lobby) => (lobby.players.len(), lobby.max_players, false),
                    // A started game is locked to its players; report the
                    // count as the cap since nobody else can join.
                    RoomPhase::Playing(game) => (game.players.len(), game.players.len(), true),
                };
                LobbyListing {
                    room_id: room.id.clone(),
                    name: room.name.clone(),
                    player_count,
                    max_players,
                    in_progress,
                }
            })
            .collect();
        listings.sort_by(|a, b| {
            a.in_progress
                .cmp(&b.in_progress)
                .then_with(|| a.name.cmp(&b.name))
        });
        listings
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

    pub async fn leave_room(
        server: &Arc<GameServer>,
        room_id: RoomId,
        player_id: PlayerID,
    ) -> Result<(), String> {
        GameServer::handle_disconnect(server, room_id, player_id).await;
        Ok(())
    }

    /// `server` is an Arc so the turn timer can be re-armed when the
    /// disconnect passes the turn to the next player.
    pub async fn handle_disconnect(server: &Arc<GameServer>, room_id: RoomId, player_id: PlayerID) {
        let mut rooms = server.rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_id) {
            let should_remove = room.handle_disconnect(player_id);
            if should_remove {
                tracing::info!(%room_id, "room empty, removing");
                rooms.remove(&room_id);
            } else {
                // If the turn changed hands this arms the new deadline; other
                // cases spawn a duplicate that no-ops on its generation check.
                GameServer::arm_turn_timer(server, room);
            }
        }
    }

    /// Arm a one-shot timer for the room's current turn. The task holds only
    /// (room id, generation): when it fires it re-checks the room, so a turn
    /// played in the meantime — or a finished/removed game — makes it a no-op.
    /// Duplicate arms for the same turn are harmless for the same reason.
    /// Call after any change that may have started a new turn.
    pub fn arm_turn_timer(server: &Arc<GameServer>, room: &GameRoom) {
        let (Some(deadline), Some(generation)) = (room.turn_deadline, room.turn_generation())
        else {
            return;
        };
        let server = Arc::clone(server);
        let room_id = room.id.clone();
        tokio::spawn(async move {
            tokio::time::sleep_until(deadline).await;
            GameServer::fire_turn_timer(server, room_id, generation).await;
        });
    }

    /// Timer expiry: if the room is still waiting on the same turn, play it
    /// for the slow player and arm the clock for the next one. Returns whether
    /// a move was forced (stale/finished timers return false).
    pub async fn fire_turn_timer(
        server: Arc<GameServer>,
        room_id: RoomId,
        generation: (usize, PlayerID),
    ) -> bool {
        let mut rooms = server.rooms.write().await;
        let Some(room) = rooms.get_mut(&room_id) else {
            return false;
        };
        if room.turn_generation() != Some(generation) {
            return false; // the awaited turn was played (or the game ended)
        }
        if let Err(e) = room.force_current_move() {
            tracing::warn!(%room_id, error = %e, "failed to auto-play timed-out turn");
            return false;
        }
        GameServer::arm_turn_timer(&server, room);
        true
    }
}
