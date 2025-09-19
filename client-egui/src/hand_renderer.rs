use egui::{ScrollArea, Widget};
use std::sync::mpsc;
use tsurust_common::board::*;
use crate::tile_button::TileButton;
use crate::app::Message;

pub struct HandRenderer {
    tiles: Vec<Tile>,
    sender: mpsc::Sender<Message>,
}

impl HandRenderer {
    pub fn new(tiles: Vec<Tile>, sender: mpsc::Sender<Message>) -> Self {
        Self { tiles, sender }
    }
}

impl Widget for HandRenderer {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical_centered(|ui| {
            ScrollArea::vertical()
                .show(ui, |ui| {
                    for (index, tile) in self.tiles.iter().enumerate() {
                        ui.add_space(10.);
                        let button = TileButton::new(*tile, index, self.sender.clone());
                        ui.add(button);
                    }
                });
        }).response
    }
}