use egui::{Color32, Ui, Vec2, Widget};
use tsurust_common::board::Player;
use tsurust_common::game::PlayerStats;

/// Display statistics for a single player
pub struct PlayerStatsDisplay<'a> {
    player: &'a Player,
    stats: &'a PlayerStats,
    is_winner: bool,
    is_you: bool,
}

impl<'a> PlayerStatsDisplay<'a> {
    pub fn new(player: &'a Player, stats: &'a PlayerStats) -> Self {
        Self {
            player,
            stats,
            is_winner: false,
            is_you: false,
        }
    }

    pub fn winner(mut self) -> Self {
        self.is_winner = true;
        self
    }

    pub fn you(mut self) -> Self {
        self.is_you = true;
        self
    }
}

impl<'a> Widget for PlayerStatsDisplay<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let desired_size = Vec2::new(280.0, 170.0); // Increased height for more stats
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // Background with winner highlight - darker for better contrast
            let bg_color = if self.is_winner {
                Color32::from_rgba_unmultiplied(60, 50, 0, 220) // Dark gold background for winner
            } else {
                Color32::from_rgba_unmultiplied(40, 40, 50, 220) // Dark background
            };
            ui.painter().rect_filled(rect, 6.0, bg_color);

            // Border
            let border_color = if self.is_winner {
                Color32::from_rgb(255, 215, 0) // Bright gold border
            } else {
                Color32::from_gray(100)
            };
            ui.painter().rect_stroke(rect, 6.0, (2.0, border_color));

            // Player color circle
            let circle_center = rect.min + Vec2::new(20.0, 25.0);
            let circle_radius = 14.0;
            let player_color = Color32::from_rgb(self.player.color.0, self.player.color.1, self.player.color.2);

            if self.player.alive {
                ui.painter().circle_filled(circle_center, circle_radius, player_color);
                ui.painter().circle_stroke(circle_center, circle_radius, (1.5, Color32::WHITE));
            } else {
                // Dead player: gray circle with X
                ui.painter().circle_filled(circle_center, circle_radius, Color32::from_gray(100));
                ui.painter().circle_stroke(circle_center, circle_radius, (1.5, Color32::WHITE));
                let x_size = circle_radius * 0.6;
                ui.painter().line_segment(
                    [circle_center - Vec2::new(x_size, x_size), circle_center + Vec2::new(x_size, x_size)],
                    (3.0, player_color)
                );
                ui.painter().line_segment(
                    [circle_center - Vec2::new(x_size, -x_size), circle_center + Vec2::new(x_size, -x_size)],
                    (3.0, player_color)
                );
            }

            // Player name with crown and "(You)" indicator
            let mut name_pos = rect.min + Vec2::new(42.0, 16.0);
            let name = &self.player.name;
            let display_name = if self.is_you {
                format!("{} (You)", name)
            } else {
                name.to_string()
            };

            // Crown for winner
            if self.is_winner {
                ui.painter().text(name_pos, egui::Align2::LEFT_TOP, "👑 ", egui::FontId::proportional(20.0), Color32::GOLD);
                name_pos.x += 22.0;
            }

            let font_id = egui::FontId::proportional(18.0);
            ui.painter().text(name_pos, egui::Align2::LEFT_TOP, display_name, font_id.clone(), Color32::WHITE);

            // Statistics display
            let stats_start = rect.min + Vec2::new(10.0, 50.0);
            let line_height = 18.0;
            let stats_font = egui::FontId::proportional(14.0);
            let label_color = Color32::from_gray(180); // Brighter grey for labels
            let value_color = Color32::WHITE; // White for values

            // Row 1: Turns Survived | Path Length
            ui.painter().text(
                stats_start + Vec2::new(0.0, 0.0),
                egui::Align2::LEFT_TOP,
                "Turns:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(80.0, 0.0),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.turns_survived),
                stats_font.clone(),
                value_color
            );

            ui.painter().text(
                stats_start + Vec2::new(140.0, 0.0),
                egui::Align2::LEFT_TOP,
                "Path:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(200.0, 0.0),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.path_length),
                stats_font.clone(),
                value_color
            );

            // Row 2: Tiles Placed | Dragon Turns
            ui.painter().text(
                stats_start + Vec2::new(0.0, line_height),
                egui::Align2::LEFT_TOP,
                "Tiles:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(80.0, line_height),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.tiles_placed),
                stats_font.clone(),
                value_color
            );

            ui.painter().text(
                stats_start + Vec2::new(140.0, line_height),
                egui::Align2::LEFT_TOP,
                "Dragon:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(200.0, line_height),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.dragon_turns),
                stats_font.clone(),
                value_color
            );

            // Row 3: Players Eliminated | Tiles Visited %
            ui.painter().text(
                stats_start + Vec2::new(0.0, line_height * 2.0),
                egui::Align2::LEFT_TOP,
                "Elims:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(80.0, line_height * 2.0),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.players_eliminated),
                stats_font.clone(),
                value_color
            );

            let coverage_pct = (self.stats.unique_tiles_visited as f32 / 36.0) * 100.0; // 6x6 board
            ui.painter().text(
                stats_start + Vec2::new(140.0, line_height * 2.0),
                egui::Align2::LEFT_TOP,
                "Coverage:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(217.0, line_height * 2.0),
                egui::Align2::LEFT_TOP,
                format!("{:.1}%", coverage_pct),
                stats_font.clone(),
                value_color
            );

            // Row 4: Max Revisits | Efficiency
            ui.painter().text(
                stats_start + Vec2::new(0.0, line_height * 3.0),
                egui::Align2::LEFT_TOP,
                "Revisit:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(80.0, line_height * 3.0),
                egui::Align2::LEFT_TOP,
                format!("{}", self.stats.max_visits_to_single_tile),
                stats_font.clone(),
                value_color
            );

            let efficiency = if self.stats.tiles_placed > 0 {
                self.stats.path_length as f32 / self.stats.tiles_placed as f32
            } else {
                0.0
            };
            ui.painter().text(
                stats_start + Vec2::new(140.0, line_height * 3.0),
                egui::Align2::LEFT_TOP,
                "Efficiency:",
                stats_font.clone(),
                label_color
            );
            ui.painter().text(
                stats_start + Vec2::new(217.0, line_height * 3.0),
                egui::Align2::LEFT_TOP,
                format!("{:.2}", efficiency),
                stats_font.clone(),
                value_color
            );

            // Row 5: Elimination info
            if let Some(elim_turn) = self.stats.elimination_turn {
                ui.painter().text(
                    stats_start + Vec2::new(0.0, line_height * 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("Eliminated on turn {}", elim_turn),
                    stats_font.clone(),
                    Color32::from_rgb(180, 60, 60)
                );
            } else if self.is_winner {
                ui.painter().text(
                    stats_start + Vec2::new(0.0, line_height * 4.0),
                    egui::Align2::LEFT_TOP,
                    "WINNER",
                    egui::FontId::proportional(16.0),
                    Color32::GOLD
                );
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
