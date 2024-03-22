use std::collections::HashMap;

use crate::board::*;
use crate::deck::Deck;

pub struct Game {
    pub deck: Deck,
    pub board: Board,
    players: Vec<Player>,
    hands: HashMap<PlayerID, Vec<Tile>>,
    dragon: Option<PlayerID>,
}

impl Game {
    pub fn new(players: Vec<Player>) -> Game {
        let mut deck = Deck::new();
        let mut hands = HashMap::new();
        for mut player in &players {
            hands.insert(player.id, deck.take_up_to(3));
        }

        Game {
            players, hands, deck,
            board: Board::new(),
            dragon: None,
        }
    }

    pub fn perform_move(&mut self, mov: Move) {
        // to-do: check player is current player
        // to-do: introduce TurnResult type or similar, design that api
        // self.deduct_tile_from_hand(mov)
        //     .expect("this returns validation err l8r");

        self.board.place_tile(mov);

        //self.update_players();

        //self.fill_hands();

        //self.complete_turn(mov.player_id);
    }

    fn deduct_tile_from_hand(&mut self, mov: Move) -> Result<(), &'static str> {
        match self.hands[&mov.player_id].iter().find(|&tile| {tile.eq(&mov.tile)}) {
            Some(_) => Ok(()),
            _ => Err("fuck the client"),
        }
    }
    fn update_players(&mut self) {
        for mut player in alive_players(&mut self.players) {
            let new_pos = self.board.traverse_from(player.pos);
            player.pos = new_pos;
            player.alive = !new_pos.on_edge();

            if !player.alive {
                //self.deck.
            }
        }
    }
    fn fill_hands(&self) {}
    fn complete_turn(&self, for_player: PlayerID) {}
}

fn alive_players(players: &mut Vec<Player>) -> Vec<&mut Player> {
    players.iter_mut().filter(|player| player.alive).collect()
}
