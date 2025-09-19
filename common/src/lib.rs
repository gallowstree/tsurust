/// # The Board
/// The board consists of a square grid of cells.
/// Each cell has 8 entry points (2 on each side) identified by a number from 0 to 7.
/// Players can place tiles inside the cell they occupy.
/// Placing a `Tile` inside a cell connects its exits with the ones on neighboring cells according
/// to the paths inside the tile.
///
/// All cell entry points with their ID.
///  ┌ 5 ──── 4 ┐
///  6          3
///  │          │
///  7          2
///  └ 0 ──── 1 ┘
/// Todo: rename references to "endpoint" to say "entry" or "entry point"
pub mod board;
pub mod colors;
mod deck;
pub mod game;
pub mod trail;
