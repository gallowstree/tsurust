use std::collections::HashMap;
use std::sync::mpsc;

use eframe::egui::{self, Context};

use tsurust_common::board::{Player, PlayerID};
use tsurust_common::game::Game;

use crate::app::{Message, PlayerAnimation, TilePlacementAnimation};
use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use crate::messaging::send_ui_message;
use crate::player_card::PlayerCard;

pub fn render_game_ui(
    ctx: &Context,
    game: &mut Game,
    client_player_id: PlayerID,
    waiting_for_server: bool,
    lobby_name: Option<&str>,
    sender: &mpsc::Sender<Message>,
    last_rotated_tile: Option<(usize, bool)>,
    player_animations: &HashMap<PlayerID, PlayerAnimation>,
    tile_placement_animation: &Option<TilePlacementAnimation>,
) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui.button("🔄 Restart Game").clicked() {
                    send_ui_message(sender, Message::RestartGame);
                }
                if ui.button("⬅ Back to Menu").clicked() {
                    send_ui_message(sender, Message::BackToMainMenu);
                }

                // Show lobby name and/or waiting indicator on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);

                    if waiting_for_server {
                        ui.label("⏳ Waiting for server...");
                    }

                    if let Some(name) = lobby_name {
                        if waiting_for_server {
                            ui.separator();
                        }
                        ui.label(format!("📋 {}", name));
                    }
                });
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(20.);
            ui.add(BoardRenderer::new(&game.board.history, &game.players, &game.tile_trails, &game.player_trails, player_animations, tile_placement_animation));
        });
    });

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.vertical(|ui| {
            let is_game_over = game.is_game_over();

            if is_game_over {
                // Show final statistics instead of player cards
                use crate::stats_display::PlayerStatsDisplay;

                ui.heading("Game Over");
                ui.add_space(10.0);

                // Determine the winner
                let alive_count = game.players.iter().filter(|p| p.alive).count();
                if alive_count == 1 {
                    let winner = game.players.iter().find(|p| p.alive).expect("Should have exactly one alive player");
                    ui.label(egui::RichText::new(format!("🎉 {} Wins!", color_to_name(winner.color))).size(18.0).strong());
                } else {
                    ui.label(egui::RichText::new("All Players Eliminated").size(16.0));
                }

                ui.add_space(15.0);

                // Sort players by performance
                let mut sorted_players: Vec<_> = game.players.iter().collect();
                sorted_players.sort_by(|a, b| {
                    match (a.alive, b.alive) {
                        (true, false) => return std::cmp::Ordering::Less,
                        (false, true) => return std::cmp::Ordering::Greater,
                        _ => {}
                    }

                    let stats_a = game.stats.get(&a.id);
                    let stats_b = game.stats.get(&b.id);

                    match (stats_a, stats_b) {
                        (Some(sa), Some(sb)) => {
                            match sb.turns_survived.cmp(&sa.turns_survived) {
                                std::cmp::Ordering::Equal => sb.path_length.cmp(&sa.path_length),
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
                            let placement = index + 1;
                            let placement_text = match placement {
                                1 => "1st",
                                2 => "2nd",
                                3 => "3rd",
                                _ => &format!("{}th", placement),
                            };
                            ui.label(egui::RichText::new(placement_text).size(14.0).strong());
                            ui.add_space(5.0);

                            let mut stats_display = PlayerStatsDisplay::new(player, stats);
                            if player.alive {
                                stats_display = stats_display.winner();
                            }
                            if player.id == client_player_id {
                                stats_display = stats_display.you();
                            }
                            ui.add(stats_display);
                        });
                        ui.add_space(8.0);
                    }
                }

                ui.add_space(10.0);
            } else {
                // Normal gameplay - show player cards
                // Sort players: alive first, then dead
                let mut sorted_players: Vec<&Player> = game.players.iter().collect();
                sorted_players.sort_by_key(|p| !p.alive);

                // Determine winner: if only one player is alive, they're the winner
                let alive_count = game.players.iter().filter(|p| p.alive).count();
                let winner_id = if alive_count == 1 {
                    game.players.iter().find(|p| p.alive).map(|p| p.id)
                } else {
                    None
                };

                for player in sorted_players {
                    let hand_size = game.hands.get(&player.id).map(|h| h.len()).unwrap_or(0);
                    let has_dragon = game.dragon == Some(player.id);
                    let is_current = player.id == game.current_player_id;
                    let is_winner = winner_id == Some(player.id);

                    ui.horizontal(|ui| {
                        // Arrow indicator for current player
                        let (arrow_rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 60.0), egui::Sense::hover());

                        if is_current {
                            let triangle_center = arrow_rect.center();
                            let triangle_size = 12.0;

                            let points = [
                                triangle_center + egui::Vec2::new(-triangle_size/2.0, -triangle_size/2.0),
                                triangle_center + egui::Vec2::new(-triangle_size/2.0, triangle_size/2.0),
                                triangle_center + egui::Vec2::new(triangle_size/2.0, 0.0),
                            ];

                            ui.painter().add(egui::Shape::convex_polygon(
                                points.to_vec(),
                                egui::Color32::from_rgb(100, 150, 255),
                                egui::Stroke::NONE
                            ));
                        }

                        let mut card = PlayerCard::new(player, hand_size, has_dragon);
                        if is_current {
                            card = card.current_player();
                        }
                        if is_winner {
                            card = card.winner();
                        }
                        if player.id == client_player_id {
                            card = card.you();
                        }
                        ui.add(card);
                    });
                }
            }

            ui.add_space(20.0);
            ui.separator();
        });

        // Hand section - show this client's hand, not the current player's hand
        let hand = game.hands.get(&client_player_id)
            .cloned()
            .unwrap_or_default();
        ui.add(
            HandRenderer::new(hand, sender.clone())
                .with_last_rotated(last_rotated_tile)
        );
    });
}

fn render_game_over_overlay(ctx: &Context, game: &Game, client_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
    use crate::stats_display::PlayerStatsDisplay;

    // Semi-transparent dark background
    egui::Area::new("game_over_overlay")
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let screen_rect = ctx.screen_rect();
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_black_alpha(200)
            );
        });

    // Centered stats panel
    egui::Window::new("Game Over")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);

                // Determine the winner
                let alive_count = game.players.iter().filter(|p| p.alive).count();
                if alive_count == 1 {
                    let winner = game.players.iter().find(|p| p.alive).expect("Should have exactly one alive player");
                    ui.heading(egui::RichText::new(format!("🎉 {} Wins! 🎉", color_to_name(winner.color))).size(24.0));
                } else {
                    ui.heading(egui::RichText::new("Game Over - All Players Eliminated").size(24.0));
                }

                ui.add_space(20.0);
                ui.label(egui::RichText::new("Final Statistics").size(18.0).strong());
                ui.add_space(15.0);

                // Sort players by performance
                let mut sorted_players: Vec<_> = game.players.iter().collect();
                sorted_players.sort_by(|a, b| {
                    match (a.alive, b.alive) {
                        (true, false) => return std::cmp::Ordering::Less,
                        (false, true) => return std::cmp::Ordering::Greater,
                        _ => {}
                    }

                    let stats_a = game.stats.get(&a.id);
                    let stats_b = game.stats.get(&b.id);

                    match (stats_a, stats_b) {
                        (Some(sa), Some(sb)) => {
                            match sb.turns_survived.cmp(&sa.turns_survived) {
                                std::cmp::Ordering::Equal => sb.path_length.cmp(&sa.path_length),
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
                            let placement = index + 1;
                            let placement_text = match placement {
                                1 => "1st",
                                2 => "2nd",
                                3 => "3rd",
                                _ => &format!("{}th", placement),
                            };
                            ui.label(egui::RichText::new(placement_text).size(16.0).strong());
                            ui.add_space(10.0);

                            let mut stats_display = PlayerStatsDisplay::new(player, stats);
                            if player.alive {
                                stats_display = stats_display.winner();
                            }
                            if player.id == client_player_id {
                                stats_display = stats_display.you();
                            }
                            ui.add(stats_display);
                        });
                        ui.add_space(10.0);
                    }
                }

                ui.add_space(20.0);

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