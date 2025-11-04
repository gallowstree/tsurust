use std::sync::mpsc;

use eframe::egui::{self, Context};

use tsurust_common::board::PlayerID;
use tsurust_common::game::Game;

use crate::app::Message;
use crate::messaging::send_ui_message;
use crate::stats_display::PlayerStatsDisplay;

pub fn render_game_over_ui(ctx: &Context, game: &Game, client_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(false)
        .min_height(60.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);

                // Determine the winner
                let alive_count = game.players.iter().filter(|p| p.alive).count();
                if alive_count == 1 {
                    let winner = game.players.iter().find(|p| p.alive).expect("Should have exactly one alive player");
                    ui.heading(format!("🎉 {} Wins! 🎉", color_to_name(winner.color)));
                } else {
                    ui.heading("Game Over - All Players Eliminated");
                }

                ui.add_space(5.0);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading("Final Statistics");
            ui.add_space(30.0);

            // Sort players by performance (winner first, then by turns survived, then by path length)
            let mut sorted_players: Vec<_> = game.players.iter().collect();
            sorted_players.sort_by(|a, b| {
                // Winners always come first
                match (a.alive, b.alive) {
                    (true, false) => return std::cmp::Ordering::Less,
                    (false, true) => return std::cmp::Ordering::Greater,
                    _ => {}
                }

                // Then sort by turns survived (descending)
                let stats_a = game.stats.get(&a.id);
                let stats_b = game.stats.get(&b.id);

                match (stats_a, stats_b) {
                    (Some(sa), Some(sb)) => {
                        // First by turns survived
                        match sb.turns_survived.cmp(&sa.turns_survived) {
                            std::cmp::Ordering::Equal => {
                                // Then by path length
                                sb.path_length.cmp(&sa.path_length)
                            }
                            other => other
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });

            // Display stats for each player
            for (index, player) in sorted_players.iter().enumerate() {
                if let Some(stats) = game.stats.get(&player.id) {
                    ui.horizontal(|ui| {
                        // Placement number
                        let placement = index + 1;
                        let placement_text = match placement {
                            1 => "1st",
                            2 => "2nd",
                            3 => "3rd",
                            _ => &format!("{}th", placement),
                        };
                        ui.label(egui::RichText::new(placement_text).size(16.0).strong());
                        ui.add_space(10.0);

                        // Player stats card
                        let mut stats_display = PlayerStatsDisplay::new(player, stats);
                        if player.alive {
                            stats_display = stats_display.winner();
                        }
                        if player.id == client_player_id {
                            stats_display = stats_display.you();
                        }
                        ui.add(stats_display);
                    });
                    ui.add_space(15.0);
                }
            }

            ui.add_space(30.0);

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("🔄 Play Again").clicked() {
                    send_ui_message(sender, Message::RestartGame);
                }
                ui.add_space(10.0);
                if ui.button("⬅ Back to Menu").clicked() {
                    send_ui_message(sender, Message::BackToMainMenu);
                }
            });
        });
    });
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
