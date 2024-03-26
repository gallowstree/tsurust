use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::Vec2;
use eframe::epaint::{Color32, Stroke};
use egui::Pos2;
use tsurust_common::board::*;

use crate::rendering::{paint_tile, PINK};

const TILE_SIZE: Vec2 = Vec2::new(110., 110.);
pub struct BoardRenderer<'a> {
    history: &'a mut Vec<Move>, //to-do, alias this type, do these folks need to be mutable?
    players: &'a mut Vec<Player>
}

impl <'a> BoardRenderer<'a> {
    pub(crate) fn new(history: &'a mut Vec<Move>, players: &'a mut Vec<Player>) -> Self {
        Self { history, players }
    }
}

impl Widget for BoardRenderer<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rows, cols) = (6.,6.);
        let tile_length: f32 = 110.0;

        let (board_rect, response) = ui.allocate_at_least(
            vec2(rows * tile_length, cols * tile_length),
            Sense::click().union(Sense::hover())
        );
        
        background(ui, board_rect);

        ui.vertical_centered(|ui| {
            tiles(ui, self.history, board_rect);
        });

        for player in self.players {
            let cell_rect = rect_at_coord(player.pos.cell, board_rect);
            // cell_rect - TILE_SIZE * Vec2::new(0.5, 0.5);
        }

        response
    }
}

fn rect_at_coord(cell_coord: CellCoord, board_rect: Rect) -> Rect {
    let pos = Pos2::new(cell_coord.col as f32 * 110., cell_coord.row as f32 * 110.) + board_rect.min.to_vec2();
    Rect::from_min_size(pos, TILE_SIZE)
}

fn tiles(ui: &mut Ui, history: &Vec<Move>, board_rect: Rect) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let painter = ui.painter();

        for mov in history {
            let rect = rect_at_coord(mov.cell, board_rect);
            paint_tile(&mov.tile, rect, painter);
        }
    });

}

fn background(ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.6, Color32::BLACK);
    ui.painter().rect_stroke(rect, 0.5, Stroke::new(4.0, PINK));

    //crate::backgr_render::draw_yin_yang(ui, 120.);
}