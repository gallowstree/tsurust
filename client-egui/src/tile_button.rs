use eframe::egui::{vec2, Frame, Rect, Sense, Widget};

use tsurust_common::board::*;

use crate::rendering::{paint_tile, paint_tile_button_hoverlay, tile_to_screen_transform};

pub struct TileButton<'a> {
    tile: &'a mut Tile,
}
impl<'a> TileButton<'a> {
    pub fn new(tile: &'a mut Tile) -> Self {
        Self { tile }
    }
}
impl<'a> Widget for TileButton<'a> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(120.0, 120.0), Sense::click().union(Sense::hover()));

        let to_screen = tile_to_screen_transform(rect);
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let pos = to_screen.inverse().transform_pos(pos);
                if pos.x < 1. {
                    *self.tile = self.tile.rotated(false)
                } else if pos.x > 2. {
                    *self.tile = self.tile.rotated(true)
                }
            }
        }

        Frame::canvas(ui.style()).show(ui, |ui| {
            let painter = ui.painter();
            let rect = response.rect;
            if response.hovered() {
                paint_tile_button_hoverlay(rect, painter);
            }
            paint_tile(
                self.tile,
                Rect::from_center_size(rect.center(), vec2(119., 119.)),
                painter,
            );
        });
        response
    }
}
