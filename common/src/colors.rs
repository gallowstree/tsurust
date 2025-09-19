/// Player color constants as RGB tuples
pub const RED: (u8, u8, u8) = (255, 80, 80);
pub const BLUE: (u8, u8, u8) = (80, 80, 255);
pub const GREEN: (u8, u8, u8) = (80, 255, 80);
pub const YELLOW: (u8, u8, u8) = (255, 255, 80);
pub const PURPLE: (u8, u8, u8) = (255, 80, 255);
pub const ORANGE: (u8, u8, u8) = (255, 165, 80);

/// Get color for a player based on their ID
pub fn get_player_color(player_id: usize) -> (u8, u8, u8) {
    match player_id % 6 {
        0 => RED,
        1 => BLUE,
        2 => GREEN,
        3 => YELLOW,
        4 => PURPLE,
        5 => ORANGE,
        _ => unreachable!(),
    }
}