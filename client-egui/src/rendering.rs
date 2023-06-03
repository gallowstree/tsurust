use egui::{Color32, emath::{RectTransform, Rot2}, Frame, Painter, pos2, Pos2, Rect, Sense, Stroke, vec2, Vec2, Widget};

use tsurust_common::board::{Board, Segment, Tile, TileEndpoint};

pub fn paint_board(board: &Board, ) {

}

pub fn paint_tile(tile: Tile, rect: Rect, painter: &Painter) {
    let painter_proportions = rect.square_proportions();
    let to_screen = RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, 3. * painter_proportions),
        rect,
    );
    let stroke = Stroke::new(2., Color32::from_rgba_premultiplied(255, 255, 255, 191));

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
