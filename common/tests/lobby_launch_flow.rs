use tsurust_common::lobby::*;
use tsurust_common::board::*;

#[test]
fn test_complete_create_and_join_flow() {
    // === Create Lobby Flow ===

    // Creator creates lobby and auto-joins
    let (mut lobby1, creator_id) = Lobby::new_with_creator(
        "Game Room".to_string(),
        "Alice".to_string()
    );

    // Verify lobby created with proper ID
    assert_eq!(lobby1.id.len(), 4);
    assert_eq!(lobby1.name, "Game Room");
    assert_eq!(lobby1.players.len(), 1);

    // Verify creator auto-joined
    assert_eq!(creator_id, 1);
    assert_eq!(lobby1.players[&creator_id].name, "Alice");
    assert_eq!(lobby1.players[&creator_id].color, (220, 50, 47)); // Red

    // Creator places pawn
    lobby1.handle_event(LobbyEvent::PawnPlaced {
        player_id: creator_id,
        position: PlayerPos::new(0, 2, 4),
    }).expect("Creator should be able to place pawn");

    assert_eq!(
        lobby1.players[&creator_id].spawn_position,
        Some(PlayerPos::new(0, 2, 4))
    );

    // === Join Lobby Flow ===

    // Simulate second player joining (via normalized lobby ID)
    let lobby_id = lobby1.id.clone();
    let normalized = normalize_lobby_id(&lobby_id.to_lowercase())
        .expect("Generated ID should be valid");
    assert_eq!(normalized, lobby_id);

    // Second player joins
    lobby1.handle_event(LobbyEvent::PlayerJoined {
        player_id: 2,
        player_name: "Bob".to_string(),
    }).expect("Second player should be able to join");

    assert_eq!(lobby1.players.len(), 2);
    assert_eq!(lobby1.players[&2].name, "Bob");
    assert_eq!(lobby1.players[&2].color, (133, 153, 0)); // Green

    // Second player places pawn
    lobby1.handle_event(LobbyEvent::PawnPlaced {
        player_id: 2,
        position: PlayerPos::new(5, 3, 0),
    }).expect("Second player should be able to place pawn");

    // === Start Game ===

    // Lobby should be ready to start
    assert!(lobby1.can_start());

    lobby1.handle_event(LobbyEvent::StartGame)
        .expect("Lobby should be able to start");
    assert!(lobby1.started);

    // Convert to game
    let game = lobby1.to_game().expect("Should convert to game");

    // Verify game state
    assert_eq!(game.players.len(), 2);
    assert_eq!(game.players[0].name, "Alice");
    assert_eq!(game.players[0].pos, PlayerPos::new(0, 2, 4));
    assert_eq!(game.players[0].color, (220, 50, 47));
    assert_eq!(game.players[1].name, "Bob");
    assert_eq!(game.players[1].pos, PlayerPos::new(5, 3, 0));
    assert_eq!(game.players[1].color, (133, 153, 0));
}