pub mod board {
    use arrayvec::ArrayVec;
    use std::cmp::{min, max};

    pub type TileEndpoint = usize;
    /// # Tiles
    pub const BOARD_LENGTH:usize = 6;
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

    /// Position (row, col) of a cell in the board, where a tile may be placed
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct CellPos {
        pub row: usize,
        pub col: usize
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct PlayerPos {
        pub cell: CellPos,
        pub endpoint: TileEndpoint
    }

    pub struct Player {
        pub pos: PlayerPos,
        // name, id or sumthing
        // hand
        // dragon: bool?
    }

    pub struct Move {
        pub tile: Tile,
        pub cell: CellPos
    }

    pub struct BoardState {
        //setup: Vec<SetupTurn|Player>  or something reflecting initial conditions?
        history: Vec<Move>,
        alive_players: Vec<Player>,
        //deck
    }

    impl BoardState {
        pub fn tile_at(&self, row:usize, col:usize) -> Option<Tile>{
            self.history
                .iter()
                .find(|&mov| mov.cell.row == row && mov.cell.col == col)
                .map(|mov| mov.tile)
        }
    }

    // server code

    impl BoardState {
        fn process_move(&mut self, mov:Move) {
            self.history.push(mov);

            for player in &self.alive_players {

            }
        }

        fn facing_pos(&self, current:&PlayerPos) -> PlayerPos {
            let (current_row, current_col) = (current.cell.row, current.cell.col);
            // if there is no tile at the player's position, they are at their starting position
            if self.tile_at(current_row, current_col).is_none() {
                return current.clone();
            }
            let (min, max) = (0usize, BOARD_LENGTH - 1);
            // otherwise find the row,col and entry in front of them
            let (row, col) = match (current.endpoint, current_row, current_col) {
                // They are on the edge of their starting position
                (0 | 1, row, col) if row == max => (row, col),
                (2 | 3, row, col) if col == max => (row, col),
                (4 | 5, row, col) if row == min => (row, col),
                (6 | 7, row, col) if col == min => (row, col),
                // They are inside the board
                (0 | 1, row, col) => (row + 1, col),
                (2 | 3, row, col) => (row, col + 1),
                (4 | 5, row, col) => (row - 1, col),
                (6 | 7, row, col) => (row, col - 1),
                _ => panic!("non existent entry {}",  current.endpoint)
            };

            let facing_entry:TileEndpoint = if (row, col) == (current_row, current_col) {
                // this was their first move, so they are facing at the tile they just placed and not moved yet
                current.endpoint
            } else {
                //get the endpoint of the facing cell connected to the current endpoint
                match current.endpoint {
                    0 => 5,
                    1 => 4,
                    2 => 7,
                    3 => 6,
                    4 => 1,
                    5 => 0,
                    6 => 3,
                    7 => 2,
                    _ => panic!("non existent entry {}",  current.endpoint)
                }
            };
            PlayerPos { cell: CellPos {row, col},  endpoint: facing_entry}
        }
    }


    #[test]
    fn facing_empty_cell_board_edge() {
        let board = BoardState {history: Vec::new(), alive_players: Vec::new()};

        let start_pos = PlayerPos { cell: CellPos { row: 0, col: 0 }, endpoint: 0 };
        assert_eq!(board.facing_pos(&start_pos), start_pos);

        let start_pos = PlayerPos { cell: CellPos { row: 1, col: 0 }, endpoint: 0 };
        assert_eq!(board.facing_pos(&start_pos), start_pos);
        // Finish me...
    }
}

#[cfg(test)]
mod tests {
    use super::board::*;
    use std::io::empty;

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
