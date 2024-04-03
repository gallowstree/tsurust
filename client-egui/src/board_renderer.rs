use std::ops::{Add, Mul};
use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::{RectTransform, Vec2};
use eframe::epaint::{Color32, Stroke};
use egui::Pos2;
use tsurust_common::board::*;

use crate::rendering::{paint_tile, PINK};

const TILE_LENGTH: f32 = 125.0;

const TILE_SIZE: Vec2 = Vec2::new(TILE_LENGTH, TILE_LENGTH);
const PLAYER_RADIUS: f32 = TILE_LENGTH / 7.;

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

        let (board_rect, response) = ui.allocate_at_least(
            vec2(rows * TILE_LENGTH, cols * TILE_LENGTH),
            Sense::click().union(Sense::hover())
        );
        
        background(ui, board_rect);

        ui.vertical_centered(|ui| {
            tiles(ui, self.history, board_rect);
        });

        for player in self.players {
            let cell_rect = rect_at_coord(player.pos.cell, board_rect);
            let offset = path_index_position(player.pos.endpoint).add(Vec2::new(1., 1.));
            let transform = RectTransform::from_to(board_rect, cell_rect);

            let center = transform.transform_rect(cell_rect).min + offset.mul(cell_rect.min.to_vec2() - Vec2::new(PLAYER_RADIUS*0.5, PLAYER_RADIUS));
            ui.painter().circle(center, PLAYER_RADIUS, Color32::WHITE, Stroke::default());
            ui.painter().circle_filled(center, PLAYER_RADIUS*0.8, Color32::DARK_GREEN);
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

fn path_index_position(i: TileEndpoint) -> Vec2 {
    let (x,y) = match i {
        0 => (1./3., 1.),
        1 => (2./3., 1.),
        2 => (1., 2./3.),
        3 => (1., 1./3.),
        4 => (2./3., 0.),
        5 => (1./3., 0.),
        6 => (0., 1./3.),
        7 => (0., 2./3.),
        _ => panic!("non existent path index {}", i)
    };
    Vec2::new(x,y)
}

fn background(ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.6, Color32::BLACK);
    ui.painter().rect_stroke(rect, 0.5, Stroke::new(4.0, PINK));

    for x in 0..= 6 {
        let x = x as f32 * TILE_SIZE.x;
        let start = Pos2::new(x , 0.) + rect.min.to_vec2();
        let end = Pos2::new(x, TILE_SIZE.x * 6.) + rect.min.to_vec2();

        ui.painter().line_segment([start, end], Stroke::new(0.2, Color32::LIGHT_YELLOW));

        let y = x;
        let start = Pos2::new(0. , y) + rect.min.to_vec2();
        let end = Pos2::new(TILE_SIZE.x * 6., y) + rect.min.to_vec2();
        ui.painter().line_segment([start, end], Stroke::new(0.2, Color32::LIGHT_YELLOW));

    }

    //crate::backgr_render::draw_yin_yang(ui, 120.);
}