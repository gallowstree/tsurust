use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::board::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Deck {
    tiles: Vec<Tile>,
}

impl Deck {
    pub fn take(&mut self) -> Option<Tile> {
        self.tiles.pop()
    }

    pub fn take_up_to(&mut self, n: usize) -> Vec<Tile> {
        let new_len = self.tiles.len().saturating_sub(n);
        self.tiles.split_off(new_len)
    }

    pub fn put(&mut self, tiles: &mut Vec<Tile>) {
        self.tiles.append(tiles)
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn new() -> Deck {
        let mut tiles = vec![
            Tile::new([seg(0, 1), seg(2, 3), seg(4, 5), seg(6, 7)]),
            Tile::new([seg(0, 1), seg(2, 3), seg(4, 6), seg(5, 7)]),
            Tile::new([seg(0, 1), seg(2, 3), seg(4, 7), seg(5, 6)]),
            Tile::new([seg(0, 1), seg(2, 4), seg(3, 6), seg(5, 7)]),
            Tile::new([seg(0, 1), seg(2, 4), seg(3, 7), seg(5, 6)]),
            Tile::new([seg(0, 1), seg(2, 5), seg(3, 6), seg(4, 7)]),
            Tile::new([seg(0, 1), seg(2, 5), seg(3, 7), seg(4, 6)]),
            Tile::new([seg(0, 1), seg(2, 6), seg(3, 4), seg(5, 7)]),
            Tile::new([seg(0, 1), seg(2, 6), seg(3, 5), seg(4, 7)]),
            Tile::new([seg(0, 1), seg(2, 6), seg(3, 7), seg(4, 5)]),
            Tile::new([seg(0, 1), seg(2, 7), seg(3, 4), seg(5, 6)]),
            Tile::new([seg(0, 1), seg(2, 7), seg(3, 5), seg(4, 6)]),
            Tile::new([seg(0, 1), seg(2, 7), seg(3, 6), seg(4, 5)]),
            Tile::new([seg(0, 2), seg(1, 3), seg(4, 6), seg(5, 7)]),
            Tile::new([seg(0, 2), seg(1, 3), seg(4, 7), seg(5, 6)]),
            Tile::new([seg(0, 2), seg(1, 4), seg(3, 6), seg(5, 7)]),
            Tile::new([seg(0, 2), seg(1, 4), seg(3, 7), seg(5, 6)]),
            Tile::new([seg(0, 2), seg(1, 5), seg(3, 6), seg(4, 7)]),
            Tile::new([seg(0, 2), seg(1, 5), seg(3, 7), seg(4, 6)]),
            Tile::new([seg(0, 2), seg(1, 6), seg(3, 4), seg(5, 7)]),
            Tile::new([seg(0, 2), seg(1, 6), seg(3, 5), seg(4, 7)]),
            Tile::new([seg(0, 2), seg(1, 7), seg(3, 4), seg(5, 6)]),
            Tile::new([seg(0, 2), seg(1, 7), seg(3, 5), seg(4, 6)]),
            Tile::new([seg(0, 3), seg(1, 2), seg(4, 7), seg(5, 6)]),
            Tile::new([seg(0, 3), seg(1, 4), seg(2, 6), seg(5, 7)]),
            Tile::new([seg(0, 3), seg(1, 4), seg(2, 7), seg(5, 6)]),
            Tile::new([seg(0, 3), seg(1, 5), seg(2, 6), seg(4, 7)]),
            Tile::new([seg(0, 3), seg(1, 6), seg(2, 5), seg(4, 7)]),
            Tile::new([seg(0, 4), seg(1, 2), seg(3, 6), seg(5, 7)]),
            Tile::new([seg(0, 4), seg(1, 2), seg(3, 7), seg(5, 6)]),
            Tile::new([seg(0, 4), seg(1, 3), seg(2, 6), seg(5, 7)]),
            Tile::new([seg(0, 4), seg(1, 5), seg(2, 6), seg(3, 7)]),
            Tile::new([seg(0, 4), seg(1, 5), seg(2, 7), seg(3, 6)]),
            Tile::new([seg(0, 5), seg(1, 4), seg(2, 7), seg(3, 6)]),
            Tile::new([seg(0, 7), seg(1, 2), seg(3, 4), seg(5, 6)]),
        ];

        let mut rng = thread_rng();
        tiles.shuffle(&mut rng);

        Deck { tiles }
    }
}

#[cfg(test)]
mod tests {
    use super::Deck;

    #[test]
    fn take_works() {
        let mut deck = Deck::new();

        assert!(deck.take().is_some());
        assert_eq!(deck.take_up_to(3).len(), 3);
        assert_eq!(deck.take_up_to(50).len(), 31); // deck is only 35 tiles, so the remaining 31 should pop off
        assert!(deck.take().is_none()); //depleted
    }
}
