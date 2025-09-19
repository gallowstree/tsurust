use eframe::egui;
use egui::{Context, ScrollArea};
use std::sync::mpsc;

use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use tsurust_common::board::*;
use tsurust_common::game::Game;

#[derive(Debug, Clone)]
pub enum Message {
    TilePlaced(usize),                // tile index - place at current player position
    TileRotated(usize, bool),         // tile index, clockwise
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    #[serde(skip)]
    game: tsurust_common::game::Game,
    current_player: PlayerID,
    #[serde(skip)]
    sender: Option<mpsc::Sender<Message>>,
    #[serde(skip)]
    receiver: Option<mpsc::Receiver<Message>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let current_player = 1;
        let mut game = Game::new(vec![Player{alive: true, id: current_player, pos: PlayerPos::new(0, 0, 7)}]);
        let mut random_tiles = game.deck.take_up_to(36);

        let t = game.hands.get_mut(&current_player).expect("hand").append(&mut random_tiles);

        let (sender, receiver) = mpsc::channel();

        Self {
            label: "Hello Year of the Dragon of Wood - Hello Tsurust!".to_owned(),
            game,
            current_player,
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
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("[server: local - room 01 - room host: alyosha] ");
                        ui.heading(" [turn 1 (alyosha) - tiles left: 0 - ] ");
                        ui.heading("(alyosha) [Automat] [Pig] [Rooster] [Dragon]");
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(20.);
                ui.add(BoardRenderer::new(&mut game.board.history, &mut game.players));
            });
        });

        egui::SidePanel::right("right_panel").show(ctx, |ui| {
            let hand = game.curr_player_hand().clone();
            ui.add(HandRenderer::new(hand, sender.clone()));
        });
    }
}



impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let Self { label: _, game, current_player, sender, receiver } = self;

        // Process received messages
        if let Some(rx) = receiver {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::TileRotated(tile_index, clockwise) => {
                        let hand = game.hands.get_mut(current_player).expect("current player should always have a hand");
                        hand[tile_index] = hand[tile_index].rotated(clockwise);
                    }
                    Message::TilePlaced(tile_index) => {
                        let player_cell = game.players.iter()
                            .find(|p| p.id == *current_player && p.alive)
                            .expect("current player should exist and be alive")
                            .pos.cell;

                        let hand = game.hands.get(current_player)
                            .expect("current player should always have a hand");

                        let tile = hand[tile_index];

                        let mov = Move {
                            tile,
                            cell: player_cell,
                            player_id: *current_player,
                        };

                        match game.perform_move(mov) {
                            Ok(()) => {
                                println!("Tile placed successfully at {:?}!", player_cell);
                                println!("  Tile: {:?}", tile);
                                println!("  Player {} new position: {:?}", *current_player,
                                    game.players.iter().find(|p| p.id == *current_player)
                                        .expect("current player should exist").pos);
                            }
                            Err(error) => println!("Failed to place tile: {}", error),
                        }
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
