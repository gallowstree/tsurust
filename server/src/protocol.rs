use serde::{Deserialize, Serialize};
use tsurust_common::board::{Move, PlayerID};
use tsurust_common::game::{Game, TurnResult};

pub type RoomId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateRoom {
        room_name: String,
        creator_name: String,
    },
    JoinRoom {
        room_id: RoomId,
        player_name: String,
    },
    LeaveRoom {
        room_id: RoomId,
        player_id: PlayerID,
    },
    PlaceTile {
        room_id: RoomId,
        player_id: PlayerID,
        mov: Move,
    },
    GetGameState {
        room_id: RoomId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    RoomCreated {
        room_id: RoomId,
        player_id: PlayerID,
    },
    PlayerJoined {
        room_id: RoomId,
        player_id: PlayerID,
        player_name: String,
    },
    PlayerLeft {
        room_id: RoomId,
        player_id: PlayerID,
    },
    GameStateUpdate {
        room_id: RoomId,
        state: Game,
    },
    TurnCompleted {
        room_id: RoomId,
        result: TurnResult,
    },
    Error {
        message: String,
    },
}
