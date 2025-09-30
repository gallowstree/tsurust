use tsurust_common::lobby::*;
use tsurust_common::board::PlayerPos;

#[test]
fn test_full_lobby_to_game_flow() {
    // Test complete flow from empty lobby to game start
    let mut lobby = Lobby::new(LobbyId(42), "Integration Test".to_string());

    // Add players
    for i in 1..=4 {
        lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: i,
            player_name: format!("Player{}", i),
        }).unwrap();
    }

    // Place pawns on different edges
    let positions = [
        PlayerPos::new(0, 1, 4), // Top edge
        PlayerPos::new(2, 5, 2), // Right edge
        PlayerPos::new(5, 3, 0), // Bottom edge
        PlayerPos::new(3, 0, 6), // Left edge
    ];

    for (i, &pos) in positions.iter().enumerate() {
        lobby.handle_event(LobbyEvent::PawnPlaced {
            player_id: i + 1,
            position: pos,
        }).unwrap();
    }

    // Start game
    assert!(lobby.can_start());
    lobby.handle_event(LobbyEvent::StartGame).unwrap();

    // Convert to game
    let game = lobby.to_game().unwrap();
    assert_eq!(game.players.len(), 4);

    // Verify each player has a valid position and color assigned
    for player in &game.players {
        // Check that this player's position is one of the expected positions
        assert!(positions.contains(&player.pos));
        // Basic color sanity check - should not be black (0,0,0)
        assert_ne!(player.color, (0, 0, 0));
    }

    // Verify all expected player IDs are present
    let player_ids: std::collections::HashSet<_> = game.players.iter().map(|p| p.id).collect();
    for expected_id in 1..=4 {
        assert!(player_ids.contains(&expected_id));
    }
}

#[test]
fn test_lobby_state_transitions() {
    let mut lobby = Lobby::new(LobbyId(1), "State Test".to_string());

    // Initial state
    assert!(!lobby.started);
    assert!(!lobby.can_start());

    // Add two players
    lobby.handle_event(LobbyEvent::PlayerJoined {
        player_id: 1,
        player_name: "Alice".to_string(),
    }).unwrap();
    lobby.handle_event(LobbyEvent::PlayerJoined {
        player_id: 2,
        player_name: "Bob".to_string(),
    }).unwrap();

    // Still can't start without positions
    assert!(!lobby.can_start());

    // Place pawns
    lobby.handle_event(LobbyEvent::PawnPlaced {
        player_id: 1,
        position: PlayerPos::new(0, 2, 4),
    }).unwrap();
    lobby.handle_event(LobbyEvent::PawnPlaced {
        player_id: 2,
        position: PlayerPos::new(5, 3, 0),
    }).unwrap();

    // Now can start
    assert!(lobby.can_start());

    // Start game
    lobby.handle_event(LobbyEvent::StartGame).unwrap();
    assert!(lobby.started);

    // Can convert to game
    let game = lobby.to_game().unwrap();
    assert_eq!(game.players.len(), 2);
}

#[test]
fn test_error_handling_workflow() {
    let mut lobby = Lobby::new(LobbyId(1), "Error Test".to_string());
    lobby.max_players = 2;

    // Test lobby full
    lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();
    lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 2, player_name: "Bob".to_string() }).unwrap();

    let result = lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 3, player_name: "Charlie".to_string() });
    assert!(matches!(result, Err(LobbyError::LobbyFull)));

    // Test invalid position
    let result = lobby.handle_event(LobbyEvent::PawnPlaced {
        player_id: 1,
        position: PlayerPos::new(2, 2, 0), // Center position
    });
    assert!(matches!(result, Err(LobbyError::InvalidSpawnPosition)));

    // Test position conflict
    let edge_pos = PlayerPos::new(0, 2, 4);
    lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 1, position: edge_pos }).unwrap();

    let result = lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 2, position: edge_pos });
    assert!(matches!(result, Err(LobbyError::PositionTaken)));

    // Test starting when not ready
    let result = lobby.handle_event(LobbyEvent::StartGame);
    assert!(matches!(result, Err(LobbyError::NotReadyToStart)));
}