use std::sync::mpsc;

use eframe::egui;
use egui::Context;

use tsurust_common::board::*;
use tsurust_common::game::{Game, TurnResult};
use tsurust_common::lobby::{Lobby, LobbyEvent};

use crate::screens;
use crate::ws_client::{GameClient, ServerMessage};

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),                // tile index - place at current player position
    TileRotated(usize, bool),         // tile index, clockwise
    RestartGame,                      // restart the game
    #[allow(dead_code)]
    StartLobby,                       // start a new lobby (old, for compatibility)
    StartSampleGame,                  // start sample game (current behavior)
    #[allow(dead_code)]
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
        // Poll for server messages
        if let Some(client) = &self.game_client {
            while let Some(server_msg) = client.try_recv() {
                Self::handle_server_message(
                    server_msg,
                    &mut self.current_room_id,
                    &mut self.current_player_id,
                );
            }

            // Request repaint if we're connected (to keep polling for messages)
            ctx.request_repaint();
        }

        // Process received UI messages
        let mut messages = Vec::new();
        if let Some(rx) = &self.receiver {
            while let Ok(message) = rx.try_recv() {
                messages.push(message);
            }
        }
        for message in messages {
            self.handle_ui_message(message);
        }

        // Render UI
        if let Some(tx) = &self.sender {
            Self::render_ui(ctx, &mut self.app_state, self.current_player_id, tx);
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

impl TemplateApp {
    // ========== Message Handler Methods ==========

    fn handle_ui_message(&mut self, message: Message) {
        match message {
            Message::StartLobby => self.handle_start_lobby(),
            Message::StartSampleGame => self.handle_start_sample_game(),
            Message::ShowCreateLobbyForm => self.handle_show_create_lobby_form(),
            Message::ShowJoinLobbyForm => self.handle_show_join_lobby_form(),
            Message::CreateAndJoinLobby(name, player) => self.handle_create_and_join_lobby(name, player),
            Message::JoinLobbyWithId(id, player) => self.handle_join_lobby_with_id(id, player),
            Message::BackToMainMenu => self.handle_back_to_main_menu(),
            Message::JoinLobby(player_name) => self.handle_join_lobby(player_name),
            Message::PlacePawn(position) => self.handle_place_pawn(position),
            Message::StartGameFromLobby => self.handle_start_game_from_lobby(),
            Message::DebugAddPlayer => self.handle_debug_add_player(),
            Message::DebugPlacePawn(player_id) => self.handle_debug_place_pawn(player_id),
            Message::DebugCyclePlayer(next) => self.handle_debug_cycle_player(next),
            Message::TileRotated(idx, cw) => self.handle_tile_rotated(idx, cw),
            Message::TilePlaced(idx) => self.handle_tile_placed(idx),
            Message::RestartGame => self.handle_restart_game(),
        }
    }

    // ========== Message Handler Methods ==========

    fn handle_server_message(
        server_msg: ServerMessage,
        current_room_id: &mut Option<String>,
        current_player_id: &mut PlayerID,
    ) {
        match server_msg {
            ServerMessage::RoomCreated { room_id, player_id } => {
                *current_room_id = Some(room_id);
                *current_player_id = player_id;
            }
            ServerMessage::PlayerJoined { room_id: _, player_id: _, player_name: _ } => {
                // Player joined notification received
            }
            ServerMessage::GameStateUpdate { room_id: _, state: _ } => {
                // TODO: Update local game state from server
                // This will be needed when the server broadcasts game state updates
                // to keep all clients in sync during multiplayer gameplay.
                // Implementation blocked on: converting server state format to client Game struct
            }
            ServerMessage::TurnCompleted { room_id: _, result: _ } => {
                // Turn completed notification received
            }
            ServerMessage::Error { message } => {
                eprintln!("Server error: {}", message);
            }
            ServerMessage::PlayerLeft { room_id: _, player_id: _ } => {
                // Player left notification received
            }
        }
    }

    fn handle_start_lobby(&mut self) {
        let mut lobby = Lobby::new("MAIN".to_string(), "Main Room".to_string());
        if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: self.current_player_id,
            player_name: "Player".to_string(),
        }) {
            eprintln!("Failed to auto-join lobby: {:?}", e);
        }
        self.app_state = AppState::Lobby(lobby);
    }

    fn handle_start_sample_game(&mut self) {
        let players = vec![
            Player::new(1, PlayerPos::new(0, 2, 4)),
            Player::new(2, PlayerPos::new(2, 5, 2)),
            Player::new(3, PlayerPos::new(5, 3, 0)),
            Player::new(4, PlayerPos::new(3, 0, 6)),
        ];
        self.app_state = AppState::Game(Game::new(players));
    }

    fn handle_show_create_lobby_form(&mut self) {
        self.app_state = AppState::CreateLobbyForm {
            lobby_name: "Test Lobby".to_string(),
            player_name: "Player 1".to_string(),
        };
    }

    fn handle_show_join_lobby_form(&mut self) {
        self.app_state = AppState::JoinLobbyForm {
            lobby_id: String::new(),
            player_name: "Player 1".to_string(),
        };
    }

    fn handle_create_and_join_lobby(&mut self, lobby_name: String, player_name: String) {
        match GameClient::connect("ws://127.0.0.1:8080") {
            Ok(mut client) => {
                client.create_room(lobby_name.clone(), player_name.clone());
                self.game_client = Some(client);

                let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
                self.current_player_id = player_id;
                self.app_state = AppState::Lobby(lobby);
            }
            Err(e) => {
                eprintln!("Failed to connect to server: {}", e);
                let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
                self.current_player_id = player_id;
                self.app_state = AppState::Lobby(lobby);
            }
        }
    }

    fn handle_join_lobby_with_id(&mut self, lobby_id: String, player_name: String) {
        use tsurust_common::lobby::normalize_lobby_id;
        if let Some(normalized_id) = normalize_lobby_id(&lobby_id) {
            let (lobby, player_id) = Lobby::new_with_creator(
                format!("Lobby {}", normalized_id),
                player_name
            );
            self.current_player_id = player_id;
            self.app_state = AppState::Lobby(lobby);
        } else {
            // TODO: Handle invalid lobby ID error
            // Should show error message to user and remain on join form
            // Currently silently fails - need to add error display mechanism
            eprintln!("Invalid lobby ID format: {}", lobby_id);
        }
    }

    fn handle_back_to_main_menu(&mut self) {
        match &self.app_state {
            AppState::LobbyPlacingFor(lobby, _) => {
                self.app_state = AppState::Lobby(lobby.clone());
            }
            _ => {
                self.app_state = AppState::MainMenu;
            }
        }
    }

    fn handle_join_lobby(&mut self, player_name: String) {
        if let AppState::Lobby(lobby) = &mut self.app_state {
            if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                player_id: self.current_player_id,
                player_name,
            }) {
                eprintln!("Failed to join lobby: {:?}", e);
            }
        }
    }

    fn handle_place_pawn(&mut self, position: PlayerPos) {
        match &mut self.app_state {
            AppState::Lobby(lobby) => {
                if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                    player_id: self.current_player_id,
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
                    self.app_state = AppState::Lobby(lobby.clone());
                }
            }
            _ => {}
        }
    }

    fn handle_start_game_from_lobby(&mut self) {
        if let AppState::Lobby(lobby) = &mut self.app_state {
            if let Err(e) = lobby.handle_event(LobbyEvent::StartGame) {
                eprintln!("Failed to start game: {:?}", e);
            } else {
                match lobby.to_game() {
                    Ok(game) => {
                        self.app_state = AppState::Game(game);
                    }
                    Err(e) => {
                        eprintln!("Failed to convert lobby to game: {:?}", e);
                    }
                }
            }
        }
    }

    fn handle_debug_add_player(&mut self) {
        let next_player_id = match &self.app_state {
            AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) => {
                lobby.players.keys().max().unwrap_or(&0) + 1
            }
            _ => return,
        };

        let lobby = match &mut self.app_state {
            AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) => lobby,
            _ => return,
        };

        let player_name = format!("Test Player {}", next_player_id);
        if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
            player_id: next_player_id,
            player_name,
        }) {
            eprintln!("Failed to add test player: {:?}", e);
        } else {
            self.app_state = AppState::LobbyPlacingFor(lobby.clone(), next_player_id);
        }
    }

    fn handle_debug_place_pawn(&mut self, player_id: PlayerID) {
        if let AppState::Lobby(lobby) = &self.app_state {
            self.app_state = AppState::LobbyPlacingFor(lobby.clone(), player_id);
        }
    }

    fn handle_debug_cycle_player(&mut self, next: bool) {
        if let AppState::LobbyPlacingFor(lobby, current_placing_id) = &self.app_state {
            let mut unplaced_players: Vec<PlayerID> = lobby.players.iter()
                .filter(|(_, p)| p.spawn_position.is_none())
                .map(|(id, _)| *id)
                .collect();
            unplaced_players.sort();

            if !unplaced_players.is_empty() {
                let current_idx = unplaced_players.iter()
                    .position(|id| id == current_placing_id)
                    .unwrap_or(0);

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
                self.app_state = AppState::LobbyPlacingFor(lobby.clone(), new_player_id);
            }
        }
    }

    fn handle_tile_rotated(&mut self, tile_index: usize, clockwise: bool) {
        if let AppState::Game(game) = &mut self.app_state {
            let hand = game.hands.get_mut(&game.current_player_id)
                .expect("current player should always have a hand");
            hand[tile_index] = hand[tile_index].rotated(clockwise);
        }
    }

    fn handle_tile_placed(&mut self, tile_index: usize) {
        if let AppState::Game(game) = &mut self.app_state {
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
                }
                Err(error) => println!("Failed to place tile: {}", error),
            }
        }
    }


    fn handle_restart_game(&mut self) {
            if let AppState::Game(game) = &mut self.app_state {
                let players = vec![
                    Player::new(1, PlayerPos::new(0, 2, 5)),
                    Player::new(2, PlayerPos::new(2, 5, 2)),
                    Player::new(3, PlayerPos::new(5, 3, 0)),
                    Player::new(4, PlayerPos::new(3, 0, 6)),
                ];
                *game = Game::new(players);
        }
    }
}
