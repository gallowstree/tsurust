use std::collections::HashMap;

use crate::board::*;
use crate::deck::Deck;

#[derive(Debug, Clone)]
pub enum TurnResult {
    TurnAdvanced { turn_number: usize, next_player: PlayerID, eliminated: Vec<PlayerID> },
    PlayerWins { turn_number: usize, winner: PlayerID, eliminated: Vec<PlayerID> },
    Extinction { turn_number: usize, eliminated: Vec<PlayerID> },
}

pub struct Game {
    pub deck: Deck,
    pub board: Board,
    pub players: Vec<Player>,
    pub hands: HashMap<PlayerID, Vec<Tile>>,
    pub tile_trails: HashMap<CellCoord, HashMap<TileEndpoint, PlayerID>>, // tile -> segment -> player
    pub current_player_id: PlayerID,
    dragon: Option<PlayerID>,
}

impl Game {
    pub fn new(players: Vec<Player>) -> Game {
        let mut deck = Deck::new();
        let mut hands = HashMap::new();
        let board = Board::new();

        for mut player in &players {
            hands.insert(player.id, deck.take_up_to(3));
        }

        let current_player_id = if players.is_empty() { 1 } else { players[0].id };

        Game {
            players, hands, deck, board,
            tile_trails: HashMap::new(),
            current_player_id,
            dragon: None,
        }
    }

    pub fn curr_player_hand(&self) -> Vec<Tile> {
        self.hands[&self.current_player_id].clone()
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

        // Place the tile on the board
        self.board.place_tile(mov);

        // Update player positions and record trails for players who actually move
        let eliminated = self.update_players_and_trails();

        // Refill hands (basic implementation for now)
        self.fill_hands();

        // Complete the turn (advances to next player)
        let turn_result = self.complete_turn(eliminated);

        Ok(turn_result)
    }

    fn deduct_tile_from_hand(&mut self, mov: Move) -> Result<(), &'static str> {
        if let Some(hand) = self.hands.get_mut(&mov.player_id) {
            if let Some(pos) = hand.iter().position(|&tile| tile.eq(&mov.tile)) {
                hand.remove(pos);
                Ok(())
            } else {
                Err("Player does not have this tile in hand")
            }
        } else {
            Err("Player not found")
        }
    }
    fn update_players_and_trails(&mut self) -> Vec<PlayerID> {
        let mut eliminated = Vec::new();
        let mut trails_to_record = Vec::new();

        for player in alive_players(&mut self.players) {
            let old_pos = player.pos;
            let new_pos = self.board.traverse_from(player.pos);

            // Mark that this player has moved if position changed
            if old_pos != new_pos {
                player.has_moved = true;
            }

            // Collect trail information if player moved to a different cell
            if old_pos.cell != new_pos.cell {
                println!("DEBUG: Player {} moved from cell {:?} to cell {:?}",
                    player.id, old_pos.cell, new_pos.cell);
                trails_to_record.push((player.id, old_pos)); // Record where they came FROM
            } else {
                println!("DEBUG: Player {} stayed in same cell {:?} (endpoint {} -> {})",
                    player.id, old_pos.cell, old_pos.endpoint, new_pos.endpoint);
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
                    .or_insert_with(HashMap::new)
                    .insert(segment_key, player_id);

                println!("DEBUG: Recording trail for player {} at cell {:?}, segment {}",
                    player_id, exit_pos.cell, segment_key);
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
