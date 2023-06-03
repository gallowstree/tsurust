use eframe::egui::{Response, Ui};
use egui::{Button, Color32, emath::{RectTransform, Rot2}, Frame, Painter, pos2, Pos2, Rect, Sense, Stroke, vec2, Vec2, Widget};

use tsurust_common::board::*;
use crate::rendering::paint_tile;


pub struct TileButton {
    tile: Tile,
    selected: bool,
}

impl TileButton {
    pub fn new(tile: Tile) -> Self {
        Self {
            tile,
            selected: false,
        }
    }
}

impl Widget for TileButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(120.0, 120.0), Sense::click().union(Sense::hover()));

        Frame::canvas(ui.style()).show(ui, |ui| {
            let painter = ui.painter();
            let rect = response.rect;
            if response.hovered() {
                painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::GOLD));
            }
            paint_tile(self.tile, Rect::from_center_size(rect.center(), vec2(119., 119.)), painter);
        });
        response
    }
}