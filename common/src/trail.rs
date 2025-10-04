use crate::board::{Board, Move, PlayerID, PlayerPos, Tile, TileEndpoint};
use std::collections::HashMap;

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

/// Get the segment tail points (entry point and inner point) for an endpoint
/// Based on the same logic as the rendering system
fn segment_tail(endpoint: TileEndpoint) -> [(f32, f32); 2] {
    let (a, b) = match endpoint {
        0 => ((1./3., 1.), (1./3., 5./6.)),      // Bottom left: entry and inward
        1 => ((2./3., 1.), (2./3., 5./6.)),      // Bottom right: entry and inward
        2 => ((1., 2./3.), (5./6., 2./3.)),      // Right top: entry and inward
        3 => ((1., 1./3.), (5./6., 1./3.)),      // Right bottom: entry and inward
        4 => ((2./3., 0.), (2./3., 1./6.)),      // Top right: entry and inward
        5 => ((1./3., 0.), (1./3., 1./6.)),      // Top left: entry and inward
        6 => ((0., 1./3.), (1./6., 1./3.)),      // Left bottom: entry and inward
        7 => ((0., 2./3.), (1./6., 2./3.)),      // Left top: entry and inward
        _ => panic!("non existent endpoint {}", endpoint),
    };
    [a, b]
}

/// A trail for a single player consisting of normalized path points
#[derive(Debug, Clone)]
pub struct Trail {
    pub player_id: PlayerID,
    pub points: Vec<(f32, f32)>, // Normalized coordinates (0.0 to 1.0) within tiles
    pub cells: Vec<crate::board::CellCoord>, // Which cell each point belongs to
}

impl Trail {
    pub fn new(player_id: PlayerID) -> Self {
        Self {
            player_id,
            points: Vec::new(),
            cells: Vec::new(),
        }
    }

    /// Add a path segment for movement through a tile
    pub fn add_tile_segment(&mut self, from_pos: PlayerPos, to_pos: PlayerPos, tile: &Tile) {
        // Find the segment that contains the FROM endpoint (where player enters)
        let segment = tile.segments
            .iter()
            .find(|&seg| seg.a == from_pos.endpoint || seg.b == from_pos.endpoint);

        let segment = match segment {
            Some(seg) => seg,
            None => {
                println!("ERROR: No segment found containing FROM endpoint {} in tile {:?}",
                    from_pos.endpoint, tile);
                println!("Available segments: {:?}", tile.segments);
                return; // Skip this trail segment instead of panicking
            }
        };

        // Get the exit endpoint (the other endpoint in the same segment)
        let exit_endpoint = if segment.a == from_pos.endpoint {
            segment.b
        } else {
            segment.a
        };

        println!("DEBUG: Player enters at {} via segment {:?}, exits at {} (should be {})",
            from_pos.endpoint, segment, exit_endpoint, to_pos.endpoint);

        // Get the tail points for the path (exactly like tile rendering)
        let start_chunk = segment_tail(from_pos.endpoint);  // [entry_point, inner_point]
        let end_chunk = segment_tail(exit_endpoint);        // [exit_point, inner_point]

        // Add the 3 line segments exactly like tile rendering:
        // 1. start_chunk: entry_point -> inner_point
        self.points.push(start_chunk[0]); // Entry point
        self.cells.push(from_pos.cell);
        self.points.push(start_chunk[1]); // Inner point (from side)
        self.cells.push(from_pos.cell);

        // 2. middle_chunk: inner_point (from) -> inner_point (to)
        self.points.push(start_chunk[1]); // Inner point (from side)
        self.cells.push(from_pos.cell);
        self.points.push(end_chunk[1]);   // Inner point (to side)
        self.cells.push(to_pos.cell);

        // 3. end_chunk: inner_point -> exit_point
        self.points.push(end_chunk[1]);   // Inner point (to side)
        self.cells.push(to_pos.cell);
        self.points.push(end_chunk[0]);   // Exit point
        self.cells.push(to_pos.cell);

        println!("DEBUG: Trail for player {} now has {} points (3 segments of 2 points each)", self.player_id, self.points.len());
    }
}

/// Manages trails for all players
#[derive(Debug)]
pub struct TrailTracker {
    trails: HashMap<PlayerID, Trail>,
}

impl TrailTracker {
    pub fn new() -> Self {
        Self {
            trails: HashMap::new(),
        }
    }

    /// Initialize or get a trail for a player
    pub fn get_or_create_trail(&mut self, player_id: PlayerID) -> &mut Trail {
        self.trails.entry(player_id).or_insert_with(|| Trail::new(player_id))
    }

    /// Get a trail for a player (read-only)
    pub fn get_trail(&self, player_id: PlayerID) -> Option<&Trail> {
        self.trails.get(&player_id)
    }

    /// Update trails based on a new move (call this before updating player positions)
    pub fn update_for_move(&mut self, mov: &Move, players_before_move: &[crate::board::Player]) {
        for player in players_before_move {
            if player.pos.cell == mov.cell {
                let new_pos = Board::traverse_tile(&mov.tile, player.pos);
                if new_pos != player.pos {
                    let trail = self.get_or_create_trail(player.id);
                    trail.add_tile_segment(player.pos, new_pos, &mov.tile);
                }
            }
        }
    }

    /// Get all trails
    pub fn all_trails(&self) -> &HashMap<PlayerID, Trail> {
        &self.trails
    }
}

/// Calculate player trail by simulating movement through board history
/// (kept for backward compatibility)
pub fn calculate_player_trail(
    player_id: PlayerID,
    initial_pos: PlayerPos,
    board_history: &[Move]
) -> Vec<PlayerPos> {
    let mut trail = vec![initial_pos];
    let mut current_pos = initial_pos;

    for mov in board_history {
        // If this move affects our player (they're at the cell being modified)
        if current_pos.cell == mov.cell {
            // Calculate new position using just the placed tile
            let new_pos = Board::traverse_tile(&mov.tile, current_pos);
            if new_pos != current_pos {
                trail.push(new_pos);
                current_pos = new_pos;
            }
        }
    }

    trail
}