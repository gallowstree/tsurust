use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};

use tsurust_common::board::*;

use crate::rendering::{board_to_screen_transform, paint_tile};

pub struct BoardRenderer<'a> {
    history: &'a mut Vec<Move> //to-do, alias this type
}

impl Widget for BoardRenderer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rows, cols) = (6.,6.);
        let tile_length: f32 = 120.0;

        let (rect, response) = ui.allocate_exact_size(
            vec2(rows * tile_length, cols * tile_length),
            Sense::click().union(Sense::hover())
        );

        let to_screen = board_to_screen_transform(rect);

        Frame::canvas(ui.style()).show(ui, |ui| {
            let painter = ui.painter();
            let rect = response.rect;
            let size = Rect::from_center_size(rect.center(), vec2(119., 119.));

            let tiles = self.history
                .iter()
                .map(|mov| mov.tile);

            for tile in tiles {
                paint_tile(&tile, size, painter);
            }
        });

        response
    }
}