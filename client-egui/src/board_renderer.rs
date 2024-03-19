use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::Vec2;
use eframe::epaint::Stroke;
use egui::Pos2;

use tsurust_common::board::*;

use crate::rendering::{paint_tile, TRANSPARENT_GOLD};

pub struct BoardRenderer<'a> {
    history: &'a mut Vec<Move> //to-do, alias this type
}

impl <'a> BoardRenderer<'a> {
    pub(crate) fn new(history: &'a mut Vec<Move>) -> Self {
        Self { history }
    }
}

impl Widget for BoardRenderer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rows, cols) = (6.,6.);
        let tile_length: f32 = 120.0;

        let (rect, response) = ui.allocate_exact_size(
            vec2(rows * tile_length, cols * tile_length),
            Sense::click().union(Sense::hover())
        );

        ui.painter().rect_stroke(rect, 0.5, Stroke::new(2.0, TRANSPARENT_GOLD));

        Frame::canvas(ui.style()).show(ui, |ui| {
            let painter = ui.painter();
            let rect = response.rect;
            let size = Rect::from_min_size(Pos2::ZERO, Vec2::new(tile_length, tile_length));

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