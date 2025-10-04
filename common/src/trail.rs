use crate::board::{PlayerPos, TileEndpoint};

/// Represents a single segment of a player's trail through one tile
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrailSegment {
    pub board_pos: (usize, usize),  // Board cell position (row, col)
    pub entry_point: TileEndpoint,   // Entry point into this tile (0-7)
    pub exit_point: TileEndpoint,    // Exit point from this tile (0-7)
}

/// Represents a complete trail for a player
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trail {
    pub segments: Vec<TrailSegment>,
    pub start_pos: PlayerPos,
    pub end_pos: PlayerPos,
    pub completed: bool,  // true if trail ends at board edge or collision
}

impl Trail {
    pub fn new(start_pos: PlayerPos) -> Self {
        Self {
            segments: Vec::new(),
            start_pos,
            end_pos: start_pos,
            completed: false,
        }
    }

    /// Returns the number of segments in this trail
    pub fn length(&self) -> usize {
        self.segments.len()
    }

    /// Add a segment to the trail
    pub fn add_segment(&mut self, segment: TrailSegment) {
        self.segments.push(segment);
    }
}