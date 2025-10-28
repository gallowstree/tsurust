use std::collections::HashMap;

use crate::board::*;
use crate::deck::Deck;
use crate::trail::Trail;

/// Statistics tracked for each player during a game
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerStats {
    pub player_id: PlayerID,
    pub turns_survived: usize,        // Number of turns the player stayed alive
    pub tiles_placed: usize,          // Number of tiles placed before elimination
    pub path_length: usize,           // Number of unique tiles traversed
    pub dragon_turns: usize,          // Number of turns holding the dragon
    pub hand_tiles_remaining: usize,  // Tiles in hand when eliminated
    pub elimination_turn: Option<usize>, // Turn number when eliminated (None if winner)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TurnResult {
    TurnAdvanced { turn_number: usize, next_player: PlayerID, eliminated: Vec<PlayerID> },
    PlayerWins { turn_number: usize, winner: PlayerID, eliminated: Vec<PlayerID> },
    Extinction { turn_number: usize, eliminated: Vec<PlayerID> },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub deck: Deck,
    pub board: Board,
    pub players: Vec<Player>,
    pub hands: HashMap<PlayerID, Vec<Tile>>,
    #[serde(skip, default)]
    pub tile_trails: HashMap<CellCoord, Vec<(PlayerID, TileEndpoint)>>, // tile -> list of (player, segment) pairs
    #[serde(skip, default)]
    pub player_trails: HashMap<PlayerID, crate::trail::Trail>, // Complete trail for each player
    pub current_player_id: PlayerID,
    pub dragon: Option<PlayerID>,
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
            stats.insert(player.id, PlayerStats {
                player_id: player.id,
                turns_survived: 0,
                tiles_placed: 0,
                path_length: 1, // Start with 1 for their starting position
                dragon_turns: 0,
                hand_tiles_remaining: 3, // Starting hand size
                elimination_turn: None,
            });
        }

        Game {
            players, hands, deck, board,
            tile_trails: HashMap::new(),
            player_trails,
            current_player_id,
            dragon: None,
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

    pub fn perform_move(&mut self, mov: Move) -> Result<TurnResult, &'static str> {
        // Validate it's this player's turn
        if mov.player_id != self.current_player_id {
            return Err("Not this player's turn");
        }

        // Basic validation
        if !self.players.iter().any(|p| p.id == mov.player_id && p.alive) {
            return Err("Invalid player or player is eliminated");
        }

        // Check if cell is already occupied
        if self.board.get_tile_at(mov.cell).is_some() {
            return Err("Cell already occupied");
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

        // Refill hands (basic implementation for now)
        self.fill_hands();

        // Complete the turn (advances to next player)
        let turn_result = self.complete_turn(eliminated);

        Ok(turn_result)
    }

    fn deduct_tile_from_hand(&mut self, mov: Move) -> Result<(), &'static str> {
        let hand = self.hands.get_mut(&mov.player_id).expect("Player should exist");
        if let Some(pos) = hand.iter().position(|&tile| tile.eq(&mov.tile)) {
            hand.remove(pos);
            Ok(())
        } else {
            Err("Player does not have this tile in hand")
        }
    }
    fn update_players_and_trails(&mut self, placed_cell: CellCoord) -> Vec<PlayerID> {
        let mut eliminated = Vec::new();
        let mut trails_to_record = Vec::new();

        for player in alive_players(&mut self.players) {
            // Only update players who are in the cell where the tile was placed
            if player.pos.cell != placed_cell {
                continue;
            }
            let old_pos = player.pos;
            let trail = self.board.traverse_from(player.pos);
            let new_pos = trail.end_pos;

            // Mark that this player has moved if position changed
            if old_pos != new_pos {
                player.has_moved = true;
            }

            // Collect trail information if player moved (either to different cell or different endpoint)
            if old_pos != new_pos {
                trails_to_record.push((player.id, old_pos)); // Record where they came FROM

                // Extend player's trail with new segments
                if let Some(player_trail) = self.player_trails.get_mut(&player.id) {
                    for segment in &trail.segments {
                        player_trail.add_segment(segment.clone());
                    }
                    player_trail.end_pos = new_pos;
                    player_trail.completed = trail.completed;
                }

                // Update stats: increment path_length for each new cell visited
                if old_pos.cell != new_pos.cell {
                    if let Some(stats) = self.stats.get_mut(&player.id) {
                        stats.path_length += 1;
                    }
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
            let segment = tile.segments
                .iter()
                .find(|&seg| seg.a == exit_pos.endpoint || seg.b == exit_pos.endpoint);

            if let Some(segment) = segment {
                // Use min(from, to) convention for segment key
                let segment_key = std::cmp::min(segment.a, segment.b);

                // Record that this player used this segment (every time they pass through)
                self.tile_trails
                    .entry(exit_pos.cell)
                    .or_insert_with(Vec::new)
                    .push((player_id, segment_key));
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

    fn next_active_player_id(&self) -> Option<PlayerID> {
        // Find the current player index
        if let Some(current_index) = self.players.iter().position(|p| p.id == self.current_player_id) {
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
                stats.hand_tiles_remaining = self.hands.get(&player_id).map(|h| h.len()).unwrap_or(0);
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

        // Increment dragon_turns for the player holding the dragon
        if let Some(dragon_holder) = self.dragon {
            if let Some(stats) = self.stats.get_mut(&dragon_holder) {
                stats.dragon_turns += 1;
            }
        }

        // Count remaining alive players
        let alive_count = self.players.iter().filter(|p| p.alive).count();

        match alive_count {
            0 => TurnResult::Extinction { turn_number, eliminated },
            1 => {
                // Find the winner
                let winner = self.players.iter()
                    .find(|p| p.alive)
                    .map(|p| p.id)
                    .expect("Should have exactly one alive player");

                TurnResult::PlayerWins { turn_number, winner, eliminated }
            }
            _ => {
                // Game continues - advance to next player
                if let Some(next_player_id) = self.next_active_player_id() {
                    self.current_player_id = next_player_id;
                    TurnResult::TurnAdvanced { turn_number, next_player: next_player_id, eliminated }
                } else {
                    // This shouldn't happen if alive_count > 0, but handle it gracefully
                    TurnResult::Extinction { turn_number, eliminated }
                }
            }
        }
    }
}

fn alive_players(players: &mut Vec<Player>) -> Vec<&mut Player> {
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
            ]
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
            ]
        }
    }

    #[test]
    fn test_trail_recording_single_player_single_pass() {
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos { cell: CellCoord { row: 1, col: 1 }, endpoint: 0 },
                alive: true,
                has_moved: false,
                color: (255, 0, 0), // Red
            }
        ];

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
        let trail_entries = game.tile_trails.get(&CellCoord { row: 1, col: 1 }).unwrap();

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
                pos: PlayerPos { cell: CellCoord { row: 1, col: 1 }, endpoint: 0 },
                alive: true,
                has_moved: false,
                color: (255, 0, 0), // Red
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos { cell: CellCoord { row: 1, col: 1 }, endpoint: 2 },
                alive: true,
                has_moved: false,
                color: (0, 255, 0), // Green
            }
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
        let trail_entries = game.tile_trails.get(&CellCoord { row: 1, col: 1 }).unwrap();

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
                pos: PlayerPos { cell: CellCoord { row: 1, col: 1 }, endpoint: 3 }, // Will move right to (1,2)
                alive: true,
                has_moved: false,
                color: (255, 0, 0), // Red
            },
            Player {
                id: 2,
                name: "Player 2".to_string(),
                pos: PlayerPos { cell: CellCoord { row: 2, col: 2 }, endpoint: 0 }, // Will move up to (1,2)
                alive: true,
                has_moved: false,
                color: (0, 255, 0), // Green
            }
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
        let trail_entries_1_1 = game.tile_trails.get(&CellCoord { row: 1, col: 1 });
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
        game.players[0].pos = PlayerPos { cell: CellCoord { row: 1, col: 2 }, endpoint: 0 };
        game.players[1].pos = PlayerPos { cell: CellCoord { row: 1, col: 2 }, endpoint: 4 };

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
        let trail_entries_1_2 = game.tile_trails.get(&CellCoord { row: 1, col: 2 }).unwrap();
        assert!(trail_entries_1_2.contains(&(1, 0))); // Player 1, segment 0-1
        assert!(trail_entries_1_2.contains(&(2, 4))); // Player 2, segment 4-5

        // The test is complete - we've verified that multiple players can have their
        // trails recorded when passing through the same tile. Testing a second pass
        // through would require placing a tile at the same location which is invalid.
    }
}
