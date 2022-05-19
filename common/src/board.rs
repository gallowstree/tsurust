
use arrayvec::ArrayVec;
use std::cmp::{min, max};

/// 6 rows & 6 cols
pub const BOARD_LENGTH:usize = 6;
/// max valid row/col index
pub const MAX:usize = BOARD_LENGTH - 1;
/// min valid row/col index
pub const MIN:usize = 0;

pub type TileEndpoint = usize;

// TODO: probably want to change to PlayerName at some point, ID is good enough for now
pub type PlayerID = usize;

///  We represent a `Tile` as a collection of four `Segment`s, which are just pairs of entry points connected by each segment.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Tile {
    pub segments: [Segment; 4]
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Segment {
    a: TileEndpoint,
    b: TileEndpoint
}

/// Game data about a player
pub struct Player {
    pub id: PlayerID,
    pub pos: PlayerPos,
    pub alive: bool
    // name, id or sumthing
    // hand: Vec<Tile>
    // dragon: bool?, actually this could be an Optional on the game state
}

/// A position inside the board's grid
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CellCoord {
    pub row: usize,
    pub col: usize
}

/// The position of a `Player`. Made of the cell coordinates and the current entry point id.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerPos {
    pub cell: CellCoord,
    pub endpoint: TileEndpoint
}

/// Represents a Player's move: which tile was placed where
pub struct Move {
    pub tile: Tile,
    pub cell: CellCoord,
    // player id
}

/// Board state: all the moves that have been played and operations to calculate their results
pub struct Board {
    //setup: Vec<Player> something reflecting initial conditions?
    history: Vec<Move>,
    //deck
}

impl Tile {
    /// Creates a new `Tile`. The segments are always sorted so there is only one way of representing a given tile.
    pub fn new(mut segments: [Segment; 4]) -> Tile {
        segments.sort();
        Tile {segments}
    }

    /// Returns a new `Tile` with the same segment shape, but rotated according to the `clockwise` param.
    pub fn rotated(&self, clockwise: bool) -> Tile {
        let rotated_segments: ArrayVec<Segment, 4> = self.segments
            .into_iter()
            .map(|seg| seg.rotated(clockwise))
            .collect();
        Tile::new(rotated_segments.into_inner().unwrap())
    }
}

impl Segment {
    pub fn new(a:TileEndpoint, b: TileEndpoint) -> Segment {
        let min = min(a, b);
        let max = max(a, b);
        Segment {a: min, b: max}
    }

    pub fn a(&self) -> TileEndpoint { self.a }
    pub fn b(&self) -> TileEndpoint { self.b }

    pub fn rotated(&self, clockwise: bool) -> Segment {
        let num_endpoints = 8;
        let offset = if clockwise { 6 } else { 2 };
        Segment::new((self.a + offset) % num_endpoints, (self.b + offset) % num_endpoints)
    }
}

impl PlayerPos {
    pub fn new(row: usize, col: usize, entry:TileEndpoint) -> PlayerPos {
        PlayerPos { cell: CellCoord {row, col}, endpoint: entry}
    }

    /// Is the player on the outer edge of the board? This means the player died or is at a starting position.
    pub fn on_edge(&self) -> bool {
        match (self.endpoint, self.cell.row, self.cell.col) {
            (0 | 1, row, _) => row == MAX,
            (2 | 3, _, col) => col == MAX,
            (4 | 5, row, _) => row == MIN,
            (6 | 7, _, col) => col == MIN,
            _ => false
        }
    }
}

impl Board {

    /// Get the `Tile` occupying the specified cell, if there is one.
    pub fn get_tile_at(&self, pos: CellCoord) -> Option<&Tile> {
        self.history
            .iter()
            .find(|&mov| mov.cell == pos)
            .map(|mov| &mov.tile)
    }

    /// Place a tile as specified by the `Move`
    pub fn place_tile(&mut self, mov:Move) {
        self.history.push(mov);
    }

    /// Returns the final position after traversing the path starting at the given position
    pub fn traverse_from(&self, starting_point:PlayerPos) -> PlayerPos {
        match self.get_tile_at(starting_point.cell) {
            None => starting_point,   // There is no tile to follow its path, so we're done
            Some(tile) => {
                let next_pos = Board::traverse_tile(tile, starting_point);
                self.traverse_from(next_pos)
            }
        }
    }

    /// Returns the immediate next position of a player starting at the given position and following the path of the given `Tile`
    fn traverse_tile(tile:&Tile, from: PlayerPos) -> PlayerPos {
        let tile_exit = tile.segments.iter()
            .find(|&seg| seg.a() == from.endpoint || seg.b() == from.endpoint)
            .map(|seg| if seg.a() != from.endpoint {seg.a()} else {seg.b()})
            .expect("there is an invalid tile") as TileEndpoint;
        let neighbor_entry = Board::neighboring_entry(tile_exit);

        match (tile_exit, from.cell.row, from.cell.col) {
            // player reached the end of the board, don't increment row/col
            (0 | 1, row, col) if row == MAX => PlayerPos::new(row, col, tile_exit),
            (2 | 3, row, col) if col == MAX => PlayerPos::new(row, col, tile_exit),
            (4 | 5, row, col) if row == MIN => PlayerPos::new(row, col, tile_exit),
            (6 | 7, row, col) if col == MIN => PlayerPos::new(row, col, tile_exit),
            // player is inside the board, they move to the next row/col and the neighboring cell's entry adjacent to the exit point
            (0 | 1, row, col) => PlayerPos::new(row + 1, col, neighbor_entry),
            (2 | 3, row, col) => PlayerPos::new(row, col + 1, neighbor_entry),
            (4 | 5, row, col) => PlayerPos::new(row - 1, col, neighbor_entry),
            (6 | 7, row, col) => PlayerPos::new(row, col - 1, neighbor_entry),
            _ => panic!("there is a bug or an invalid tile")
        }
    }

    /// Returns the entry point connected to the given entry point in the neighboring cell
    fn neighboring_entry(exit:TileEndpoint) -> TileEndpoint {
        (if exit % 2 == 0 {exit + 5} else {exit + 3}) % 8
    }
}

pub fn seg(a: TileEndpoint, b:TileEndpoint) -> Segment {
    Segment::new(a,b)
}

mod tests {
    use super::*;

    #[test]
    fn test_next_pos_edge() {
        let tile = Tile::new([seg(5,3), seg(6,7), seg(4,0), seg(1,2)]);

        let from = PlayerPos::new(0,0,0);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 4));
        let from = PlayerPos::new(0,0,6);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 7));
        let from = PlayerPos::new(0,0,7);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 6));

        let from = PlayerPos::new(5,5,4);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(5, 5, 0));
        let from = PlayerPos::new(5,5,1);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(5, 5, 2));
    }

    #[test]
    fn test_next_pos_not_edge() {
        let tile = Tile::new([seg(5,3), seg(6,7), seg(4,0), seg(1,2)]);

        let from = PlayerPos::new(0,0,1);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 1, 7));
    }

    #[test]
    fn test_neighboring_entry() {
        assert_eq!(Board::neighboring_entry(0), 5);
        assert_eq!(Board::neighboring_entry(1), 4);
        assert_eq!(Board::neighboring_entry(2), 7);
        assert_eq!(Board::neighboring_entry(3), 6);
        assert_eq!(Board::neighboring_entry(4), 1);
        assert_eq!(Board::neighboring_entry(5), 0);
        assert_eq!(Board::neighboring_entry(6), 3);
        assert_eq!(Board::neighboring_entry(7), 2);

        assert_eq!(Board::neighboring_entry(5), 0);
        assert_eq!(Board::neighboring_entry(4), 1);
        assert_eq!(Board::neighboring_entry(7), 2);
        assert_eq!(Board::neighboring_entry(6), 3);
        assert_eq!(Board::neighboring_entry(1), 4);
        assert_eq!(Board::neighboring_entry(0), 5);
        assert_eq!(Board::neighboring_entry(3), 6);
        assert_eq!(Board::neighboring_entry(2), 7);
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


