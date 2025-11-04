use super::protocol::{ClientMessage, ServerMessage};
use crate::board::{CellCoord, Move, Player, PlayerPos, Segment, Tile};
use crate::game::{Game, TurnResult};
use crate::lobby::Lobby;

/// Test that all ServerMessage variants can be serialized and deserialized
#[test]
fn test_server_message_serialization() {
    let test_cases: Vec<ServerMessage> = vec![
        ServerMessage::RoomCreated {
            room_id: "TEST123".to_string(),
            player_id: 1,
        },
        ServerMessage::PlayerJoined {
            room_id: "TEST123".to_string(),
            player_id: 2,
            player_name: "Alice".to_string(),
        },
        ServerMessage::LobbyStateUpdate {
            room_id: "TEST123".to_string(),
            lobby: Lobby::new("TEST123".to_string(), "Test Lobby".to_string()),
        },
        ServerMessage::PawnPlaced {
            room_id: "TEST123".to_string(),
            player_id: 1,
            position: PlayerPos::new(0, 0, 4),
        },
        ServerMessage::GameStarted {
            room_id: "TEST123".to_string(),
            game: create_test_game(),
        },
        ServerMessage::GameStateUpdate {
            room_id: "TEST123".to_string(),
            state: create_test_game(),
        },
        ServerMessage::TurnCompleted {
            room_id: "TEST123".to_string(),
            result: TurnResult::TurnAdvanced {
                turn_number: 1,
                next_player: 2,
                eliminated: vec![],
            },
        },
    ];

    for (i, message) in test_cases.iter().enumerate() {
        // Test serialization
        let json = serde_json::to_string(message)
            .expect(&format!("Failed to serialize ServerMessage variant #{}", i));

        assert!(!json.is_empty(), "Serialized JSON should not be empty");

        // Test deserialization
        let deserialized: ServerMessage = serde_json::from_str(&json)
            .expect(&format!("Failed to deserialize ServerMessage variant #{}", i));

        // Verify discriminant matches (same variant)
        assert_eq!(
            std::mem::discriminant(message),
            std::mem::discriminant(&deserialized),
            "Deserialized message should have same variant as original"
        );
    }
}

/// Test that all ClientMessage variants can be serialized and deserialized
#[test]
fn test_client_message_serialization() {
    let test_cases: Vec<ClientMessage> = vec![
        ClientMessage::CreateRoom {
            room_name: "TEST123".to_string(),
            creator_name: "Alice".to_string(),
        },
        ClientMessage::JoinRoom {
            room_id: "TEST123".to_string(),
            player_name: "Bob".to_string(),
        },
        ClientMessage::LeaveRoom {
            room_id: "TEST123".to_string(),
            player_id: 1,
        },
        ClientMessage::PlacePawn {
            room_id: "TEST123".to_string(),
            player_id: 1,
            position: PlayerPos::new(0, 0, 4),
        },
        ClientMessage::StartGame {
            room_id: "TEST123".to_string(),
        },
        ClientMessage::PlaceTile {
            room_id: "TEST123".to_string(),
            player_id: 1,
            mov: Move {
                tile: create_test_tile(),
                cell: CellCoord { row: 0, col: 0 },
                player_id: 1,
            },
        },
        ClientMessage::GetGameState {
            room_id: "TEST123".to_string(),
        },
    ];

    for (i, message) in test_cases.iter().enumerate() {
        // Test serialization
        let json = serde_json::to_string(message)
            .expect(&format!("Failed to serialize ClientMessage variant #{}", i));

        assert!(!json.is_empty(), "Serialized JSON should not be empty");

        // Test deserialization
        let deserialized: ClientMessage = serde_json::from_str(&json)
            .expect(&format!("Failed to deserialize ClientMessage variant #{}", i));

        // Verify discriminant matches (same variant)
        assert_eq!(
            std::mem::discriminant(message),
            std::mem::discriminant(&deserialized),
            "Deserialized message should have same variant as original"
        );
    }
}

/// Test that Game struct can be serialized (critical for GameStateUpdate)
#[test]
fn test_game_serialization() {
    let game = create_test_game();

    // This should not panic - if it does, online multiplayer is broken
    let json = serde_json::to_string(&game)
        .expect("Game struct must be serializable for online multiplayer to work");

    assert!(!json.is_empty());

    // Verify it can be deserialized
    let deserialized: Game = serde_json::from_str(&json)
        .expect("Game struct must be deserializable");

    assert_eq!(deserialized.current_player_id, game.current_player_id);
    assert_eq!(deserialized.players.len(), game.players.len());
}

/// Test that Game struct with tiles placed can be serialized
/// This is the scenario that was broken before (tile_trails and player_trails)
#[test]
fn test_game_with_moves_serialization() {
    let mut game = create_test_game();

    // Add the tile to player 1's hand first
    let tile = create_test_tile();
    game.hands.get_mut(&1).unwrap().push(tile);

    // Place a tile (this populates tile_trails and player_trails)
    let mov = Move {
        tile,
        cell: CellCoord { row: 0, col: 0 },
        player_id: 1,
    };

    game.perform_move(mov).expect("Move should be valid");

    // This is the critical test - serialization should work even after moves
    let json = serde_json::to_string(&game)
        .expect("Game with moves must be serializable");

    assert!(!json.is_empty());

    // Verify it can be deserialized
    let deserialized: Game = serde_json::from_str(&json)
        .expect("Game with moves must be deserializable");

    // Verify tile_trails were preserved
    assert!(!deserialized.tile_trails.is_empty(), "tile_trails should be preserved after serialization");
    assert!(!deserialized.player_trails.is_empty(), "player_trails should be preserved after serialization");
}

/// Test that tile_trails (Vec of tuples) serializes correctly
/// This was the specific bug that broke multiplayer - JSON requires string map keys
#[test]
fn test_tile_trails_vec_serialization() {
    // Create game with player starting in a safe interior position
    let players = vec![
        Player::new(1, PlayerPos::new(2, 2, 4)), // Middle of board
    ];
    let mut game = Game::new(players);

    // Place a single tile to populate tile_trails
    let tile = create_test_tile();
    game.hands.get_mut(&1).unwrap().push(tile);

    let mov = Move {
        tile,
        cell: CellCoord { row: 2, col: 2 },
        player_id: 1,
    };

    game.perform_move(mov).expect("Move should be valid");

    // Verify tile_trails is populated
    assert!(!game.tile_trails.is_empty(), "tile_trails should have entries");

    // Critical: This must not fail with "key must be a string" error
    let json = serde_json::to_string(&game)
        .expect("tile_trails (Vec) must serialize to JSON");

    // Verify the JSON doesn't contain problematic structures
    assert!(json.contains("tile_trails"), "JSON should contain tile_trails field");

    // Verify round-trip works
    let deserialized: Game = serde_json::from_str(&json)
        .expect("tile_trails must deserialize from JSON");

    // Verify tile_trails structure is preserved
    assert_eq!(deserialized.tile_trails.len(), game.tile_trails.len(),
               "tile_trails length should be preserved");
}

// Helper function to create a test game
fn create_test_game() -> Game {
    let players = vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
    ];
    Game::new(players)
}

// Helper function to create a test tile
fn create_test_tile() -> Tile {
    Tile::new([
        Segment { a: 0, b: 1 },
        Segment { a: 2, b: 3 },
        Segment { a: 4, b: 5 },
        Segment { a: 6, b: 7 },
    ])
}
