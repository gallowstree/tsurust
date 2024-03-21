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
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[serde(skip)]
    value: f32,

    #[serde(skip)]
    tile: Tile,

    #[serde(skip)]
    game: tsurust_common::game::Game,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            tile: Tile::new([seg(0, 2), seg(1, 4), seg(3, 5), seg(6, 7)]),
            game: Game::new(vec![Player{alive: true, id: 1, pos: PlayerPos::new(0,0, 0)}])
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let Self { label, value, tile, game } = self;

        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("🐉🐉[server: local - room #1 - room host: alyosha] 🐉🐉");
                        ui.heading("🐉🐉 [turn #:1 - tiles left: 0 - ] 🐉🐉");
                        ui.heading("[Automat] [Pig] [Rooster] [Dragon]");
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
                let cell = CellCoord {row:2, col: 3};
                let tile = Tile::new(
                    [seg(0, 7), seg(5, 4), seg(3, 2), seg(6, 1)]);
                ui.add(BoardRenderer::new(
                    &mut vec![Move {tile, cell, player_id: 1}]
                ));
            });
        });
    }



    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
