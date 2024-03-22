use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::Vec2;
use eframe::epaint::{Color32, Stroke};
use tsurust_common::board::*;

use crate::rendering::{paint_tile, PINK};

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
        let tile_length: f32 = 110.0;

        let (rect, response) = ui.allocate_at_least(
            vec2(rows * tile_length, cols * tile_length),
            Sense::click().union(Sense::hover())
        );
        
        background(ui, rect);

        ui.vertical_centered(|ui| {
            tiles(ui, self.history, rect);
        });

        response
    }
}

fn tiles(ui: &mut Ui, history: &Vec<Move> ,rect: Rect) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let painter = ui.painter();
        let rect = rect;
        let size = Rect::from_center_size(rect.center(), Vec2::new(110., 110.));

        let tiles = history
            .iter()
            .map(|mov| mov.tile);

        for tile in tiles {
            paint_tile(&tile, size, painter);
        }
    });

}

fn background(ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.6, Color32::BLACK);
    ui.painter().rect_stroke(rect, 0.5, Stroke::new(2.0, PINK));

    //rate::backgr_render::draw_yin_yang(ui, 12.10);
}