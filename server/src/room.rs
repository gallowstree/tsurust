use tokio::sync::broadcast;

use tsurust_common::board::{Move, PlayerID};
use tsurust_common::game::{Game, TurnResult};

use tsurust_common::protocol::{RoomId, ServerMessage};

pub struct GameRoom {
    pub id: RoomId,
    pub game: Game,
    // Broadcast channel for pushing updates to all connected clients
    pub update_tx: broadcast::Sender<ServerMessage>,
}

impl GameRoom {
    pub fn new(id: RoomId, game: Game) -> Self {
        let (update_tx, _) = broadcast::channel(100);
        Self {
            id,
            game,
            update_tx,
        }
    }

    pub fn place_tile(&mut self, _player_id: PlayerID, mov: Move) -> Result<TurnResult, String> {
        // Validate and perform move
        let result = self.game.perform_move(mov)
            .map_err(|e| format!("Invalid move: {:?}", e))?;

        // Broadcast turn completed to all clients in this room
        let turn_update = ServerMessage::TurnCompleted {
            room_id: self.id.clone(),
            result: result.clone(),
        };
        let _ = self.update_tx.send(turn_update);

        // Also send full state update
        let state_update = ServerMessage::GameStateUpdate {
            room_id: self.id.clone(),
            state: self.game.clone(),
        };
        let _ = self.update_tx.send(state_update);

        Ok(result)
    }

    pub fn broadcast(&self, message: ServerMessage) {
        let _ = self.update_tx.send(message);
    }
}
