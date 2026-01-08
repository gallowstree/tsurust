use std::cmp::min;
use std::collections::HashMap;

use eframe::egui::{
    emath::RectTransform, pos2, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke,
};

use tsurust_common::board::{Board, PlayerID, Segment, Tile, TileEndpoint};

pub const TRANSPARENT_WHITE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 191);
pub const TRANSPARENT_GOLD: Color32 = Color32::from_rgba_premultiplied(255, 215, 0, 191);
pub const PINK: Color32 = Color32::from_rgba_premultiplied(200, 50, 125, 44);
pub const TILE_BACKGROUND: Color32 = Color32::from_rgba_premultiplied(45, 45, 55, 180); // Dark blue-gray background

/// Placeholder for future board painting functionality
#[allow(dead_code)]
pub fn paint_board(_board: &Board) {}

pub fn paint_tile_with_rotation(tile: &Tile, rect: Rect, painter: &Painter, rotation_angle: f32) {
    paint_tile_with_trails_and_rotation(tile, rect, painter, &HashMap::new(), rotation_angle);
}

pub fn paint_tile_with_trails(
    tile: &Tile,
    rect: Rect,
    painter: &Painter,
    player_paths: &HashMap<TileEndpoint, (PlayerID, Color32)>
) {
    paint_tile_with_trails_rotation_and_alpha(tile, rect, painter, player_paths, 0.0, 1.0);
}

pub fn paint_tile_with_trails_and_rotation(
    tile: &Tile,
    rect: Rect,
    painter: &Painter,
    player_paths: &HashMap<TileEndpoint, (PlayerID, Color32)>,
    rotation_angle: f32
) {
    paint_tile_with_trails_rotation_and_alpha(tile, rect, painter, player_paths, rotation_angle, 1.0);
}

pub fn paint_tile_with_trails_rotation_and_alpha(
    tile: &Tile,
    rect: Rect,
    painter: &Painter,
    player_paths: &HashMap<TileEndpoint, (PlayerID, Color32)>,
    rotation_angle: f32,
    alpha: f32
) {
    // Apply alpha to tile background
    let bg_color = Color32::from_rgba_premultiplied(
        TILE_BACKGROUND.r(),
        TILE_BACKGROUND.g(),
        TILE_BACKGROUND.b(),
        (TILE_BACKGROUND.a() as f32 * alpha) as u8
    );
    let stroke_color = Color32::from_rgba_premultiplied(
        80,
        80,
        80,
        (255.0 * alpha) as u8
    );

    // Draw tile background
    painter.rect_filled(rect, 4.0, bg_color);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, stroke_color));

    let to_screen = tile_to_screen_transform(rect);
    let center = rect.center();

    tile.segments
        .iter()
        .for_each(|&Segment { a: from, b: to }| {
            // Use min(from, to) as convention - only look up trails for endpoints 0-3
            let segment_key = min(from, to);

            let segment_color = if let Some((_, player_color)) = player_paths.get(&segment_key) {
                // Make player trail semi-transparent but visible, with animation alpha
                Color32::from_rgba_unmultiplied(
                    player_color.r(),
                    player_color.g(),
                    player_color.b(),
                    (180.0 * alpha) as u8
                )
            } else {
                // Apply alpha to default tile color
                Color32::from_rgba_premultiplied(
                    TRANSPARENT_WHITE.r(),
                    TRANSPARENT_WHITE.g(),
                    TRANSPARENT_WHITE.b(),
                    (TRANSPARENT_WHITE.a() as f32 * alpha) as u8
                )
            };

            let stroke = Stroke::new(2., segment_color);

            let start_chunk = segment_tail(from);
            let end_chunk = segment_tail(to);
            let middle_chunk = [start_chunk[1], end_chunk[1]];

            [start_chunk, middle_chunk, end_chunk]
                .iter()
                .for_each(|line| {
                    let points = line.map(|point| {
                        let screen_pos = to_screen.transform_pos(point);
                        // Apply rotation around the tile center
                        rotate_point(screen_pos, center, rotation_angle)
                    });
                    painter.line_segment(points, stroke);
                });
        });
}

/// Rotate a point around a center by the given angle (in radians)
fn rotate_point(point: Pos2, center: Pos2, angle: f32) -> Pos2 {
    if angle == 0.0 {
        return point;
    }

    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Translate to origin
    let dx = point.x - center.x;
    let dy = point.y - center.y;

    // Rotate
    let rotated_x = dx * cos_a - dy * sin_a;
    let rotated_y = dx * sin_a + dy * cos_a;

    // Translate back
    pos2(center.x + rotated_x, center.y + rotated_y)
}

pub fn paint_tile_button_hoverlay(rect: Rect, painter: &Painter) {
    let to_screen = tile_to_screen_transform(rect);
    let font_size = rect.size().x / 7.;
    painter.rect_stroke(rect, 0.5, Stroke::new(2.0, TRANSPARENT_GOLD));

    let radius = font_size * 0.86;
    let rotate_cw_pos = to_screen.transform_pos(pos2(3., 1.5));
    let rotate_ccw_pos = to_screen.transform_pos(pos2(0., 1.5));

    painter.circle_filled(rotate_cw_pos, radius, Color32::BLACK);
    painter.circle_filled(rotate_ccw_pos, radius, Color32::BLACK);

    painter.text(
        rotate_cw_pos,
        Align2::CENTER_CENTER,
        "⟳",
        FontId::monospace(font_size),
        TRANSPARENT_WHITE,
    );
    painter.text(
        rotate_ccw_pos,
        Align2::CENTER_CENTER,
        "⟲",
        FontId::monospace(font_size),
        TRANSPARENT_WHITE,
    );
}

pub fn paint_tile_button_hoverlay_with_highlight(rect: Rect, painter: &Painter, highlight: Option<bool>) {
    let to_screen = tile_to_screen_transform(rect);
    let font_size = rect.size().x / 7.;
    let radius = font_size * 0.86;

    let rotate_cw_pos = to_screen.transform_pos(pos2(3., 1.5));
    let rotate_ccw_pos = to_screen.transform_pos(pos2(0., 1.5));

    // Show border only when tile would be placed (center area, no highlight)
    if highlight.is_none() {
        painter.rect_stroke(rect, 0.5, Stroke::new(2.0, TRANSPARENT_GOLD));
    }

    // Always show both rotation buttons
    // Left button (counterclockwise)
    if highlight == Some(false) {
        // Highlight left button
        painter.circle_filled(rotate_ccw_pos, radius * 1.2, Color32::from_rgba_unmultiplied(255, 255, 0, 100));
    }
    painter.circle_filled(rotate_ccw_pos, radius, Color32::BLACK);
    painter.text(
        rotate_ccw_pos,
        Align2::CENTER_CENTER,
        "⟲",
        FontId::monospace(font_size),
        TRANSPARENT_WHITE,
    );

    // Right button (clockwise)
    if highlight == Some(true) {
        // Highlight right button
        painter.circle_filled(rotate_cw_pos, radius * 1.2, Color32::from_rgba_unmultiplied(255, 255, 0, 100));
    }
    painter.circle_filled(rotate_cw_pos, radius, Color32::BLACK);
    painter.text(
        rotate_cw_pos,
        Align2::CENTER_CENTER,
        "⟳",
        FontId::monospace(font_size),
        TRANSPARENT_WHITE,
    );
}

pub fn tile_to_screen_transform(rect: Rect) -> RectTransform {
    let painter_proportions = rect.square_proportions();

    RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, 3.* painter_proportions),
        rect
    )
}

/// Get the normalized position (0.0 to 1.0) of an endpoint within a tile
pub fn endpoint_position(endpoint: TileEndpoint) -> (f32, f32) {
    match endpoint {
        0 => (1./3., 1.),
        1 => (2./3., 1.),
        2 => (1., 2./3.),
        3 => (1., 1./3.),
        4 => (2./3., 0.),
        5 => (1./3., 0.),
        6 => (0., 1./3.),
        7 => (0., 2./3.),
        _ => panic!("non existent endpoint {}", endpoint),
    }
}

/// Convert a Trail's topological data to world (screen) coordinates for rendering
/// Returns a vec of (Pos2, Pos2) line segments ready to draw
pub fn trail_to_world_coords(
    trail: &tsurust_common::trail::Trail,
    tile_size: f32,
    board_offset: Pos2
) -> Vec<(Pos2, Pos2)> {
    let mut line_segments = Vec::new();

    for segment in &trail.segments {
        // Calculate the tile's top-left position
        let tile_x = board_offset.x + segment.board_pos.1 as f32 * tile_size;
        let tile_y = board_offset.y + segment.board_pos.0 as f32 * tile_size;

        // Get the 3 line segments for this trail segment (same as tile rendering)
        let start_chunk = segment_tail_normalized(segment.entry_point);
        let end_chunk = segment_tail_normalized(segment.exit_point);

        // Convert normalized coordinates to screen coordinates
        let to_screen = |norm_x: f32, norm_y: f32| -> Pos2 {
            Pos2::new(
                tile_x + norm_x * tile_size,
                tile_y + norm_y * tile_size
            )
        };

        // 1. Entry tail: entry point -> inner point
        line_segments.push((
            to_screen(start_chunk[0].0, start_chunk[0].1),
            to_screen(start_chunk[1].0, start_chunk[1].1)
        ));

        // 2. Middle segment: inner point -> inner point
        line_segments.push((
            to_screen(start_chunk[1].0, start_chunk[1].1),
            to_screen(end_chunk[1].0, end_chunk[1].1)
        ));

        // 3. Exit tail: inner point -> exit point
        line_segments.push((
            to_screen(end_chunk[1].0, end_chunk[1].1),
            to_screen(end_chunk[0].0, end_chunk[0].1)
        ));
    }

    line_segments
}

/// Get segment tail points in normalized 0-1 coordinates
fn segment_tail_normalized(endpoint: TileEndpoint) -> [(f32, f32); 2] {
    match endpoint {
        0 => [(1./3., 1.), (1./3., 5./6.)],      // Bottom left
        1 => [(2./3., 1.), (2./3., 5./6.)],      // Bottom right
        2 => [(1., 2./3.), (5./6., 2./3.)],      // Right top
        3 => [(1., 1./3.), (5./6., 1./3.)],      // Right bottom
        4 => [(2./3., 0.), (2./3., 1./6.)],      // Top right
        5 => [(1./3., 0.), (1./3., 1./6.)],      // Top left
        6 => [(0., 1./3.), (1./6., 1./3.)],      // Left bottom
        7 => [(0., 2./3.), (1./6., 2./3.)],      // Left top
        _ => panic!("non existent endpoint {}", endpoint),
    }
}

fn segment_tail(index: TileEndpoint) -> [Pos2; 2] {
    let (a, b) = match index {
        0 => ((1., 3.), (1., 2.5)),
        1 => ((2., 3.), (2., 2.5)),
        2 => ((3., 2.), (2.5, 2.)),
        3 => ((3., 1.), (2.5, 1.)),
        4 => ((2., 0.), (2., 0.5)),
        5 => ((1., 0.), (1., 0.5)),
        6 => ((0., 1.), (0.5, 1.)),
        7 => ((0., 2.), (0.5, 2.)),
        _ => panic!("non existent endpoint index {}", index),
    };
    [pos2(a.0, a.1), pos2(b.0, b.1)]
}
