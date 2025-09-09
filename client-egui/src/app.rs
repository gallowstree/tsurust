use eframe::egui;
use eframe::epaint::Color32;
use egui::{Context, ScrollArea, Visuals};

use tsurust_common::board::*;
use tsurust_common::game::Game;
use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use crate::tile_button::TileButton;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    #[serde(skip)]
    game: tsurust_common::game::Game,
    current_player: PlayerID
}

impl Default for TemplateApp {
    fn default() -> Self {
        let current_player = 1;
        let mut game = Game::new(vec![Player{alive: true, id: current_player, pos: PlayerPos::new(0, 0, 7)}]);
        let mut random_tiles = game.deck.take_up_to(36);

        let t = game.hands.get_mut(&current_player).expect("hand").append(&mut random_tiles);

        Self {
            label: "Hello Year of the Dragon of Wood - Hello Tsurust!".to_owned(),
            game,
            current_player
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

    fn render_ui(ctx: &Context, game: &mut Game) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("🐉🐉[server: local - room 01 - room host: alyosha] 🐉🐉");
                        ui.heading("🐉🐉 [turn 1 (alyosha) - tiles left: 0 - ] 🐉🐉");
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
            ui.add(HandRenderer::new(hand));
        });
    }
}



impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let Self { label, game , current_player} = self;

        Self::render_ui(ctx, game);
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
