use std::collections::HashMap;

use crate::board::*;
use crate::deck::Deck;

pub struct Game {
    pub deck: Deck,
    pub board: Board,
    pub players: Vec<Player>,
    pub hands: HashMap<PlayerID, Vec<Tile>>,
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

        Game {
            players, hands, deck, board,
            dragon: None,
        }
    }

    pub fn curr_player_hand(&self) -> Vec<Tile> {
        let curr_player: PlayerID = self.players[0].id;
        self.hands[&curr_player].clone()
    }

    pub fn perform_move(&mut self, mov: Move) -> Result<(), &'static str> {
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

        // Update player positions based on new tile placement
        self.update_players();

        // Refill hands (basic implementation for now)
        self.fill_hands();

        Ok(())
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
    fn update_players(&mut self) {
        for mut player in alive_players(&mut self.players) {
            let new_pos = self.board.traverse_from(player.pos);
            player.pos = new_pos;
            player.alive = !new_pos.on_edge();

            if !player.alive {
                self.deck.put(self.hands.get_mut(&player.id).expect("hand"));
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
    fn complete_turn(&self, for_player: PlayerID) {}
}

fn alive_players(players: &mut Vec<Player>) -> Vec<&mut Player> {
    players.iter_mut().filter(|player| player.alive).collect()
}
