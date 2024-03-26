use eframe::egui::{
    emath::RectTransform, pos2, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke,
};
use tsurust_common::board::{Board, Segment, Tile, TileEndpoint};

pub const TRANSPARENT_WHITE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 191);
pub const TRANSPARENT_GOLD: Color32 = Color32::from_rgba_premultiplied(255, 215, 0, 191);
pub const PINK: Color32 = Color32::from_rgba_premultiplied(200, 50, 125, 44);
pub fn paint_board(board: &Board) {}

pub fn paint_tile(tile: &Tile, rect: Rect, painter: &Painter) {
    let to_screen = tile_to_screen_transform(rect);
    let stroke = Stroke::new(2., TRANSPARENT_WHITE);

    tile.segments
        .iter()
        .for_each(|&Segment { a: from, b: to }| {
            let start_chunk = segment_tail(from);
            let end_chunk = segment_tail(to);
            let middle_chunk = [start_chunk[1], end_chunk[1]];

            [start_chunk, middle_chunk, end_chunk]
                .iter()
                .for_each(|line| {
                    let points = line.map(|point| to_screen.transform_pos(point));
                    painter.line_segment(points, stroke);
                });
        });
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

pub fn tile_to_screen_transform(rect: Rect) -> RectTransform {
    let painter_proportions = rect.square_proportions();

    RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, 3.* painter_proportions),
        rect
    )
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
