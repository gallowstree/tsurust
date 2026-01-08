use std::sync::mpsc;

use eframe::egui::{self, Context};

use tsurust_common::game::Game;

use crate::app::Message;
use crate::board_renderer::BoardRenderer;
use crate::messaging::send_ui_message;
use crate::replay_state::{PlaybackStatus, ReplayState};

pub fn render_replay_viewer_ui(
    ctx: &Context,
    replay_state: &mut ReplayState,
    current_game: &Game,
    sender: &mpsc::Sender<Message>,
) {
    // Top panel - replay controls
    egui::TopBottomPanel::top("replay_controls")
        .resizable(false)
        .min_height(60.0)
        .show(ctx, |ui| {
            ui.add_space(5.0);

            // First row - playback controls
            ui.horizontal(|ui| {
                ui.add_space(10.0);

                // Jump to start
                if ui.button("⏮ Start").clicked() {
                    send_ui_message(sender, Message::ReplayJumpToStart);
                }

                // Step backward
                let can_step_back = replay_state.can_step_backward();
                ui.add_enabled_ui(can_step_back, |ui| {
                    if ui.button("⏪ Back").clicked() {
                        send_ui_message(sender, Message::ReplayStepBackward);
                    }
                });

                // Play/Pause toggle
                if replay_state.playback_status == PlaybackStatus::Playing {
                    if ui.button("⏸ Pause").clicked() {
                        send_ui_message(sender, Message::ReplayPause);
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        send_ui_message(sender, Message::ReplayPlay);
                    }
                }

                // Step forward
                let can_step_forward = replay_state.can_step_forward();
                ui.add_enabled_ui(can_step_forward, |ui| {
                    if ui.button("⏩ Forward").clicked() {
                        send_ui_message(sender, Message::ReplayStepForward);
                    }
                });

                // Jump to end
                if ui.button("⏭ End").clicked() {
                    send_ui_message(sender, Message::ReplayJumpToEnd);
                }

                ui.separator();

                // Speed control
                ui.label("Speed:");
                for &speed in &[0.5, 1.0, 2.0, 4.0] {
                    let text = if speed == 1.0 {
                        "1x".to_string()
                    } else {
                        format!("{}x", speed)
                    };
                    let is_selected = (replay_state.playback_speed - speed).abs() < 0.01;
                    if ui.selectable_label(is_selected, text).clicked() {
                        send_ui_message(sender, Message::ReplaySetSpeed(speed));
                    }
                }

                ui.separator();

                // Move counter
                ui.label(format!(
                    "Move {}/{}",
                    replay_state.current_move_index,
                    replay_state.export.metadata.total_turns
                ));

                // Exit button on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    if ui.button("⬅ Exit Replay").clicked() {
                        send_ui_message(sender, Message::ExitReplay);
                    }

                    // Room name if available
                    if let Some(room_name) = &replay_state.export.metadata.room_name {
                        ui.label(format!("📋 {}", room_name));
                    } else {
                        ui.label("📋 Replay");
                    }
                });
            });

            ui.add_space(5.0);

            // Second row - progress slider
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                let mut move_index = replay_state.current_move_index;
                let max_moves = replay_state.export.metadata.total_turns;

                if ui.add(egui::Slider::new(
                    &mut move_index,
                    0..=max_moves
                ).show_value(false)).changed() {
                    send_ui_message(sender, Message::ReplayJumpToMove(move_index));
                }
                ui.add_space(10.0);
            });

            ui.add_space(5.0);
        });

    // Central panel - board view
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.add(BoardRenderer::new(
                &current_game.board.history,
                &current_game.players,
                &current_game.tile_trails,
                &current_game.player_trails,
                &std::collections::HashMap::new(), // No animations in replay
                &None, // No tile placement animation
            ));
        });
    });

    // Right panel - replay info and player states
    egui::SidePanel::right("replay_info")
        .min_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Replay Info");
            ui.add_space(10.0);

            // Show metadata
            ui.label(format!("Game Mode: {:?}", replay_state.export.metadata.game_mode));

            // Parse and display timestamp
            if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&replay_state.export.timestamp) {
                ui.label(format!("Date: {}", datetime.format("%Y-%m-%d %H:%M")));
            }

            ui.label(format!("Status: {}",
                if replay_state.export.metadata.completed { "Completed" } else { "In Progress" }
            ));

            if replay_state.export.metadata.completed {
                if let Some(winner_id) = replay_state.export.metadata.winner_id {
                    if let Some(winner) = current_game.players.iter().find(|p| p.id == winner_id) {
                        let winner_color = egui::Color32::from_rgb(winner.color.0, winner.color.1, winner.color.2);
                        ui.label(egui::RichText::new(format!("Winner: {}", winner.name)).color(winner_color).strong());
                    }
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(20.0);

            // Show current player states
            ui.heading("Players");
            ui.add_space(5.0);

            for player in &current_game.players {
                ui.horizontal(|ui| {
                    let color = egui::Color32::from_rgb(player.color.0, player.color.1, player.color.2);
                    let status_icon = if player.alive { "✓" } else { "✗" };
                    ui.label(egui::RichText::new(status_icon).color(color).strong());
                    ui.label(&player.name);

                    // Show stats if available
                    if let Some(stats) = current_game.stats.get(&player.id) {
                        ui.label(egui::RichText::new(format!("(T:{})", stats.turns_survived)).weak());
                    }
                });

                // Show hand count if available
                if let Some(&count) = replay_state.export.hand_counts.get(&player.id) {
                    ui.label(egui::RichText::new(format!("  {} tiles in hand", count)).weak().small());
                }
            }

            // Show deck count
            ui.add_space(10.0);
            ui.label(egui::RichText::new(format!("Deck: {} tiles", replay_state.export.deck_count)).weak());
        });
}
