use eframe::egui;
use eframe::epaint::Color32;
use egui::Visuals;

use tsurust_common::board::*;
use tsurust_common::game::Game;
use crate::board_renderer::BoardRenderer;
use crate::tile_button::TileButton;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    label: String,
    #[serde(skip)]
    tile: Tile,
    #[serde(skip)]
    game: tsurust_common::game::Game,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut game = Game::new(vec![Player{alive: true, id: 1, pos: PlayerPos::new(0, 0, 0)}]);
        let random_tiles = game.deck.take_up_to(36);

        random_tiles.iter()
            .enumerate()
            .for_each(|(i, tile)| {
                let (row, col) = (i/6, i % 6);
                let coord = CellCoord {row, col};
                let tile = tile.clone();
                game.perform_move(Move {tile, cell: coord ,player_id: 1})
            });


        Self {
            label: "Hello Year of the Dragon of Wood - Hello Tsurust!".to_owned(),
            tile: Tile::new([seg(0, 2), seg(1, 4), seg(3, 5), seg(6, 7)]),
            game
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
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let Self { label, tile, game } = self;

        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("🐉🐉[server: local - room 01 - room host: alyosha] 🐉🐉");
                        ui.heading("🐉🐉 [turn 1 (alyosha) - tiles left: 0 - ] 🐉🐉");
                        ui.heading("(alyosha) [Automat] [Pig] [Rooster] [Dragon]");
                    });
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(20.);
                    ui.add(TileButton::new(tile));
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(BoardRenderer::new(&mut self.game.board.history, &mut self.game.players));
            });
        });
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
