use tsurust_common::board::{CellCoord, Move, Player, PlayerPos, Segment, Tile};
use tsurust_common::game::Game;
use crate::room::GameRoom;

// Helper function to create a test tile
fn create_test_tile() -> Tile {
    Tile::new([
        Segment { a: 0, b: 1 },
        Segment { a: 2, b: 3 },
        Segment { a: 4, b: 5 },
        Segment { a: 6, b: 7 },
    ])
}

#[test]
fn test_place_tile_validates_turn() {
    // Setup: Create a game with 2 players
    let players = vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
    ];
    let game = Game::new(players);
    let mut room = GameRoom::new("TEST".to_string(), "Test Room".to_string(), game);

    // Start the game
    room.game.deck.take_up_to(3); // Simulate taking tiles for hands

    // Player 1's turn (current_player_id should be 1)
    assert_eq!(room.game.current_player_id, 1);

    // Player 2 tries to place a tile (should fail - not their turn!)
    let mov = Move {
        tile: create_test_tile(),
        cell: CellCoord { row: 0, col: 0 },
        player_id: 2, // Wrong player!
    };

    let result = room.place_tile(2, mov);

    assert!(result.is_err(), "Should reject move from wrong player");
    assert!(
        result.unwrap_err().contains("Not your turn"),
        "Error message should mention turn validation"
    );
}

#[test]
fn test_place_tile_validates_move_player_id() {
    // Setup
    let players = vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
    ];
    let game = Game::new(players);
    let mut room = GameRoom::new("TEST".to_string(), "Test Room".to_string(), game);

    // Player 1's turn
    assert_eq!(room.game.current_player_id, 1);

    // Player 1 tries to place a tile but the Move has wrong player_id
    let mov = Move {
        tile: create_test_tile(),
        cell: CellCoord { row: 0, col: 0 },
        player_id: 2, // Wrong player_id in the move!
    };

    let result = room.place_tile(1, mov);

    assert!(result.is_err(), "Should reject move with mismatched player_id");
    assert!(
        result.unwrap_err().contains("player_id mismatch"),
        "Error message should mention player_id mismatch"
    );
}

#[test]
fn test_place_tile_accepts_valid_move() {
    // Setup
    let players = vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
    ];
    let game = Game::new(players);
    let mut room = GameRoom::new("TEST".to_string(), "Test Room".to_string(), game);

    // Player 1's turn
    assert_eq!(room.game.current_player_id, 1);

    // Add tile to player 1's hand
    let tile = create_test_tile();
    room.game.hands.get_mut(&1).unwrap().push(tile);

    // Player 1 places a valid tile
    let mov = Move {
        tile,
        cell: CellCoord { row: 0, col: 0 },
        player_id: 1,
    };

    let result = room.place_tile(1, mov);

    assert!(result.is_ok(), "Should accept valid move from current player");
}
