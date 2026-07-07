use std::time::Instant;

use tokio::sync::broadcast;

use tsurust_common::board::{Move, PlayerID, PlayerPos};
use tsurust_common::game::{Game, TurnResult};
use tsurust_common::lobby::{Lobby, LobbyEvent, Visibility};

use tsurust_common::protocol::{RoomId, ServerMessage};

/// A room is either gathering players in a lobby or running a game — never
/// both. The variants own their state, so a lobby-phase room has no `Game`
/// to poke and a playing room has no lobby to re-join.
pub enum RoomPhase {
    Lobby(Lobby),
    Playing(Game),
}

pub struct GameRoom {
    pub id: RoomId,
    /// Room name, kept here as well as in the lobby so it survives the
    /// transition into the Playing phase (the lobby is consumed on start).
    pub name: String,
    /// Public rooms appear in the lobby directory; private rooms are
    /// reachable only by their room code.
    pub visibility: Visibility,
    pub phase: RoomPhase,
    // Broadcast channel for pushing updates to all connected clients
    pub update_tx: broadcast::Sender<ServerMessage>,
    /// When this room was last created/acted on — used by the idle-room reaper
    /// as a grace period for rooms with no connected clients.
    pub last_activity: Instant,
}

impl GameRoom {
    pub fn new(id: RoomId, room_name: String, visibility: Visibility) -> Self {
        let (update_tx, _) = broadcast::channel(100);

        let mut lobby = Lobby::new(id.clone(), room_name.clone());
        lobby.visibility = visibility;

        Self {
            phase: RoomPhase::Lobby(lobby),
            id,
            name: room_name,
            visibility,
            update_tx,
            last_activity: Instant::now(),
        }
    }

    pub(crate) fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn lobby(&self) -> Option<&Lobby> {
        match &self.phase {
            RoomPhase::Lobby(lobby) => Some(lobby),
            RoomPhase::Playing(_) => None,
        }
    }

    pub fn game(&self) -> Option<&Game> {
        match &self.phase {
            RoomPhase::Playing(game) => Some(game),
            RoomPhase::Lobby(_) => None,
        }
    }

    pub fn place_tile(&mut self, player_id: PlayerID, mov: Move) -> Result<TurnResult, String> {
        self.touch();
        let RoomPhase::Playing(game) = &mut self.phase else {
            return Err("Game has not started yet".to_string());
        };

        // Validate it's this player's turn
        if game.current_player_id != player_id {
            return Err(format!(
                "Not your turn! Current player is {}, you are {}",
                game.current_player_id, player_id
            ));
        }

        // Validate the move is for the correct player
        if mov.player_id != player_id {
            return Err(format!(
                "Move player_id mismatch! Expected {}, got {}",
                player_id, mov.player_id
            ));
        }

        // Validate and perform move
        let result = game
            .perform_move(mov)
            .map_err(|e| format!("Invalid move: {}", e))?;

        // Broadcast game state update to all clients in this room
        // This contains all information clients need (game state, current player, etc.)
        let state_update = ServerMessage::GameStateUpdate {
            room_id: self.id.clone(),
            state: game.clone(),
        };
        if self.update_tx.send(state_update).is_err() {
            // Benign: broadcast::send only fails when no client is subscribed
            tracing::debug!(room_id = %self.id, "no subscribers for GameStateUpdate broadcast");
        }

        Ok(result)
    }

    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.update_tx.send(message);
    }

    pub fn place_pawn(&mut self, player_id: PlayerID, position: PlayerPos) -> Result<(), String> {
        self.touch();
        let RoomPhase::Lobby(lobby) = &mut self.phase else {
            return Err("Game has already started, cannot place pawns".to_string());
        };

        // Handle pawn placement in lobby
        lobby
            .handle_event(LobbyEvent::PawnPlaced {
                player_id,
                position,
            })
            .map_err(|e| format!("Failed to place pawn: {}", e))?;

        let lobby_snapshot = lobby.clone();

        // Broadcast pawn placement to all clients
        let pawn_msg = ServerMessage::PawnPlaced {
            room_id: self.id.clone(),
            player_id,
            position,
        };
        self.broadcast(pawn_msg);

        // Also broadcast full lobby state update
        let lobby_update = ServerMessage::LobbyStateUpdate {
            room_id: self.id.clone(),
            lobby: lobby_snapshot,
        };
        self.broadcast(lobby_update);

        Ok(())
    }

    /// Returns true if the room is now empty and should be removed.
    pub fn handle_disconnect(&mut self, player_id: PlayerID) -> bool {
        self.touch();
        match &mut self.phase {
            RoomPhase::Lobby(lobby) => {
                // Lobby phase: remove the player entirely
                let _ = lobby.handle_event(LobbyEvent::PlayerLeft { player_id });

                if lobby.players.is_empty() {
                    return true; // Empty room
                }

                // Broadcast updated lobby state
                let lobby_update = ServerMessage::LobbyStateUpdate {
                    room_id: self.id.clone(),
                    lobby: lobby.clone(),
                };
                self.broadcast(lobby_update);
            }
            RoomPhase::Playing(game) => {
                // Game in progress: same bookkeeping as an on-board elimination —
                // hand back to deck, stats closed, turn passed in rotation order
                game.eliminate_player(player_id);

                if !game.players.iter().any(|p| p.alive) {
                    return true; // Empty room
                }

                // Broadcast updated game state; clients derive a win from
                // is_game_over() when one player remains
                let state_update = ServerMessage::GameStateUpdate {
                    room_id: self.id.clone(),
                    state: game.clone(),
                };
                self.broadcast(state_update);
            }
        }

        // Broadcast that the player left
        self.broadcast(ServerMessage::PlayerLeft {
            room_id: self.id.clone(),
            player_id,
        });

        false
    }

    pub fn start_game(&mut self) -> Result<(), String> {
        self.touch();
        let RoomPhase::Lobby(lobby) = &mut self.phase else {
            return Err("Game has already started".to_string());
        };

        // Validate readiness and build the game; the lobby is only consumed on
        // success, so a failed start leaves the room in a usable lobby phase
        lobby
            .handle_event(LobbyEvent::StartGame)
            .map_err(|e| format!("Failed to start game: {}", e))?;
        let game = lobby
            .to_game()
            .map_err(|e| format!("Failed to start game: {}", e))?;

        self.phase = RoomPhase::Playing(game.clone());

        tracing::info!(
            room_id = %self.id,
            players = ?game.players.iter().map(|p| p.id).collect::<Vec<_>>(),
            first_turn = game.current_player_id,
            "game started"
        );

        // Broadcast game started to all clients
        let game_started = ServerMessage::GameStarted {
            room_id: self.id.clone(),
            game,
        };
        self.broadcast(game_started);

        Ok(())
    }
}
