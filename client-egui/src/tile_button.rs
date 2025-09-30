use eframe::egui::{vec2, Frame, Rect, Sense, Widget};
use std::sync::mpsc;

use tsurust_common::board::*;
use crate::app::Message;

use crate::rendering::{paint_tile, paint_tile_button_hoverlay, paint_tile_button_hoverlay_left, paint_tile_button_hoverlay_right, paint_tile_button_hoverlay_center, tile_to_screen_transform};

pub struct TileButton {
    tile: Tile,
    index: usize,
    sender: mpsc::Sender<Message>,
}

impl TileButton {
    pub fn new(tile: Tile, index: usize, sender: mpsc::Sender<Message>) -> Self {
        Self { tile, index, sender }
    }
}

impl Widget for TileButton {
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(140.0, 140.0), Sense::click().union(Sense::hover()));

        let to_screen = tile_to_screen_transform(rect);
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let pos = to_screen.inverse().transform_pos(pos);
                if pos.x < 1. {
                    // Left side clicked - rotate counterclockwise
                    if let Err(e) = self.sender.send(Message::TileRotated(self.index, false)) {
                        eprintln!("Failed to send TileRotated message: {}", e);
                    }
                } else if pos.x > 2. {
                    // Right side clicked - rotate clockwise
                    if let Err(e) = self.sender.send(Message::TileRotated(self.index, true)) {
                        eprintln!("Failed to send TileRotated message: {}", e);
                    }
                } else {
                    // Center clicked - place tile at current player position
                    if let Err(e) = self.sender.send(Message::TilePlaced(self.index)) {
                        eprintln!("Failed to send TilePlaced message: {}", e);
                    }
                }
            }
        }

        Frame::canvas(ui.style())
            .show(ui, |ui| {
                let painter = ui.painter();
                let rect = response.rect;

                if response.hovered() {
                    if let Some(pos) = response.hover_pos() {
                        let pos = to_screen.inverse().transform_pos(pos);
                        if pos.x < 1. {
                            // Hovering over left rotation area - highlight left rotate button
                            paint_tile_button_hoverlay_left(rect, painter);
                        } else if pos.x > 2. {
                            // Hovering over right rotation area - highlight right rotate button
                            paint_tile_button_hoverlay_right(rect, painter);
                        } else {
                            // Hovering over center placement area - show normal hover
                            paint_tile_button_hoverlay_center(rect, painter);
                        }
                    } else {
                        paint_tile_button_hoverlay(rect, painter);
                    }
                }

                paint_tile(
                    &self.tile,
                    Rect::from_center_size(rect.center(), vec2(139., 139.)),
                    painter,
                );
            });
        response
    }
}
