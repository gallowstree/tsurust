use egui::{Color32, Rect, Response, Sense, Ui, Vec2, Widget};
use tsurust_common::board::Player;

pub struct PlayerCard<'a> {
    player: &'a Player,
    hand_size: usize,
    max_hand_size: usize,
    is_current: bool,
    has_dragon: bool,
    is_winner: bool,
}

impl<'a> PlayerCard<'a> {
    pub fn new(player: &'a Player, hand_size: usize, has_dragon: bool) -> Self {
        Self {
            player,
            hand_size,
            max_hand_size: 3,
            is_current: false,
            has_dragon,
            is_winner: false,
        }
    }

    pub fn current_player(mut self) -> Self {
        self.is_current = true;
        self
    }

    pub fn winner(mut self) -> Self {
        self.is_winner = true;
        self
    }
}

impl<'a> Widget for PlayerCard<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(120.0, 60.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // Background with winner highlight or gentle current player highlight
            let bg_color = if self.is_winner {
                Color32::from_rgba_unmultiplied(255, 215, 0, 60) // Gold highlight for winner
            } else if self.is_current {
                Color32::from_rgba_unmultiplied(230, 240, 255, 25) // Gentle blue highlight for current
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 4.0, bg_color);

            // Add border to player card
            let border_color = if self.is_winner {
                Color32::GOLD
            } else if self.is_current {
                Color32::from_rgb(100, 150, 255)
            } else {
                Color32::from_gray(120)
            };
            ui.painter().rect_stroke(rect, 4.0, (1.0, border_color));

            // Player color circle (larger, 24px diameter)
            let circle_center = rect.min + Vec2::new(16.0, 20.0);
            let circle_radius = 12.0;
            let player_color = Color32::from_rgb(self.player.color.0, self.player.color.1, self.player.color.2);

            if self.player.alive {
                ui.painter().circle_filled(circle_center, circle_radius, player_color);
                ui.painter().circle_stroke(circle_center, circle_radius, (1.0, Color32::WHITE));
            } else {
                // Dead player: gray circle with X
                ui.painter().circle_filled(circle_center, circle_radius, Color32::from_gray(100));
                ui.painter().circle_stroke(circle_center, circle_radius, (1.0, Color32::WHITE));
                let x_size = circle_radius * 0.6;
                ui.painter().line_segment(
                    [circle_center - Vec2::new(x_size, x_size), circle_center + Vec2::new(x_size, x_size)],
                    (4.0, player_color)
                );
                ui.painter().line_segment(
                    [circle_center - Vec2::new(x_size, -x_size), circle_center + Vec2::new(x_size, -x_size)],
                    (4.0, player_color)
                );
            }

            // Player name (color name)
            let name_pos = rect.min + Vec2::new(32.0, 12.0);
            let name = color_to_name(self.player.color);
            let font_id = egui::FontId::proportional(16.0); // Larger font
            ui.painter().text(name_pos, egui::Align2::LEFT_TOP, name, font_id, visuals.text_color());

            // Hand indicator - rectangles for tiles (for alive players and winners)
            if self.player.alive || self.is_winner {
                let hand_start = rect.min + Vec2::new(32.0, 32.0);
                let tile_size = Vec2::new(10.0, 10.0); // Square tiles
                let spacing = 4.0;

                for i in 0..self.max_hand_size {
                    let tile_rect = Rect::from_min_size(
                        hand_start + Vec2::new((tile_size.x + spacing) * i as f32, 0.0),
                        tile_size
                    );

                    let (fill_color, stroke_color) = if i < self.hand_size {
                        if self.is_winner {
                            (Color32::from_rgba_unmultiplied(255, 215, 0, 150), Color32::GOLD) // Gold tiles for winner
                        } else {
                            (Color32::from_gray(180), Color32::WHITE) // Normal filled tile
                        }
                    } else {
                        (Color32::TRANSPARENT, Color32::from_gray(100)) // Empty slot
                    };

                    ui.painter().rect(tile_rect, 2.0, fill_color, (1.0, stroke_color));
                }

                // Dragon indicator (only for alive players)
                if self.has_dragon && self.player.alive {
                    let dragon_pos = rect.min + Vec2::new(32.0, 45.0);
                    ui.painter().text(dragon_pos, egui::Align2::LEFT_TOP, "🐉", egui::FontId::default(), Color32::GOLD);
                }
            }


            // Winner indication (crown)
            if self.is_winner {
                let crown_pos = rect.min + Vec2::new(100.0, 10.0);
                ui.painter().text(crown_pos, egui::Align2::LEFT_TOP, "👑", egui::FontId::default(), Color32::GOLD);
            }

        }

        response
    }
}

fn color_to_name(color: (u8, u8, u8)) -> &'static str {
    match color {
        (255, 0, 0) => "Red",
        (0, 255, 0) => "Green",
        (0, 0, 255) => "Blue",
        (255, 255, 0) => "Yellow",
        (255, 0, 255) => "Magenta",
        (0, 255, 255) => "Cyan",
        (255, 128, 0) => "Orange",
        (128, 0, 128) => "Purple",
        _ => "Unknown",
    }
}