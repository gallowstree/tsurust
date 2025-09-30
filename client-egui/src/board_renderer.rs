use eframe::egui::{vec2, Frame, Rect, Sense, Widget, Response, Ui};
use eframe::emath::Vec2;
use eframe::epaint::{Color32, Stroke};
use egui::Pos2;
use tsurust_common::board::*;
use std::collections::HashMap;

use crate::rendering::{paint_tile_with_trails, PINK};

const TILE_LENGTH: f32 = 120.0;
const TILE_SIZE: Vec2 = Vec2::new(TILE_LENGTH, TILE_LENGTH);
const PLAYER_RADIUS: f32 = TILE_LENGTH / 7.;

pub struct BoardRenderer<'a> {
    history: &'a Vec<Move>,
    players: &'a Vec<Player>,
    tile_trails: &'a HashMap<CellCoord, Vec<(PlayerID, TileEndpoint)>>,
}

impl <'a> BoardRenderer<'a> {
    pub(crate) fn new(history: &'a Vec<Move>, players: &'a Vec<Player>, tile_trails: &'a HashMap<CellCoord, Vec<(PlayerID, TileEndpoint)>>) -> Self {
        Self { history, players, tile_trails }
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
            render_board_tiles(ui, self.history, self.tile_trails, self.players, board_rect);
        });

        for player in self.players.iter() {
            let player_color = Color32::from_rgb(player.color.0, player.color.1, player.color.2);

            // Render current player position
            let cell_rect = rect_at_coord(player.pos.cell, board_rect);
            let endpoint_offset = path_index_position(player.pos.endpoint);

            let player_pos = cell_rect.min + Vec2::new(
                endpoint_offset.x * cell_rect.width(),
                endpoint_offset.y * cell_rect.height()
            );

            if player.alive {
                ui.painter().circle(player_pos, PLAYER_RADIUS, Color32::WHITE, Stroke::default());
                ui.painter().circle_filled(player_pos, PLAYER_RADIUS*0.8, player_color);
            } else {
                // Dead player: gray circle with colored X
                ui.painter().circle(player_pos, PLAYER_RADIUS, Color32::WHITE, Stroke::default());
                ui.painter().circle_filled(player_pos, PLAYER_RADIUS*0.8, Color32::from_gray(100));
                let x_size = PLAYER_RADIUS * 0.6;
                ui.painter().line_segment(
                    [player_pos - Vec2::new(x_size, x_size), player_pos + Vec2::new(x_size, x_size)],
                    Stroke::new(4.0, player_color)
                );
                ui.painter().line_segment(
                    [player_pos - Vec2::new(x_size, -x_size), player_pos + Vec2::new(x_size, -x_size)],
                    Stroke::new(4.0, player_color)
                );
            }
        }


        response
    }
}

fn rect_at_coord(cell_coord: CellCoord, board_rect: Rect) -> Rect {
    let pos = Pos2::new(cell_coord.col as f32 * TILE_LENGTH, cell_coord.row as f32 * TILE_LENGTH) + board_rect.min.to_vec2();
    Rect::from_min_size(pos, TILE_SIZE)
}

fn render_board_tiles(
    ui: &mut Ui,
    history: &Vec<Move>,
    tile_trails: &HashMap<CellCoord, Vec<(PlayerID, TileEndpoint)>>,
    players: &Vec<Player>,
    board_rect: Rect
) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let painter = ui.painter();

        for mov in history {
            let rect = rect_at_coord(mov.cell, board_rect);

            // Get player paths for this tile
            let mut player_paths = HashMap::new();
            if let Some(trail_entries) = tile_trails.get(&mov.cell) {
                for &(player_id, segment_key) in trail_entries {
                    // Find player color
                    if let Some(player) = players.iter().find(|p| p.id == player_id) {
                        let player_color = Color32::from_rgb(player.color.0, player.color.1, player.color.2);
                        player_paths.insert(segment_key, (player_id, player_color));
                    }
                }
            }

            paint_tile_with_trails(&mov.tile, rect, painter, &player_paths);
        }
    });
}

fn path_index_position(i: TileEndpoint) -> Vec2 {
    let (x, y) = tsurust_common::trail::endpoint_position(i);
    Vec2::new(x, y)
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