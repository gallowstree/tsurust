use tsurust_common::game::TurnResult;
use std::sync::mpsc;

use eframe::egui;
use egui::Context;

use tsurust_common::board::*;
use tsurust_common::game::Game;
use tsurust_common::lobby::{Lobby, LobbyEvent};

use crate::screens;
use crate::ws_client::{GameClient, ServerMessage};

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),                // tile index - place at current player position
    TileRotated(usize, bool),         // tile index, clockwise
    RestartGame,                      // restart the game
    StartLobby,                       // start a new lobby (old, for compatibility)
    StartSampleGame,                  // start sample game (current behavior)
    JoinLobby(String),               // join lobby with player name
    PlacePawn(PlayerPos),            // place pawn at position in lobby
    StartGameFromLobby,              // start game from lobby
    ShowCreateLobbyForm,             // show create lobby form
    ShowJoinLobbyForm,               // show join lobby form
    CreateAndJoinLobby(String, String), // (lobby_name, player_name)
    JoinLobbyWithId(String, String), // (lobby_id, player_name)
    BackToMainMenu,                  // return to main menu
    DebugAddPlayer,                  // debug: simulate player joining
    DebugPlacePawn(PlayerID),        // debug: place pawn for specific player
    DebugCyclePlayer(bool),          // debug: cycle active player (true = next, false = prev)
}

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
    LobbyPlacingFor(Lobby, PlayerID),  // Placing pawn for specific player
    Game(Game),
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    #[serde(skip)]
    app_state: AppState,
    #[serde(skip)]
    sender: Option<mpsc::Sender<Message>>,
    #[serde(skip)]
    receiver: Option<mpsc::Receiver<Message>>,
    #[serde(skip)]
    current_player_id: PlayerID, // For lobby, tracks this client's player ID
    #[serde(skip)]
    game_client: Option<GameClient>, // WebSocket connection to server
    #[serde(skip)]
    current_room_id: Option<String>, // Track current room we're in
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            label: "Tsurust - Year of the Dragon of Wood".to_owned(),
            app_state: AppState::MainMenu,
            sender: Some(sender),
            receiver: Some(receiver),
            current_player_id: 1, // Default player ID
            game_client: None,
            current_room_id: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    fn render_ui(ctx: &Context, app_state: &mut AppState, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
        match app_state {
            AppState::MainMenu => screens::main_menu::render(ctx, sender),
            AppState::CreateLobbyForm { lobby_name, player_name } => {
                screens::lobby_forms::render_create_lobby_form(ctx, lobby_name, player_name, sender)
            }
            AppState::JoinLobbyForm { lobby_id, player_name } => {
                screens::lobby_forms::render_join_lobby_form(ctx, lobby_id, player_name, sender)
            }
            AppState::Lobby(lobby) => screens::lobby::render_lobby_ui(ctx, lobby, current_player_id, sender),
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                screens::lobby::render_lobby_placing_ui(ctx, lobby, *placing_for_id, sender)
            }
            AppState::Game(game) => screens::game::render_game_ui(ctx, game, sender),
        }
    }

}



impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let Self { label: _, app_state, sender, receiver, current_player_id, game_client, current_room_id } = self;

        // Poll for server messages
        if let Some(client) = game_client {
            while let Some(server_msg) = client.try_recv() {
                match server_msg {
                    ServerMessage::RoomCreated { room_id, player_id } => {
                        println!("Room created: {} with player ID: {}", room_id, player_id);
                        *current_room_id = Some(room_id);
                        *current_player_id = player_id;
                    }
                    ServerMessage::PlayerJoined { room_id, player_id: joined_player_id, player_name } => {
                        println!("Player {} (ID: {}) joined room {}", player_name, joined_player_id, room_id);
                    }
                    ServerMessage::GameStateUpdate { room_id, state } => {
                        println!("Game state update for room {}", room_id);
                        // TODO: Update local game state from server
                    }
                    ServerMessage::TurnCompleted { room_id, result } => {
                        println!("Turn completed in room {}: {:?}", room_id, result);
                    }
                    ServerMessage::Error { message } => {
                        eprintln!("Server error: {}", message);
                    }
                    ServerMessage::PlayerLeft { room_id, player_id: left_player_id } => {
                        println!("Player {} left room {}", left_player_id, room_id);
                    }
                }
            }

            // Request repaint if we're connected (to keep polling for messages)
            ctx.request_repaint();
        }

        // Process received messages
        if let Some(rx) = receiver {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::StartLobby => {
                        let mut lobby = Lobby::new("MAIN".to_string(), "Main Room".to_string());
                        // Auto-join the lobby as Player 1
                        if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                            player_id: *current_player_id,
                            player_name: "Player".to_string(),
                        }) {
                            eprintln!("Failed to auto-join lobby: {:?}", e);
                        }
                        *app_state = AppState::Lobby(lobby);
                    }
                    Message::StartSampleGame => {
                        let players = vec![
                            Player::new(1, PlayerPos::new(0, 2, 4)),
                            Player::new(2, PlayerPos::new(2, 5, 2)),
                            Player::new(3, PlayerPos::new(5, 3, 0)),
                            Player::new(4, PlayerPos::new(3, 0, 6)),
                        ];
                        let game = Game::new(players);
                        *app_state = AppState::Game(game);
                    }
                    Message::ShowCreateLobbyForm => {
                        *app_state = AppState::CreateLobbyForm {
                            lobby_name: "Test Lobby".to_string(),
                            player_name: "Player 1".to_string(),
                        };
                    }
                    Message::ShowJoinLobbyForm => {
                        *app_state = AppState::JoinLobbyForm {
                            lobby_id: String::new(),
                            player_name: "Player 1".to_string(),
                        };
                    }
                    Message::CreateAndJoinLobby(lobby_name, player_name) => {
                        // Connect to WebSocket server
                        match GameClient::connect("ws://127.0.0.1:8080") {
                            Ok(mut client) => {
                                println!("Connected to server, creating room: {}", lobby_name);

                                // Send CreateRoom message
                                client.create_room(lobby_name.clone(), player_name.clone());

                                // Store the client for future communication
                                self.game_client = Some(client);

                                // For now, still create local lobby (will be replaced by server state)
                                let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
                                *current_player_id = player_id;
                                *app_state = AppState::Lobby(lobby);
                            }
                            Err(e) => {
                                eprintln!("Failed to connect to server: {}", e);
                                // Fallback to local-only lobby
                                let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
                                *current_player_id = player_id;
                                *app_state = AppState::Lobby(lobby);
                            }
                        }
                    }
                    Message::JoinLobbyWithId(lobby_id, player_name) => {
                        use tsurust_common::lobby::normalize_lobby_id;
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
                    Message::BackToMainMenu => {
                        match app_state {
                            AppState::LobbyPlacingFor(lobby, _) => {
                                // Return to lobby instead of main menu
                                let lobby_copy = lobby.clone();
                                *app_state = AppState::Lobby(lobby_copy);
                            }
                            _ => {
                                *app_state = AppState::MainMenu;
                            }
                        }
                    }
                    Message::JoinLobby(player_name) => {
                        if let AppState::Lobby(lobby) = app_state {
                            if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                                player_id: *current_player_id,
                                player_name,
                            }) {
                                eprintln!("Failed to join lobby: {:?}", e);
                            }
                        }
                    }
                    Message::PlacePawn(position) => {
                        match app_state {
                            AppState::Lobby(lobby) => {
                                if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                                    player_id: *current_player_id,
                                    position,
                                }) {
                                    eprintln!("Failed to place pawn: {:?}", e);
                                }
                            }
                            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                                if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                                    player_id: *placing_for_id,
                                    position,
                                }) {
                                    eprintln!("Failed to place pawn: {:?}", e);
                                } else {
                                    // Return to normal lobby view
                                    let lobby_copy = lobby.clone();
                                    *app_state = AppState::Lobby(lobby_copy);
                                }
                            }
                            _ => {}
                        }
                    }
                    Message::StartGameFromLobby => {
                        if let AppState::Lobby(lobby) = app_state {
                            if let Err(e) = lobby.handle_event(LobbyEvent::StartGame) {
                                eprintln!("Failed to start game: {:?}", e);
                            } else {
                                match lobby.to_game() {
                                    Ok(game) => {
                                        *app_state = AppState::Game(game);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to convert lobby to game: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                    Message::DebugAddPlayer => {
                        match app_state {
                            AppState::Lobby(lobby) => {
                                let next_player_id = lobby.players.keys().max().unwrap_or(&0) + 1;
                                let player_name = format!("Test Player {}", next_player_id);
                                if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                                    player_id: next_player_id,
                                    player_name,
                                }) {
                                    eprintln!("Failed to add test player: {:?}", e);
                                } else {
                                    // Switch to placing mode for the new player
                                    let lobby_copy = lobby.clone();
                                    *app_state = AppState::LobbyPlacingFor(lobby_copy, next_player_id);
                                }
                            }
                            AppState::LobbyPlacingFor(lobby, _) => {
                                let next_player_id = lobby.players.keys().max().unwrap_or(&0) + 1;
                                let player_name = format!("Test Player {}", next_player_id);
                                if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                                    player_id: next_player_id,
                                    player_name,
                                }) {
                                    eprintln!("Failed to add test player: {:?}", e);
                                } else {
                                    // Switch to placing mode for the new player
                                    let lobby_copy = lobby.clone();
                                    *app_state = AppState::LobbyPlacingFor(lobby_copy, next_player_id);
                                }
                            }
                            _ => {}
                        }
                    }
                    Message::DebugPlacePawn(player_id) => {
                        if let AppState::Lobby(lobby) = app_state {
                            // Switch to placing mode for this player
                            let lobby_copy = lobby.clone();
                            *app_state = AppState::LobbyPlacingFor(lobby_copy, player_id);
                        }
                    }
                    Message::DebugCyclePlayer(next) => {
                        if let AppState::LobbyPlacingFor(lobby, current_placing_id) = app_state {
                            // Get all player IDs without spawn positions, sorted
                            let mut unplaced_players: Vec<PlayerID> = lobby.players.iter()
                                .filter(|(_, p)| p.spawn_position.is_none())
                                .map(|(id, _)| *id)
                                .collect();
                            unplaced_players.sort();

                            if !unplaced_players.is_empty() {
                                // Find current index
                                let current_idx = unplaced_players.iter()
                                    .position(|id| id == current_placing_id)
                                    .unwrap_or(0);

                                // Calculate new index
                                let new_idx = if next {
                                    (current_idx + 1) % unplaced_players.len()
                                } else {
                                    if current_idx == 0 {
                                        unplaced_players.len() - 1
                                    } else {
                                        current_idx - 1
                                    }
                                };

                                let new_player_id = unplaced_players[new_idx];
                                let lobby_copy = lobby.clone();
                                *app_state = AppState::LobbyPlacingFor(lobby_copy, new_player_id);
                            }
                        }
                    }
                    Message::TileRotated(tile_index, clockwise) => {
                        if let AppState::Game(game) = app_state {
                            let hand = game.hands.get_mut(&game.current_player_id).expect("current player should always have a hand");
                            hand[tile_index] = hand[tile_index].rotated(clockwise);
                        }
                    }
                    Message::TilePlaced(tile_index) => {
                        if let AppState::Game(game) = app_state {
                            let player_cell = game.players.iter()
                                .find(|p| p.id == game.current_player_id && p.alive)
                                .expect("current player should exist and be alive")
                                .pos.cell;

                            let hand = game.hands.get(&game.current_player_id)
                                .expect("current player should always have a hand");

                            let tile = hand[tile_index];

                            let mov = Move {
                                tile,
                                cell: player_cell,
                                player_id: game.current_player_id,
                            };

                            match game.perform_move(mov) {
                                Ok(turn_result) => {
                                    println!("Tile placed successfully at {:?}!", player_cell);
                                    println!("  Tile: {:?}", tile);

                                    match &turn_result {
                                        TurnResult::TurnAdvanced { turn_number, next_player, eliminated } => {
                                            println!("Turn {} completed. Next player: {}", turn_number, next_player);
                                            if !eliminated.is_empty() {
                                                println!("  Players eliminated: {:?}", eliminated);
                                            }
                                        }
                                        TurnResult::PlayerWins { turn_number, winner, eliminated } => {
                                            println!("GAME OVER! Player {} wins on turn {}!", winner, turn_number);
                                            if !eliminated.is_empty() {
                                                println!("  Final eliminations: {:?}", eliminated);
                                            }
                                        }
                                        TurnResult::Extinction { turn_number, eliminated } => {
                                            println!("EXTINCTION! All players eliminated on turn {}!", turn_number);
                                            println!("  Final eliminations: {:?}", eliminated);
                                        }
                                    }

                                    println!("  All player positions after move:");
                                    for player in &game.players {
                                        println!("    Player {} ({}): {:?}",
                                            player.id,
                                            if player.alive { "alive" } else { "eliminated" },
                                            player.pos);
                                    }
                                }
                                Err(error) => println!("Failed to place tile: {}", error),
                            }
                        }
                    }
                    Message::RestartGame => {
                        if let AppState::Game(game) = app_state {
                            // Create a new game with fresh players
                            let players = vec![
                                Player::new(1, PlayerPos::new(0, 2, 1)),
                                Player::new(2, PlayerPos::new(2, 5, 2)),
                                Player::new(3, PlayerPos::new(5, 3, 0)),
                                Player::new(4, PlayerPos::new(3, 0, 6)),
                            ];
                            *game = Game::new(players);
                            println!("Game restarted!");
                        }
                    }
                }
            }
        }

        // Render UI with sender
        if let Some(tx) = sender {
            Self::render_ui(ctx, app_state, *current_player_id, tx);
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
