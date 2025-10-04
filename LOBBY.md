# Lobby System Design

## Overview

A simple pre-game lobby system that allows players to join, select spawn positions, and start the game. Core game logic remains separate from rendering.

**Implementation Status**: Basic lobby system implemented in `common/src/lobby.rs` with comprehensive tests. UI integration completed in `client-egui/src/app.rs` with MainMenu, Lobby, and Game states.

## Server Logic (Future)
- A room: allows players to keep playing among themselves in between games. It manages Lobby -> Game -> Result -> Lobby.

## Client-Server comms (Future)
Might need to introduce ws even if we are using tarpc. (client can't listen freely for server messages).
Or maybe just a tcp socket, maybe it can be wrapped in a tarpc transport or sum.

## On App Launched

When the app launches, present users with three options: **Create Lobby**, **Join Lobby**, or **Sample Game**. This requires extending the current MainMenu state and adding lobby ID generation and validation.

### Core Components

#### 1. Lobby ID System
```rust
// In common/src/lobby.rs
pub type LobbyId = String;  // 4 case-insensitive alphanumeric characters

pub fn next_lobby_id() -> LobbyId {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Excludes I, O, 0, 1 for clarity
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect::<String>()
}

pub fn normalize_lobby_id(id: &str) -> Option<LobbyId> {
    let normalized = id.trim().to_uppercase();
    if normalized.len() == 4 && normalized.chars().all(|c| c.is_alphanumeric()) {
        Some(normalized)
    } else {
        None
    }
}
```

#### 2. Extended Lobby Structure
```rust
// Add to existing Lobby struct in common/src/lobby.rs
impl Lobby {
    pub fn new_with_creator(name: String, creator_name: String) -> (Self, PlayerID) {
        let lobby_id = next_lobby_id();
        let mut lobby = Self {
            id: lobby_id,
            name,
            players: HashMap::new(),
            started: false,
            max_players: 8,
        };

        // Auto-join creator as first player
        let creator_id = 1;
        lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: creator_id,
            player_name: creator_name,
        }).expect("Creator should always be able to join empty lobby");

        (lobby, creator_id)
    }
}
```

#### 3. UI State Machine
```rust
// In client-egui/src/app.rs
#[derive(Debug)]
pub enum AppState {
    MainMenu,
    CreateLobbyForm {
        lobby_name: String,
        player_name: String,
    },
    JoinLobbyForm {
        lobby_id: String,
        player_name: String,
    },
    Lobby(Lobby),
    Game(Game),
}

pub enum Message {
    // ... existing messages ...

    // Main menu actions
    ShowCreateLobbyForm,
    ShowJoinLobbyForm,
    ShowSampleGame,

    // Create lobby flow
    CreateAndJoinLobby(String, String), // (lobby_name, player_name)

    // Join lobby flow
    JoinLobby(String, String), // (lobby_id, player_name)

    // ... existing messages ...
}
```

#### 4. UI Implementation
```rust
// In client-egui/src/app.rs
impl TemplateApp {
    fn render_main_menu(ui: &mut egui::Ui, sender: &mpsc::Sender<Message>) {
        ui.vertical_centered(|ui| {
            ui.heading("Tsuro");
            ui.add_space(40.0);

            if ui.button("Create Lobby").clicked() {
                sender.send(Message::ShowCreateLobbyForm).expect("Send failed");
            }
            ui.add_space(10.0);

            if ui.button("Join Lobby").clicked() {
                sender.send(Message::ShowJoinLobbyForm).expect("Send failed");
            }
            ui.add_space(10.0);

            if ui.button("Sample Game").clicked() {
                sender.send(Message::ShowSampleGame).expect("Send failed");
            }
        });
    }

    fn render_create_lobby_form(
        ui: &mut egui::Ui,
        lobby_name: &mut String,
        player_name: &mut String,
        sender: &mpsc::Sender<Message>
    ) {
        ui.vertical_centered(|ui| {
            ui.heading("Create Lobby");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Lobby Name:");
                ui.text_edit_singleline(lobby_name);
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Your Name:");
                ui.text_edit_singleline(player_name);
            });
            ui.add_space(20.0);

            let can_create = !lobby_name.trim().is_empty() && !player_name.trim().is_empty();

            ui.horizontal(|ui| {
                if ui.add_enabled(can_create, egui::Button::new("Create & Join")).clicked() {
                    sender.send(Message::CreateAndJoinLobby(
                        lobby_name.clone(),
                        player_name.clone()
                    )).expect("Send failed");
                }

                if ui.button("Back").clicked() {
                    sender.send(Message::BackToMainMenu).expect("Send failed");
                }
            });
        });
    }

    fn render_join_lobby_form(
        ui: &mut egui::Ui,
        lobby_id: &mut String,
        player_name: &mut String,
        sender: &mpsc::Sender<Message>
    ) {
        ui.vertical_centered(|ui| {
            ui.heading("Join Lobby");
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.label("Lobby ID:");
                ui.text_edit_singleline(lobby_id);
            });
            ui.label("(4-character code)");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Your Name:");
                ui.text_edit_singleline(player_name);
            });
            ui.add_space(10.0);

            let can_join = lobby_id.trim().len() == 4 && !player_name.trim().is_empty();

            ui.horizontal(|ui| {
                if ui.add_enabled(can_join, egui::Button::new("Join")).clicked() {
                    sender.send(Message::JoinLobby(
                        lobby_id.clone(),
                        player_name.clone()
                    )).expect("Send failed");
                }

                if ui.button("Back").clicked() {
                    sender.send(Message::BackToMainMenu).expect("Send failed");
                }
            });
        });
    }
}
```

#### 5. Message Handling
```rust
// In client-egui/src/app.rs update() method
while let Ok(msg) = receiver.try_recv() {
    match msg {
        Message::ShowCreateLobbyForm => {
            *app_state = AppState::CreateLobbyForm {
                lobby_name: String::new(),
                player_name: String::new(),
            };
        }

        Message::ShowJoinLobbyForm => {
            *app_state = AppState::JoinLobbyForm {
                lobby_id: String::new(),
                player_name: String::new(),
            };
        }

        Message::CreateAndJoinLobby(lobby_name, player_name) => {
            let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
            *current_player_id = player_id;
            *app_state = AppState::Lobby(lobby);
        }

        Message::JoinLobby(lobby_id, player_name) => {
            // Normalize and validate lobby ID
            if let Some(normalized_id) = normalize_lobby_id(&lobby_id) {
                // In real implementation, would query server and join existing lobby
                // For now, create a new lobby as placeholder
                let (lobby, player_id) = Lobby::new_with_creator(
                    format!("Lobby {}", normalized_id),
                    player_name
                );
                *current_player_id = player_id;
                *app_state = AppState::Lobby(lobby);
            }
            // TODO: Handle invalid lobby ID error
        }

        // ... existing message handlers ...
    }
}
```

### Implementation Steps

1. **Add dependency**: Add `rand` crate to `common/Cargo.toml` for lobby ID generation
2. **Extend lobby module**: Add `next_lobby_id()`, `normalize_lobby_id()`, and `new_with_creator()`
3. **Update AppState**: Add `CreateLobbyForm` and `JoinLobbyForm` states
4. **Add UI render functions**: Implement `render_create_lobby_form()` and `render_join_lobby_form()`
5. **Update message handlers**: Handle `CreateAndJoinLobby` and `JoinLobby` messages
6. **Display lobby info**: Show lobby ID and name in the existing lobby UI

### Key Benefits

- **Clear user flow**: Three distinct paths from launch (Create/Join/Sample)
- **Auto-join creator**: Simplifies UX by auto-joining after creation
- **Simple validation**: Lobby ID normalized and validated inline
- **Extensible**: Easy to add server lookup for join flow later

### Notes for Future Server Integration

When adding client-server communication:
- `CreateAndJoinLobby` would call server to create lobby and return real ID
- `JoinLobby` would query server for lobby details and join existing lobby
- Add error handling for network failures and lobby-not-found cases

## Key Implementation Notes

- **Separation of Concerns**: Lobby logic completely separate from game logic and rendering
- **Simple State Management**: Clear state transitions and validation
- **Extensible**: Easy to add features like player names, custom colors, room browser
- **Board Edge Enforcement**: Automatic validation ensures only valid spawn positions

## Test Plan for "On App Launched" Implementation

### Unit Tests (common/src/lobby.rs)

#### 1. Lobby ID Generation and Validation
```rust
#[cfg(test)]
mod lobby_id_tests {
    use super::*;

    #[test]
    fn test_next_lobby_id_format() {
        let id = next_lobby_id();
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(id.chars().all(|c| c.is_uppercase()));
    }

    #[test]
    fn test_normalize_lobby_id_valid() {
        assert_eq!(normalize_lobby_id("abcd"), Some("ABCD".to_string()));
        assert_eq!(normalize_lobby_id("  AB12  "), Some("AB12".to_string()));
        assert_eq!(normalize_lobby_id("xyz9"), Some("XYZ9".to_string()));
    }

    #[test]
    fn test_normalize_lobby_id_invalid() {
        assert_eq!(normalize_lobby_id("abc"), None);  // Too short
        assert_eq!(normalize_lobby_id("abcde"), None);  // Too long
        assert_eq!(normalize_lobby_id("ab-d"), None);  // Invalid character
        assert_eq!(normalize_lobby_id(""), None);  // Empty
        assert_eq!(normalize_lobby_id("   "), None);  // Whitespace only
    }
}
```

#### 2. Lobby Creation with Auto-Join
```rust
#[test]
fn test_new_with_creator() {
    let (lobby, creator_id) = Lobby::new_with_creator(
        "Test Room".to_string(),
        "Alice".to_string()
    );

    // Verify lobby properties
    assert_eq!(lobby.name, "Test Room");
    assert_eq!(lobby.id.len(), 4);
    assert_eq!(lobby.players.len(), 1);
    assert!(!lobby.started);

    // Verify creator is auto-joined
    assert_eq!(creator_id, 1);
    let creator = &lobby.players[&creator_id];
    assert_eq!(creator.name, "Alice");
    assert_eq!(creator.id, creator_id);
    assert!(creator.spawn_position.is_none());
}

#[test]
fn test_new_with_creator_assigns_first_color() {
    let (lobby, creator_id) = Lobby::new_with_creator(
        "Test".to_string(),
        "Bob".to_string()
    );

    let creator = &lobby.players[&creator_id];
    assert_eq!(creator.color, (220, 50, 47)); // First Solarized color (red)
}
```

### Integration Test (common/tests/lobby_launch_flow.rs)

This single integration test covers the entire launch flow, eliminating the need for many unit tests:

```rust
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

#[test]
fn test_lobby_id_normalization_edge_cases() {
    // Test various input formats
    assert_eq!(normalize_lobby_id("abcd"), Some("ABCD".to_string()));
    assert_eq!(normalize_lobby_id("ABCD"), Some("ABCD".to_string()));
    assert_eq!(normalize_lobby_id("  abcd  "), Some("ABCD".to_string()));
    assert_eq!(normalize_lobby_id("1234"), Some("1234".to_string()));
    assert_eq!(normalize_lobby_id("a1b2"), Some("A1B2".to_string()));

    // Invalid cases
    assert_eq!(normalize_lobby_id("abc"), None);
    assert_eq!(normalize_lobby_id("abcde"), None);
    assert_eq!(normalize_lobby_id("ab cd"), None);
    assert_eq!(normalize_lobby_id(""), None);
}
```

### Test Coverage Analysis

**What the integration test covers (eliminating need for unit tests):**

1. ✅ `new_with_creator()` functionality
2. ✅ Auto-join creator with correct ID
3. ✅ Creator gets first color assigned
4. ✅ Lobby ID generation (4 characters)
5. ✅ Lobby ID normalization (case-insensitive)
6. ✅ Second player joining
7. ✅ Color assignment sequence
8. ✅ Pawn placement for multiple players
9. ✅ `can_start()` validation
10. ✅ Game start flow
11. ✅ `to_game()` conversion with player data

**Unit tests still needed (not covered by integration test):**

1. ✅ `next_lobby_id()` format validation
2. ✅ `normalize_lobby_id()` edge cases (various invalid inputs)

### Implementation Plan

1. **Add `rand` dependency** to `common/Cargo.toml`
2. **Implement lobby ID functions** in `common/src/lobby.rs`:
   - `next_lobby_id()`
   - `normalize_lobby_id()`
3. **Implement `new_with_creator()`** method
4. **Add unit tests** for lobby ID functions (2 tests)
5. **Add integration test** for complete flow (1 comprehensive test)
6. **Verify all tests pass** before moving to UI implementation

This approach gives comprehensive coverage with only 3-4 test functions instead of 10+ separate unit tests, while maintaining confidence in correctness.