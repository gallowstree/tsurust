use std::collections::HashMap;
use crate::board::*;
use crate::deck::Deck;

pub struct Game {
    players: Vec<Player>,
    hands: HashMap<PlayerID, Vec<Tile>>,
    deck: Deck,
    board: Board,
    dragon: Option<PlayerID>
}

impl Game {
    pub fn new(players: Vec<Player>) -> Game {
        let mut deck = Deck::new();
        let mut hands = HashMap::new();
        for mut player in &players {
            hands.insert(player.id, deck.take_up_to(3));
        }

        Game {players, hands, deck, board: Board::new(), dragon: None}
    }

    pub fn perform_move(&mut self, mov: Move) {
        self.board.place_tile(mov);

        let alive_players = self.players
            .iter_mut()
            .filter(|player| player.alive);

        /// make alive players follow the path in front of them
        for mut player in alive_players {
            let new_pos = self.board.traverse_from(player.pos);
            player.pos = new_pos;
            player.alive = !new_pos.on_edge();
        }
    }
}
