use tokio::sync::broadcast;

use tsurust_common::board::{Move, PlayerID, PlayerPos};
use tsurust_common::game::{Game, TurnResult};
use tsurust_common::lobby::{Lobby, LobbyEvent};

use tsurust_common::protocol::{RoomId, ServerMessage};

pub struct GameRoom {
    pub id: RoomId,
    pub game: Game,
    pub lobby: Option<Lobby>, // Lobby state (None when game has started)
    // Broadcast channel for pushing updates to all connected clients
    pub update_tx: broadcast::Sender<ServerMessage>,
}

impl GameRoom {
    pub fn new(id: RoomId, room_name: String, game: Game) -> Self {
        let (update_tx, _) = broadcast::channel(100);

        // Create a lobby with the custom name
        let lobby = Lobby::new(id.clone(), room_name);

        Self {
            id,
            game,
            lobby: Some(lobby),
            update_tx,
        }
    }

    pub fn place_tile(&mut self, player_id: PlayerID, mov: Move) -> Result<TurnResult, String> {
        // Validate it's this player's turn
        if self.game.current_player_id != player_id {
            return Err(format!(
                "Not your turn! Current player is {}, you are {}",
                self.game.current_player_id, player_id
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
        let result = self.game.perform_move(mov)
            .map_err(|e| format!("Invalid move: {:?}", e))?;

        // Broadcast game state update to all clients in this room
        // This contains all information clients need (game state, current player, etc.)
        println!("[SERVER] Broadcasting GameStateUpdate - current_player: {}", self.game.current_player_id);
        for (pid, hand) in &self.game.hands {
            println!("[SERVER]   Player {} hand ({} tiles): {:?}", pid, hand.len(), hand);
        }
        let state_update = ServerMessage::GameStateUpdate {
            room_id: self.id.clone(),
            state: self.game.clone(),
        };
        if let Err(e) = self.update_tx.send(state_update) {
            eprintln!("Failed to broadcast GameStateUpdate: {:?}", e);
        }

        Ok(result)
    }

    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.update_tx.send(message);
    }

    pub fn place_pawn(&mut self, player_id: PlayerID, position: PlayerPos) -> Result<(), String> {
        let lobby = self.lobby.as_mut()
            .ok_or("Game has already started, cannot place pawns".to_string())?;

        // Handle pawn placement in lobby
        lobby.handle_event(LobbyEvent::PawnPlaced {
            player_id,
            position,
        }).map_err(|e| format!("Failed to place pawn: {:?}", e))?;

        // Clone the lobby before broadcasting to avoid borrow checker issues
        let lobby_clone = lobby.clone();

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
            lobby: lobby_clone,
        };
        self.broadcast(lobby_update);

        Ok(())
    }

    pub fn start_game(&mut self) -> Result<(), String> {
        let lobby = self.lobby.take()
            .ok_or("Game has already started or lobby does not exist".to_string())?;

        // Start the game in the lobby
        let mut lobby_clone = lobby.clone();
        lobby_clone.handle_event(tsurust_common::lobby::LobbyEvent::StartGame)
            .map_err(|e| format!("Failed to start game in lobby: {:?}", e))?;

        // Convert lobby to game
        let game = lobby_clone.to_game()
            .map_err(|e| format!("Failed to convert lobby to game: {:?}", e))?;

        // Update the game state
        self.game = game.clone();

        // Debug: show initial game state
        println!("[SERVER] Game started! Players: {:?}", self.game.players.iter().map(|p| p.id).collect::<Vec<_>>());
        println!("[SERVER] Current player: {}", self.game.current_player_id);
        for (pid, hand) in &self.game.hands {
            println!("[SERVER]   Player {} initial hand ({} tiles): {:?}", pid, hand.len(), hand);
        }

        // Broadcast game started to all clients
        let game_started = ServerMessage::GameStarted {
            room_id: self.id.clone(),
            game,
        };
        self.broadcast(game_started);

        Ok(())
    }
}
