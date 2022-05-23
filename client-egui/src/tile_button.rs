use tsurust_common::board::Tile;

use egui::{emath::{RectTransform, Rot2}, vec2, Color32, Frame, Pos2, Rect, Sense, Stroke, Vec2, Widget, pos2};
use eframe::egui::{Ui, Response};

pub struct TileButton {
    //tile: Tile
}

impl TileButton {
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget for TileButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(120.0, 120.0), Sense::click().union(Sense::hover()));

        Frame::canvas(ui.style()).show(ui, |ui| {
            //ui.ctx().request_repaint();
            let painter = ui.painter();

            // normalize painter coordinatesfrom [0,0] to [1,1]
            let painter_proportions = response.rect.square_proportions();
            let to_screen = RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO,painter_proportions),
                response.rect,
            );

            let x = [pos2(0., 0.), pos2(1., 1.)].map(|p| to_screen.transform_pos(p));

            painter.line_segment(x, Stroke::new(0.5, Color32::RED));
        });

        response
    }
}

