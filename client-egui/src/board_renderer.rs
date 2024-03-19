use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::Numeric;

use tsurust_common::board::*;

use crate::rendering::{paint_tile, paint_tile_button_hoverlay, tile_to_screen_transform};

pub struct BoardRenderer {
}

impl Widget for BoardRenderer {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rows, cols) = (6.,6.);
        let tile_length: f32 = 120.0;
        let (rect, response) =
            ui.allocate_exact_size(
                vec2(rows * tile_length, cols * tile_length),
                Sense::click().union(Sense::hover())
            );

        response




    }
}