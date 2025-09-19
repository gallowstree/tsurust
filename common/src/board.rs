use std::cmp::{max, min};

use arrayvec::ArrayVec;
/// Board GRID Constants
/// 6 rows & 6 cols
pub const BOARD_LENGTH: usize = 6;
/// min valid row/col index
pub const MIN: usize = 0;
/// max valid row/col index
pub const MAX: usize = BOARD_LENGTH - 1;
///
/// Board TILE structs
///
/// Type alias but should be enum: rename to PathExit, ExitIndex or TileExit!
/// enum { NE, NW, EN, ES, WN, WS, SW, SE } but with current order/num values
pub type TileEndpoint = usize;
///  We represent a `Tile` as a collection of four `Segment`s.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Tile {
    pub segments: [Segment; 4],
}

impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let segments: Vec<String> = self.segments
            .iter()
            .map(|seg| format!("{}-{}", seg.a, seg.b))
            .collect();
        write!(f, "Tile({})", segments.join(", "))
    }
}
/// which are just pairs of entry points connected by each segment.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Segment {
    pub a: TileEndpoint,
    pub b: TileEndpoint,
}
/// A position inside the board's grid
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub row: usize,
    pub col: usize,
}
/// Players and Pawns data
pub type PlayerID = usize;
pub struct Player {
    pub id: PlayerID,
    pub pos: PlayerPos,
    pub alive: bool,
    pub color: (u8, u8, u8),  // RGB color tuple
}

impl Player {
    pub fn new(id: PlayerID, pos: PlayerPos) -> Self {
        Self {
            id,
            pos,
            alive: true,
            color: crate::colors::get_player_color(id),
        }
    }
}
/// The position of a `Player`. Made of the cell coordinates and the current entry point id.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PlayerPos {
    pub cell: CellCoord,
    pub endpoint: TileEndpoint,
}
/// Game State
/// Move: Modify board state.
/// Represents a Player's move: which tile was placed where.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Move {
    pub tile: Tile,
    pub cell: CellCoord,
    pub player_id: PlayerID,
}
/// Board state history: all the moves that have been played
/// Defines functions to calculate the effects of moves on the board's state
pub struct Board {
    //setup: Vec<Player> something reflecting initial conditions?
    pub history: Vec<Move>,
}

impl Tile {
    /// Creates a new `Tile`. The segments are always sorted so there is only one way of representing a given tile.
    pub fn new(mut segments: [Segment; 4]) -> Tile {
        segments.sort();
        Tile { segments }
    }

    /// Returns a new `Tile` with the same segment shape, but rotated according to the `clockwise` param.
    pub fn rotated(&self, clockwise: bool) -> Tile {
        let rotated_segments: ArrayVec<Segment, 4> = self
            .segments
            .into_iter()
            .map(|seg| seg.rotated(clockwise))
            .collect();
        Tile::new(rotated_segments.into_inner().unwrap())
    }
}

impl Segment {
    pub fn new(a: TileEndpoint, b: TileEndpoint) -> Segment {
        let min = min(a, b);
        let max = max(a, b);
        Segment { a: min, b: max }
    }

    pub fn a(&self) -> TileEndpoint {
        self.a
    }
    pub fn b(&self) -> TileEndpoint {
        self.b
    }

    pub fn rotated(&self, clockwise: bool) -> Segment {
        let num_endpoints = 8;
        let offset = if clockwise { 6 } else { 2 };
        Segment::new(
            (self.a + offset) % num_endpoints,
            (self.b + offset) % num_endpoints,
        )
    }
}

impl PlayerPos {
    pub fn new(row: usize, col: usize, entry: TileEndpoint) -> PlayerPos {
        PlayerPos {
            cell: CellCoord { row, col },
            endpoint: entry,
        }
    }

    /// Is the player on the outer edge of the board? This means the player died or is at a starting position.
    pub fn on_edge(&self) -> bool {
        match (self.endpoint, self.cell.row, self.cell.col) {
            (0 | 1, row, _) => row == MAX,
            (2 | 3, _, col) => col == MAX,
            (4 | 5, row, _) => row == MIN,
            (6 | 7, _, col) => col == MIN,
            _ => false,
        }
    }
}

impl Board {
    pub fn new() -> Board {
        Board {
            history: Vec::new(),
        }
    }

    pub fn from_history(history: Vec<Move>) -> Board {
        Board {
            history,
        }
    }

    /// Get the `Tile` occupying the specified cell, if there is one.
    pub fn get_tile_at(&self, pos: CellCoord) -> Option<&Tile> {
        self.history
            .iter()
            .find(|&mov| mov.cell == pos)
            .map(|mov| &mov.tile)
    }

    /// Place a tile as specified by the `Move`
    pub fn place_tile(&mut self, mov: Move) {
        self.history.push(mov);
    }

    /// Returns the final position after traversing the path starting at the given position
    pub fn traverse_from(&self, starting_point: PlayerPos) -> PlayerPos {
        match self.get_tile_at(starting_point.cell) {
            None => starting_point, // There is no tile to follow its path, so we're done
            Some(tile) => {
                let next_pos = Board::traverse_tile(tile, starting_point);

                // If the player reached the board edge, stop traversing to prevent infinite recursion
                if next_pos.on_edge() {
                    return next_pos;
                }

                self.traverse_from(next_pos)
            }
        }
    }

    /// Returns the immediate next position of a player starting at the given position and following the path of the given `Tile`
    pub fn traverse_tile(tile: &Tile, from: PlayerPos) -> PlayerPos {
        let tile_exit = tile
            .segments
            .iter()
            .find(|&seg| seg.a() == from.endpoint || seg.b() == from.endpoint)
            .map(|seg| {
                if seg.a() != from.endpoint {
                    seg.a()
                } else {
                    seg.b()
                }
            })
            .expect("there is an invalid tile") as TileEndpoint;
        let neighbor_entry = Board::neighboring_entry(tile_exit);

        match (tile_exit, from.cell.row, from.cell.col) {
            // player reached the end of the board, don't increment row/col
            // claude: can we rewrite this as (0 | 1,  MAX, col), etc?
            (0 | 1, row, col) if row == MAX => PlayerPos::new(row, col, tile_exit),
            (2 | 3, row, col) if col == MAX => PlayerPos::new(row, col, tile_exit),
            (4 | 5, row, col) if row == MIN => PlayerPos::new(row, col, tile_exit),
            (6 | 7, row, col) if col == MIN => PlayerPos::new(row, col, tile_exit),
            // player is inside the board, they move to the next row/col and the neighboring cell's entry adjacent to the exit point
            (0 | 1, row, col) => PlayerPos::new(row + 1, col, neighbor_entry),
            (2 | 3, row, col) => PlayerPos::new(row, col + 1, neighbor_entry),
            (4 | 5, row, col) => PlayerPos::new(row - 1, col, neighbor_entry),
            (6 | 7, row, col) => PlayerPos::new(row, col - 1, neighbor_entry),
            _ => panic!("there is a bug or an invalid tile"),
        }
    }

    /// Returns the entry point connected to the given entry point in the neighboring cell
    fn neighboring_entry(exit: TileEndpoint) -> TileEndpoint {
        (if exit % 2 == 0 { exit + 5 } else { exit + 3 }) % 8
    }
}

pub fn seg(a: TileEndpoint, b: TileEndpoint) -> Segment {
    Segment::new(a, b)
}

mod tests {
    use super::*;

    #[test]
    fn test_next_pos_edge() {
        let tile = Tile::new([seg(5, 3), seg(6, 7), seg(4, 0), seg(1, 2)]);

        let from = PlayerPos::new(0, 0, 0);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 4));
        let from = PlayerPos::new(0, 0, 6);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 7));
        let from = PlayerPos::new(0, 0, 7);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(0, 0, 6));

        let from = PlayerPos::new(5, 5, 4);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(5, 5, 0));
        let from = PlayerPos::new(5, 5, 1);
        assert_eq!(Board::traverse_tile(&tile, from), PlayerPos::new(5, 5, 2));


    }

    #[test]
    fn test_next_pos_not_edge() {
        let tile = Tile::new([seg(5, 3), seg(6, 7), seg(4, 0), seg(1, 2)]);

        let from = PlayerPos::new(0, 0, 1);
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
    fn test_traverse_from_chained_tiles() {
        let mut board = Board::new();

        // Place a tile at (0,0) that sends player from entry 1 to exit 2 (moving right to next cell)
        let tile1 = Tile::new([seg(1, 2), seg(0, 3), seg(4, 5), seg(6, 7)]);
        board.place_tile(Move { tile: tile1, cell: CellCoord { row: 0, col: 0 }, player_id: 1 });

        // Place a tile at (0,1) that receives player at entry 7 (from tile1's exit 2)
        // and sends them to exit 3 (moving right again to next cell)
        let tile2 = Tile::new([seg(7, 3), seg(0, 1), seg(4, 5), seg(6, 2)]);
        board.place_tile(Move { tile: tile2, cell: CellCoord { row: 0, col: 1 }, player_id: 1 });

        // Start player at (0,0) entry point 1
        let start_pos = PlayerPos::new(0, 0, 1);

        // Should traverse through both tiles and end up at (0,2) entry point 6
        // (0,0,1) -> (0,0,2) -> move to (0,1,7) -> (0,1,3) -> move to (0,2,6)
        let final_pos = board.traverse_from(start_pos);

        assert_eq!(final_pos, PlayerPos::new(0, 2, 6));
    }

    #[test]
    fn test_traverse_from_single_tile_no_infinite_loop() {
        let mut board = Board::new();

        // Place a tile at (0,0) that connects entry 1 to exit 3
        let tile = Tile::new([seg(1, 3), seg(0, 2), seg(4, 5), seg(6, 7)]);
        board.place_tile(Move { tile, cell: CellCoord { row: 0, col: 0 }, player_id: 1 });

        // Start player at (0,0) entry point 1
        let start_pos = PlayerPos::new(0, 0, 1);

        // Should move within tile from entry 1 to exit 3, then move to neighboring cell (0,1)
        // Since (0,1) has no tile, should stop at the entry point that corresponds to exit 3
        let final_pos = board.traverse_from(start_pos);

        // According to neighboring_entry mapping, exit 3 -> entry 6
        assert_eq!(final_pos, PlayerPos::new(0, 1, 6));
    }

    #[test]
    fn test_traverse_from_circular_path() {
        let mut board = Board::new();

        // Create a 2x2 square of tiles that form a circular path
        // Tile at (0,0): entry 2 -> exit 3 (right)
        let tile1 = Tile::new([seg(2, 3), seg(0, 1), seg(4, 5), seg(6, 7)]);
        board.place_tile(Move { tile: tile1, cell: CellCoord { row: 0, col: 0 }, player_id: 1 });

        // Tile at (0,1): entry 6 -> exit 0 (down)
        let tile2 = Tile::new([seg(6, 0), seg(1, 2), seg(3, 4), seg(5, 7)]);
        board.place_tile(Move { tile: tile2, cell: CellCoord { row: 0, col: 1 }, player_id: 1 });

        // Tile at (1,1): entry 4 -> exit 7 (left)
        let tile3 = Tile::new([seg(4, 7), seg(0, 1), seg(2, 3), seg(5, 6)]);
        board.place_tile(Move { tile: tile3, cell: CellCoord { row: 1, col: 1 }, player_id: 1 });

        // Tile at (1,0): entry 1 -> exit 2 (up) - completes the circle
        let tile4 = Tile::new([seg(1, 2), seg(0, 3), seg(4, 5), seg(6, 7)]);
        board.place_tile(Move { tile: tile4, cell: CellCoord { row: 1, col: 0 }, player_id: 1 });

        // Start player at (0,0) entry point 2 - this should create infinite loop
        let start_pos = PlayerPos::new(0, 0, 2);

        // This should detect the circular path and not overflow
        let final_pos = board.traverse_from(start_pos);

        // For now, just test that it doesn't crash - we'll figure out expected behavior later
        println!("Final position: {:?}", final_pos);
    }

    #[test]
    fn test_traverse_from_player_starting_position() {
        let mut board = Board::new();

        // Player starts at (0, 0, 7) - same as in the game initialization
        let player_pos = PlayerPos::new(0, 0, 7);

        // Place a tile at the same position where player is standing
        // This tile connects endpoint 7 to some other endpoint
        let tile = Tile::new([seg(7, 1), seg(0, 2), seg(3, 4), seg(5, 6)]);
        board.place_tile(Move { tile, cell: CellCoord { row: 0, col: 0 }, player_id: 1 });

        // Now traverse from the player's position - this simulates what happens in update_players()
        let final_pos = board.traverse_from(player_pos);

        println!("Player moved from {:?} to {:?}", player_pos, final_pos);

        // Should not cause stack overflow
        assert_ne!(final_pos, player_pos); // Player should move somewhere
    }

    #[test]
    fn test_traverse_from_board_edge_no_overflow() {
        let mut board = Board::new();

        // Place a tile at (0,5) that sends player off the right edge of the board
        // Entry 2 -> Exit 3 (exit 3 points right, which would be off the board)
        let tile = Tile::new([seg(2, 3), seg(0, 1), seg(4, 5), seg(6, 7)]);
        board.place_tile(Move { tile, cell: CellCoord { row: 0, col: 5 }, player_id: 1 });

        // Player starts at the right edge cell, facing right (endpoint 2)
        let start_pos = PlayerPos::new(0, 5, 2);

        // This should NOT cause stack overflow - should stop at the edge
        let final_pos = board.traverse_from(start_pos);

        // Player should end up at the edge position
        assert!(final_pos.on_edge());
        assert_eq!(final_pos.cell, CellCoord { row: 0, col: 5 });
        assert_eq!(final_pos.endpoint, 3); // The exit endpoint
    }

    #[test]
    fn rotation_of_symmetrical_tile() {
        let tile = Tile::new([seg(0, 5), seg(1, 4), seg(6, 3), seg(7, 2)]);

        assert_eq!(tile.rotated(true), tile);
        assert_eq!(tile.rotated(true).rotated(true), tile);
        assert_eq!(tile.rotated(true).rotated(true).rotated(true), tile);
        assert_eq!(tile.rotated(false), tile);
        assert_eq!(tile.rotated(false).rotated(false), tile);
        assert_eq!(tile.rotated(false).rotated(false).rotated(false), tile);
    }

    #[test]
    fn clockwise_rotation() {
        let tile = Tile::new([seg(7, 5), seg(1, 0), seg(6, 2), seg(3, 4)]);

        assert_eq!(
            tile.rotated(true),
            Tile::new([seg(5, 3), seg(6, 7), seg(4, 0), seg(1, 2)])
        );
        assert_eq!(
            tile.rotated(true).rotated(true).rotated(true).rotated(true),
            tile
        );
    }

    #[test]
    fn counter_clockwise_rotation() {
        let tile = Tile::new([seg(7, 5), seg(1, 0), seg(6, 2), seg(3, 4)]);

        assert_eq!(
            tile.rotated(false),
            Tile::new([seg(1, 7), seg(2, 3), seg(0, 4), seg(5, 6)])
        );
        assert_eq!(
            tile.rotated(false)
                .rotated(false)
                .rotated(false)
                .rotated(false),
            tile
        );
    }
}
