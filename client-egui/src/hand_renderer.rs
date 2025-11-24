use std::sync::mpsc;

use egui::{ScrollArea, Widget};

use tsurust_common::board::*;

use crate::app::Message;
use crate::tile_button::TileButton;

pub struct HandRenderer {
    tiles: Vec<Tile>,
    sender: mpsc::Sender<Message>,
    last_rotated_tile: Option<(usize, bool)>, // (tile_index, clockwise)
}

impl HandRenderer {
    pub fn new(tiles: Vec<Tile>, sender: mpsc::Sender<Message>) -> Self {
        Self {
            tiles,
            sender,
            last_rotated_tile: None,
        }
    }

    pub fn with_last_rotated(mut self, last_rotated: Option<(usize, bool)>) -> Self {
        self.last_rotated_tile = last_rotated;
        self
    }
}

impl Widget for HandRenderer {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        ui.vertical_centered(|ui| {
            ScrollArea::vertical()
                .show(ui, |ui| {
                    for (index, tile) in self.tiles.iter().enumerate() {
                        ui.add_space(10.);
                        let mut button = TileButton::new(*tile, index, self.sender.clone());

                        // Apply rotation animation if this tile was just rotated
                        if let Some((rotated_index, clockwise)) = self.last_rotated_tile {
                            if rotated_index == index {
                                button = button.with_rotation_animation(clockwise);
                            }
                        }

                        ui.add(button);
                    }
                });
        }).response
    }
}