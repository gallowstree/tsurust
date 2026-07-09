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
        None,
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

    // The test tile drives player 1 straight off the top edge, so it must be
    // their whole hand: beside a surviving alternative the forced-suicide
    // rule would reject it.
    let tile = create_test_tile();
    game.hands.insert(1, vec![tile]);
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
        None,
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
        None,
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

/// A tile that lets a pawn at endpoint 4 of top-edge cell (0,0) escape
/// downward (4 → 1 into the empty cell below), unlike `create_test_tile`,
/// whose 4 → 5 keeps it on the top edge.
fn survivor_tile() -> Tile {
    Tile::new([
        Segment { a: 4, b: 1 },
        Segment { a: 5, b: 0 },
        Segment { a: 2, b: 3 },
        Segment { a: 6, b: 7 },
    ])
}

/// A timed room whose game is in progress, with its clock started.
fn timed_in_game_room(game: Game, timer_secs: u64) -> GameRoom {
    let mut room = GameRoom::new(
        "TEST".to_string(),
        "Test Room".to_string(),
        Visibility::Private,
        Some(timer_secs),
    );
    room.phase = RoomPhase::Playing(game);
    room.reset_turn_clock();
    room
}

#[test]
fn test_force_current_move_picks_a_survivable_placement() {
    let mut game = two_player_game();
    // One fatal and one survivable tile: the forced move must not kill.
    game.hands
        .insert(1, vec![create_test_tile(), survivor_tile()]);
    let mut room = timed_in_game_room(game, 60);
    let mut rx = room.update_tx.subscribe();

    room.force_current_move()
        .expect("a player with tiles can always be auto-played");

    let game = room.game().expect("room is still playing");
    assert_eq!(game.board.history.len(), 1, "one tile was placed");
    assert!(
        game.players[0].alive,
        "with a survivable option the auto-play never suicides"
    );
    assert_eq!(game.current_player_id, 2, "the turn advanced");
    assert!(
        room.turn_deadline.is_some(),
        "the next player's clock is running"
    );

    // The auto-play announces itself, then the fresh state (with a clock).
    let tsurust_common::protocol::ServerMessage::TurnCompleted { auto_played, .. } =
        rx.try_recv().expect("TurnCompleted was broadcast")
    else {
        panic!("expected TurnCompleted first");
    };
    assert!(auto_played, "the turn is flagged as played by the server");
    let tsurust_common::protocol::ServerMessage::GameStateUpdate {
        turn_deadline_secs, ..
    } = rx.try_recv().expect("GameStateUpdate was broadcast")
    else {
        panic!("expected GameStateUpdate second");
    };
    assert!(
        turn_deadline_secs.is_some(),
        "the broadcast state carries the running clock"
    );
}

#[test]
fn test_force_current_move_plays_fatal_hand_and_ends_game() {
    let mut game = two_player_game();
    // Every option is fatal, so the forced move (legally) eliminates player 1.
    game.hands.insert(1, vec![create_test_tile()]);
    let mut room = timed_in_game_room(game, 60);

    room.force_current_move()
        .expect("an all-fatal hand is still playable");

    let game = room.game().expect("room is still playing");
    assert!(!game.players[0].alive, "player 1 drove off the edge");
    assert!(game.is_game_over(), "player 2 is the last one standing");
    assert!(
        room.turn_deadline.is_none(),
        "a finished game runs no clock"
    );
    assert!(
        room.turn_generation().is_none(),
        "no further timers will fire"
    );
}

#[test]
fn test_force_current_move_eliminates_player_without_tiles() {
    let mut game = two_player_game();
    game.hands.insert(1, vec![]);
    let mut room = timed_in_game_room(game, 60);

    room.force_current_move()
        .expect("a tile-less player is eliminated, not stalled on");

    let game = room.game().expect("room is still playing");
    assert!(!game.players[0].alive);
    assert_eq!(game.board.history.len(), 0, "no tile was placed");
}

#[test]
fn test_bystander_disconnect_keeps_current_players_clock() {
    // Four players, so the game survives both disconnects below.
    let game = Game::new(vec![
        Player::new(1, PlayerPos::new(0, 0, 4)),
        Player::new(2, PlayerPos::new(2, 5, 2)),
        Player::new(3, PlayerPos::new(4, 0, 6)),
        Player::new(4, PlayerPos::new(5, 3, 0)),
    ]);
    let mut room = timed_in_game_room(game, 60);
    let deadline = room.turn_deadline.expect("clock is running");

    // Player 3 (not the turn holder) disconnects: player 1's clock must not
    // restart.
    room.handle_disconnect(3);
    assert_eq!(
        room.turn_deadline,
        Some(deadline),
        "a bystander's disconnect must not gift the current player time"
    );

    // The turn holder disconnects: the turn passes and the clock restarts.
    std::thread::sleep(std::time::Duration::from_millis(5));
    room.handle_disconnect(1);
    let new_deadline = room.turn_deadline.expect("next player's clock runs");
    assert!(
        new_deadline > deadline,
        "the new turn holder gets a fresh clock"
    );
}
