pub mod board {
    use arrayvec::ArrayVec;
    use std::cmp::{min, max};

    pub type TileEndpoint = usize;

    /// # Tiles
    ///  A tile has 4 pairs of endpoints, with each pair connected by a `Segment`.
    ///  We represent a `Tile` as a collection of four `Segment`s, which are just pairs of endpoints.
    ///  Endpoints are identified by a number representing their position on a tile.
    ///  Endpoint ids remain constant after tile rotations. Rotating yields a new tile with PathSegments
    ///  of the same shape but their endpoints offset by the rotation.
    ///
    ///  ┌ 5 ── 4 ┐
    ///  6        3   All tile endpoints identified by numbers 0 to 7
    ///  │        │
    ///  7        2
    ///  └ 0 ── 1 ┘
    #[derive(Debug, Copy, Clone, PartialEq)]
    pub struct Tile {
        segments: [Segment; 4]
    }

    #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
    pub struct Segment {
        a: TileEndpoint,
        b: TileEndpoint
    }

    impl Segment {
        pub fn new(a:TileEndpoint, b: TileEndpoint) -> Segment {
            let min = min(a, b);
            let max = max(a, b);
            Segment {a: min, b: max}
        }

        pub fn a(&self) -> TileEndpoint { self.a }
        pub fn b(&self) -> TileEndpoint { self.b }

        /// Offset the endpoints by 2 or -2 depending on direction. We convert to signed integers to operate and then back to unsigned.
        pub fn rotated(&self, clockwise: bool) -> Segment {
            let num_endpoints = 8;
            let offset:i8 = if clockwise { -2 } else { 2 };

            // `(x+offset) % num_endpoints` but % is not equal to mod for negatives
            let (a,b) = ( self.a as i8 + offset, self.b as i8 + offset );
            let (a,b) = (a.rem_euclid(num_endpoints) as TileEndpoint, b.rem_euclid(num_endpoints) as TileEndpoint);
            Segment::new(a, b)
        }
    }

    impl Tile {
        pub fn new(mut segments: [Segment; 4]) -> Tile {
            // We could validate here
            segments.sort();
            Tile {segments}
        }

        pub fn rotated(&self, clockwise: bool) -> Tile {
            let rotated_segments: ArrayVec<Segment, 4> = self.segments
                .into_iter()
                .map(|seg| seg.rotated(clockwise))
                .collect();
            Tile::new(rotated_segments.into_inner().unwrap())
        }
    }

}

#[cfg(test)]
mod tests {
    use super::board::*;

    fn seg(a: TileEndpoint, b:TileEndpoint) -> Segment {
        Segment::new(a,b)
    }

    #[test]
    fn rotation_of_symmetrical_tile() {
        let tile =  Tile::new([seg(0,5), seg(1,4), seg(6,3), seg(7,2)]);

        assert_eq!(tile.rotated(true), tile);
        assert_eq!(tile.rotated(true).rotated(true), tile);
        assert_eq!(tile.rotated(true).rotated(true).rotated(true), tile);
        assert_eq!(tile.rotated(false), tile);
        assert_eq!(tile.rotated(false).rotated(false), tile);
        assert_eq!(tile.rotated(false).rotated(false).rotated(false), tile);
    }

    #[test]
    fn clockwise_rotation() {
        let tile = Tile::new([seg(7,5), seg(1,0), seg(6,2), seg(3,4)]);

        assert_eq!(tile.rotated(true), Tile::new([seg(5,3), seg(6,7), seg(4,0), seg(1,2)]));
        assert_eq!(tile.rotated(true).rotated(true).rotated(true).rotated(true),tile);
    }

    #[test]
    fn counter_clockwise_rotation() {
        let tile = Tile::new([seg(7,5), seg(1,0), seg(6,2), seg(3,4)]);

        assert_eq!(tile.rotated(false), Tile::new([seg(1,7), seg(2,3), seg(0,4), seg(5,6)]));
        assert_eq!(tile.rotated(false).rotated(false).rotated(false).rotated(false),tile);
    }

}
