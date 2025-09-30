use eframe::egui;
use egui::Context;
use std::sync::mpsc;

use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use crate::player_card::PlayerCard;
use crate::components::LobbyBoard;
use tsurust_common::board::*;
use tsurust_common::game::{Game, TurnResult};
use tsurust_common::lobby::{Lobby, LobbyEvent};

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
            AppState::MainMenu => Self::render_main_menu(ctx, sender),
            AppState::CreateLobbyForm { lobby_name, player_name } => {
                Self::render_create_lobby_form(ctx, lobby_name, player_name, sender)
            }
            AppState::JoinLobbyForm { lobby_id, player_name } => {
                Self::render_join_lobby_form(ctx, lobby_id, player_name, sender)
            }
            AppState::Lobby(lobby) => Self::render_lobby_ui(ctx, lobby, current_player_id, sender),
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                Self::render_lobby_placing_ui(ctx, lobby, *placing_for_id, sender)
            }
            AppState::Game(game) => Self::render_game_ui(ctx, game, sender),
        }
    }

    fn render_main_menu(ctx: &Context, sender: &mpsc::Sender<Message>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("🐉 Tsurust");
                ui.add_space(20.0);
                ui.label("Year of the Dragon of Wood");
                ui.add_space(50.0);

                if ui.button("➕ Create Lobby").clicked() {
                    sender.send(Message::ShowCreateLobbyForm).expect("Failed to send message");
                }

                ui.add_space(10.0);

                if ui.button("🔗 Join Lobby").clicked() {
                    sender.send(Message::ShowJoinLobbyForm).expect("Failed to send message");
                }

                ui.add_space(10.0);

                if ui.button("🎮 Sample Game").clicked() {
                    sender.send(Message::StartSampleGame).expect("Failed to send message");
                }
            });
        });
    }

    fn render_create_lobby_form(ctx: &Context, lobby_name: &mut String, player_name: &mut String, sender: &mpsc::Sender<Message>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("Create Lobby");
                ui.add_space(40.0);

                ui.horizontal(|ui| {
                    ui.label("Lobby Name:");
                    let lobby_name_response = ui.text_edit_singleline(lobby_name);
                    // Auto-focus on first render
                    if lobby_name.is_empty() && player_name.is_empty() {
                        lobby_name_response.request_focus();
                    }
                });
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Your Name:");
                    ui.text_edit_singleline(player_name);
                });
                ui.add_space(30.0);

                let can_create = !lobby_name.trim().is_empty() && !player_name.trim().is_empty();

                // Submit on Enter key
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_create {
                    sender.send(Message::CreateAndJoinLobby(
                        lobby_name.clone(),
                        player_name.clone()
                    )).expect("Failed to send message");
                }

                ui.horizontal(|ui| {
                    if ui.add_enabled(can_create, egui::Button::new("Create & Join")).clicked() {
                        sender.send(Message::CreateAndJoinLobby(
                            lobby_name.clone(),
                            player_name.clone()
                        )).expect("Failed to send message");
                    }

                    if ui.button("Back").clicked() {
                        sender.send(Message::BackToMainMenu).expect("Failed to send message");
                    }
                });
            });
        });
    }

    fn render_join_lobby_form(ctx: &Context, lobby_id: &mut String, player_name: &mut String, sender: &mpsc::Sender<Message>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("Join Lobby");
                ui.add_space(40.0);

                ui.horizontal(|ui| {
                    ui.label("Lobby ID:");
                    let lobby_id_response = ui.text_edit_singleline(lobby_id);
                    // Auto-focus on first render
                    if lobby_id.is_empty() && player_name.is_empty() {
                        lobby_id_response.request_focus();
                    }
                });
                ui.label("(4-character code)");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Your Name:");
                    ui.text_edit_singleline(player_name);
                });
                ui.add_space(30.0);

                let can_join = lobby_id.trim().len() == 4 && !player_name.trim().is_empty();

                // Submit on Enter key
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_join {
                    sender.send(Message::JoinLobbyWithId(
                        lobby_id.clone(),
                        player_name.clone()
                    )).expect("Failed to send message");
                }

                ui.horizontal(|ui| {
                    if ui.add_enabled(can_join, egui::Button::new("Join")).clicked() {
                        sender.send(Message::JoinLobbyWithId(
                            lobby_id.clone(),
                            player_name.clone()
                        )).expect("Failed to send message");
                    }

                    if ui.button("Back").clicked() {
                        sender.send(Message::BackToMainMenu).expect("Failed to send message");
                    }
                });
            });
        });
    }

    fn render_lobby_ui(ctx: &Context, lobby: &mut Lobby, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.heading(format!("Lobby: {}", lobby.name));
                    ui.separator();
                    ui.label(format!("Room ID: {}", lobby.id));
                    ui.separator();
                    ui.label(format!("Players: {}/{}", lobby.players.len(), lobby.max_players));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if lobby.can_start() {
                            if ui.button("🚀 Start Game").clicked() {
                                if let Err(e) = sender.send(Message::StartGameFromLobby) {
                                    eprintln!("Failed to send StartGameFromLobby message: {}", e);
                                }
                            }
                        } else {
                            ui.add_enabled(false, egui::Button::new("⏳ Waiting for players..."));
                        }
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("Select your starting position");
                ui.label("Click on any board edge to place your pawn");
                ui.add_space(20.0);

                // Render a simplified board for pawn placement
                Self::render_lobby_board(ui, lobby, current_player_id, sender);
            });
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Players");
                ui.separator();

                for (player_id, lobby_player) in &lobby.players {
                    ui.horizontal(|ui| {
                        // Color circle
                        let player_color = egui::Color32::from_rgb(
                            lobby_player.color.0,
                            lobby_player.color.1,
                            lobby_player.color.2
                        );

                        let circle_center = ui.cursor().min + egui::Vec2::new(12.0, 12.0);
                        ui.painter().circle_filled(circle_center, 8.0, player_color);
                        ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));

                        ui.add_space(20.0);
                        ui.label(&lobby_player.name);

                        if lobby_player.spawn_position.is_some() {
                            ui.label("✔ Ready");
                        } else {
                            ui.label("⏳ Placing pawn...");
                        }

                        if *player_id == current_player_id {
                            ui.label("(You)");
                        }
                    });
                    ui.add_space(5.0);
                }

                ui.add_space(20.0);
                ui.separator();

                if lobby.players.len() < lobby.max_players {
                    ui.label("Waiting for more players to join...");
                }

                ui.add_space(20.0);
                ui.separator();
                ui.heading("Debug Tools");
                ui.add_space(10.0);

                if ui.button("➕ Add Test Player").clicked() {
                    sender.send(Message::DebugAddPlayer).expect("Failed to send message");
                }

                ui.add_space(10.0);
                ui.label("Place pawn for:");
                for (player_id, lobby_player) in &lobby.players {
                    if lobby_player.spawn_position.is_none() {
                        let player_color = egui::Color32::from_rgb(
                            lobby_player.color.0,
                            lobby_player.color.1,
                            lobby_player.color.2
                        );

                        ui.horizontal(|ui| {
                            let circle_center = ui.cursor().min + egui::Vec2::new(8.0, 8.0);
                            ui.painter().circle_filled(circle_center, 6.0, player_color);
                            ui.add_space(16.0);

                            if ui.button(&lobby_player.name).clicked() {
                                sender.send(Message::DebugPlacePawn(*player_id)).expect("Failed to send message");
                            }
                        });
                    }
                }
            });
        });
    }

    fn render_lobby_board(ui: &mut egui::Ui, lobby: &Lobby, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
        let board = LobbyBoard::new(lobby, current_player_id);
        board.render(ui, 300.0, sender);
    }

    fn render_lobby_placing_ui(ctx: &Context, lobby: &mut Lobby, placing_for_id: PlayerID, sender: &mpsc::Sender<Message>) {
        let placing_player = lobby.players.get(&placing_for_id);
        let player_name = placing_player.map(|p| p.name.as_str()).unwrap_or("Unknown");
        let player_color = placing_player.map(|p| p.color).unwrap_or((128, 128, 128));

        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.heading(format!("Lobby: {}", lobby.name));
                    ui.separator();
                    ui.label(format!("Room ID: {}", lobby.id));
                    ui.separator();
                    ui.label(format!("Players: {}/{}", lobby.players.len(), lobby.max_players));
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                let player_color_ui = egui::Color32::from_rgb(player_color.0, player_color.1, player_color.2);

                ui.horizontal(|ui| {
                    ui.heading("Placing pawn for:");
                    let circle_center = ui.cursor().min + egui::Vec2::new(8.0, 12.0);
                    ui.painter().circle_filled(circle_center, 8.0, player_color_ui);
                    ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));
                    ui.add_space(20.0);
                    ui.heading(player_name);
                });

                ui.label("Click on any board edge to place their pawn");
                ui.add_space(20.0);

                Self::render_lobby_board(ui, lobby, placing_for_id, sender);
            });
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Players");
                ui.separator();

                for (player_id, lobby_player) in &lobby.players {
                    ui.horizontal(|ui| {
                        let player_color = egui::Color32::from_rgb(
                            lobby_player.color.0,
                            lobby_player.color.1,
                            lobby_player.color.2
                        );

                        let circle_center = ui.cursor().min + egui::Vec2::new(12.0, 12.0);
                        ui.painter().circle_filled(circle_center, 8.0, player_color);
                        ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));

                        ui.add_space(20.0);
                        ui.label(&lobby_player.name);

                        if lobby_player.spawn_position.is_some() {
                            ui.label("✔ Ready");
                        } else if *player_id == placing_for_id {
                            ui.label("👈 Placing now...");
                        } else {
                            ui.label("⏳ Waiting...");
                        }
                    });
                    ui.add_space(5.0);
                }

                ui.add_space(20.0);
                ui.separator();
                ui.heading("Switch Player");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("⬅ Previous").clicked() {
                        sender.send(Message::DebugCyclePlayer(false)).expect("Failed to send message");
                    }
                    if ui.button("Next ➡").clicked() {
                        sender.send(Message::DebugCyclePlayer(true)).expect("Failed to send message");
                    }
                });

                ui.add_space(20.0);
                ui.separator();

                if ui.button("⬅ Back to Lobby").clicked() {
                    sender.send(Message::BackToMainMenu).expect("Failed to send message");
                }
            });
        });
    }

    fn render_game_ui(ctx: &Context, game: &mut Game, sender: &mpsc::Sender<Message>) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("🔄 Restart Game").clicked() {
                        if let Err(e) = sender.send(Message::RestartGame) {
                            eprintln!("Failed to send RestartGame message: {}", e);
                        }
                    }
                    if ui.button("⬅ Back to Menu").clicked() {
                        if let Err(e) = sender.send(Message::BackToMainMenu) {
                            eprintln!("Failed to send BackToMainMenu message: {}", e);
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(20.);
                ui.add(BoardRenderer::new(&game.board.history, &game.players, &game.tile_trails));
            });
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            // Player cards section
            ui.vertical(|ui| {
                ui.heading("Players");
                ui.separator();

                // Sort players: alive first, then dead
                let mut sorted_players: Vec<&Player> = game.players.iter().collect();
                sorted_players.sort_by_key(|p| !p.alive); // false (alive) comes before true (dead)

                // Determine winner: if only one player is alive, they're the winner
                let alive_count = game.players.iter().filter(|p| p.alive).count();
                let winner_id = if alive_count == 1 {
                    game.players.iter().find(|p| p.alive).map(|p| p.id)
                } else {
                    None
                };

                for player in sorted_players {
                    let hand_size = game.hands.get(&player.id).map(|h| h.len()).unwrap_or(0);
                    let has_dragon = game.dragon == Some(player.id);
                    let is_current = player.id == game.current_player_id;
                    let is_winner = winner_id == Some(player.id);

                    ui.horizontal(|ui| {
                        // Arrow indicator for current player (drawn triangle)
                        let (arrow_rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 60.0), egui::Sense::hover());

                        if is_current {
                            let triangle_center = arrow_rect.center();
                            let triangle_size = 12.0;

                            // Draw triangle pointing right
                            let points = [
                                triangle_center + egui::Vec2::new(-triangle_size/2.0, -triangle_size/2.0),
                                triangle_center + egui::Vec2::new(-triangle_size/2.0, triangle_size/2.0),
                                triangle_center + egui::Vec2::new(triangle_size/2.0, 0.0),
                            ];

                            ui.painter().add(egui::Shape::convex_polygon(
                                points.to_vec(),
                                egui::Color32::from_rgb(100, 150, 255),
                                egui::Stroke::NONE
                            ));
                        }

                        let mut card = PlayerCard::new(player, hand_size, has_dragon);
                        if is_current {
                            card = card.current_player();
                        }
                        if is_winner {
                            card = card.winner();
                        }
                        ui.add(card);
                    });
                }

                ui.add_space(20.0);
                ui.separator();
                ui.heading("Your Hand");
            });

            // Hand section
            let hand = game.curr_player_hand().clone();
            ui.add(HandRenderer::new(hand, sender.clone()));
        });
    }
}



impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let Self { label: _, app_state, sender, receiver, current_player_id } = self;

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
