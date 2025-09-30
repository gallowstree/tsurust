use std::collections::{HashMap, HashSet};
use crate::board::{Player, PlayerPos, PlayerID, CellCoord};
use crate::game::Game;

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
                    Player::new(lobby_player.id, pos)
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
        // Basic color assignment check - just verify a color was assigned
        assert_ne!(lobby.players[&1].color, (0, 0, 0));
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

        // Check that both expected players are present (order doesn't matter)
        let player_ids: std::collections::HashSet<_> = game.players.iter().map(|p| p.id).collect();
        assert!(player_ids.contains(&1));
        assert!(player_ids.contains(&2));
    }

    #[test]
    fn test_color_assignment_sequence() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

        // Test first 3 colors are assigned and different
        for i in 1..=3 {
            lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: i,
                player_name: format!("Player{}", i),
            }).unwrap();
        }

        // Verify all players have different colors
        let color1 = lobby.players[&1].color;
        let color2 = lobby.players[&2].color;
        let color3 = lobby.players[&3].color;

        assert_ne!(color1, color2);
        assert_ne!(color1, color3);
        assert_ne!(color2, color3);
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

    #[test]
    fn test_player_not_found_error() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

        let result = lobby.handle_event(LobbyEvent::PawnPlaced {
            player_id: 999, // Non-existent player
            position: PlayerPos::new(0, 2, 4),
        });

        assert!(matches!(result, Err(LobbyError::PlayerNotFound)));
    }

    #[test]
    fn test_game_not_started_error() {
        let lobby = setup_ready_lobby();
        // Don't start the game

        let result = lobby.to_game();
        assert!(matches!(result, Err(LobbyError::GameNotStarted)));
    }

    #[test]
    fn test_no_available_colors() {
        let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
        lobby.max_players = 10; // More than available colors

        // Add 8 players (all available colors)
        for i in 1..=8 {
            lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: i,
                player_name: format!("Player{}", i),
            }).unwrap();
        }

        // 9th player should fail due to no available colors
        let result = lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: 9,
            player_name: "Player9".to_string(),
        });

        assert!(matches!(result, Err(LobbyError::NoAvailableColors)));
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

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_edge_position_validation(
            row in 0usize..=7,
            col in 0usize..=7,
            endpoint in 0usize..=7
        ) {
            let lobby = Lobby::new(LobbyId(1), "Test".to_string());
            let pos = PlayerPos::new(row, col, endpoint);

            let is_valid_edge = (row == 0 || row == 5 || col == 0 || col == 5) && row <= 5 && col <= 5;
            let validation_result = lobby.is_valid_edge_position(&pos);

            if is_valid_edge {
                // Valid edge positions should be accepted
                prop_assert!(validation_result);
            } else {
                // Invalid positions (center or out-of-bounds) should be rejected
                prop_assert!(!validation_result);
            }
        }

        #[test]
        fn prop_lobby_id_consistency(id in 0u64..1000000u64) {
            let lobby_id = LobbyId(id);
            let lobby = Lobby::new(lobby_id.clone(), "Test".to_string());
            prop_assert_eq!(lobby.id, lobby_id);
        }

        #[test]
        fn prop_player_name_handling(name in "\\PC{1,50}") {
            let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());

            let result = lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: 1,
                player_name: name.clone(),
            });

            prop_assert!(result.is_ok());
            prop_assert_eq!(&lobby.players[&1].name, &name);
        }

        #[test]
        fn prop_max_players_limit(max_players in 2usize..8) { // Limit to 8 due to color constraint
            let mut lobby = Lobby::new(LobbyId(1), "Test".to_string());
            lobby.max_players = max_players;

            // Add players up to the limit
            for i in 1..=max_players {
                let result = lobby.handle_event(LobbyEvent::PlayerJoined {
                    player_id: i,
                    player_name: format!("Player{}", i),
                });
                prop_assert!(result.is_ok());
            }

            // One more player should fail
            let result = lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: max_players + 1,
                player_name: "ExtraPlayer".to_string(),
            });
            prop_assert!(matches!(result, Err(LobbyError::LobbyFull)));
        }
    }
}