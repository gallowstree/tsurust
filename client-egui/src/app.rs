use std::sync::mpsc;

use eframe::egui;
use egui::Context;

use tsurust_common::board::*;
use tsurust_common::game::{Game, TurnResult};
use tsurust_common::lobby::{Lobby, LobbyEvent};

use tsurust_common::protocol::{ClientMessage, ServerMessage};

use crate::screens;
use crate::ws_client::GameClient;

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),                // tile index - place at current player position
    TileRotated(usize, bool),         // tile index, clockwise
    RestartGame,                      // restart the game
    StartLobby,                       // start a local lobby (offline multiplayer)
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
    StartLocalServer,                // start a local server process
    SendToServer(ClientMessage),     // send a message to the server via WebSocket
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
    Lobby(Lobby),                       // Normal lobby view (place own pawn)
    LobbyPlacingFor(Lobby, PlayerID),  // Debug mode: placing pawn for specific player
    Game(Game),                         // Local game - client authoritative
    OnlineGame {
        game: Game,                     // Server's authoritative state
        room_id: String,
        lobby_name: String,             // Display name of the lobby/game
        waiting_for_server: bool,       // Show loading state during server round-trip
    },
    GameOver(Game),                     // Game ended - show statistics
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
/// Most fields are skipped because they contain runtime state (channels, WebSocket connections,
/// active games) that can't/shouldn't be serialized. Only UI preferences like label are persisted.
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
    #[serde(skip)]
    local_server_status: LocalServerStatus, // Status of locally launched server
    #[serde(skip)]
    last_rotated_tile: Option<(usize, bool)>, // (tile_index, clockwise) - for animation tracking
    #[serde(skip)]
    player_animations: std::collections::HashMap<PlayerID, PlayerAnimation>, // Active player movement animations
    #[serde(skip)]
    tile_placement_animation: Option<TilePlacementAnimation>, // Animation for the most recently placed tile
}

/// Tracks animation state for a player moving along their trail
#[derive(Debug, Clone)]
pub struct PlayerAnimation {
    pub trail: tsurust_common::trail::Trail,
    pub progress: f32, // 0.0 = start, 1.0 = end
    pub start_time: std::time::Instant,
    pub duration_secs: f32,
}

/// Tracks animation state for a tile being placed on the board
#[derive(Debug, Clone)]
pub struct TilePlacementAnimation {
    pub cell: tsurust_common::board::CellCoord,
    pub progress: f32, // 0.0 = start, 1.0 = end
    pub start_time: std::time::Instant,
    pub duration_secs: f32,
}

/// Tracks the status of a locally spawned server process.
/// The client can launch its own server instance for convenience (see handle_start_local_server).
#[derive(Debug, Clone, Default)]
pub enum LocalServerStatus {
    #[default]
    NotStarted,
    Running(u32), // PID of the server process
    Failed(String), // Error message
}

impl Default for TemplateApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            label: "Tsurust".to_owned(),
            app_state: AppState::MainMenu,
            sender: Some(sender),
            last_rotated_tile: None,
            player_animations: std::collections::HashMap::new(),
            tile_placement_animation: None,
            receiver: Some(receiver),
            current_player_id: 1, // Default player ID
            game_client: None,
            current_room_id: None,
            local_server_status: LocalServerStatus::NotStarted,
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

    fn start_player_animations(&mut self, game: &Game) {
        Self::start_player_animations_static(&mut self.player_animations, game);
    }

    fn start_player_animations_static(
        player_animations: &mut std::collections::HashMap<PlayerID, PlayerAnimation>,
        game: &Game
    ) {
        // Clear existing animations
        player_animations.clear();

        let now = std::time::Instant::now();
        let animation_speed = 2.0; // tiles per second

        // Create animations for players who moved this turn (use current_turn_trails, not cumulative player_trails)
        for (player_id, trail) in &game.current_turn_trails {
            if trail.segments.is_empty() {
                continue; // No movement
            }

            // Calculate animation duration based on trail length
            let duration = trail.length() as f32 / animation_speed;

            player_animations.insert(*player_id, PlayerAnimation {
                trail: trail.clone(),
                progress: 0.0,
                start_time: now,
                duration_secs: duration.max(0.3), // Minimum 0.3 seconds
            });
        }
    }

    fn update_player_animations(&mut self, ctx: &Context) {
        let now = std::time::Instant::now();
        let mut completed_animations = Vec::new();

        // Update progress for all active animations
        for (player_id, animation) in self.player_animations.iter_mut() {
            let elapsed = now.duration_since(animation.start_time).as_secs_f32();
            animation.progress = (elapsed / animation.duration_secs).min(1.0);

            if animation.progress >= 1.0 {
                completed_animations.push(*player_id);
            } else {
                // Request repaint for next frame
                ctx.request_repaint();
            }
        }

        // Remove completed animations
        for player_id in completed_animations {
            self.player_animations.remove(&player_id);
        }
    }

    fn update_tile_placement_animation(&mut self, ctx: &Context) {
        if let Some(animation) = &mut self.tile_placement_animation {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(animation.start_time).as_secs_f32();
            animation.progress = (elapsed / animation.duration_secs).min(1.0);

            if animation.progress >= 1.0 {
                // Animation complete
                self.tile_placement_animation = None;
            } else {
                // Request repaint for next frame
                ctx.request_repaint();
            }
        }
    }

    fn render_ui(ctx: &Context, app_state: &mut AppState, current_player_id: PlayerID, is_online: bool, server_status: &LocalServerStatus, sender: &mpsc::Sender<Message>, last_rotated_tile: Option<(usize, bool)>, player_animations: &std::collections::HashMap<PlayerID, PlayerAnimation>, tile_placement_animation: &Option<TilePlacementAnimation>) {
        match app_state {
            AppState::MainMenu => screens::main_menu::render(ctx, server_status, sender),
            AppState::CreateLobbyForm { lobby_name, player_name } => {
                screens::lobby_forms::render_create_lobby_form(ctx, lobby_name, player_name, sender)
            }
            AppState::JoinLobbyForm { lobby_id, player_name } => {
                screens::lobby_forms::render_join_lobby_form(ctx, lobby_id, player_name, sender)
            }
            AppState::Lobby(lobby) => {
                screens::lobby::render_lobby_ui(ctx, lobby, current_player_id, is_online, sender)
            }
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                screens::lobby::render_lobby_placing_ui(ctx, lobby, *placing_for_id, is_online, sender)
            }
            AppState::Game(game) => screens::game::render_game_ui(ctx, game, current_player_id, false, None, sender, last_rotated_tile, player_animations, tile_placement_animation),
            AppState::OnlineGame { game, waiting_for_server, lobby_name, .. } => {
                screens::game::render_game_ui(ctx, game, current_player_id, *waiting_for_server, Some(lobby_name.as_str()), sender, last_rotated_tile, player_animations, tile_placement_animation)
            }
            AppState::GameOver(game) => {
                // Keep GameOver state for backward compatibility but render as normal game with overlay
                screens::game::render_game_ui(ctx, game, current_player_id, false, None, sender, last_rotated_tile, player_animations, tile_placement_animation)
            }
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
                    &mut self.app_state,
                    &mut self.player_animations,
                    &mut self.tile_placement_animation,
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
            let is_online = self.game_client.is_some();
            Self::render_ui(ctx, &mut self.app_state, self.current_player_id, is_online, &self.local_server_status, tx, self.last_rotated_tile, &self.player_animations, &self.tile_placement_animation);

            // Clear the rotation animation state after one frame
            self.last_rotated_tile = None;

            // Update animations
            self.update_player_animations(ctx);
            self.update_tile_placement_animation(ctx);
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
            Message::StartLocalServer => self.handle_start_local_server(),
            Message::SendToServer(client_msg) => self.handle_send_to_server(client_msg),
        }
    }

    // ========== Message Handler Methods ==========

    fn handle_server_message(
        server_msg: ServerMessage,
        current_room_id: &mut Option<String>,
        current_player_id: &mut PlayerID,
        app_state: &mut AppState,
        player_animations: &mut std::collections::HashMap<PlayerID, PlayerAnimation>,
        tile_placement_animation: &mut Option<TilePlacementAnimation>,
    ) {
        match server_msg {
            ServerMessage::RoomCreated { room_id, player_id } => {
                *current_room_id = Some(room_id.clone());
                *current_player_id = player_id;

                // Update the lobby's ID to match the server-generated room ID
                if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) = app_state {
                    lobby.id = room_id.clone();
                }
            }
            ServerMessage::PlayerJoined { room_id, player_id, player_name } => {
                // If we don't have a room ID yet, this might be our own join confirmation
                if current_room_id.is_none() {
                    *current_room_id = Some(room_id.clone());
                    *current_player_id = player_id;
                }

                // Update lobby state if we're in a lobby
                if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) = app_state {
                    if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                        player_id,
                        player_name: player_name.clone(),
                    }) {
                        eprintln!("Failed to sync player join to lobby: {:?}", e);
                    }
                }
            }
            ServerMessage::GameStateUpdate { room_id: _, state } => {
                // Server is source of truth - update our game state
                if let AppState::OnlineGame { game, waiting_for_server, .. } = app_state {
                    *game = state.clone();
                    *waiting_for_server = false;

                    // Start animations
                    Self::start_player_animations_static(player_animations, &state);

                    // Start tile placement animation for the most recent move
                    if let Some(last_move) = state.board.history.last() {
                        *tile_placement_animation = Some(TilePlacementAnimation {
                            cell: last_move.cell,
                            progress: 0.0,
                            start_time: std::time::Instant::now(),
                            duration_secs: 0.4,
                        });
                    }

                    // Stay in OnlineGame state even if game is over (overlay will show stats)
                }
            }
            ServerMessage::TurnCompleted { room_id: _, result: _ } => {
                // Game over will be handled by GameStateUpdate
                // No need to transition to GameOver state - overlay will show when game.is_game_over() is true
            }
            ServerMessage::Error { message } => {
                eprintln!("Server error: {}", message);
                // Clear waiting state if we were waiting
                if let AppState::OnlineGame { waiting_for_server, .. } = app_state {
                    *waiting_for_server = false;
                }
            }
            ServerMessage::PlayerLeft { room_id: _, player_id: _ } => {
                // TODO: Update lobby state when PlayerLeft event is implemented in Lobby
            }
            ServerMessage::LobbyStateUpdate { room_id: _, lobby } => {
                if let AppState::Lobby(current_lobby) | AppState::LobbyPlacingFor(current_lobby, _) = app_state {
                    *current_lobby = lobby;
                }
            }
            ServerMessage::PawnPlaced { room_id: _, player_id, position } => {
                if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) = app_state {
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        eprintln!("Failed to sync pawn placement: {:?}", e);
                    }
                }
            }
            ServerMessage::GameStarted { room_id, game } => {
                // Get lobby name from current lobby state
                let lobby_name = match app_state {
                    AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) => lobby.name.clone(),
                    _ => "Game".to_string(), // Fallback name
                };

                // Transition to online game mode with server's authoritative game state
                *app_state = AppState::OnlineGame {
                    game,
                    room_id,
                    lobby_name,
                    waiting_for_server: false,
                };
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
            Ok(client) => {
                self.game_client = Some(client);

                // Send create room message via mpsc
                if let Some(sender) = &self.sender {
                    crate::messaging::send_server_message(sender, ClientMessage::CreateRoom {
                        room_name: lobby_name.clone(),
                        creator_name: player_name.clone(),
                    });
                }

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
        match GameClient::connect("ws://127.0.0.1:8080") {
            Ok(client) => {
                self.game_client = Some(client);

                // Send join room message via mpsc
                if let Some(sender) = &self.sender {
                    crate::messaging::send_server_message(sender, ClientMessage::JoinRoom {
                        room_id: lobby_id.clone(),
                        player_name: player_name.clone(),
                    });
                }

                // Create empty lobby - it will be populated when server sends PlayerJoined messages
                let lobby = Lobby::new(lobby_id.clone(), format!("Room {}", lobby_id));
                self.app_state = AppState::Lobby(lobby);
            }
            Err(e) => {
                eprintln!("Failed to connect to server: {}", e);
                // Stay on join form so user can try again
                // TODO: Display error message in UI
            }
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
        let is_online = self.game_client.is_some();

        match &mut self.app_state {
            AppState::Lobby(lobby) => {
                let player_id = self.current_player_id;

                // For online lobbies, send to server instead of handling locally
                if is_online {
                    if let (Some(sender), Some(room_id)) = (&self.sender, &self.current_room_id) {
                        crate::messaging::send_server_message(sender, ClientMessage::PlacePawn {
                            room_id: room_id.clone(),
                            player_id,
                            position,
                        });
                    }
                } else {
                    // Local lobby - handle locally
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        eprintln!("Failed to place pawn: {:?}", e);
                    }
                }
            }
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                let player_id = *placing_for_id;

                // For online lobbies, send to server
                if is_online {
                    if let (Some(sender), Some(room_id)) = (&self.sender, &self.current_room_id) {
                        crate::messaging::send_server_message(sender, ClientMessage::PlacePawn {
                            room_id: room_id.clone(),
                            player_id,
                            position,
                        });
                        // Transition back to regular lobby view
                        self.app_state = AppState::Lobby(lobby.clone());
                    }
                } else {
                    // Local lobby - handle locally
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        eprintln!("Failed to place pawn: {:?}", e);
                    } else {
                        self.app_state = AppState::Lobby(lobby.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_start_game_from_lobby(&mut self) {
        let is_online = self.game_client.is_some();

        if let AppState::Lobby(lobby) = &mut self.app_state {
            // For online lobbies, send start game request to server
            if is_online {
                if let (Some(sender), Some(room_id)) = (&self.sender, &self.current_room_id) {
                    crate::messaging::send_server_message(sender, ClientMessage::StartGame {
                        room_id: room_id.clone(),
                    });
                    // Server will send GameStarted message to all clients
                }
                return;
            }

            // Local lobby - handle locally
            if let Err(e) = lobby.handle_event(LobbyEvent::StartGame) {
                eprintln!("Failed to start game: {:?}", e);
                return;
            }

            match lobby.to_game() {
                Ok(game) => {
                    // Local game - client is authoritative
                    self.app_state = AppState::Game(game);
                }
                Err(e) => {
                    eprintln!("Failed to convert lobby to game: {:?}", e);
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
        // Get the player whose tiles we're viewing (always client_player_id)
        let client_player_id = self.current_player_id;

        // Get mutable reference to the game, whether local or online
        let game = match &mut self.app_state {
            AppState::Game(game) => game,
            AppState::OnlineGame { game, .. } => game,  // Rotation is client-side only
            _ => return,
        };

        // Always rotate the client player's tiles (the ones being displayed)
        // This allows planning/previewing even when it's not your turn in local games
        if let Some(hand) = game.hands.get_mut(&client_player_id) {
            if tile_index < hand.len() {
                hand[tile_index] = hand[tile_index].rotated(clockwise);
                // Track the rotation for animation
                self.last_rotated_tile = Some((tile_index, clockwise));
            }
        }
    }

    fn handle_tile_placed(&mut self, tile_index: usize) {
        match &mut self.app_state {
            // Local game: perform move immediately (client authoritative)
            AppState::Game(_) => {
                // Take ownership of the game state temporarily
                if let AppState::Game(game) = std::mem::replace(&mut self.app_state, AppState::MainMenu) {
                    let (game, placed_cell) = Self::perform_local_tile_placement(game, tile_index);

                    // Start animations
                    self.start_player_animations(&game);
                    self.start_tile_placement_animation(placed_cell);

                    // Stay in Game state even if game is over (overlay will show stats)
                    self.app_state = AppState::Game(game);
                }
            }

            // Online game: send to server and wait for response (server authoritative)
            AppState::OnlineGame { game, room_id, waiting_for_server, .. } => {
                if *waiting_for_server {
                    eprintln!("Already waiting for server response");
                    return;
                }

                // Only allow placing tiles when it's this client's turn
                if game.current_player_id != self.current_player_id {
                    eprintln!("Not your turn! Current player: {}, Your player: {}",
                             game.current_player_id, self.current_player_id);
                    return;
                }

                let player_cell = game.players.iter()
                    .find(|p| p.id == self.current_player_id && p.alive)
                    .expect("current player should exist and be alive")
                    .pos.cell;

                // Get the client's own hand (not the current player's hand)
                let hand = game.hands.get(&self.current_player_id)
                    .expect("client should have a hand");

                if tile_index >= hand.len() {
                    eprintln!("Invalid tile index: {} (hand size: {})", tile_index, hand.len());
                    return;
                }

                let tile = hand[tile_index];

                let mov = Move {
                    tile,
                    cell: player_cell,
                    player_id: self.current_player_id,
                };

                // Send to server via mpsc
                if let Some(sender) = &self.sender {
                    crate::messaging::send_server_message(sender, ClientMessage::PlaceTile {
                        room_id: room_id.clone(),
                        player_id: self.current_player_id,
                        mov,
                    });
                    *waiting_for_server = true;
                }
            }

            _ => {}
        }
    }

    /// Perform a tile placement in a local game (client authoritative)
    /// Returns the updated game and the cell where the tile was placed
    fn perform_local_tile_placement(mut game: Game, tile_index: usize) -> (Game, tsurust_common::board::CellCoord) {
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
            Ok(_turn_result) => {
                (game, player_cell)
            }
            Err(error) => {
                eprintln!("Failed to place tile: {}", error);
                (game, player_cell)
            }
        }
    }

    fn start_tile_placement_animation(&mut self, cell: tsurust_common::board::CellCoord) {
        self.tile_placement_animation = Some(TilePlacementAnimation {
            cell,
            progress: 0.0,
            start_time: std::time::Instant::now(),
            duration_secs: 0.4,
        });
    }


    fn handle_restart_game(&mut self) {
        match &mut self.app_state {
            AppState::Game(game) | AppState::GameOver(game) => {
                let players = vec![
                    Player::new(1, PlayerPos::new(0, 2, 5)),
                    Player::new(2, PlayerPos::new(2, 5, 2)),
                    Player::new(3, PlayerPos::new(5, 3, 0)),
                    Player::new(4, PlayerPos::new(3, 0, 6)),
                ];
                *game = Game::new(players);
                self.app_state = AppState::Game(game.clone());
            }
            _ => {}
        }
    }

    fn handle_start_local_server(&mut self) {
        use std::process::Command;

        // Determine the server binary name based on platform
        #[cfg(target_os = "windows")]
        let server_binary = "server.exe";
        #[cfg(not(target_os = "windows"))]
        let server_binary = "server";

        // Try to launch the server binary - first try debug, then release
        let debug_path = format!("target/debug/{}", server_binary);
        let result = Command::new(&debug_path).spawn();

        match result {
            Ok(child) => {
                let pid = child.id();
                self.local_server_status = LocalServerStatus::Running(pid);
                // Note: We're not storing the child process handle, so it will run independently
            }
            Err(_) => {
                // If debug build fails, try release build
                let release_path = format!("target/release/{}", server_binary);

                match Command::new(&release_path).spawn() {
                    Ok(child) => {
                        let pid = child.id();
                        self.local_server_status = LocalServerStatus::Running(pid);
                    }
                    Err(e) => {
                        eprintln!("Failed to start local server: {}", e);
                        eprintln!("Make sure to build the server first with: cargo build --bin server");

                        let error_msg = "Binary not found. Run 'cargo build --bin server' first.".to_string();
                        self.local_server_status = LocalServerStatus::Failed(error_msg);
                    }
                }
            }
        }
    }

    fn handle_send_to_server(&mut self, client_msg: ClientMessage) {
        if let Some(client) = &mut self.game_client {
            client.send(client_msg);
        } else {
            eprintln!("Cannot send message to server: not connected");
        }
    }
}
