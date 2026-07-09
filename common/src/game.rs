use std::cmp::min;
use std::collections::HashMap;

use crate::board::*;
use crate::deck::Deck;
use crate::trail::Trail;

/// Calculate the geometric distance of a path segment between two endpoints on a tile
/// Entry points are positioned on a normalized 3x3 grid:
/// - 0 (Bottom left): (1, 3), 1 (Bottom right): (2, 3)
/// - 2 (Right top): (3, 2), 3 (Right bottom): (3, 1)
/// - 4 (Top right): (2, 0), 5 (Top left): (1, 0)
/// - 6 (Left bottom): (0, 1), 7 (Left top): (0, 2)
fn calculate_segment_distance(entry: TileEndpoint, exit: TileEndpoint) -> f32 {
    let entry_pos = match entry {
        0 => (1.0, 3.0),
        1 => (2.0, 3.0),
        2 => (3.0, 2.0),
        3 => (3.0, 1.0),
        4 => (2.0, 0.0),
        5 => (1.0, 0.0),
        6 => (0.0, 1.0),
        7 => (0.0, 2.0),
        _ => panic!("Invalid entry endpoint: {}", entry),
    };

    let exit_pos = match exit {
        0 => (1.0, 3.0),
        1 => (2.0, 3.0),
        2 => (3.0, 2.0),
        3 => (3.0, 1.0),
        4 => (2.0, 0.0),
        5 => (1.0, 0.0),
        6 => (0.0, 1.0),
        7 => (0.0, 2.0),
        _ => panic!("Invalid exit endpoint: {}", exit),
    };

    // Calculate Euclidean distance
    let dx = exit_pos.0 - entry_pos.0;
    let dy = exit_pos.1 - entry_pos.1;
    ((dx * dx + dy * dy) as f32).sqrt()
}

/// Statistics tracked for each player during a game
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerStats {
    pub player_id: PlayerID,
    pub turns_survived: usize, // Number of turns the player stayed alive
    pub tiles_placed: usize,   // Number of tiles placed before elimination
    pub path_length: usize,    // Number of cells traversed (counts revisits)
    pub trail_distance: f32,   // Actual geometric distance traveled along paths
    pub hand_tiles_remaining: usize, // Tiles in hand when eliminated
    pub elimination_turn: Option<usize>, // Turn number when eliminated (None if winner)
    pub players_eliminated: usize, // Number of other players this player eliminated
    pub unique_tiles_visited: usize, // Number of distinct board tiles visited
    pub max_visits_to_single_tile: usize, // Highest visit count to any single tile
    #[serde(skip)]
    pub cells_visited: std::collections::HashMap<CellCoord, usize>, // Visit count per cell (not exported)
}

/// Why a move was rejected by the engine. The `Display` text is what players
/// ultimately see (the server forwards it in `ServerMessage::Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    NotYourTurn,
    PlayerNotInGame,
    WrongCell,
    CellOccupied,
    TileNotInHand,
    /// Tsuro's forced-suicide rule: a self-eliminating placement is only
    /// legal when every playable option eliminates the mover too.
    ForcedSuicide,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MoveError::NotYourTurn => "it's not this player's turn",
            MoveError::PlayerNotInGame => "the player is not in the game or was eliminated",
            MoveError::WrongCell => "the tile must be placed on the player's current cell",
            MoveError::CellOccupied => "that cell already has a tile",
            MoveError::TileNotInHand => "the player does not have this tile in hand",
            MoveError::ForcedSuicide => {
                "that placement would eliminate you while another of your moves survives"
            }
        })
    }
}

impl std::error::Error for MoveError {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TurnResult {
    TurnAdvanced {
        turn_number: usize,
        next_player: PlayerID,
        eliminated: Vec<PlayerID>,
    },
    PlayerWins {
        turn_number: usize,
        winner: PlayerID,
        eliminated: Vec<PlayerID>,
    },
    Extinction {
        turn_number: usize,
        eliminated: Vec<PlayerID>,
    },
}

/// Game mode type for export metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    Local,
    Online,
}

/// Metadata about an exported game
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameMetadata {
    pub game_mode: GameMode,
    pub room_id: Option<String>,
    pub room_name: Option<String>,
    pub completed: bool,
    pub winner_id: Option<PlayerID>,
    pub total_turns: usize,
    pub player_names: Vec<(PlayerID, String)>,
}

/// Complete game export with metadata for saving/loading replays
/// For in-progress games, only the exporting player's hand is included (other hands show counts only)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameExport {
    pub version: String,
    pub timestamp: String,
    pub metadata: GameMetadata,
    pub game_state: Game,
    pub player_perspective: Option<PlayerID>, // Whose perspective this export is from (for partial info)
    pub hand_counts: std::collections::HashMap<PlayerID, usize>, // Tile counts for each player
    pub deck_count: usize,                    // Number of tiles remaining in deck
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub deck: Deck,
    pub board: Board,
    pub players: Vec<Player>,
    pub hands: HashMap<PlayerID, Vec<Tile>>,
    pub tile_trails: Vec<(CellCoord, Vec<(PlayerID, TileEndpoint)>)>, // tile -> list of (player, segment) pairs
    pub player_trails: HashMap<PlayerID, crate::trail::Trail>, // Complete cumulative trail for each player
    pub current_turn_trails: HashMap<PlayerID, crate::trail::Trail>, // Trails from just this turn (for animation)
    pub current_player_id: PlayerID,
    pub stats: HashMap<PlayerID, PlayerStats>, // Statistics for each player
}

impl Game {
    pub fn new(players: Vec<Player>) -> Game {
        let mut deck = Deck::new();
        let mut hands = HashMap::new();
        let board = Board::new();

        for player in &players {
            hands.insert(player.id, deck.take_up_to(3));
        }

        let current_player_id = if players.is_empty() { 1 } else { players[0].id };

        // Initialize trails with each player's starting position
        let mut player_trails = HashMap::new();
        for player in &players {
            player_trails.insert(player.id, Trail::new(player.pos));
        }

        // Initialize stats for each player
        let mut stats = HashMap::new();
        for player in &players {
            let mut cells_visited = std::collections::HashMap::new();
            cells_visited.insert(player.pos.cell, 1); // Start position visited once

            stats.insert(
                player.id,
                PlayerStats {
                    player_id: player.id,
                    turns_survived: 0,
                    tiles_placed: 0,
                    path_length: 1,      // Start with 1 for their starting position
                    trail_distance: 0.0, // No distance traveled yet
                    hand_tiles_remaining: 3, // Starting hand size
                    elimination_turn: None,
                    players_eliminated: 0,
                    unique_tiles_visited: 1, // Start with 1 for starting position
                    max_visits_to_single_tile: 1,
                    cells_visited,
                },
            );
        }

        Game {
            players,
            hands,
            deck,
            board,
            tile_trails: Vec::new(),
            player_trails,
            current_turn_trails: HashMap::new(),
            current_player_id,
            stats,
        }
    }

    pub fn curr_player_hand(&self) -> Vec<Tile> {
        self.hands[&self.current_player_id].clone()
    }

    /// Returns true if the game is over (1 or fewer players alive)
    pub fn is_game_over(&self) -> bool {
        self.players.iter().filter(|p| p.alive).count() <= 1
    }

    /// How many tiles each player currently holds. Tile *counts* aren't secret
    /// (opponents' hand sizes are shown in the UI); the tile identities are.
    pub fn hand_counts(&self) -> HashMap<PlayerID, usize> {
        self.hands
            .iter()
            .map(|(id, hand)| (*id, hand.len()))
            .collect()
    }

    /// Number of tiles left in the deck.
    pub fn deck_count(&self) -> usize {
        self.deck.remaining()
    }

    /// A copy of this game redacted so `viewer` sees only what it may legitimately
    /// see: every hand except the viewer's is emptied and the deck order is hidden.
    /// A `viewer` of `None` (a spectator) sees no hands at all. This is what the
    /// server sends across the connection boundary so opponents' tiles and the
    /// upcoming draws never reach a client that could read them off the wire.
    /// Counts survive redaction — read them from [`hand_counts`](Self::hand_counts)
    /// / [`deck_count`](Self::deck_count) on the full game before redacting.
    pub fn view_for(&self, viewer: Option<PlayerID>) -> Game {
        let mut view = self.clone();
        for (id, hand) in view.hands.iter_mut() {
            if Some(*id) != viewer {
                hand.clear();
            }
        }
        view.deck = Deck::new_empty();
        view
    }

    pub fn perform_move(&mut self, mov: Move) -> Result<TurnResult, MoveError> {
        // Tsuro's forced-suicide rule: reject a self-eliminating placement
        // while a surviving option exists. The simulation also surfaces any
        // basic validation error before the game is touched.
        let eliminated = self.simulate_move(mov)?;
        if eliminated.contains(&mov.player_id) && !self.survivable_moves(mov.player_id).is_empty() {
            return Err(MoveError::ForcedSuicide);
        }
        self.apply_move(mov)
    }

    /// Play a move on a clone of the game and report who it would eliminate,
    /// without the forced-suicide rule (it is the primitive that rule is built
    /// from). Reuses the whole engine — traversal, edge elimination, turn
    /// bookkeeping — so it can never disagree with a real move.
    pub fn simulate_move(&self, mov: Move) -> Result<Vec<PlayerID>, MoveError> {
        let mut sim = self.clone();
        let result = sim.apply_move(mov)?;
        Ok(match result {
            TurnResult::TurnAdvanced { eliminated, .. }
            | TurnResult::PlayerWins { eliminated, .. }
            | TurnResult::Extinction { eliminated, .. } => eliminated,
        })
    }

    /// Every distinct placement available to `player_id`: each hand tile in
    /// each of its 4 rotations on the player's (forced) cell, with rotations
    /// that repeat a symmetric tile's shape counted once.
    pub fn playable_moves(&self, player_id: PlayerID) -> Vec<Move> {
        let Some(player) = self.players.iter().find(|p| p.id == player_id && p.alive) else {
            return Vec::new();
        };
        let Some(hand) = self.hands.get(&player_id) else {
            return Vec::new();
        };
        let mut moves: Vec<Move> = Vec::new();
        for tile in hand {
            let mut rotated = *tile;
            for _ in 0..4 {
                if !moves.iter().any(|m| m.tile == rotated) {
                    moves.push(Move {
                        tile: rotated,
                        cell: player.pos.cell,
                        player_id,
                    });
                }
                rotated = rotated.rotated(true);
            }
        }
        moves
    }

    /// The playable moves that do not eliminate the mover.
    pub fn survivable_moves(&self, player_id: PlayerID) -> Vec<Move> {
        self.playable_moves(player_id)
            .into_iter()
            .filter(|mov| {
                matches!(self.simulate_move(*mov), Ok(eliminated) if !eliminated.contains(&player_id))
            })
            .collect()
    }

    /// A uniformly random move for a timed-out player: survivable if any
    /// exist, otherwise any placement (the rules require a play even when
    /// every option is fatal). `None` only when the player cannot move at all
    /// (no tiles in hand).
    pub fn random_timeout_move(&self, player_id: PlayerID) -> Option<Move> {
        use rand::seq::SliceRandom;
        let survivable = self.survivable_moves(player_id);
        let pool = if survivable.is_empty() {
            self.playable_moves(player_id)
        } else {
            survivable
        };
        pool.choose(&mut rand::thread_rng()).copied()
    }

    fn apply_move(&mut self, mov: Move) -> Result<TurnResult, MoveError> {
        // Validate it's this player's turn
        if mov.player_id != self.current_player_id {
            return Err(MoveError::NotYourTurn);
        }

        // Basic validation
        let player = self
            .players
            .iter()
            .find(|p| p.id == mov.player_id && p.alive)
            .ok_or(MoveError::PlayerNotInGame)?;

        // The tile must go on the cell the player's pawn occupies — the UI already
        // enforces this, but the engine is the authority (hand-rolled clients must
        // not be able to place tiles elsewhere on the board)
        if player.pos.cell != mov.cell {
            return Err(MoveError::WrongCell);
        }

        // Check if cell is already occupied
        if self.board.get_tile_at(mov.cell).is_some() {
            return Err(MoveError::CellOccupied);
        }

        // Validate player has the tile in hand
        self.deduct_tile_from_hand(mov)?;

        // Update stats: increment tiles_placed for the current player
        if let Some(stats) = self.stats.get_mut(&mov.player_id) {
            stats.tiles_placed += 1;
        }

        // Place the tile on the board
        self.board.place_tile(mov);

        // Update player positions and record trails for players who actually move
        let eliminated = self.update_players_and_trails(mov.cell);

        // Credit current player with eliminations (excluding self-elimination)
        if !eliminated.is_empty() {
            if let Some(stats) = self.stats.get_mut(&mov.player_id) {
                let self_eliminations =
                    eliminated.iter().filter(|&&id| id == mov.player_id).count();
                stats.players_eliminated += eliminated.len() - self_eliminations;
            }
        }

        // Refill hands (basic implementation for now)
        self.fill_hands();

        // Complete the turn (advances to next player)
        let turn_result = self.complete_turn(eliminated);

        Ok(turn_result)
    }

    fn deduct_tile_from_hand(&mut self, mov: Move) -> Result<(), MoveError> {
        let hand = self
            .hands
            .get_mut(&mov.player_id)
            .expect("Player should exist");
        // Use rotation-invariant comparison: player may have rotated the tile before playing
        if let Some(pos) = hand.iter().position(|tile| tile.is_same_tile(&mov.tile)) {
            hand.remove(pos);
            Ok(())
        } else {
            Err(MoveError::TileNotInHand)
        }
    }

    fn update_players_and_trails(&mut self, placed_cell: CellCoord) -> Vec<PlayerID> {
        let mut eliminated = Vec::new();
        let mut trails_to_record = Vec::new();

        // Clear current turn trails before processing new movements
        self.current_turn_trails.clear();

        for player in alive_players(&mut self.players) {
            // Only update players who are in the cell where the tile was placed
            if player.pos.cell != placed_cell {
                continue;
            }
            let old_pos = player.pos;
            let trail = self.board.traverse_from(player.pos);
            let new_pos = trail.end_pos;

            player.has_moved = old_pos != new_pos;

            if player.has_moved {
                trails_to_record.push((player.id, old_pos)); // Record where they came FROM

                // Store this turn's trail for animation (just the new movement)
                self.current_turn_trails.insert(player.id, trail.clone());

                // Extend player's cumulative trail with new segments and calculate distance
                if let Some(player_trail) = self.player_trails.get_mut(&player.id) {
                    for segment in &trail.segments {
                        player_trail.add_segment(segment.clone());

                        // Calculate and add geometric distance for this segment
                        if let Some(stats) = self.stats.get_mut(&player.id) {
                            let distance =
                                calculate_segment_distance(segment.entry_point, segment.exit_point);
                            stats.trail_distance += distance;
                        }
                    }
                    player_trail.end_pos = new_pos;
                    player_trail.completed = trail.completed;
                }

                // Update stats: count every cell the trail entered this move,
                // revisits included. Each segment is one tile traversed, so the
                // cells entered are each segment's cell after the first, plus the
                // final position when the path ran off the last tile into a new
                // cell (an edge exit stays on the same cell and enters nothing).
                if let Some(stats) = self.stats.get_mut(&player.id) {
                    let segment_cell = |s: &crate::trail::TrailSegment| CellCoord {
                        row: s.board_pos.0,
                        col: s.board_pos.1,
                    };
                    let entered = trail.segments.iter().skip(1).map(segment_cell).chain(
                        (trail.segments.last().map(segment_cell) != Some(new_pos.cell))
                            .then_some(new_pos.cell),
                    );
                    for cell in entered {
                        stats.path_length += 1;
                        let visit_count = stats.cells_visited.entry(cell).or_insert(0);
                        *visit_count += 1;
                        stats.max_visits_to_single_tile =
                            stats.max_visits_to_single_tile.max(*visit_count);
                    }
                    stats.unique_tiles_visited = stats.cells_visited.len();
                }
            }

            player.pos = new_pos;

            // Only eliminate players who have moved and are now on edge
            if player.has_moved && new_pos.on_edge() {
                player.alive = false;
                eliminated.push(player.id);
                self.deck.put(self.hands.get_mut(&player.id).expect("hand"));
            }
        }

        // Record all trails after updating players
        for (player_id, from_pos) in trails_to_record {
            self.record_player_trail(player_id, from_pos);
        }

        eliminated
    }

    fn record_player_trail(&mut self, player_id: PlayerID, exit_pos: PlayerPos) {
        // Record trail for the cell the player just exited from
        if let Some(tile) = self.board.get_tile_at(exit_pos.cell) {
            // Find which segment this player used to exit this tile
            let segment = tile
                .segments
                .iter()
                .find(|&seg| seg.a == exit_pos.endpoint || seg.b == exit_pos.endpoint);

            if let Some(segment) = segment {
                // Use min(from, to) convention for segment key
                let segment_key = min(segment.a, segment.b);

                // Record that this player used this segment (every time they pass through)
                // Find existing entry for this cell or add a new one
                if let Some((_, trail_entries)) = self
                    .tile_trails
                    .iter_mut()
                    .find(|(cell, _)| cell == &exit_pos.cell)
                {
                    trail_entries.push((player_id, segment_key));
                } else {
                    self.tile_trails
                        .push((exit_pos.cell, vec![(player_id, segment_key)]));
                }
            }
        }
    }
    fn fill_hands(&mut self) {
        for player in &self.players {
            if player.alive {
                if let Some(hand) = self.hands.get_mut(&player.id) {
                    while hand.len() < 3 && !self.deck.is_empty() {
                        if let Some(tile) = self.deck.take() {
                            hand.push(tile);
                        }
                    }
                }
            }
        }
    }

    /// Remove a player from active play outside of a normal move (disconnect,
    /// forfeit). Mirrors the bookkeeping of an on-board elimination: the player
    /// is marked dead, their stats are closed out, their hand returns to the
    /// deck, and — if it was their turn — the turn passes to the next alive
    /// player in rotation order.
    /// Returns false (and does nothing) if the player is unknown or already out.
    pub fn eliminate_player(&mut self, player_id: PlayerID) -> bool {
        let Some(player) = self
            .players
            .iter_mut()
            .find(|p| p.id == player_id && p.alive)
        else {
            return false;
        };
        player.alive = false;

        if let Some(stats) = self.stats.get_mut(&player_id) {
            stats.elimination_turn = Some(self.board.history.len());
            stats.hand_tiles_remaining = self.hands.get(&player_id).map(|h| h.len()).unwrap_or(0);
        }

        if let Some(hand) = self.hands.get_mut(&player_id) {
            self.deck.put(hand);
        }

        if self.current_player_id == player_id {
            if let Some(next_id) = self.next_active_player_id() {
                self.current_player_id = next_id;
            }
        }

        true
    }

    fn next_active_player_id(&self) -> Option<PlayerID> {
        // Find the current player index
        if let Some(current_index) = self
            .players
            .iter()
            .position(|p| p.id == self.current_player_id)
        {
            // Find the next alive player
            let player_count = self.players.len();
            for i in 1..=player_count {
                let next_index = (current_index + i) % player_count;
                let next_player = &self.players[next_index];

                if next_player.alive {
                    return Some(next_player.id);
                }
            }
        }
        None // No alive players found
    }

    fn complete_turn(&mut self, eliminated: Vec<PlayerID>) -> TurnResult {
        let turn_number = self.board.history.len(); // Turn number starts from 1

        // Update stats for eliminated players
        for &player_id in &eliminated {
            if let Some(stats) = self.stats.get_mut(&player_id) {
                stats.elimination_turn = Some(turn_number);
                stats.hand_tiles_remaining =
                    self.hands.get(&player_id).map(|h| h.len()).unwrap_or(0);
            }
        }

        // Increment turns_survived for all alive players
        for player in &self.players {
            if player.alive {
                if let Some(stats) = self.stats.get_mut(&player.id) {
                    stats.turns_survived += 1;
                }
            }
        }

        // Count remaining alive players
        let alive_count = self.players.iter().filter(|p| p.alive).count();

        match alive_count {
            0 => TurnResult::Extinction {
                turn_number,
                eliminated,
            },
            1 => {
                // Find the winner
                let winner = self
                    .players
                    .iter()
                    .find(|p| p.alive)
                    .map(|p| p.id)
                    .expect("Should have exactly one alive player");

                TurnResult::PlayerWins {
                    turn_number,
                    winner,
                    eliminated,
                }
            }
            _ => {
                // Game continues - advance to next player
                if let Some(next_player_id) = self.next_active_player_id() {
                    self.current_player_id = next_player_id;
                    TurnResult::TurnAdvanced {
                        turn_number,
                        next_player: next_player_id,
                        eliminated,
                    }
                } else {
                    // This shouldn't happen if alive_count > 0, but handle it gracefully
                    TurnResult::Extinction {
                        turn_number,
                        eliminated,
                    }
                }
            }
        }
    }

    /// Export the game with metadata for saving as a replay file
    /// For in-progress games, only the specified player's hand is included (others show empty)
    pub fn export(
        &self,
        metadata: GameMetadata,
        player_perspective: Option<PlayerID>,
    ) -> GameExport {
        let mut game_state = self.clone();

        // Capture counts before filtering
        let hand_counts = self.hand_counts();
        let deck_count = self.deck_count();

        // If game is not completed and a player perspective is specified, filter hands
        if !metadata.completed {
            if let Some(player_id) = player_perspective {
                // Clear all hands except the exporting player's hand
                for (id, hand) in game_state.hands.iter_mut() {
                    if *id != player_id {
                        hand.clear(); // Other players' hands are hidden
                    }
                }
            }

            // Clear deck tiles (keep the Deck struct but empty it for in-progress games)
            game_state.deck = Deck::new_empty();
        }

        GameExport {
            version: "1.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata,
            game_state,
            player_perspective,
            hand_counts,
            deck_count,
        }
    }

    /// Rebuild a game state from move history
    /// This is used for replay functionality - reconstructs the game by replaying all moves
    pub fn rebuild_from_history(
        initial_players: Vec<Player>,
        moves: &[Move],
    ) -> Result<Game, MoveError> {
        let mut game = Game::new(initial_players);

        for mov in moves {
            // For replay, we need to ensure the tile is in the player's hand
            // Add it temporarily if not present (since we don't know original deck order)
            let has_tile = game
                .hands
                .get(&mov.player_id)
                .map(|hand| hand.iter().any(|t| t.is_same_tile(&mov.tile)))
                .unwrap_or(false);

            if !has_tile {
                if let Some(hand) = game.hands.get_mut(&mov.player_id) {
                    hand.push(mov.tile);
                }
            }

            game.perform_move(*mov)?;
        }

        Ok(game)
    }
}

fn alive_players(players: &mut [Player]) -> Vec<&mut Player> {
    players.iter_mut().filter(|player| player.alive).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{CellCoord, PlayerPos};

    fn create_straight_tile() -> Tile {
        // Create a tile with straight connections: 0-1, 2-3, 4-5, 6-7
        Tile {
            segments: [
                Segment { a: 0, b: 1 },
                Segment { a: 2, b: 3 },
                Segment { a: 4, b: 5 },
                Segment { a: 6, b: 7 },
            ],
        }
    }

    fn create_curve_tile() -> Tile {
        // Create a tile with curves: 0-2, 1-3, 4-6, 5-7
        Tile {
            segments: [
                Segment { a: 0, b: 2 },
                Segment { a: 1, b: 3 },
                Segment { a: 4, b: 6 },
                Segment { a: 5, b: 7 },
            ],
        }
    }

    #[test]
    fn test_trail_recording_single_player_single_pass() {
        let players = vec![Player {
            id: 1,
            name: "Player 1".to_string(),
            pos: PlayerPos {
                cell: CellCoord { row: 1, col: 1 },
                endpoint: 0,
            },
            alive: true,
            has_moved: false,
            color: (255, 0, 0), // Red
        }];

        let mut game = Game::new(players);

        // Place a straight tile at (1,1) where the player is
        let tile = create_straight_tile(); // 0-1, 2-3, 4-5, 6-7
        let mov = Move {
            player_id: 1,
            cell: CellCoord { row: 1, col: 1 },
            tile,
        };

        // Add tile to player's hand first
        game.hands.get_mut(&1).unwrap().push(mov.tile);

        // Perform the move
        let result = game.perform_move(mov);
        assert!(result.is_ok());

        // Check that trail was recorded for the tile
        let trail_entries = game
            .tile_trails
            .iter()
            .find(|(coord, _)| coord == &CellCoord { row: 1, col: 1 })
            .map(|(_, entries)| entries)
            .unwrap();

        // Player started at endpoint 0, tile connects 0-1, so segment key should be min(0,1) = 0
        assert_eq!(trail_entries.len(), 1);
        assert_eq!(trail_entries[0], (1, 0)); // (player_id, segment_key)
    }

    #[test]
    fn test_trail_recording_multiple_players_same_tile() {
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 1, col: 1 },
                    endpoint: 0,
                },
                alive: true,
                has_moved: false,
                color: (255, 0, 0), // Red
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 1, col: 1 },
                    endpoint: 2,
                },
                alive: true,
                has_moved: false,
                color: (0, 255, 0), // Green
            },
        ];

        let mut game = Game::new(players);

        // Place a straight tile at (1,1) where both players are
        let tile = create_straight_tile(); // 0-1, 2-3, 4-5, 6-7
        let mov = Move {
            player_id: 1,
            cell: CellCoord { row: 1, col: 1 },
            tile,
        };

        game.hands.get_mut(&1).unwrap().push(mov.tile);

        let result = game.perform_move(mov);
        assert!(result.is_ok());

        // Check that trails were recorded for both players
        let trail_entries = game
            .tile_trails
            .iter()
            .find(|(coord, _)| coord == &CellCoord { row: 1, col: 1 })
            .map(|(_, entries)| entries)
            .unwrap();

        // Should have exactly 2 entries: one for each player
        assert_eq!(trail_entries.len(), 2);

        // Player 1 started at endpoint 0, uses segment 0-1 (key = 0)
        // Player 2 started at endpoint 2, uses segment 2-3 (key = 2)
        let mut found_player1 = false;
        let mut found_player2 = false;

        for &(player_id, segment_key) in trail_entries {
            if player_id == 1 {
                assert_eq!(segment_key, 0); // segment 0-1, key = min(0,1) = 0
                found_player1 = true;
            } else if player_id == 2 {
                assert_eq!(segment_key, 2); // segment 2-3, key = min(2,3) = 2
                found_player2 = true;
            }
        }

        assert!(found_player1, "Player 1 trail not found");
        assert!(found_player2, "Player 2 trail not found");
    }

    #[test]
    fn test_trail_recording_multiple_players_pass_through_same_tile_twice() {
        // Create a scenario where players will naturally return to the same tile
        // Start both players in different cells but positioned to enter the same target tile
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 1, col: 1 },
                    endpoint: 3,
                }, // Will move right to (1,2)
                alive: true,
                has_moved: false,
                color: (255, 0, 0), // Red
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 2, col: 2 },
                    endpoint: 0,
                }, // Will move up to (1,2)
                alive: true,
                has_moved: false,
                color: (0, 255, 0), // Green
            },
        ];

        let mut game = Game::new(players);

        // Step 1: Place a tile at (1,1) that sends player 1 to (1,2)
        // Player 1 is at endpoint 3, so we need a tile that connects 3 to an exit going right
        let tile1 = create_straight_tile(); // 0-1, 2-3, 4-5, 6-7
                                            // Player 1 at endpoint 3 will use segment 2-3, exiting at endpoint 2 (which goes right)
        let mov1 = Move {
            player_id: 1,
            cell: CellCoord { row: 1, col: 1 },
            tile: tile1,
        };
        game.hands.get_mut(&1).unwrap().push(mov1.tile);

        let result1 = game.perform_move(mov1);
        assert!(result1.is_ok());

        // Check that player 1 moved to a different cell and trail was recorded
        let trail_entries_1_1 = game
            .tile_trails
            .iter()
            .find(|(coord, _)| coord == &CellCoord { row: 1, col: 1 })
            .map(|(_, entries)| entries);
        if let Some(entries) = trail_entries_1_1 {
            assert!(entries.contains(&(1, 2))); // Player 1 used segment 2-3 (key=2)
        }

        // Step 2: Place a tile at (2,2) that sends player 2 to meet player 1
        let tile2 = create_curve_tile(); // 0-2, 1-3, 4-6, 5-7
                                         // Player 2 at endpoint 0 will use segment 0-2, exiting at endpoint 2 (which should go up)
        let mov2 = Move {
            player_id: 2,
            cell: CellCoord { row: 2, col: 2 },
            tile: tile2,
        };
        game.hands.get_mut(&2).unwrap().push(mov2.tile);

        let result2 = game.perform_move(mov2);
        assert!(result2.is_ok());

        // Manually position both players in cell (1,2) to simulate they arrived there
        game.players[0].pos = PlayerPos {
            cell: CellCoord { row: 1, col: 2 },
            endpoint: 0,
        };
        game.players[1].pos = PlayerPos {
            cell: CellCoord { row: 1, col: 2 },
            endpoint: 4,
        };

        // Place a tile at (1,2) that will cause both to traverse
        let tile3 = create_straight_tile(); // 0-1, 2-3, 4-5, 6-7
        let mov3 = Move {
            player_id: 1,
            cell: CellCoord { row: 1, col: 2 },
            tile: tile3,
        };
        game.hands.get_mut(&1).unwrap().push(mov3.tile);

        let result3 = game.perform_move(mov3);
        assert!(result3.is_ok());

        // Verify that both players' trails are recorded at (1,2)
        let trail_entries_1_2 = game
            .tile_trails
            .iter()
            .find(|(coord, _)| coord == &CellCoord { row: 1, col: 2 })
            .map(|(_, entries)| entries)
            .unwrap();
        assert!(trail_entries_1_2.contains(&(1, 0))); // Player 1, segment 0-1
        assert!(trail_entries_1_2.contains(&(2, 4))); // Player 2, segment 4-5

        // The test is complete - we've verified that multiple players can have their
        // trails recorded when passing through the same tile. Testing a second pass
        // through would require placing a tile at the same location which is invalid.
    }

    #[test]
    fn test_game_export_serialization() {
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 0, col: 2 },
                    endpoint: 4,
                },
                alive: true,
                has_moved: false,
                color: (255, 0, 0),
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 5, col: 3 },
                    endpoint: 0,
                },
                alive: true,
                has_moved: false,
                color: (0, 255, 0),
            },
        ];
        let game = Game::new(players);

        let metadata = GameMetadata {
            game_mode: GameMode::Local,
            room_id: None,
            room_name: Some("Test Game".to_string()),
            completed: false,
            winner_id: None,
            total_turns: 0,
            player_names: vec![(1, "Player 1".to_string()), (2, "Player 2".to_string())],
        };

        let export = game.export(metadata, Some(1));

        // Test serialization
        let json = serde_json::to_string_pretty(&export).expect("Failed to serialize");
        let deserialized: GameExport = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.version, "1.0");
        assert_eq!(deserialized.metadata.total_turns, 0);
        assert_eq!(deserialized.game_state.players.len(), 2);
        assert_eq!(deserialized.player_perspective, Some(1));
        assert_eq!(deserialized.hand_counts.get(&1), Some(&3)); // 3 tiles in hand
        assert_eq!(deserialized.hand_counts.get(&2), Some(&3));
    }

    #[test]
    fn test_export_hides_other_players_hands() {
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 0, col: 2 },
                    endpoint: 4,
                },
                alive: true,
                has_moved: false,
                color: (255, 0, 0),
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 5, col: 3 },
                    endpoint: 0,
                },
                alive: true,
                has_moved: false,
                color: (0, 255, 0),
            },
        ];
        let game = Game::new(players);

        let metadata = GameMetadata {
            game_mode: GameMode::Local,
            room_id: None,
            room_name: None,
            completed: false, // In-progress game
            winner_id: None,
            total_turns: 0,
            player_names: vec![(1, "Player 1".to_string()), (2, "Player 2".to_string())],
        };

        let export = game.export(metadata, Some(1)); // Export from player 1's perspective

        // Player 1's hand should be visible
        assert_eq!(export.game_state.hands.get(&1).unwrap().len(), 3);

        // Player 2's hand should be hidden (empty)
        assert_eq!(export.game_state.hands.get(&2).unwrap().len(), 0);

        // But counts should still be available
        assert_eq!(export.hand_counts.get(&1), Some(&3));
        assert_eq!(export.hand_counts.get(&2), Some(&3));
    }

    #[test]
    fn test_view_for_hides_other_hands_and_the_deck() {
        let game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        assert!(game.deck.remaining() > 0, "a fresh game has a stocked deck");

        // Player 1's view keeps player 1's hand and hides everyone else's.
        let view = game.view_for(Some(1));
        assert_eq!(
            view.hands[&1], game.hands[&1],
            "the viewer keeps their hand"
        );
        assert!(view.hands[&2].is_empty(), "opponents' tiles are hidden");
        assert_eq!(view.deck.remaining(), 0, "the deck order is hidden");

        // The full game reports true counts; the message layer captures these
        // before redacting so the UI can still show opponents' tile totals.
        assert_eq!(game.hand_counts()[&1], 3);
        assert_eq!(game.hand_counts()[&2], 3);
        assert_eq!(game.deck_count(), game.deck.remaining());

        // A spectator (no id) sees no hands at all.
        let spectator = game.view_for(None);
        assert!(
            spectator.hands.values().all(|h| h.is_empty()),
            "a spectator sees no tiles"
        );
        assert_eq!(spectator.deck.remaining(), 0);
        // But the board, players, and current turn are untouched.
        assert_eq!(spectator.current_player_id, game.current_player_id);
        assert_eq!(spectator.players.len(), game.players.len());
    }

    #[test]
    fn test_game_rebuild_from_history() {
        // Create a game and make some moves
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 1, col: 1 },
                    endpoint: 0,
                },
                alive: true,
                has_moved: false,
                color: (255, 0, 0),
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos {
                    cell: CellCoord { row: 2, col: 2 },
                    endpoint: 0,
                },
                alive: true,
                has_moved: false,
                color: (0, 255, 0),
            },
        ];
        let mut game = Game::new(players.clone());

        // Place a tile
        let tile = create_straight_tile();
        game.hands.get_mut(&1).unwrap().push(tile);

        let mov = Move {
            tile,
            cell: CellCoord { row: 1, col: 1 },
            player_id: 1,
        };

        game.perform_move(mov).expect("Move should succeed");

        // Now rebuild from history
        let rebuilt = Game::rebuild_from_history(players, &game.board.history)
            .expect("Rebuild should succeed");

        // Verify the rebuilt state matches
        assert_eq!(rebuilt.board.history.len(), game.board.history.len());
        assert_eq!(rebuilt.players.len(), game.players.len());
        assert_eq!(rebuilt.board.history[0].player_id, 1);
    }

    #[test]
    fn test_turn_skips_eliminated_player() {
        // Three players; the middle one is already out. Player 1 makes a surviving
        // move, so the turn must jump straight to player 3, skipping player 2.
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(2, 2, 0)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
            Player::new(3, PlayerPos::new(2, 0, 6)),
        ]);
        game.players[1].alive = false; // player 2 eliminated before this turn

        // A straight tile at (2,2) carries player 1 from endpoint 0 into the empty
        // interior cell (3,2), so they survive.
        let tile = create_straight_tile();
        game.hands
            .get_mut(&1)
            .expect("player 1 has a hand")
            .push(tile);

        let result = game
            .perform_move(Move {
                tile,
                cell: CellCoord { row: 2, col: 2 },
                player_id: 1,
            })
            .expect("player 1's move should be legal");

        match result {
            TurnResult::TurnAdvanced {
                next_player,
                eliminated,
                ..
            } => {
                assert_eq!(next_player, 3, "turn should skip eliminated player 2");
                assert!(eliminated.is_empty(), "player 1 should survive this move");
            }
            other => panic!("expected TurnAdvanced, got {:?}", other),
        }
        assert_eq!(game.current_player_id, 3);
    }

    #[test]
    fn test_last_survivor_wins_when_move_eliminates_the_rest() {
        // Players 1 and 3 are alive (player 2 already out). Player 1 drives off the
        // top edge, leaving player 3 as the sole survivor.
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
            Player::new(3, PlayerPos::new(2, 0, 6)),
        ]);
        game.players[1].alive = false;

        // The straight tile connects endpoints 5 and 4, both on the top edge, so
        // player 1 immediately exits the board and is eliminated. It must be
        // their whole hand: with any surviving alternative the forced-suicide
        // rule would reject this placement.
        let tile = create_straight_tile();
        game.hands.insert(1, vec![tile]);

        let result = game
            .perform_move(Move {
                tile,
                cell: CellCoord { row: 0, col: 2 },
                player_id: 1,
            })
            .expect("player 1's move should be legal");

        match result {
            TurnResult::PlayerWins {
                winner, eliminated, ..
            } => {
                assert_eq!(winner, 3, "player 3 is the last one standing");
                assert_eq!(eliminated, vec![1], "player 1 eliminated themselves");
            }
            other => panic!("expected PlayerWins, got {:?}", other),
        }
        assert!(!game.players.iter().find(|p| p.id == 1).unwrap().alive);
    }

    #[test]
    fn test_extinction_when_final_move_eliminates_all_remaining() {
        // Both remaining players share the top-edge cell and are driven off it by
        // the same tile, leaving nobody alive.
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(0, 2, 4)),
        ]);

        // Fatal-only hand: the forced-suicide rule permits a self-eliminating
        // placement only when no alternative survives.
        let tile = create_straight_tile();
        game.hands.insert(1, vec![tile]);

        let result = game
            .perform_move(Move {
                tile,
                cell: CellCoord { row: 0, col: 2 },
                player_id: 1,
            })
            .expect("player 1's move should be legal");

        match result {
            TurnResult::Extinction { eliminated, .. } => {
                assert_eq!(
                    eliminated,
                    vec![1, 2],
                    "both players are eliminated together"
                );
            }
            other => panic!("expected Extinction, got {:?}", other),
        }
        assert!(game.players.iter().all(|p| !p.alive));
    }

    #[test]
    fn test_move_rejected_unless_placed_on_players_cell() {
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(2, 2, 0)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        let tile = create_straight_tile();
        game.hands
            .get_mut(&1)
            .expect("player 1 has a hand")
            .push(tile);

        // Player 1's pawn is at (2,2); placing on any other cell is illegal even
        // though the cell is empty and the tile is in hand.
        let err = game
            .perform_move(Move {
                tile,
                cell: CellCoord { row: 4, col: 4 },
                player_id: 1,
            })
            .expect_err("placing away from the pawn must be rejected");
        assert_eq!(err, MoveError::WrongCell);
        assert!(
            game.board.history.is_empty(),
            "rejected move must not reach the board"
        );
        assert_eq!(
            game.hands[&1].len(),
            4,
            "rejected move must not consume the tile"
        );

        // The same tile on the pawn's own cell is legal.
        game.perform_move(Move {
            tile,
            cell: CellCoord { row: 2, col: 2 },
            player_id: 1,
        })
        .expect("placing on the pawn's cell is legal");
    }

    #[test]
    fn test_stats_count_every_cell_of_a_multi_tile_move() {
        // Player 1 sits at (1,1) entry 7. A tile is already on the board at
        // (1,2); placing one at (1,1) carries the pawn through BOTH tiles to
        // (1,3). Path length and cell visits must count each cell entered.
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(1, 1, 7)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        // Tile whose 3-7 and 2-6 segments carry the pawn one cell to the right
        let tile = Tile::new([seg(0, 1), seg(2, 6), seg(3, 7), seg(4, 5)]);

        // Pre-place a tile at (1,2) directly on the board (as if from earlier play)
        game.board.place_tile(Move {
            tile,
            cell: CellCoord { row: 1, col: 2 },
            player_id: 2,
        });

        game.hands
            .get_mut(&1)
            .expect("player 1 has a hand")
            .push(tile);
        game.perform_move(Move {
            tile,
            cell: CellCoord { row: 1, col: 1 },
            player_id: 1,
        })
        .expect("placing on the pawn's cell is legal");

        assert_eq!(
            game.players[0].pos.cell,
            CellCoord { row: 1, col: 3 },
            "pawn should traverse both tiles"
        );
        let stats = &game.stats[&1];
        assert_eq!(
            stats.path_length, 3,
            "start cell + two cells entered during the move"
        );
        assert_eq!(
            stats.unique_tiles_visited, 3,
            "cells (1,1), (1,2), (1,3) all visited"
        );
        assert_eq!(stats.cells_visited[&CellCoord { row: 1, col: 2 }], 1);
        assert_eq!(stats.cells_visited[&CellCoord { row: 1, col: 3 }], 1);
        assert_eq!(stats.max_visits_to_single_tile, 1);
    }

    #[test]
    fn test_eliminate_current_player_passes_turn_forward_in_rotation() {
        // Current player is 2 (mid-rotation). Eliminating them must pass the
        // turn forward to player 3 — not back to the first alive player (1).
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(2, 2, 0)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
            Player::new(3, PlayerPos::new(2, 0, 6)),
        ]);
        game.current_player_id = 2;
        let deck_before = game.deck.remaining();

        assert!(game.eliminate_player(2));

        assert!(!game.players[1].alive, "player 2 is out");
        assert_eq!(
            game.current_player_id, 3,
            "turn continues forward in rotation, not back to player 1"
        );
        assert_eq!(
            game.deck.remaining(),
            deck_before + 3,
            "the eliminated player's hand returns to the deck"
        );
        assert!(game.hands[&2].is_empty(), "hand is emptied");
        assert_eq!(game.stats[&2].elimination_turn, Some(0));
        assert_eq!(game.stats[&2].hand_tiles_remaining, 3);

        // Eliminating an already-dead or unknown player is a no-op.
        assert!(!game.eliminate_player(2));
        assert!(!game.eliminate_player(99));
        assert_eq!(game.current_player_id, 3);
    }

    #[test]
    fn test_eliminate_non_current_player_leaves_turn_untouched() {
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(2, 2, 0)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
            Player::new(3, PlayerPos::new(2, 0, 6)),
        ]);
        assert_eq!(game.current_player_id, 1);

        assert!(game.eliminate_player(3));

        assert_eq!(game.current_player_id, 1, "turn stays with player 1");
        assert!(!game.players[2].alive);
        assert!(
            !game.is_game_over(),
            "two players remain, the game continues"
        );

        // Eliminating one of the two survivors ends the game.
        assert!(game.eliminate_player(2));
        assert!(game.is_game_over(), "one player left means game over");
    }

    /// Two players; player 1 sits on the top edge of (0,2) with a hand chosen
    /// by the test, so forced-suicide scenarios are deterministic.
    fn suicide_rule_game(hand: Vec<Tile>) -> Game {
        let mut game = Game::new(vec![
            Player::new(1, PlayerPos::new(0, 2, 5)),
            Player::new(2, PlayerPos::new(5, 3, 0)),
        ]);
        game.hands.insert(1, hand);
        game
    }

    /// Fatal for a pawn at endpoint 5 of a top-edge cell in every rotation:
    /// the segment set {(0,1),(2,3),(4,5),(6,7)} maps to itself under
    /// rotation, and 5 → 4 stays on the top edge.
    fn killer_tile() -> Tile {
        Tile::new([seg(4, 5), seg(0, 1), seg(2, 3), seg(6, 7)])
    }

    /// Survivable as dealt: 5 → 0 leads down into the empty (1,2). Its 90°
    /// rotation routes 5 → 4 (fatal), and it is 180°-symmetric, so exactly
    /// one of its two distinct rotations survives.
    fn survivor_tile() -> Tile {
        Tile::new([seg(5, 0), seg(4, 1), seg(2, 3), seg(6, 7)])
    }

    fn move_of(tile: Tile) -> Move {
        Move {
            tile,
            cell: CellCoord { row: 0, col: 2 },
            player_id: 1,
        }
    }

    #[test]
    fn test_simulate_move_reports_eliminations_without_mutating() {
        let game = suicide_rule_game(vec![killer_tile(), survivor_tile()]);

        let eliminated = game
            .simulate_move(move_of(killer_tile()))
            .expect("the placement itself is valid");
        assert_eq!(eliminated, vec![1], "the killer tile eliminates the mover");

        assert!(game.board.history.is_empty(), "simulation must not mutate");
        assert_eq!(game.current_player_id, 1);
        assert_eq!(game.hands[&1].len(), 2);
    }

    #[test]
    fn test_playable_moves_dedupes_symmetric_rotations() {
        let game = suicide_rule_game(vec![killer_tile(), survivor_tile()]);

        // killer: fully rotation-symmetric (1 distinct placement);
        // survivor: 180°-symmetric (2 distinct placements).
        assert_eq!(game.playable_moves(1).len(), 3);

        let survivable = game.survivable_moves(1);
        assert_eq!(
            survivable.len(),
            1,
            "only the dealt survivor rotation lives"
        );
        assert!(survivable[0].tile.is_same_tile(&survivor_tile()));
    }

    #[test]
    fn test_suicide_rejected_when_a_survivable_move_exists() {
        let mut game = suicide_rule_game(vec![killer_tile(), survivor_tile()]);

        let result = game.perform_move(move_of(killer_tile()));
        assert_eq!(result.unwrap_err(), MoveError::ForcedSuicide);

        // The rejection must leave the game untouched.
        assert!(game.board.history.is_empty());
        assert_eq!(game.current_player_id, 1);
        assert_eq!(game.hands[&1].len(), 2);
        assert!(game.players[0].alive);
    }

    #[test]
    fn test_suicide_allowed_when_every_move_is_fatal() {
        let mut game = suicide_rule_game(vec![killer_tile()]);

        let result = game
            .perform_move(move_of(killer_tile()))
            .expect("with no surviving option the fatal placement is legal");

        let TurnResult::PlayerWins {
            winner, eliminated, ..
        } = result
        else {
            panic!("eliminating yourself against one opponent ends the game");
        };
        assert_eq!(winner, 2);
        assert_eq!(eliminated, vec![1]);
        assert!(!game.players[0].alive);
    }

    #[test]
    fn test_random_timeout_move_prefers_survivable() {
        let game = suicide_rule_game(vec![killer_tile(), survivor_tile()]);
        let mov = game
            .random_timeout_move(1)
            .expect("a player with tiles always has a timeout move");
        assert!(
            mov.tile.is_same_tile(&survivor_tile()),
            "with a survivable option the pick never falls on the killer"
        );
        assert!(
            !game
                .simulate_move(mov)
                .expect("the pick must be playable")
                .contains(&1),
            "the picked rotation itself survives"
        );

        // All-fatal hand: the pick falls back to a fatal placement.
        let game = suicide_rule_game(vec![killer_tile()]);
        let mov = game.random_timeout_move(1).expect("fatal moves still play");
        assert!(mov.tile.is_same_tile(&killer_tile()));

        // No tiles at all: nothing to play.
        let game = suicide_rule_game(vec![]);
        assert!(game.random_timeout_move(1).is_none());
    }
}
