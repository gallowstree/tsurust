
use crate::board::*;

pub struct Game {
    players: Vec<Player>,
    board: Board
}

impl Game {
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
