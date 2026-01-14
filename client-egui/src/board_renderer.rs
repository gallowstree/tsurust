use std::collections::HashMap;

use eframe::emath::Vec2;
use eframe::egui::{vec2, Frame, Rect, Response, Sense, Ui, Widget};
use eframe::epaint::{Color32, Stroke};
use egui::Pos2;

use tsurust_common::board::*;
use tsurust_common::trail::Trail;

use crate::app::{PlayerAnimation, TilePlacementAnimation};
use crate::rendering::{endpoint_position, paint_tile_with_trails, trail_to_world_coords, PINK};

const TILE_LENGTH: f32 = 120.0;
const TILE_SIZE: Vec2 = Vec2::new(TILE_LENGTH, TILE_LENGTH);
const PLAYER_RADIUS: f32 = TILE_LENGTH / 7.;

pub struct BoardRenderer<'a> {
    history: &'a Vec<Move>,
    players: &'a Vec<Player>,
    tile_trails: &'a Vec<(CellCoord, Vec<(PlayerID, TileEndpoint)>)>,
    player_trails: &'a HashMap<PlayerID, Trail>,
    player_animations: &'a HashMap<PlayerID, PlayerAnimation>,
    tile_placement_animation: &'a Option<TilePlacementAnimation>,
}

impl <'a> BoardRenderer<'a> {
    pub(crate) fn new(
        history: &'a Vec<Move>,
        players: &'a Vec<Player>,
        tile_trails: &'a Vec<(CellCoord, Vec<(PlayerID, TileEndpoint)>)>,
        player_trails: &'a HashMap<PlayerID, Trail>,
        player_animations: &'a HashMap<PlayerID, PlayerAnimation>,
        tile_placement_animation: &'a Option<TilePlacementAnimation>,
    ) -> Self {
        Self { history, players, tile_trails, player_trails, player_animations, tile_placement_animation }
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
            render_board_tiles(ui, self.history, self.tile_trails, self.players, board_rect, self.tile_placement_animation);
        });

        // Render player trails on top with higher opacity so tile paths don't show through as much
        for player in self.players.iter() {
            if let Some(trail) = self.player_trails.get(&player.id) {
                let player_color = Color32::from_rgb(player.color.0, player.color.1, player.color.2);
                let trail_color = Color32::from_rgba_unmultiplied(
                    player_color.r(),
                    player_color.g(),
                    player_color.b(),
                    200 // Higher opacity to minimize tile path blending
                );

                let line_segments = trail_to_world_coords(trail, TILE_LENGTH, board_rect.min);

                for (start, end) in line_segments {
                    ui.painter().line_segment(
                        [start, end],
                        Stroke::new(3.0, trail_color)
                    );
                }
            }
        }

        for player in self.players.iter() {
            let player_color = Color32::from_rgb(player.color.0, player.color.1, player.color.2);

            // Check if this player is animating
            let player_pos = if let Some(animation) = self.player_animations.get(&player.id) {
                // Interpolate position along the trail
                interpolate_position_along_trail(&animation.trail, animation.progress, board_rect)
            } else {
                // Use current player position
                let cell_rect = rect_at_coord(player.pos.cell, board_rect);
                let endpoint_offset = path_index_position(player.pos.endpoint);

                cell_rect.min + Vec2::new(
                    endpoint_offset.x * cell_rect.width(),
                    endpoint_offset.y * cell_rect.height()
                )
            };

            if player.alive {
                ui.painter().circle(player_pos, PLAYER_RADIUS, Color32::WHITE, Stroke::default());
                ui.painter().circle_filled(player_pos, PLAYER_RADIUS*0.8, player_color);
            } else {
                // Dead player: gray circle with brighter, thicker X
                ui.painter().circle(player_pos, PLAYER_RADIUS, Color32::WHITE, Stroke::default());
                ui.painter().circle_filled(player_pos, PLAYER_RADIUS*0.8, Color32::from_gray(100));

                let x_size = PLAYER_RADIUS * 0.7; // Slightly larger X

                // Brighten the player color for the X
                let bright_color = Color32::from_rgb(
                    player_color.r().saturating_add(80).min(255),
                    player_color.g().saturating_add(80).min(255),
                    player_color.b().saturating_add(80).min(255)
                );

                ui.painter().line_segment(
                    [player_pos - Vec2::new(x_size, x_size), player_pos + Vec2::new(x_size, x_size)],
                    Stroke::new(6.0, bright_color) // Thicker stroke
                );
                ui.painter().line_segment(
                    [player_pos - Vec2::new(x_size, -x_size), player_pos + Vec2::new(x_size, -x_size)],
                    Stroke::new(6.0, bright_color) // Thicker stroke
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
    tile_trails: &Vec<(CellCoord, Vec<(PlayerID, TileEndpoint)>)>,
    players: &Vec<Player>,
    board_rect: Rect,
    tile_placement_animation: &Option<TilePlacementAnimation>,
) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let painter = ui.painter();

        for mov in history {
            // Check if this tile is being animated
            let is_animating = tile_placement_animation
                .as_ref()
                .map(|anim| anim.cell == mov.cell)
                .unwrap_or(false);

            let rect = rect_at_coord(mov.cell, board_rect);

            // Get player paths for this tile
            let mut player_paths = HashMap::new();
            // Find trail entries for this cell coordinate
            for (cell_coord, trail_entries) in tile_trails {
                if cell_coord == &mov.cell {
                    for &(player_id, segment_key) in trail_entries {
                        // Find player color
                        if let Some(player) = players.iter().find(|p| p.id == player_id) {
                            let player_color = Color32::from_rgb(player.color.0, player.color.1, player.color.2);
                            player_paths.insert(segment_key, (player_id, player_color));
                        }
                    }
                    break;
                }
            }

            if is_animating {
                // Render with animation effects
                let anim = tile_placement_animation.as_ref().unwrap();

                // Ease-out cubic for smooth deceleration
                let eased_progress = 1.0 - (1.0 - anim.progress).powi(3);

                // Calculate animation parameters
                let scale = 0.80 + eased_progress * 0.20; // Scale from 80% to 100%
                let drop_offset = (1.0 - eased_progress) * 30.0; // Drop from 30px above
                let alpha = eased_progress; // Fade in from 0 to 1

                // Apply transformations
                let center = rect.center();
                let scaled_size = rect.size() * scale;
                let animated_rect = Rect::from_center_size(
                    center - Vec2::new(0.0, drop_offset),
                    scaled_size
                );

                crate::rendering::paint_tile_with_trails_rotation_and_alpha(&mov.tile, animated_rect, painter, &player_paths, 0.0, alpha);
            } else {
                // Normal rendering
                paint_tile_with_trails(&mov.tile, rect, painter, &player_paths);
            }
        }
    });
}

fn path_index_position(i: TileEndpoint) -> Vec2 {
    let (x, y) = endpoint_position(i);
    Vec2::new(x, y)
}

fn background(ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.6, Color32::BLACK);
    ui.painter().rect_stroke(rect, 0.5, Stroke::new(4.0, PINK));

    // Draw animated glowing light spinning around the border
    draw_spinning_border_glow(ui, rect);

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
}

/// Draw an animated glowing light that spins around the board border
fn draw_spinning_border_glow(ui: &mut Ui, rect: Rect) {
    // Get time for animation (seconds since some arbitrary point)
    let time = ui.input(|i| i.time);

    // Animation speed: complete one rotation every 4 seconds
    let rotation_speed = 0.25; // rotations per second
    let t = ((time * rotation_speed) % 1.0) as f32; // 0.0 to 1.0 around the perimeter

    // Calculate perimeter
    let width = rect.width();
    let height = rect.height();
    let perimeter = 2.0 * (width + height);

    // Glow length is 90% of perimeter
    let glow_length = perimeter * 0.9;

    // Current head position (0.0 to perimeter)
    let head_distance = t * perimeter;

    // Number of segments to draw the gradient smoothly
    let num_segments = 100;

    // Draw gradient from head (bright) to tail (transparent)
    for i in 0..num_segments {
        let segment_t = i as f32 / num_segments as f32;

        // Distance along perimeter for this segment (going backwards from head)
        let segment_distance = (head_distance - segment_t * glow_length + perimeter) % perimeter;

        // Get position on border for this distance
        let pos = position_at_perimeter_distance(rect, segment_distance);
        let next_distance = (segment_distance + glow_length / num_segments as f32) % perimeter;
        let next_pos = position_at_perimeter_distance(rect, next_distance);

        // Create gradient: bright at head (segment_t = 0), transparent at tail (segment_t = 1)
        // Use smooth falloff curve
        let falloff = 1.0 - segment_t;
        let smooth_falloff = falloff * falloff; // Quadratic falloff for smoother gradient

        // Multi-color gradient from white -> pink -> purple -> transparent
        let (r, g, b, opacity) = if segment_t < 0.3 {
            // White to bright pink (head)
            let local_t = segment_t / 0.3;
            let r = (255.0 * (1.0 - local_t * 0.1)) as u8;
            let g = (255.0 * (1.0 - local_t * 0.4)) as u8;
            let b = (255.0 * (1.0 - local_t * 0.1)) as u8;
            let opacity = (smooth_falloff * 255.0) as u8;
            (r, g, b, opacity)
        } else if segment_t < 0.6 {
            // Bright pink to purple
            let local_t = (segment_t - 0.3) / 0.3;
            let r = (230.0 * (1.0 - local_t * 0.2)) as u8;
            let g = (150.0 * (1.0 - local_t * 0.5)) as u8;
            let b = (230.0 + local_t * 25.0) as u8;
            let opacity = (smooth_falloff * 255.0) as u8;
            (r, g, b, opacity)
        } else {
            // Purple to transparent (tail)
            let local_t = (segment_t - 0.6) / 0.4;
            let r = (180.0 * (1.0 - local_t * 0.5)) as u8;
            let g = (80.0 * (1.0 - local_t * 0.8)) as u8;
            let b = 255;
            let opacity = (smooth_falloff * 255.0 * (1.0 - local_t * 0.5)) as u8;
            (r, g, b, opacity)
        };

        let glow_color = Color32::from_rgba_unmultiplied(r, g, b, opacity);

        // Width tapers from thick at head to thin at tail
        let width = 12.0 * (1.0 - segment_t * 0.8);

        ui.painter().line_segment(
            [pos, next_pos],
            Stroke::new(width, glow_color),
        );
    }

    // Request repaint for continuous animation
    ui.ctx().request_repaint();
}

/// Helper function to get position on board border at a given distance along perimeter
fn position_at_perimeter_distance(rect: Rect, distance: f32) -> Pos2 {
    let width = rect.width();
    let height = rect.height();

    if distance < width {
        // Top edge (left to right)
        let t = distance / width;
        Pos2::new(
            rect.left_top().x + t * width,
            rect.left_top().y,
        )
    } else if distance < width + height {
        // Right edge (top to bottom)
        let t = (distance - width) / height;
        Pos2::new(
            rect.right_top().x,
            rect.right_top().y + t * height,
        )
    } else if distance < 2.0 * width + height {
        // Bottom edge (right to left)
        let t = (distance - width - height) / width;
        Pos2::new(
            rect.right_bottom().x - t * width,
            rect.right_bottom().y,
        )
    } else {
        // Left edge (bottom to top)
        let t = (distance - 2.0 * width - height) / height;
        Pos2::new(
            rect.left_bottom().x,
            rect.left_bottom().y - t * height,
        )
    }
}

/// Interpolate player position along a trail based on animation progress (0.0 to 1.0)
fn interpolate_position_along_trail(trail: &Trail, progress: f32, board_rect: Rect) -> Pos2 {
    // Convert trail to world coordinates
    let line_segments = trail_to_world_coords(trail, TILE_LENGTH, board_rect.min);

    if line_segments.is_empty() {
        // No movement, return start position
        let cell_rect = rect_at_coord(trail.start_pos.cell, board_rect);
        let endpoint_offset = path_index_position(trail.start_pos.endpoint);
        return cell_rect.min + Vec2::new(
            endpoint_offset.x * cell_rect.width(),
            endpoint_offset.y * cell_rect.height()
        );
    }

    // Calculate total trail length
    let total_length: f32 = line_segments.iter()
        .map(|(start, end)| (*end - *start).length())
        .sum();

    // Find the target distance along the trail
    let target_distance = total_length * progress;

    // Walk along segments to find the interpolated position
    let mut accumulated_distance = 0.0;
    for (start, end) in line_segments.iter() {
        let segment_vec = *end - *start;
        let segment_length = segment_vec.length();

        if accumulated_distance + segment_length >= target_distance {
            // We're in this segment
            let distance_in_segment = target_distance - accumulated_distance;
            let t = distance_in_segment / segment_length;
            return *start + segment_vec * t;
        }

        accumulated_distance += segment_length;
    }

    // If we got here, return the end position
    line_segments.last().map(|(_, end)| *end).unwrap_or(Pos2::ZERO)
}