use eframe::egui;
use egui::Context;
use std::sync::mpsc;

use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use crate::player_card::PlayerCard;
use tsurust_common::board::*;
use tsurust_common::game::{Game, TurnResult};

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),                // tile index - place at current player position
    TileRotated(usize, bool),         // tile index, clockwise
    RestartGame,                      // restart the game
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    #[serde(skip)]
    game: tsurust_common::game::Game,
    #[serde(skip)]
    sender: Option<mpsc::Sender<Message>>,
    #[serde(skip)]
    receiver: Option<mpsc::Receiver<Message>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        // Create 4 players starting at different board edges (only edge endpoints are valid)
        let players = vec![
            Player::new(1, PlayerPos::new(0, 2, 4)),  // Player 1: top edge (row=0), endpoint 4 or 5
            Player::new(2, PlayerPos::new(2, 5, 2)),  // Player 2: right edge (col=5), endpoint 2 or 3
            Player::new(3, PlayerPos::new(5, 3, 0)),  // Player 3: bottom edge (row=5), endpoint 0 or 1
            Player::new(4, PlayerPos::new(3, 0, 6)),  // Player 4: left edge (col=0), endpoint 6 or 7
        ];

        let game = Game::new(players);

        // Each player starts with 3 tiles (normal hand size)
        // Don't add extra tiles - Game::new already gives each player 3 tiles

        let (sender, receiver) = mpsc::channel();

        Self {
            label: "Hello Year of the Dragon of Wood - Hello Tsurust!".to_owned(),
            game,
            sender: Some(sender),
            receiver: Some(receiver),
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

    fn render_ui(ctx: &Context, game: &mut Game, sender: &mpsc::Sender<Message>) {
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
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(20.);
                ui.add(BoardRenderer::new(&game.board.history, &game.players, &game.tile_trails, &game.player_trails));
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
        let Self { label: _, game, sender, receiver } = self;

        // Process received messages
        if let Some(rx) = receiver {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::TileRotated(tile_index, clockwise) => {
                        let hand = game.hands.get_mut(&game.current_player_id).expect("current player should always have a hand");
                        hand[tile_index] = hand[tile_index].rotated(clockwise);
                    }
                    Message::TilePlaced(tile_index) => {
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
                    Message::RestartGame => {
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

        // Render UI with sender
        if let Some(tx) = sender {
            Self::render_ui(ctx, game, tx);
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
