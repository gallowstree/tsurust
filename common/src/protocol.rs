use serde::{Deserialize, Serialize};

use crate::board::{Move, PlayerID, PlayerPos};
use crate::game::{Game, TurnResult};
use crate::lobby::Lobby;

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
    PlacePawn {
        room_id: RoomId,
        player_id: PlayerID,
        position: PlayerPos,
    },
    StartGame {
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
    LobbyStateUpdate {
        room_id: RoomId,
        lobby: Lobby,
    },
    PawnPlaced {
        room_id: RoomId,
        player_id: PlayerID,
        position: PlayerPos,
    },
    GameStarted {
        room_id: RoomId,
        game: Game,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{CellCoord, PlayerPos, Tile};

    #[test]
    fn test_client_message_create_room_serialization() {
        let msg = ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            creator_name: "Alice".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::CreateRoom { room_name, creator_name } => {
                assert_eq!(room_name, "Test Room");
                assert_eq!(creator_name, "Alice");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_client_message_join_room_serialization() {
        let msg = ClientMessage::JoinRoom {
            room_id: "ROOM1".to_string(),
            player_name: "Bob".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::JoinRoom { room_id, player_name } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_name, "Bob");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_client_message_leave_room_serialization() {
        let msg = ClientMessage::LeaveRoom {
            room_id: "ROOM1".to_string(),
            player_id: 42,
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::LeaveRoom { room_id, player_id } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_id, 42);
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_client_message_place_tile_serialization() {
        use crate::board::Segment;

        let mov = Move {
            tile: Tile::new([
                Segment::new(0, 1),
                Segment::new(2, 3),
                Segment::new(4, 5),
                Segment::new(6, 7),
            ]),
            cell: CellCoord { row: 2, col: 3 },
            player_id: 1,
        };

        let msg = ClientMessage::PlaceTile {
            room_id: "ROOM1".to_string(),
            player_id: 1,
            mov: mov.clone(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::PlaceTile { room_id, player_id, mov: deserialized_mov } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_id, 1);
                assert_eq!(deserialized_mov.cell, mov.cell);
                assert_eq!(deserialized_mov.player_id, mov.player_id);
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_client_message_get_game_state_serialization() {
        let msg = ClientMessage::GetGameState {
            room_id: "ROOM1".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::GetGameState { room_id } => {
                assert_eq!(room_id, "ROOM1");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_server_message_room_created_serialization() {
        let msg = ServerMessage::RoomCreated {
            room_id: "ROOM1".to_string(),
            player_id: 1,
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::RoomCreated { room_id, player_id } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_id, 1);
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_server_message_player_joined_serialization() {
        let msg = ServerMessage::PlayerJoined {
            room_id: "ROOM1".to_string(),
            player_id: 2,
            player_name: "Charlie".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::PlayerJoined { room_id, player_id, player_name } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_id, 2);
                assert_eq!(player_name, "Charlie");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_server_message_player_left_serialization() {
        let msg = ServerMessage::PlayerLeft {
            room_id: "ROOM1".to_string(),
            player_id: 3,
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::PlayerLeft { room_id, player_id } => {
                assert_eq!(room_id, "ROOM1");
                assert_eq!(player_id, 3);
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_server_message_error_serialization() {
        let msg = ServerMessage::Error {
            message: "Something went wrong".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }
}
