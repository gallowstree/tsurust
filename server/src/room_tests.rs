use crate::room::{GameRoom, RoomPhase};
use tsurust_common::board::{CellCoord, Move, Player, PlayerPos, Segment, Tile};
use tsurust_common::game::Game;
use tsurust_common::lobby::Visibility;

// Helper function to create a test tile
fn create_test_tile() -> Tile {
    Tile::new([
        Segment { a: 0, b: 1 },
        Segment { a: 2, b: 3 },
        Segment { a: 4, b: 5 },
        Segment { a: 6, b: 7 },
    ])
}

fn two_player_game() -> Game {
    Game::new(vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
    ])
}

/// A room whose game is in progress (rooms are born in the lobby phase).
fn in_game_room(game: Game) -> GameRoom {
    let mut room = GameRoom::new(
        "TEST".to_string(),
        "Test Room".to_string(),
        Visibility::Private,
    );
    room.phase = RoomPhase::Playing(game);
    room
}

#[test]
fn test_place_tile_validates_turn() {
    let game = two_player_game();
    assert_eq!(game.current_player_id, 1);
    let mut room = in_game_room(game);

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
    let game = two_player_game();
    assert_eq!(game.current_player_id, 1);
    let mut room = in_game_room(game);

    // Player 1 tries to place a tile but the Move has wrong player_id
    let mov = Move {
        tile: create_test_tile(),
        cell: CellCoord { row: 0, col: 0 },
        player_id: 2, // Wrong player_id in the move!
    };

    let result = room.place_tile(1, mov);

    assert!(
        result.is_err(),
        "Should reject move with mismatched player_id"
    );
    assert!(
        result.unwrap_err().contains("player_id mismatch"),
        "Error message should mention player_id mismatch"
    );
}

#[test]
fn test_place_tile_accepts_valid_move() {
    let mut game = two_player_game();
    assert_eq!(game.current_player_id, 1);

    // Add tile to player 1's hand before the room takes ownership
    let tile = create_test_tile();
    game.hands.get_mut(&1).unwrap().push(tile);
    let mut room = in_game_room(game);

    // Player 1 places a valid tile on their own cell
    let mov = Move {
        tile,
        cell: CellCoord { row: 0, col: 0 },
        player_id: 1,
    };

    let result = room.place_tile(1, mov);

    assert!(
        result.is_ok(),
        "Should accept valid move from current player"
    );
}

#[test]
fn test_place_tile_rejected_during_lobby_phase() {
    // Rooms are born in the lobby phase — there is no game to place tiles in.
    let mut room = GameRoom::new(
        "TEST".to_string(),
        "Test Room".to_string(),
        Visibility::Private,
    );

    let mov = Move {
        tile: create_test_tile(),
        cell: CellCoord { row: 0, col: 0 },
        player_id: 1,
    };

    let result = room.place_tile(1, mov);

    assert!(
        result.is_err(),
        "Should reject moves before the game starts"
    );
    assert!(
        result.unwrap_err().contains("not started"),
        "Error message should mention the game hasn't started"
    );
}

#[test]
fn test_failed_start_leaves_room_joinable() {
    // Starting with an unready lobby (no players placed) must fail — and must
    // NOT consume the lobby, or the room would be bricked in a phantom
    // "playing" phase with no game.
    let mut room = GameRoom::new(
        "TEST".to_string(),
        "Test Room".to_string(),
        Visibility::Private,
    );

    let result = room.start_game();
    assert!(result.is_err(), "an empty lobby cannot start a game");

    assert!(
        room.lobby().is_some(),
        "the lobby must survive a failed start"
    );
    assert!(
        room.place_pawn(1, PlayerPos::new(0, 2, 5)).is_err(),
        "pawn placement still validates (player 1 never joined)"
    );
}
