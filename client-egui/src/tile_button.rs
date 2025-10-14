use std::sync::mpsc;

use eframe::egui::{vec2, Frame, Rect, Sense, Widget};

use tsurust_common::board::*;

use crate::app::Message;
use crate::messaging::send_message;
use crate::rendering::{paint_tile, paint_tile_button_hoverlay, paint_tile_button_hoverlay_with_highlight, tile_to_screen_transform};

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
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(140.0, 140.0), Sense::click().union(Sense::hover()));

        let to_screen = tile_to_screen_transform(rect);
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let pos = to_screen.inverse().transform_pos(pos);
                if pos.x < 1. {
                    // Left side clicked - rotate counterclockwise
                    send_message(&self.sender, Message::TileRotated(self.index, false));
                } else if pos.x > 2. {
                    // Right side clicked - rotate clockwise
                    send_message(&self.sender, Message::TileRotated(self.index, true));
                } else {
                    // Center clicked - place tile at current player position
                    send_message(&self.sender, Message::TilePlaced(self.index));
                }
            }
        }

        Frame::canvas(ui.style())
            .show(ui, |ui| {
                let painter = ui.painter();
                let rect = response.rect;

                // Draw tile first
                paint_tile(
                    &self.tile,
                    Rect::from_center_size(rect.center(), vec2(139., 139.)),
                    painter,
                );

                // Draw hover overlay on top so rotation indicators are visible
                if response.hovered() {
                    if let Some(pos) = response.hover_pos() {
                        let pos = to_screen.inverse().transform_pos(pos);
                        if pos.x < 1. {
                            // Hovering over left rotation area - show both buttons, highlight left
                            paint_tile_button_hoverlay_with_highlight(rect, painter, Some(false));
                        } else if pos.x > 2. {
                            // Hovering over right rotation area - show both buttons, highlight right
                            paint_tile_button_hoverlay_with_highlight(rect, painter, Some(true));
                        } else {
                            // Hovering over center placement area - show both buttons, no highlight
                            paint_tile_button_hoverlay_with_highlight(rect, painter, None);
                        }
                    } else {
                        paint_tile_button_hoverlay(rect, painter);
                    }
                }
            });
        response
    }
}
