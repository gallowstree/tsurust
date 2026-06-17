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
    use crate::board::{CellCoord, Player, Tile};
    use crate::game::Game;
    use crate::lobby::LobbyEvent;

    /// Build a two-player game that has actually had a tile played, so the
    /// integer-keyed maps (hands, stats, player_trails, current_turn_trails) and
    /// the board history are all populated — i.e. the shape that travels inside a
    /// `GameStateUpdate` on the wire.
    fn sample_mid_game() -> Game {
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        let tile = game.hands[&1][0];
        game.perform_move(Move {
            tile,
            cell: CellCoord { row: 0, col: 2 },
            player_id: 1,
        })
        .expect("placing a tile at the current player's cell should be legal");
        game
    }

    #[test]
    fn test_client_message_create_room_serialization() {
        let msg = ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            creator_name: "Alice".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::CreateRoom {
                room_name,
                creator_name,
            } => {
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
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::JoinRoom {
                room_id,
                player_name,
            } => {
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
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

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
            mov,
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ClientMessage::PlaceTile {
                room_id,
                player_id,
                mov: deserialized_mov,
            } => {
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
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

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
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

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
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::PlayerJoined {
                room_id,
                player_id,
                player_name,
            } => {
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
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

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
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        match deserialized {
            ServerMessage::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Wrong message type after deserialization"),
        }
    }

    #[test]
    fn test_client_message_place_pawn_serialization() {
        let msg = ClientMessage::PlacePawn {
            room_id: "ROOM1".to_string(),
            player_id: 2,
            position: PlayerPos::new(5, 3, 0),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ClientMessage::PlacePawn {
            room_id,
            player_id,
            position,
        } = deserialized
        else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
        assert_eq!(player_id, 2);
        assert_eq!(position, PlayerPos::new(5, 3, 0));
    }

    #[test]
    fn test_client_message_start_game_serialization() {
        let msg = ClientMessage::StartGame {
            room_id: "ROOM1".to_string(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ClientMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ClientMessage::StartGame { room_id } = deserialized else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
    }

    #[test]
    fn test_server_message_pawn_placed_serialization() {
        let msg = ServerMessage::PawnPlaced {
            room_id: "ROOM1".to_string(),
            player_id: 1,
            position: PlayerPos::new(0, 2, 5),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ServerMessage::PawnPlaced {
            room_id,
            player_id,
            position,
        } = deserialized
        else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
        assert_eq!(player_id, 1);
        assert_eq!(position, PlayerPos::new(0, 2, 5));
    }

    #[test]
    fn test_server_message_turn_completed_serialization() {
        // Cover every TurnResult variant. TurnResult has no PartialEq, so compare
        // via debug formatting.
        let results = [
            TurnResult::TurnAdvanced {
                turn_number: 3,
                next_player: 2,
                eliminated: vec![],
            },
            TurnResult::PlayerWins {
                turn_number: 5,
                winner: 1,
                eliminated: vec![2],
            },
            TurnResult::Extinction {
                turn_number: 7,
                eliminated: vec![1, 2],
            },
        ];

        for result in results {
            let msg = ServerMessage::TurnCompleted {
                room_id: "ROOM1".to_string(),
                result: result.clone(),
            };

            let json = serde_json::to_string(&msg).expect("Failed to serialize");
            let deserialized: ServerMessage =
                serde_json::from_str(&json).expect("Failed to deserialize");

            let ServerMessage::TurnCompleted {
                room_id,
                result: round_tripped,
            } = deserialized
            else {
                panic!("Wrong message type after deserialization");
            };
            assert_eq!(room_id, "ROOM1");
            assert_eq!(format!("{:?}", round_tripped), format!("{:?}", result));
        }
    }

    #[test]
    fn test_server_message_lobby_state_update_serialization() {
        let mut lobby = Lobby::new("ROOM1".to_string(), "Test Room".to_string());
        lobby
            .handle_event(LobbyEvent::PlayerJoined {
                player_id: 1,
                player_name: "Alice".to_string(),
            })
            .expect("join should succeed");
        lobby
            .handle_event(LobbyEvent::PlayerJoined {
                player_id: 2,
                player_name: "Bob".to_string(),
            })
            .expect("join should succeed");
        lobby
            .handle_event(LobbyEvent::PawnPlaced {
                player_id: 1,
                position: PlayerPos::new(0, 2, 5),
            })
            .expect("pawn placement should succeed");

        let msg = ServerMessage::LobbyStateUpdate {
            room_id: "ROOM1".to_string(),
            lobby: lobby.clone(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ServerMessage::LobbyStateUpdate {
            room_id,
            lobby: round_tripped,
        } = deserialized
        else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
        // The HashMap<PlayerID, LobbyPlayer> must survive the JSON round-trip.
        assert_eq!(round_tripped.players.len(), 2);
        assert_eq!(round_tripped.players[&1].name, "Alice");
        assert_eq!(
            round_tripped.players[&1].spawn_position,
            Some(PlayerPos::new(0, 2, 5))
        );
        assert_eq!(round_tripped.players[&2].name, "Bob");
        assert_eq!(round_tripped.players[&2].spawn_position, None);
    }

    #[test]
    fn test_server_message_game_state_update_serialization() {
        let game = sample_mid_game();
        let msg = ServerMessage::GameStateUpdate {
            room_id: "ROOM1".to_string(),
            state: game.clone(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ServerMessage::GameStateUpdate {
            room_id,
            state: round_tripped,
        } = deserialized
        else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
        assert_eq!(round_tripped.current_player_id, game.current_player_id);
        assert_eq!(round_tripped.board.history, game.board.history);
        assert!(
            !round_tripped.board.history.is_empty(),
            "the sample game should have a move in its history"
        );
        // Every integer-keyed map must round-trip with all of its keys intact.
        assert_eq!(round_tripped.hands, game.hands);
        for id in game.hands.keys() {
            assert!(round_tripped.stats.contains_key(id), "stats lost key {id}");
            assert!(
                round_tripped.player_trails.contains_key(id),
                "player_trails lost key {id}"
            );
        }
    }

    #[test]
    fn test_server_message_game_started_serialization() {
        let game = sample_mid_game();
        let msg = ServerMessage::GameStarted {
            room_id: "ROOM1".to_string(),
            game: game.clone(),
        };

        let json = serde_json::to_string(&msg).expect("Failed to serialize");
        let deserialized: ServerMessage =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let ServerMessage::GameStarted {
            room_id,
            game: round_tripped,
        } = deserialized
        else {
            panic!("Wrong message type after deserialization");
        };
        assert_eq!(room_id, "ROOM1");
        assert_eq!(round_tripped.current_player_id, game.current_player_id);
        assert_eq!(round_tripped.hands, game.hands);
        assert_eq!(round_tripped.players.len(), game.players.len());
    }
}
