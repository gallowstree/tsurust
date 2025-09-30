# Lobby System Design

## Overview

A simple pre-game lobby system that allows players to join, select spawn positions, and start the game. Core game logic remains separate from rendering.

## Core Data Structures

```rust
// In common/src/lobby.rs
#[derive(Debug, Clone, PartialEq)]
pub struct LobbyId(pub u64);

#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: LobbyId,
    pub name: String,
    pub players: HashMap<PlayerID, LobbyPlayer>,
    pub started: bool,
    pub max_players: usize, // Default: 8, minimum: 2
}

#[derive(Debug, Clone)]
pub struct LobbyPlayer {
    pub id: PlayerID,
    pub name: String,
    pub color: (u8, u8, u8),
    pub spawn_position: Option<PlayerPos>, // Must be board edge, ready when Some(_)
}
```

## Events

```rust
// In common/src/lobby.rs
#[derive(Debug, Clone)]
pub enum LobbyEvent {
    PlayerJoined {
        player_id: PlayerID,
        player_name: String,
    },
    PawnPlaced {
        player_id: PlayerID,
        position: PlayerPos, // Must be valid board edge position
    },
    StartGame,
}
```

## Core Logic Implementation

```rust
impl Lobby {
    pub fn new(id: LobbyId, name: String) -> Self {
        Self {
            id,
            name,
            players: HashMap::new(),
            started: false,
            max_players: 8,
        }
    }

    pub fn handle_event(&mut self, event: LobbyEvent) -> Result<(), LobbyError> {
        match event {
            LobbyEvent::PlayerJoined { player_id, player_name } => {
                if self.players.len() >= self.max_players {
                    return Err(LobbyError::LobbyFull);
                }

                let color = self.assign_next_color()?;
                let lobby_player = LobbyPlayer {
                    id: player_id,
                    name: player_name,
                    color,
                    spawn_position: None,
                };

                self.players.insert(player_id, lobby_player);
                Ok(())
            }

            LobbyEvent::PawnPlaced { player_id, position } => {
                if !self.is_valid_edge_position(&position) {
                    return Err(LobbyError::InvalidSpawnPosition);
                }

                if self.is_position_taken(&position, player_id) {
                    return Err(LobbyError::PositionTaken);
                }

                if let Some(player) = self.players.get_mut(&player_id) {
                    player.spawn_position = Some(position);
                    Ok(())
                } else {
                    Err(LobbyError::PlayerNotFound)
                }
            }

            LobbyEvent::StartGame => {
                if !self.can_start() {
                    return Err(LobbyError::NotReadyToStart);
                }

                self.started = true;
                Ok(())
            }
        }
    }

    pub fn can_start(&self) -> bool {
        self.players.len() >= 2 &&
        self.players.values().all(|p| p.spawn_position.is_some())
    }

    pub fn to_game(&self) -> Result<Game, LobbyError> {
        if !self.started {
            return Err(LobbyError::GameNotStarted);
        }

        let players: Vec<Player> = self.players
            .values()
            .filter_map(|lobby_player| {
                lobby_player.spawn_position.map(|pos| {
                    Player::new(lobby_player.id, pos, lobby_player.color)
                })
            })
            .collect();

        if players.len() < 2 {
            return Err(LobbyError::NotEnoughPlayers);
        }

        Ok(Game::new(players))
    }

    fn is_valid_edge_position(&self, pos: &PlayerPos) -> bool {
        let CellCoord { row, col } = pos.cell;
        // Valid board edge positions only
        (row == 0 || row == 5 || col == 0 || col == 5) &&
        row <= 5 && col <= 5
    }

    fn is_position_taken(&self, pos: &PlayerPos, requesting_player: PlayerID) -> bool {
        self.players.values().any(|p|
            p.id != requesting_player &&
            p.spawn_position == Some(*pos)
        )
    }

    fn assign_next_color(&self) -> Result<(u8, u8, u8), LobbyError> {
        // Solarized color scheme
        const COLORS: &[(u8, u8, u8)] = &[
            (220, 50, 47),   // Red
            (133, 153, 0),   // Green
            (38, 139, 210),  // Blue
            (181, 137, 0),   // Yellow
            (211, 54, 130),  // Magenta
            (42, 161, 152),  // Cyan
            (203, 75, 22),   // Orange
            (108, 113, 196), // Violet
        ];

        let used_colors: HashSet<_> = self.players.values()
            .map(|p| p.color)
            .collect();

        COLORS.iter()
            .find(|color| !used_colors.contains(color))
            .copied()
            .ok_or(LobbyError::NoAvailableColors)
    }

}

#[derive(Debug, Clone)]
pub enum LobbyError {
    LobbyFull,
    PlayerNotFound,
    InvalidSpawnPosition,
    PositionTaken,
    NotReadyToStart,
    GameNotStarted,
    NotEnoughPlayers,
    NoAvailableColors,
}
```

## UI Integration Pattern

```rust
// In client-egui/src/lobby_ui.rs (hypothetical)
pub enum LobbyMessage {
    JoinLobby(String), // player name
    SelectSpawnPosition(PlayerPos),
    StartGame,
    LeaveLobby,
}

// Main app would have:
pub enum AppState {
    Lobby(Lobby),
    Game(Game),
}
```

## Simplifications Applied

1. **No Network Layer**: Pure local state management, easy to extend to multiplayer later
2. **Fixed Color Assignment**: Automatic color assignment eliminates player choice complexity
3. **Simple Ready State**: Player becomes ready automatically when they place pawn (spawn_position.is_some())
4. **Direct Position Selection**: Click-to-place on board edges, no dropdown menus
5. **Minimal Validation**: Only essential checks (edge positions, no conflicts)
6. **Boolean State**: Simple started/not-started instead of complex state machine
7. **No Player Removal**: Removed PlayerLeft event for initial simplicity

## Integration with Existing Code

1. **Player Creation**: Extend `Player::new()` to accept color parameter
2. **Game Initialization**: Replace hardcoded player creation with `Lobby::to_game()`
3. **Message System**: Add `LobbyMessage` variants to existing message enum
4. **App State**: Wrap existing game state in `AppState` enum

## Key Benefits

- **Separation of Concerns**: Lobby logic completely separate from game logic and rendering
- **Simple State Management**: Clear state transitions and validation
- **Extensible**: Easy to add features like player names, custom colors, room browser
- **Minimal Changes**: Integrates with existing architecture without major refactoring
- **Board Edge Enforcement**: Automatic validation ensures only valid spawn positions

## Test Coverage Proposal

### Unit Tests for Lobby Logic

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_lobby() {
        let lobby = Lobby::new(LobbyId(1), "Test Room".to_string());
        assert_eq!(lobby.id, LobbyId(1));
        assert_eq!(lobby.name, "Test Room");
        assert_eq!(lobby.players.len(), 0);
        assert!(!lobby.started);
        assert!(!lobby.can_start());
    }

    #[test]
    fn test_player_joining() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

        let result = lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: 1,
            player_name: "Alice".to_string(),
        });

        assert!(result.is_ok());
        assert_eq!(lobby.players.len(), 1);
        assert_eq!(lobby.players[&1].name, "Alice");
        assert_eq!(lobby.players[&1].color, (220, 50, 47)); // First solarized color
        assert!(lobby.players[&1].spawn_position.is_none());
    }

    #[test]
    fn test_lobby_full() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.max_players = 2;

        // Fill lobby
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 2, player_name: "Bob".to_string() }).unwrap();

        // Third player should fail
        let result = lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: 3,
            player_name: "Charlie".to_string(),
        });

        assert!(matches!(result, Err(LobbyError::LobbyFull)));
    }

    #[test]
    fn test_pawn_placement_valid() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();

        let edge_pos = PlayerPos::new(0, 2, 4); // Top edge
        let result = lobby.handle_event(LobbyEvent::PawnPlaced {
            player_id: 1,
            position: edge_pos,
        });

        assert!(result.is_ok());
        assert_eq!(lobby.players[&1].spawn_position, Some(edge_pos));
    }

    #[test]
    fn test_pawn_placement_invalid_position() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();

        let center_pos = PlayerPos::new(2, 2, 0); // Center of board - invalid
        let result = lobby.handle_event(LobbyEvent::PawnPlaced {
            player_id: 1,
            position: center_pos,
        });

        assert!(matches!(result, Err(LobbyError::InvalidSpawnPosition)));
    }

    #[test]
    fn test_pawn_placement_position_taken() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 2, player_name: "Bob".to_string() }).unwrap();

        let edge_pos = PlayerPos::new(0, 2, 4);

        // First player places pawn
        lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 1, position: edge_pos }).unwrap();

        // Second player tries same position
        let result = lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 2, position: edge_pos });

        assert!(matches!(result, Err(LobbyError::PositionTaken)));
    }

    #[test]
    fn test_can_start_conditions() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

        // Empty lobby cannot start
        assert!(!lobby.can_start());

        // Single player cannot start
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();
        assert!(!lobby.can_start());

        // Two players without positions cannot start
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 2, player_name: "Bob".to_string() }).unwrap();
        assert!(!lobby.can_start());

        // One player with position cannot start
        lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 1, position: PlayerPos::new(0, 2, 4) }).unwrap();
        assert!(!lobby.can_start());

        // Both players with positions can start
        lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 2, position: PlayerPos::new(5, 3, 0) }).unwrap();
        assert!(lobby.can_start());
    }

    #[test]
    fn test_start_game() {
        let mut lobby = setup_ready_lobby();

        let result = lobby.handle_event(LobbyEvent::StartGame);
        assert!(result.is_ok());
        assert!(lobby.started);
    }

    #[test]
    fn test_start_game_not_ready() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();

        let result = lobby.handle_event(LobbyEvent::StartGame);
        assert!(matches!(result, Err(LobbyError::NotReadyToStart)));
    }

    #[test]
    fn test_to_game_conversion() {
        let mut lobby = setup_ready_lobby();
        lobby.handle_event(LobbyEvent::StartGame).unwrap();

        let game_result = lobby.to_game();
        assert!(game_result.is_ok());

        let game = game_result.unwrap();
        assert_eq!(game.players.len(), 2);
        assert_eq!(game.players[0].color, (220, 50, 47)); // Solarized red
        assert_eq!(game.players[1].color, (133, 153, 0)); // Solarized green
    }

    #[test]
    fn test_color_assignment_sequence() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

        // Test first 3 colors are assigned in order
        for i in 1..=3 {
            lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: i,
                player_name: format!("Player{}", i),
            }).unwrap();
        }

        assert_eq!(lobby.players[&1].color, (220, 50, 47));   // Red
        assert_eq!(lobby.players[&2].color, (133, 153, 0));   // Green
        assert_eq!(lobby.players[&3].color, (38, 139, 210));  // Blue
    }

    #[test]
    fn test_edge_position_validation() {
        let lobby = Lobby::new(LobbyId(1), "Test".to_string());

        // Valid edge positions
        assert!(lobby.is_valid_edge_position(&PlayerPos::new(0, 2, 4))); // Top edge
        assert!(lobby.is_valid_edge_position(&PlayerPos::new(5, 3, 0))); // Bottom edge
        assert!(lobby.is_valid_edge_position(&PlayerPos::new(2, 0, 6))); // Left edge
        assert!(lobby.is_valid_edge_position(&PlayerPos::new(3, 5, 2))); // Right edge

        // Invalid center positions
        assert!(!lobby.is_valid_edge_position(&PlayerPos::new(2, 2, 0)));
        assert!(!lobby.is_valid_edge_position(&PlayerPos::new(3, 3, 4)));

        // Invalid out-of-bounds positions
        assert!(!lobby.is_valid_edge_position(&PlayerPos::new(6, 2, 0)));
        assert!(!lobby.is_valid_edge_position(&PlayerPos::new(2, 6, 0)));
    }

    // Helper function for tests that need a ready lobby
    fn setup_ready_lobby() -> Lobby {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 1, player_name: "Alice".to_string() }).unwrap();
        lobby.handle_event(LobbyEvent::PlayerJoined { player_id: 2, player_name: "Bob".to_string() }).unwrap();
        lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 1, position: PlayerPos::new(0, 2, 4) }).unwrap();
        lobby.handle_event(LobbyEvent::PawnPlaced { player_id: 2, position: PlayerPos::new(5, 3, 0) }).unwrap();
        lobby
    }
}
```

### Integration Tests

```rust
// In tests/lobby_integration.rs
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
            player_id: (i + 1) as u32,
            position: pos,
        }).unwrap();
    }

    // Start game
    assert!(lobby.can_start());
    lobby.handle_event(LobbyEvent::StartGame).unwrap();

    // Convert to game
    let game = lobby.to_game().unwrap();
    assert_eq!(game.players.len(), 4);

    // Verify each player has correct position and solarized color
    for (i, player) in game.players.iter().enumerate() {
        assert_eq!(player.pos, positions[i]);
        // Colors should be assigned from solarized palette
        assert!(is_solarized_color(player.color));
    }
}

fn is_solarized_color(color: (u8, u8, u8)) -> bool {
    const SOLARIZED_COLORS: &[(u8, u8, u8)] = &[
        (220, 50, 47), (133, 153, 0), (38, 139, 210), (181, 137, 0),
        (211, 54, 130), (42, 161, 152), (203, 75, 22), (108, 113, 196),
    ];
    SOLARIZED_COLORS.contains(&color)
}
```

### Property-Based Tests (Optional)

For more robust testing, consider using `proptest` for property-based testing:

```rust
// Test that valid edge positions are always accepted
proptest! {
    #[test]
    fn prop_valid_edge_positions_accepted(
        row in 0u8..=5,
        col in 0u8..=5,
        endpoint in 0u8..=7
    ) {
        let lobby = Lobby::new(LobbyId(1), "Test".to_string());
        let pos = PlayerPos::new(row, col, endpoint);

        // Only test actual edge positions
        if row == 0 || row == 5 || col == 0 || col == 5 {
            assert!(lobby.is_valid_edge_position(&pos));
        }
    }
}
```

This design provides a solid foundation for multiplayer while keeping the implementation simple and maintainable.