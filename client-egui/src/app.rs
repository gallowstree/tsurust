use std::sync::mpsc;

use eframe::egui;
use egui::Context;

use tsurust_common::board::*;
use tsurust_common::game::Game;
use tsurust_common::lobby::{Lobby, LobbyEvent, Visibility};

use tsurust_common::protocol::{ClientMessage, LobbyListing, ServerMessage};

use crate::screens;
use crate::ws_client::{ConnectionStatus, GameClient};

// Cross-platform time support
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// How long a transient error toast stays visible before it auto-dismisses.
const ERROR_TOAST_DURATION: std::time::Duration = std::time::Duration::from_secs(6);

/// All animation timing lives here so it can be tuned in one place.
pub mod animation {
    /// How fast pawns travel along their trail, in tiles per second.
    pub const PLAYER_SPEED_TILES_PER_SEC: f32 = 2.0;
    /// Pawn movement never animates faster than this, even for short hops.
    pub const PLAYER_MIN_DURATION_SECS: f32 = 0.3;
    /// Drop-in animation when a tile lands on the board.
    pub const TILE_PLACEMENT_DURATION_SECS: f32 = 0.4;
    /// Spin animation when a hand tile is rotated.
    pub const TILE_ROTATION_DURATION_SECS: f32 = 0.5;
}

/// Extract a room id from the browser location, so `http://host/#ABCD` (or a
/// served path of `/ABCD`) opens the join form prefilled with that room.
/// Only wired up on wasm (there is no URL on native), but kept
/// target-independent so the parsing is unit-tested natively.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn room_id_from_location(path: &str, hash: &str) -> Option<String> {
    let hash = hash.trim_start_matches('#');
    let candidate = if hash.is_empty() {
        path.trim_matches('/')
    } else {
        hash
    };
    tsurust_common::lobby::normalize_lobby_id(candidate)
}

/// The WebSocket URL used when nothing else is configured. On the web this is a
/// deliberate dead-end — it points at the *visitor's own* machine — so the main
/// menu warns when it's in effect. Only useful for a host testing locally.
pub const DEFAULT_WS_URL: &str = "ws://127.0.0.1:8080";

/// Pull a `server` value out of a raw location query string
/// (e.g. `?server=wss%3A%2F%2Fhost`), percent-decoded. This is the shareable
/// invite path: a host sends `.../tsurust/?server=wss://their-tunnel`. Kept
/// target-independent so it can be unit-tested natively.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn server_url_from_query(search: &str) -> Option<String> {
    let search = search.trim_start_matches('?');
    for pair in search.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some("server") {
            let value = percent_decode(kv.next().unwrap_or(""));
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Minimal `%XX`/`+` percent-decoder for query values — enough to recover a
/// `wss://…` URL that was URL-encoded into a `?server=` link.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match (
                    (bytes[i + 1] as char).to_digit(16),
                    (bytes[i + 2] as char).to_digit(16),
                ) {
                    (Some(hi), Some(lo)) => {
                        out.push((hi * 16 + lo) as u8);
                        i += 3;
                    }
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolve the initial WebSocket server URL on the web, in priority order:
/// 1. a `?server=<url>` query param (the shareable-invite path),
/// 2. a baked `window.TSURUST_CONFIG.wsServerUrl`,
/// 3. the localhost default (a deliberate dead-end the UI warns about).
#[cfg(target_arch = "wasm32")]
fn get_websocket_url() -> String {
    if let Some(window) = web_sys::window() {
        // 1. Explicit ?server= param wins — this is how invite links work.
        if let Ok(search) = window.location().search() {
            if let Some(url) = server_url_from_query(&search) {
                web_sys::console::log_1(
                    &format!("Using WebSocket URL from ?server=: {}", url).into(),
                );
                return url;
            }
        }
        // 2. Build-time config baked into index.html.
        if let Ok(config) = js_sys::Reflect::get(&window, &JsValue::from_str("TSURUST_CONFIG")) {
            if !config.is_undefined() {
                if let Ok(ws_url) = js_sys::Reflect::get(&config, &JsValue::from_str("wsServerUrl"))
                {
                    if let Some(url_str) = ws_url.as_string() {
                        web_sys::console::log_1(
                            &format!("Using WebSocket URL from config: {}", url_str).into(),
                        );
                        return url_str;
                    }
                }
            }
        }
    }

    // 3. Fallback to the localhost default — warn, since on the web this reaches
    //    only the visitor's own machine (the main menu shows this too).
    web_sys::console::warn_1(
        &format!(
            "No ?server= or TSURUST_CONFIG set — falling back to {} (your own machine)",
            DEFAULT_WS_URL
        )
        .into(),
    );
    DEFAULT_WS_URL.to_string()
}

/// Get WebSocket server URL for native builds
#[cfg(not(target_arch = "wasm32"))]
fn get_websocket_url() -> String {
    std::env::var("WS_SERVER_URL").unwrap_or_else(|_| DEFAULT_WS_URL.to_string())
}

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),        // tile index - place at current player position
    TileRotated(usize, bool), // tile index, clockwise
    RestartGame,              // restart the game
    StartLobby,               // start a local lobby (offline multiplayer)
    StartSampleGame,          // start sample game
    #[allow(dead_code)]
    JoinLobby(String), // join lobby with player name
    PlacePawn(PlayerPos),     // place pawn at position in lobby
    StartGameFromLobby,       // start game from lobby
    ShowCreateLobbyForm,      // show create lobby form
    ShowJoinLobbyForm,        // show join lobby form
    CreateAndJoinLobby(String, String, Visibility, Option<u64>), // (lobby_name, player_name, visibility, turn_timer_secs)
    JoinLobbyWithId(String, String),                             // (lobby_id, player_name)
    RefreshLobbies,                // re-fetch the public lobby directory
    SpectateLobby(String, String), // (room_id, room_name) watch a game in progress
    BackToMainMenu,                // return to main menu
    DebugAddPlayer,                // debug: simulate player joining
    DebugPlacePawn(PlayerID),      // debug: place pawn for specific player
    DebugCyclePlayer(bool),        // debug: cycle active player (true = next, false = prev)
    StartLocalServer,              // start a local server process
    SendToServer(ClientMessage),   // send a message to the server via WebSocket
    // Export/Import
    ExportGame,   // Export current game to JSON file
    ImportReplay, // Import replay from JSON file
    #[allow(dead_code)]
    ReplayLoaded(Box<tsurust_common::game::GameExport>), // Replay file loaded (WASM async callback)
    // Replay controls
    ReplayPlay,              // Start replay playback
    ReplayPause,             // Pause replay playback
    ReplayStepForward,       // Step forward one move
    ReplayStepBackward,      // Step backward one move
    ReplaySetSpeed(f32),     // Set playback speed (moves per second)
    ReplayJumpToMove(usize), // Jump to specific move index
    ReplayJumpToStart,       // Jump to start of replay
    ReplayJumpToEnd,         // Jump to end of replay
    ExitReplay,              // Exit replay viewer and return to main menu
}

/// An in-flight request from the join screen; the form shows a spinner and
/// stays put until the server confirms or rejects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinScreenRequest {
    /// Joining a lobby as a player (by code or from the public directory).
    Join,
    /// Spectating an in-progress game from the public directory.
    Spectate { room_id: String, room_name: String },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum AppState {
    MainMenu,
    CreateLobbyForm {
        lobby_name: String,
        player_name: String,
        /// Whether the room will be listed in the public lobby directory.
        public: bool,
        /// Per-turn clock for the game; the server auto-plays when it lapses.
        turn_timer_secs: Option<u64>,
    },
    JoinLobbyForm {
        lobby_id: String,
        player_name: String,
        pending: Option<JoinScreenRequest>,
        /// The server's public lobby directory, fetched when the screen
        /// opens and on demand via the refresh button.
        lobbies: Vec<LobbyListing>,
    },
    Lobby(Lobby),                     // Normal lobby view (place own pawn)
    LobbyPlacingFor(Lobby, PlayerID), // Debug mode: placing pawn for specific player
    Game(Game),                       // Local game - client authoritative
    OnlineGame {
        game: Game, // Server's authoritative state (redacted: only our own hand)
        room_id: String,
        lobby_name: String,       // Display name of the lobby/game
        waiting_for_server: bool, // Show loading state during server round-trip
        /// Every player's hand size, from the server. The redacted `game` only
        /// carries our own tiles, so opponents' counts come from here.
        hand_counts: std::collections::HashMap<PlayerID, usize>,
        /// Watching only: no hand, no moves; just the live board.
        is_spectator: bool,
    },
    ReplayViewer {
        replay_state: crate::replay_state::ReplayState,
        current_game: Game, // Cached game state at current move index
    },
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
    server_url: String, // Effective WebSocket URL (from ?server=, baked config, or default)
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
    #[serde(skip)]
    last_error: Option<(String, Instant)>, // (message, shown_at) for the transient error toast
    #[serde(skip)]
    turn_deadline: Option<Instant>, // When the current turn's clock lapses (from the server; None = untimed)
    #[serde(skip)]
    egui_ctx: Option<Context>, // For repaint wakeups from the WebSocket thread
}

/// Tracks animation state for a player moving along their trail
#[derive(Debug, Clone)]
pub struct PlayerAnimation {
    pub trail: tsurust_common::trail::Trail,
    pub progress: f32, // 0.0 = start, 1.0 = end
    pub start_time: Instant,
    pub duration_secs: f32,
}

/// Tracks animation state for a tile being placed on the board
#[derive(Debug, Clone)]
pub struct TilePlacementAnimation {
    pub cell: tsurust_common::board::CellCoord,
    pub progress: f32, // 0.0 = start, 1.0 = end
    pub start_time: Instant,
    pub duration_secs: f32,
}

/// Tracks the status of a locally spawned server process.
/// The client can launch its own server instance for convenience (see handle_start_local_server).
#[derive(Debug, Clone, Default)]
pub enum LocalServerStatus {
    #[default]
    NotStarted,
    Running(u32),   // PID of the server process
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
            server_url: get_websocket_url(),
            current_room_id: None,
            local_server_status: LocalServerStatus::NotStarted,
            last_error: None,
            turn_deadline: None,
            egui_ctx: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: TemplateApp = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        app.egui_ctx = Some(cc.egui_ctx.clone());

        // On the web, a room id in the URL (path or #fragment) opens the join
        // form prefilled with it — a shareable "join my game" link.
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            let location = window.location();
            let path = location.pathname().unwrap_or_default();
            let hash = location.hash().unwrap_or_default();
            if let Some(room_id) = room_id_from_location(&path, &hash) {
                app.app_state = AppState::JoinLobbyForm {
                    lobby_id: room_id,
                    player_name: String::new(),
                    pending: None,
                    lobbies: Vec::new(),
                };
            }
        }

        app
    }

    /// The room this client is currently in, if any. Read-only view for
    /// end-to-end UI tests; exposes only what's already visible on screen.
    pub fn current_room_id(&self) -> Option<&str> {
        self.current_room_id.as_deref()
    }

    /// This client's player id. Read-only view for end-to-end UI tests.
    pub fn client_player_id(&self) -> PlayerID {
        self.current_player_id
    }

    /// The game being displayed (local or online), if any. Read-only view for
    /// end-to-end UI tests.
    pub fn visible_game(&self) -> Option<&Game> {
        match &self.app_state {
            AppState::Game(game) | AppState::OnlineGame { game, .. } => Some(game),
            _ => None,
        }
    }

    /// The lobby being displayed, if any. Read-only view for end-to-end UI tests.
    pub fn visible_lobby(&self) -> Option<&Lobby> {
        match &self.app_state {
            AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) => Some(lobby),
            _ => None,
        }
    }

    /// Wakeup callback handed to the WebSocket client: request a repaint when a
    /// socket event arrives, so `update` gets called to drain it. This replaces
    /// polling with `request_repaint()` every frame while a connection exists.
    fn ws_wakeup(&self) -> impl Fn() + Send + Sync + 'static {
        let ctx = self.egui_ctx.clone();
        move || {
            if let Some(ctx) = &ctx {
                ctx.request_repaint();
            }
        }
    }

    fn start_player_animations(&mut self, game: &Game) {
        Self::start_player_animations_static(&mut self.player_animations, game);
    }

    fn start_player_animations_static(
        player_animations: &mut std::collections::HashMap<PlayerID, PlayerAnimation>,
        game: &Game,
    ) {
        // Clear existing animations
        player_animations.clear();

        let now = Instant::now();

        // Create animations for players who moved this turn (use current_turn_trails, not cumulative player_trails)
        for (player_id, trail) in &game.current_turn_trails {
            if trail.segments.is_empty() {
                continue; // No movement
            }

            // Calculate animation duration based on trail length
            let duration = trail.length() as f32 / animation::PLAYER_SPEED_TILES_PER_SEC;

            player_animations.insert(
                *player_id,
                PlayerAnimation {
                    trail: trail.clone(),
                    progress: 0.0,
                    start_time: now,
                    duration_secs: duration.max(animation::PLAYER_MIN_DURATION_SECS),
                },
            );
        }
    }

    fn update_player_animations(&mut self, ctx: &Context) {
        let now = Instant::now();
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
            let now = Instant::now();
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

    #[allow(clippy::too_many_arguments)]
    fn render_ui(
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        current_player_id: PlayerID,
        connection: Option<&ConnectionStatus>,
        server_status: &LocalServerStatus,
        server_url: &mut String,
        sender: &mpsc::Sender<Message>,
        last_rotated_tile: Option<(usize, bool)>,
        player_animations: &std::collections::HashMap<PlayerID, PlayerAnimation>,
        tile_placement_animation: &Option<TilePlacementAnimation>,
        turn_deadline: Option<Instant>,
    ) {
        match app_state {
            AppState::MainMenu => screens::main_menu::render(ui, server_status, server_url, sender),
            AppState::CreateLobbyForm {
                lobby_name,
                player_name,
                public,
                turn_timer_secs,
            } => screens::lobby_forms::render_create_lobby_form(
                ui,
                lobby_name,
                player_name,
                public,
                turn_timer_secs,
                sender,
            ),
            AppState::JoinLobbyForm {
                lobby_id,
                player_name,
                pending,
                lobbies,
            } => {
                let pending = pending.clone();
                screens::lobby_forms::render_join_lobby_form(
                    ui,
                    lobby_id,
                    player_name,
                    pending.as_ref(),
                    lobbies,
                    sender,
                )
            }
            AppState::Lobby(lobby) => {
                screens::lobby::render_lobby_ui(ui, lobby, current_player_id, connection, sender)
            }
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                screens::lobby::render_lobby_placing_ui(
                    ui,
                    lobby,
                    *placing_for_id,
                    connection,
                    sender,
                )
            }
            AppState::Game(game) => {
                // Local hot-seat play has no fixed client identity: the hand to
                // show (and act on) belongs to whoever's turn it currently is,
                // not the online-identity `current_player_id`.
                let active_player = game.current_player_id;
                screens::game::render_game_ui(
                    ui,
                    game,
                    active_player,
                    false,
                    None,
                    None,
                    // Local games hold every hand, so counts come from the game itself.
                    None,
                    false,
                    sender,
                    last_rotated_tile,
                    player_animations,
                    tile_placement_animation,
                    // Local games are untimed (the timer is a server feature).
                    None,
                )
            }
            AppState::OnlineGame {
                game,
                waiting_for_server,
                lobby_name,
                hand_counts,
                is_spectator,
                ..
            } => screens::game::render_game_ui(
                ui,
                game,
                current_player_id,
                *waiting_for_server,
                Some(lobby_name.as_str()),
                connection,
                Some(hand_counts),
                *is_spectator,
                sender,
                last_rotated_tile,
                player_animations,
                tile_placement_animation,
                turn_deadline,
            ),
            AppState::ReplayViewer {
                replay_state,
                current_game,
            } => screens::replay_viewer::render_replay_viewer_ui(
                ui,
                replay_state,
                current_game,
                sender,
            ),
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Poll for server messages from the WebSocket connection
        if let Some(client) = &mut self.game_client {
            while let Some(server_msg) = client.try_recv() {
                Self::handle_server_message(
                    server_msg,
                    &mut self.current_room_id,
                    &mut self.current_player_id,
                    &mut self.app_state,
                    &mut self.player_animations,
                    &mut self.tile_placement_animation,
                    &mut self.last_error,
                    &mut self.turn_deadline,
                );
            }
            // No polling needed here: the ws_wakeup callback requests a repaint
            // whenever a socket event arrives.
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

        // Handle replay auto-advance
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.update(&ctx) {
                *current_game = new_game;
            }
        }

        // Render UI
        let connection_status = self.game_client.as_ref().map(|c| c.status.clone());
        if let Some(tx) = &self.sender {
            Self::render_ui(
                ui,
                &mut self.app_state,
                self.current_player_id,
                connection_status.as_ref(),
                &self.local_server_status,
                &mut self.server_url,
                tx,
                self.last_rotated_tile,
                &self.player_animations,
                &self.tile_placement_animation,
                self.turn_deadline,
            );

            // Clear the rotation animation state after one frame
            self.last_rotated_tile = None;

            // Update animations
            self.update_player_animations(&ctx);
            self.update_tile_placement_animation(&ctx);
        }

        // Fail-closed disconnect handling (see proposals/004, Option B): if the
        // socket dropped, show a modal and route back to the main menu on confirm.
        let disconnect_reason = self.game_client.as_ref().and_then(|c| match &c.status {
            ConnectionStatus::Disconnected { reason } => Some(reason.clone()),
            _ => None,
        });
        // Browsing lobbies (or idling in a menu) isn't a game: losing the
        // socket there gets a toast and a re-enabled form, not the modal.
        if let (Some(reason), None) = (&disconnect_reason, &self.current_room_id) {
            self.last_error = Some((
                format!("Lost connection to server: {reason}"),
                Instant::now(),
            ));
            self.game_client = None;
            if let AppState::JoinLobbyForm {
                pending, lobbies, ..
            } = &mut self.app_state
            {
                *pending = None;
                lobbies.clear();
            }
        } else if let Some(reason) = disconnect_reason {
            let mut return_to_menu = false;
            egui::Window::new("⚠ Disconnected from server")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(&ctx, |ui| {
                    ui.label(format!("Connection lost: {}", reason));
                    ui.label("Your game has ended.");
                    ui.add_space(8.0);
                    if ui.button("Return to menu").clicked() {
                        return_to_menu = true;
                    }
                });
            if return_to_menu {
                self.disconnect_and_return_to_menu();
            }
        }

        // Transient error toast (server errors, failed connects). Rendered last so
        // it floats above whatever screen is active.
        self.render_error_toast(&ctx);
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
            Message::CreateAndJoinLobby(name, player, visibility, turn_timer_secs) => {
                self.handle_create_and_join_lobby(name, player, visibility, turn_timer_secs)
            }
            Message::JoinLobbyWithId(id, player) => self.handle_join_lobby_with_id(id, player),
            Message::RefreshLobbies => self.request_lobby_list(),
            Message::SpectateLobby(room_id, room_name) => {
                self.handle_spectate_lobby(room_id, room_name)
            }
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
            // Export/Import
            Message::ExportGame => self.handle_export_game(),
            Message::ImportReplay => self.handle_import_replay(),
            Message::ReplayLoaded(export) => {
                // Replay file loaded asynchronously (WASM only)
                self.open_replay(*export);
            }
            // Replay controls
            Message::ReplayPlay => self.handle_replay_play(),
            Message::ReplayPause => self.handle_replay_pause(),
            Message::ReplayStepForward => self.handle_replay_step_forward(),
            Message::ReplayStepBackward => self.handle_replay_step_backward(),
            Message::ReplaySetSpeed(speed) => self.handle_replay_set_speed(speed),
            Message::ReplayJumpToMove(index) => self.handle_replay_jump_to_move(index),
            Message::ReplayJumpToStart => self.handle_replay_jump_to_start(),
            Message::ReplayJumpToEnd => self.handle_replay_jump_to_end(),
            Message::ExitReplay => self.handle_exit_replay(),
        }
    }

    // ========== Message Handler Methods ==========

    #[allow(clippy::too_many_arguments)]
    fn handle_server_message(
        server_msg: ServerMessage,
        current_room_id: &mut Option<String>,
        current_player_id: &mut PlayerID,
        app_state: &mut AppState,
        player_animations: &mut std::collections::HashMap<PlayerID, PlayerAnimation>,
        tile_placement_animation: &mut Option<TilePlacementAnimation>,
        last_error: &mut Option<(String, Instant)>,
        turn_deadline: &mut Option<Instant>,
    ) {
        println!("[SERVER->CLIENT] Received: {:?}", server_msg);
        match server_msg {
            ServerMessage::RoomCreated { room_id, player_id } => {
                *current_room_id = Some(room_id.clone());
                *current_player_id = player_id;

                // Update the lobby's ID to match the server-generated room ID
                if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) = app_state {
                    lobby.id = room_id.clone();
                }
            }
            ServerMessage::PlayerJoined {
                room_id,
                player_id,
                player_name,
            } => {
                // If we don't have a room ID yet, this might be our own join confirmation
                if current_room_id.is_none() {
                    *current_room_id = Some(room_id.clone());
                    *current_player_id = player_id;
                }

                // Our own join confirmed while still on the join form (the
                // LobbyStateUpdate that normally precedes this transitions us;
                // this is the fallback if it arrives out of order): enter a
                // provisional lobby that the next lobby update fills in.
                if let AppState::JoinLobbyForm {
                    pending: Some(JoinScreenRequest::Join),
                    ..
                } = app_state
                {
                    let mut lobby = Lobby::new(room_id.clone(), format!("Room {}", room_id));
                    if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                        player_id,
                        player_name: player_name.clone(),
                    }) {
                        eprintln!("Failed to add self to provisional lobby: {:?}", e);
                    }
                    *app_state = AppState::Lobby(lobby);
                }
                // Update lobby state if we're in a lobby
                else if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) =
                    app_state
                {
                    if let Err(e) = lobby.handle_event(LobbyEvent::PlayerJoined {
                        player_id,
                        player_name: player_name.clone(),
                    }) {
                        eprintln!("Failed to sync player join to lobby: {:?}", e);
                    }
                }
            }
            ServerMessage::GameStateUpdate {
                room_id,
                mut state,
                hand_counts,
                turn_deadline_secs,
                ..
            } => {
                // Restart the local countdown from the server's clock reading.
                *turn_deadline =
                    turn_deadline_secs.map(|s| Instant::now() + std::time::Duration::from_secs(s));

                // A pending spectate resolved: enter the game as an observer.
                if let AppState::JoinLobbyForm {
                    pending:
                        Some(JoinScreenRequest::Spectate {
                            room_id: wanted,
                            room_name,
                        }),
                    ..
                } = app_state
                {
                    if *wanted == room_id {
                        let lobby_name = room_name.clone();
                        *current_room_id = Some(room_id.clone());
                        // Spectators are nobody: 0 matches no player, so no
                        // "(You)" badge and no hand is ever considered ours.
                        *current_player_id = 0;
                        *app_state = AppState::OnlineGame {
                            game: state,
                            room_id,
                            lobby_name,
                            waiting_for_server: false,
                            hand_counts,
                            is_spectator: true,
                        };
                        return;
                    }
                }

                // Server is source of truth - update our game state
                if let AppState::OnlineGame {
                    game,
                    waiting_for_server,
                    hand_counts: current_counts,
                    ..
                } = app_state
                {
                    *current_counts = hand_counts;
                    // Rotation is presentation-only (the server matches tiles
                    // rotation-invariantly), so keep the locally rotated tile
                    // wherever the server's hand still holds the same tile in
                    // the same slot — otherwise every update snaps them back.
                    let me = *current_player_id;
                    if let (Some(old_hand), Some(new_hand)) =
                        (game.hands.get(&me), state.hands.get_mut(&me))
                    {
                        for (slot, tile) in new_hand.iter_mut().enumerate() {
                            if let Some(old) = old_hand.get(slot) {
                                if old.is_same_tile(tile) {
                                    *tile = *old;
                                }
                            }
                        }
                    }

                    *game = state.clone();
                    *waiting_for_server = false;

                    // Start animations
                    Self::start_player_animations_static(player_animations, &state);

                    // Start tile placement animation for the most recent move
                    if let Some(last_move) = state.board.history.last() {
                        *tile_placement_animation = Some(TilePlacementAnimation {
                            cell: last_move.cell,
                            progress: 0.0,
                            start_time: Instant::now(),
                            duration_secs: animation::TILE_PLACEMENT_DURATION_SECS,
                        });
                    }

                    // Stay in OnlineGame state even if game is over (overlay will show stats)
                }
            }
            ServerMessage::TurnCompleted {
                room_id: _,
                result: _,
                auto_played,
            } => {
                // Game over will be handled by GameStateUpdate
                // No need to transition to GameOver state - overlay will show when game.is_game_over() is true
                if auto_played {
                    *last_error = Some((
                        "The turn clock ran out — the server played that turn.".to_string(),
                        Instant::now(),
                    ));
                }
            }
            ServerMessage::Error { message } => {
                eprintln!("Server error: {}", message);
                // Clear waiting state if we were waiting
                if let AppState::OnlineGame {
                    waiting_for_server, ..
                } = app_state
                {
                    *waiting_for_server = false;
                }
                // A rejected join/spectate re-enables the form for a retry.
                if let AppState::JoinLobbyForm { pending, .. } = app_state {
                    *pending = None;
                }
                // Surface the error to the player via a transient toast.
                *last_error = Some((message, Instant::now()));
            }
            ServerMessage::PlayerLeft {
                room_id: _,
                player_id: _,
            } => {
                // TODO: Update lobby state when PlayerLeft event is implemented in Lobby
            }
            ServerMessage::LobbyList { lobbies: list } => {
                if let AppState::JoinLobbyForm { lobbies, .. } = app_state {
                    *lobbies = list;
                }
            }
            ServerMessage::LobbyStateUpdate { room_id: _, lobby } => {
                match app_state {
                    AppState::Lobby(current_lobby)
                    | AppState::LobbyPlacingFor(current_lobby, _) => {
                        *current_lobby = lobby;
                    }
                    // A pending join resolved: the server's first lobby state
                    // is our cue to leave the form and enter the lobby. Our
                    // own identity (room + player id) is NOT set here — the
                    // PlayerJoined confirmation that follows carries it, and
                    // its "no room yet" check must still fire.
                    AppState::JoinLobbyForm {
                        pending: Some(JoinScreenRequest::Join),
                        ..
                    } => {
                        *app_state = AppState::Lobby(lobby);
                    }
                    _ => {}
                }
            }
            ServerMessage::PawnPlaced {
                room_id: _,
                player_id,
                position,
            } => {
                if let AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) = app_state {
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        eprintln!("Failed to sync pawn placement: {:?}", e);
                    }
                }
            }
            ServerMessage::GameStarted {
                room_id,
                game,
                hand_counts,
                turn_deadline_secs,
                ..
            } => {
                // Start the first turn's countdown (None for untimed rooms).
                *turn_deadline =
                    turn_deadline_secs.map(|s| Instant::now() + std::time::Duration::from_secs(s));

                // Get lobby name from current lobby state
                let lobby_name = match app_state {
                    AppState::Lobby(lobby) | AppState::LobbyPlacingFor(lobby, _) => {
                        lobby.name.clone()
                    }
                    _ => "Game".to_string(), // Fallback name
                };

                // Transition to online game mode with server's authoritative game state
                *app_state = AppState::OnlineGame {
                    game,
                    room_id,
                    lobby_name,
                    waiting_for_server: false,
                    hand_counts,
                    is_spectator: false,
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

    /// Connect to the server if not already connected. On failure, surface a
    /// toast and return false.
    fn ensure_server_connection(&mut self) -> bool {
        if self.game_client.is_some() {
            return true;
        }
        let ws_url = self.server_url.trim().to_string();
        if ws_url.is_empty() {
            self.last_error = Some((
                "No server URL set — enter one on the main menu.".to_string(),
                Instant::now(),
            ));
            return false;
        }
        match GameClient::connect(&ws_url, self.ws_wakeup()) {
            Ok(client) => {
                self.game_client = Some(client);
                true
            }
            Err(e) => {
                eprintln!("Failed to connect to server: {}", e);
                self.last_error =
                    Some((format!("Couldn't connect to server: {e}"), Instant::now()));
                false
            }
        }
    }

    /// Ask the server for its public lobby directory (the response updates
    /// the join screen's browser).
    fn request_lobby_list(&mut self) {
        if !self.ensure_server_connection() {
            return;
        }
        if let Some(sender) = &self.sender {
            crate::messaging::send_server_message(sender, ClientMessage::ListLobbies);
        }
    }

    fn handle_show_create_lobby_form(&mut self) {
        self.app_state = AppState::CreateLobbyForm {
            lobby_name: "Test Lobby".to_string(),
            player_name: "Player 1".to_string(),
            public: true,
            turn_timer_secs: None,
        };
    }

    fn handle_show_join_lobby_form(&mut self) {
        self.app_state = AppState::JoinLobbyForm {
            lobby_id: String::new(),
            player_name: "Player 1".to_string(),
            pending: None,
            lobbies: Vec::new(),
        };
        // Populate the public-lobby browser right away.
        self.request_lobby_list();
    }

    fn handle_create_and_join_lobby(
        &mut self,
        lobby_name: String,
        player_name: String,
        visibility: Visibility,
        turn_timer_secs: Option<u64>,
    ) {
        if self.ensure_server_connection() {
            // Send create room message via mpsc
            if let Some(sender) = &self.sender {
                crate::messaging::send_server_message(
                    sender,
                    ClientMessage::CreateRoom {
                        room_name: lobby_name.clone(),
                        creator_name: player_name.clone(),
                        visibility,
                        turn_timer_secs,
                    },
                );
            }

            let (mut lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
            lobby.visibility = visibility;
            lobby.turn_timer_secs = turn_timer_secs;
            self.current_player_id = player_id;
            self.app_state = AppState::Lobby(lobby);
        } else {
            // Fall back to an offline lobby; the connect failure is already
            // toasted by ensure_server_connection.
            self.last_error = Some((
                "Couldn't reach the server, so a local lobby was started instead.".to_string(),
                Instant::now(),
            ));
            let (lobby, player_id) = Lobby::new_with_creator(lobby_name, player_name);
            self.current_player_id = player_id;
            self.app_state = AppState::Lobby(lobby);
        }
    }

    fn handle_join_lobby_with_id(&mut self, lobby_id: String, player_name: String) {
        if !self.ensure_server_connection() {
            // Stay on the join form so the user can retry; the failure is toasted.
            return;
        }

        // Send join room message via mpsc
        if let Some(sender) = &self.sender {
            crate::messaging::send_server_message(
                sender,
                ClientMessage::JoinRoom {
                    room_id: lobby_id.clone(),
                    player_name: player_name.clone(),
                },
            );
        }

        // Stay on the form with a spinner until the server confirms
        // the join (or rejects it, which re-enables the form).
        if let AppState::JoinLobbyForm { pending, .. } = &mut self.app_state {
            *pending = Some(JoinScreenRequest::Join);
        }
    }

    fn handle_spectate_lobby(&mut self, room_id: String, room_name: String) {
        if !self.ensure_server_connection() {
            return;
        }

        if let Some(sender) = &self.sender {
            crate::messaging::send_server_message(
                sender,
                ClientMessage::SpectateRoom {
                    room_id: room_id.clone(),
                },
            );
        }

        // The server answers with the game state; that transition happens in
        // handle_server_message when it arrives.
        if let AppState::JoinLobbyForm { pending, .. } = &mut self.app_state {
            *pending = Some(JoinScreenRequest::Spectate { room_id, room_name });
        }
    }

    fn handle_back_to_main_menu(&mut self) {
        match &self.app_state {
            AppState::LobbyPlacingFor(lobby, _) => {
                self.app_state = AppState::Lobby(lobby.clone());
            }
            _ => {
                // Leaving for the main menu ends any online session; a stale
                // connection would make a later local lobby look online.
                self.game_client = None;
                self.current_room_id = None;
                self.app_state = AppState::MainMenu;
            }
        }
    }

    /// Render the transient error toast, if one is active. Auto-expires after
    /// `ERROR_TOAST_DURATION` and can be dismissed early with the ✕ button.
    fn render_error_toast(&mut self, ctx: &Context) {
        let Some((message, shown_at)) = &self.last_error else {
            return;
        };

        // Auto-expire once the toast has been on screen long enough.
        let remaining = ERROR_TOAST_DURATION.checked_sub(shown_at.elapsed());
        let Some(remaining) = remaining else {
            self.last_error = None;
            return;
        };

        let message = message.clone();
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("error_toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -16.0))
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgb(120, 30, 30))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 90, 90)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::WHITE, format!("⚠ {message}"));
                            if ui.button("✕").on_hover_text("Dismiss").clicked() {
                                dismiss = true;
                            }
                        });
                    });
            });

        if dismiss {
            self.last_error = None;
        } else {
            // Keep repainting so the toast expires on time even when the app is
            // otherwise idle (e.g. a connect failure with no active game loop).
            ctx.request_repaint_after(remaining);
        }
    }

    /// Tear down the WebSocket connection and return to the main menu. Used by the
    /// fail-closed disconnect modal: there's no resume, so we drop all online state.
    fn disconnect_and_return_to_menu(&mut self) {
        self.game_client = None;
        self.current_room_id = None;
        self.player_animations.clear();
        self.tile_placement_animation = None;
        self.app_state = AppState::MainMenu;
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

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(
                &format!(
                    "handle_place_pawn: is_online={}, sender={}, room_id={}",
                    is_online,
                    self.sender.is_some(),
                    self.current_room_id.is_some()
                )
                .into(),
            );
        }

        match &mut self.app_state {
            AppState::Lobby(lobby) => {
                let player_id = self.current_player_id;

                // For online lobbies, send to server instead of handling locally
                if is_online {
                    if let (Some(sender), Some(room_id)) = (&self.sender, &self.current_room_id) {
                        #[cfg(target_arch = "wasm32")]
                        {
                            web_sys::console::log_1(
                                &format!(
                                    "Sending PlacePawn to server: room={}, player={}, pos={:?}",
                                    room_id, player_id, position
                                )
                                .into(),
                            );
                        }
                        crate::messaging::send_server_message(
                            sender,
                            ClientMessage::PlacePawn {
                                room_id: room_id.clone(),
                                player_id,
                                position,
                            },
                        );
                    } else {
                        #[cfg(target_arch = "wasm32")]
                        {
                            web_sys::console::log_1(
                                &"PlacePawn NOT sent: sender or room_id is None".into(),
                            );
                        }
                    }
                } else {
                    // Local lobby - handle locally
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        self.last_error =
                            Some((format!("Can't place the pawn there: {e}"), Instant::now()));
                    }
                }
            }
            AppState::LobbyPlacingFor(lobby, placing_for_id) => {
                let player_id = *placing_for_id;

                // For online lobbies, send to server
                if is_online {
                    if let (Some(sender), Some(room_id)) = (&self.sender, &self.current_room_id) {
                        crate::messaging::send_server_message(
                            sender,
                            ClientMessage::PlacePawn {
                                room_id: room_id.clone(),
                                player_id,
                                position,
                            },
                        );
                        // Transition back to regular lobby view
                        self.app_state = AppState::Lobby(lobby.clone());
                    }
                } else {
                    // Local lobby - handle locally
                    if let Err(e) = lobby.handle_event(LobbyEvent::PawnPlaced {
                        player_id,
                        position,
                    }) {
                        self.last_error =
                            Some((format!("Can't place the pawn there: {e}"), Instant::now()));
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
                    crate::messaging::send_server_message(
                        sender,
                        ClientMessage::StartGame {
                            room_id: room_id.clone(),
                        },
                    );
                    // Server will send GameStarted message to all clients
                }
                return;
            }

            // Local lobby - handle locally
            if let Err(e) = lobby.handle_event(LobbyEvent::StartGame) {
                self.last_error = Some((format!("Can't start the game: {e}"), Instant::now()));
                return;
            }

            match lobby.to_game() {
                Ok(game) => {
                    // Local game - client is authoritative
                    self.app_state = AppState::Game(game);
                }
                Err(e) => {
                    self.last_error = Some((format!("Can't start the game: {e}"), Instant::now()));
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
            let mut unplaced_players: Vec<PlayerID> = lobby
                .players
                .iter()
                .filter(|(_, p)| p.spawn_position.is_none())
                .map(|(id, _)| *id)
                .collect();
            unplaced_players.sort();

            if !unplaced_players.is_empty() {
                let current_idx = unplaced_players
                    .iter()
                    .position(|id| id == current_placing_id)
                    .unwrap_or(0);

                let new_idx = if next {
                    (current_idx + 1) % unplaced_players.len()
                } else if current_idx == 0 {
                    unplaced_players.len() - 1
                } else {
                    current_idx - 1
                };

                let new_player_id = unplaced_players[new_idx];
                self.app_state = AppState::LobbyPlacingFor(lobby.clone(), new_player_id);
            }
        }
    }

    fn handle_tile_rotated(&mut self, tile_index: usize, clockwise: bool) {
        let online_player_id = self.current_player_id;

        // Rotate the hand that's actually on screen: the current player's in a
        // local hot-seat game, or our own (fixed identity) in an online game.
        // Placement reads the same hand, so display and placement stay in sync.
        let (game, hand_owner) = match &mut self.app_state {
            AppState::Game(game) => {
                let owner = game.current_player_id;
                (game, owner)
            }
            AppState::OnlineGame { game, .. } => (game, online_player_id),
            _ => return,
        };

        if let Some(hand) = game.hands.get_mut(&hand_owner) {
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
                if let AppState::Game(game) =
                    std::mem::replace(&mut self.app_state, AppState::MainMenu)
                {
                    let (game, placed) = Self::perform_local_tile_placement(game, tile_index);

                    match placed {
                        Ok(cell) => {
                            self.start_player_animations(&game);
                            self.start_tile_placement_animation(cell);
                        }
                        Err(e) => {
                            self.last_error =
                                Some((format!("Can't place that tile: {e}"), Instant::now()));
                        }
                    }

                    // Stay in Game state even if game is over (overlay will show stats)
                    self.app_state = AppState::Game(game);
                }
            }

            // Online game: send to server and wait for response (server authoritative)
            AppState::OnlineGame {
                game,
                room_id,
                waiting_for_server,
                is_spectator,
                ..
            } => {
                // Spectators have no hand; nothing to place.
                if *is_spectator {
                    return;
                }

                if *waiting_for_server {
                    // Quietly ignore: the previous move is still in flight and
                    // the top panel already shows the waiting indicator.
                    return;
                }

                // Only allow placing tiles when it's this client's turn
                if game.current_player_id != self.current_player_id {
                    self.last_error = Some(("It's not your turn yet.".to_string(), Instant::now()));
                    return;
                }

                let player_cell = game
                    .players
                    .iter()
                    .find(|p| p.id == self.current_player_id && p.alive)
                    .expect("current player should exist and be alive")
                    .pos
                    .cell;

                // Get the client's own hand (not the current player's hand)
                let hand = game
                    .hands
                    .get(&self.current_player_id)
                    .expect("client should have a hand");

                if tile_index >= hand.len() {
                    eprintln!(
                        "Invalid tile index: {} (hand size: {})",
                        tile_index,
                        hand.len()
                    );
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
                    crate::messaging::send_server_message(
                        sender,
                        ClientMessage::PlaceTile {
                            room_id: room_id.clone(),
                            player_id: self.current_player_id,
                            mov,
                        },
                    );
                    *waiting_for_server = true;
                }
            }

            _ => {}
        }
    }

    /// Perform a tile placement in a local game (client authoritative).
    /// Returns the updated game and the placed cell, or the engine's
    /// rejection message for the caller to surface.
    fn perform_local_tile_placement(
        mut game: Game,
        tile_index: usize,
    ) -> (Game, Result<tsurust_common::board::CellCoord, String>) {
        let player_cell = game
            .players
            .iter()
            .find(|p| p.id == game.current_player_id && p.alive)
            .expect("current player should exist and be alive")
            .pos
            .cell;

        let hand = game
            .hands
            .get(&game.current_player_id)
            .expect("current player should always have a hand");

        let tile = hand[tile_index];

        let mov = Move {
            tile,
            cell: player_cell,
            player_id: game.current_player_id,
        };

        match game.perform_move(mov) {
            Ok(_turn_result) => (game, Ok(player_cell)),
            Err(error) => (game, Err(error.to_string())),
        }
    }

    fn start_tile_placement_animation(&mut self, cell: tsurust_common::board::CellCoord) {
        self.tile_placement_animation = Some(TilePlacementAnimation {
            cell,
            progress: 0.0,
            start_time: Instant::now(),
            duration_secs: animation::TILE_PLACEMENT_DURATION_SECS,
        });
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
            self.app_state = AppState::Game(game.clone());
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
                        eprintln!(
                            "Make sure to build the server first with: cargo build --bin server"
                        );

                        let error_msg =
                            "Binary not found. Run 'cargo build --bin server' first.".to_string();
                        self.local_server_status = LocalServerStatus::Failed(error_msg);
                    }
                }
            }
        }
    }

    fn handle_send_to_server(&mut self, client_msg: ClientMessage) {
        if let Some(client) = &mut self.game_client {
            println!("[CLIENT->SERVER] Sending: {:?}", client_msg);
            client.send(client_msg);
        } else {
            eprintln!("Cannot send message to server: not connected");
        }
    }

    // ========== Export/Import Handlers ==========

    fn handle_export_game(&mut self) {
        use tsurust_common::game::{GameMetadata, GameMode};

        let (game, metadata) = match &self.app_state {
            AppState::Game(game) => {
                let metadata = GameMetadata {
                    game_mode: GameMode::Local,
                    room_id: None,
                    room_name: None,
                    completed: game.is_game_over(),
                    winner_id: if game.is_game_over() {
                        game.players.iter().find(|p| p.alive).map(|p| p.id)
                    } else {
                        None
                    },
                    total_turns: game.board.history.len(),
                    player_names: game
                        .players
                        .iter()
                        .map(|p| (p.id, p.name.clone()))
                        .collect(),
                };
                (game, metadata)
            }
            AppState::OnlineGame {
                game,
                room_id,
                lobby_name,
                ..
            } => {
                let metadata = GameMetadata {
                    game_mode: GameMode::Online,
                    room_id: Some(room_id.clone()),
                    room_name: Some(lobby_name.clone()),
                    completed: game.is_game_over(),
                    winner_id: if game.is_game_over() {
                        game.players.iter().find(|p| p.alive).map(|p| p.id)
                    } else {
                        None
                    },
                    total_turns: game.board.history.len(),
                    player_names: game
                        .players
                        .iter()
                        .map(|p| (p.id, p.name.clone()))
                        .collect(),
                };
                (game, metadata)
            }
            _ => {
                eprintln!("Cannot export: not in a game");
                return;
            }
        };

        let export = game.export(metadata, Some(self.current_player_id));
        crate::file_io::save_game_export(&export);
    }

    /// Open the replay viewer for a loaded export, or surface a toast if the
    /// file can't be replayed (e.g. its history doesn't replay through the
    /// engine, as happens when a disconnect force-advanced the turn).
    fn open_replay(&mut self, export: tsurust_common::game::GameExport) {
        match crate::replay_state::ReplayState::new(export) {
            Ok(replay_state) => {
                let current_game = replay_state.current_game_state();
                self.app_state = AppState::ReplayViewer {
                    replay_state,
                    current_game,
                };
            }
            Err(e) => {
                self.last_error = Some((e, Instant::now()));
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_import_replay(&mut self) {
        if let Some(export) = crate::file_io::load_game_export() {
            self.open_replay(export);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn handle_import_replay(&mut self) {
        // For WASM, we use a callback that sends a message when the file is loaded
        let sender = match &self.sender {
            Some(s) => s.clone(),
            None => {
                web_sys::console::error_1(&"No sender available for replay import".into());
                return;
            }
        };

        crate::file_io::load_game_export(move |export| {
            // Send the loaded export via the message channel
            if let Err(e) = sender.send(Message::ReplayLoaded(Box::new(export))) {
                web_sys::console::error_1(
                    &format!("Failed to send ReplayLoaded message: {}", e).into(),
                );
            }
        });
    }

    // ========== Replay Control Handlers ==========

    fn handle_replay_play(&mut self) {
        if let AppState::ReplayViewer { replay_state, .. } = &mut self.app_state {
            replay_state.play();
        }
    }

    fn handle_replay_pause(&mut self) {
        if let AppState::ReplayViewer { replay_state, .. } = &mut self.app_state {
            replay_state.pause();
        }
    }

    fn handle_replay_step_forward(&mut self) {
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.step_forward() {
                *current_game = new_game;
            }
        }
    }

    fn handle_replay_step_backward(&mut self) {
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.step_backward() {
                *current_game = new_game;
            }
        }
    }

    fn handle_replay_set_speed(&mut self, speed: f32) {
        if let AppState::ReplayViewer { replay_state, .. } = &mut self.app_state {
            replay_state.set_speed(speed);
        }
    }

    fn handle_replay_jump_to_move(&mut self, index: usize) {
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.set_move_index(index) {
                *current_game = new_game;
            }
        }
    }

    fn handle_replay_jump_to_start(&mut self) {
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.jump_to_start() {
                *current_game = new_game;
            }
        }
    }

    fn handle_replay_jump_to_end(&mut self) {
        if let AppState::ReplayViewer {
            replay_state,
            current_game,
        } = &mut self.app_state
        {
            if let Some(new_game) = replay_state.jump_to_end() {
                *current_game = new_game;
            }
        }
    }

    fn handle_exit_replay(&mut self) {
        self.app_state = AppState::MainMenu;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsurust_common::board::{seg, Player, Tile};

    /// URL-based room joining: `/ABCD` or `#ABCD` should yield a normalized
    /// room id; anything else should be ignored.
    #[test]
    fn room_ids_are_parsed_from_url_path_or_fragment() {
        assert_eq!(room_id_from_location("/ABCD", ""), Some("ABCD".into()));
        assert_eq!(room_id_from_location("/abcd/", ""), Some("ABCD".into()));
        assert_eq!(room_id_from_location("/", "#WXYZ"), Some("WXYZ".into()));
        // The fragment wins over the path when both are present.
        assert_eq!(room_id_from_location("/ABCD", "#WXYZ"), Some("WXYZ".into()));
        assert_eq!(room_id_from_location("/", ""), None);
        assert_eq!(room_id_from_location("/index.html", ""), None);
        assert_eq!(room_id_from_location("/toolong", "#nope!"), None);
    }

    /// A `?server=` invite param yields the (percent-decoded) server URL; when
    /// it's absent we return None so the config/default fallback takes over.
    #[test]
    fn server_url_is_parsed_from_query() {
        assert_eq!(
            server_url_from_query("?server=wss://host.example:8080"),
            Some("wss://host.example:8080".into())
        );
        // Percent-encoded (the shape produced by encodeURIComponent).
        assert_eq!(
            server_url_from_query("?server=wss%3A%2F%2Fabc123.trycloudflare.com"),
            Some("wss://abc123.trycloudflare.com".into())
        );
        // Coexists with other params (e.g. a room id) in any order.
        assert_eq!(
            server_url_from_query("?room=ABCD&server=wss://h"),
            Some("wss://h".into())
        );
        assert_eq!(server_url_from_query("?room=ABCD"), None);
        assert_eq!(server_url_from_query(""), None);
        // Present but empty is treated as absent.
        assert_eq!(server_url_from_query("?server="), None);
    }

    #[test]
    fn percent_decode_handles_escapes_and_stray_percent() {
        assert_eq!(percent_decode("wss%3A%2F%2Fh"), "wss://h");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("plain"), "plain");
        // A truncated escape is passed through rather than dropped.
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("bad%zz"), "bad%zz");
    }

    /// Local tile rotations are presentation-only and must survive a
    /// GameStateUpdate; the server's (unrotated) tiles would otherwise snap
    /// rotated tiles back after every opponent move.
    #[test]
    fn local_rotations_survive_game_state_updates() {
        let me: PlayerID = 1;
        let mut server_game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        // Pin a rotation-asymmetric tile into my first slot on both sides.
        let known = Tile::new([seg(0, 2), seg(1, 3), seg(4, 6), seg(5, 7)]);
        server_game.hands.get_mut(&me).expect("my hand")[0] = known;

        let rotated = known.rotated(true);
        assert_ne!(rotated, known, "test tile must be rotation-asymmetric");

        // The client's copy has the tile rotated for planning.
        let mut client_game = server_game.clone();
        client_game.hands.get_mut(&me).expect("my hand")[0] = rotated;

        let mut app_state = AppState::OnlineGame {
            game: client_game,
            room_id: "ROOM".to_string(),
            lobby_name: "Room".to_string(),
            waiting_for_server: true,
            hand_counts: server_game.hand_counts(),
            is_spectator: false,
        };

        TemplateApp::handle_server_message(
            ServerMessage::game_state_update("ROOM".to_string(), &server_game),
            &mut Some("ROOM".to_string()),
            &mut { me },
            &mut app_state,
            &mut std::collections::HashMap::new(),
            &mut None,
            &mut None,
            &mut None,
        );

        let AppState::OnlineGame { game, .. } = &app_state else {
            panic!("should stay in the online game");
        };
        assert_eq!(
            game.hands[&me][0], rotated,
            "my local rotation should survive the server update"
        );
        assert_eq!(
            game.hands[&2], server_game.hands[&2],
            "the opponent's hand comes straight from the server"
        );
    }
}
