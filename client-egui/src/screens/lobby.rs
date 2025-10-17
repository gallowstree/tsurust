use std::sync::mpsc;

use eframe::egui::{self, Context};

use tsurust_common::board::PlayerID;
use tsurust_common::lobby::Lobby;

use crate::app::Message;
use crate::components::LobbyBoard;
use crate::messaging::send_ui_message;

/// Helper function to render a player color indicator circle
fn render_player_color_circle(ui: &mut egui::Ui, color: (u8, u8, u8), radius: f32) {
    let player_color_ui = egui::Color32::from_rgb(color.0, color.1, color.2);
    let circle_center = ui.cursor().min + egui::Vec2::new(radius + 4.0, radius + 4.0);
    ui.painter().circle_filled(circle_center, radius, player_color_ui);
    ui.painter().circle_stroke(circle_center, radius, (1.0, egui::Color32::WHITE));
    ui.add_space(radius * 2.0 + 8.0);
}

/// Render the top panel with lobby information
fn render_lobby_top_panel(ctx: &Context, lobby: &Lobby, show_start_button: bool, is_online: bool, sender: &mpsc::Sender<Message>) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.heading(format!("Lobby: {}", lobby.name));
                ui.separator();

                // Only show Room ID for online lobbies
                if is_online {
                    ui.label(format!("Room ID: {}", lobby.id));
                    ui.separator();
                }

                ui.label(format!("Players: {}/{}", lobby.players.len(), lobby.max_players));

                if show_start_button {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if lobby.can_start() {
                            if ui.button("🚀 Start Game").clicked() {
                                println!("[DEBUG] Start Game button clicked!");
                                send_ui_message(sender, Message::StartGameFromLobby);
                            }
                        } else {
                            ui.add_enabled(false, egui::Button::new("⏳ Waiting for players..."));
                        }
                    });
                }
            });
        });
}

/// Render the player list in the side panel
fn render_player_list(ui: &mut egui::Ui, lobby: &Lobby, current_player_id: PlayerID, placing_for_id: Option<PlayerID>) {
    for (player_id, lobby_player) in &lobby.players {
        ui.horizontal(|ui| {
            render_player_color_circle(ui, lobby_player.color, 8.0);
            ui.label(&lobby_player.name);

            if lobby_player.spawn_position.is_some() {
                ui.label("✔ Ready");
            } else if let Some(placing_id) = placing_for_id {
                if *player_id == placing_id {
                    ui.label("👈 Placing now...");
                } else {
                    ui.label("⏳ Waiting...");
                }
            } else {
                ui.label("⏳ Placing pawn...");
            }

            if *player_id == current_player_id {
                ui.label("(You)");
            }
        });
        ui.add_space(5.0);
    }
}

/// Render debug tools section
fn render_debug_tools(ui: &mut egui::Ui, lobby: &Lobby, show_cycle_controls: bool, sender: &mpsc::Sender<Message>) {
    ui.separator();
    ui.heading("Debug Tools");
    ui.add_space(10.0);

    if ui.button("➕ Add Test Player").clicked() {
        send_ui_message(sender, Message::DebugAddPlayer);
    }

    ui.add_space(10.0);

    if show_cycle_controls {
        ui.label("Switch Player:");
        ui.horizontal(|ui| {
            if ui.button("⬅ Previous").clicked() {
                send_ui_message(sender, Message::DebugCyclePlayer(false));
            }
            if ui.button("Next ➡").clicked() {
                send_ui_message(sender, Message::DebugCyclePlayer(true));
            }
        });
    } else {
        ui.label("Place pawn for:");
        for (player_id, lobby_player) in &lobby.players {
            if lobby_player.spawn_position.is_none() {
                ui.horizontal(|ui| {
                    render_player_color_circle(ui, lobby_player.color, 6.0);

                    if ui.button(&lobby_player.name).clicked() {
                        send_ui_message(sender, Message::DebugPlacePawn(*player_id));
                    }
                });
            }
        }
    }
}

pub fn render_lobby_ui(ctx: &Context, lobby: &mut Lobby, current_player_id: PlayerID, is_online: bool, sender: &mpsc::Sender<Message>) {
    render_lobby_top_panel(ctx, lobby, true, is_online, sender);

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.heading("Select your starting position");
            ui.label("Click on any board edge to place your pawn");
            ui.add_space(20.0);

            render_lobby_board(ui, lobby, current_player_id, sender);
        });
    });

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.vertical(|ui| {
            render_player_list(ui, lobby, current_player_id, None);

            ui.add_space(20.0);
            ui.separator();

            if is_online && lobby.players.len() < lobby.max_players {
                ui.label("Waiting for more players to join...");
            }

            // Only show debug tools for local lobbies
            if !is_online {
                ui.add_space(20.0);
                render_debug_tools(ui, lobby, false, sender);
            }
        });
    });
}

fn render_lobby_board(ui: &mut egui::Ui, lobby: &Lobby, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
    let board = LobbyBoard::new(lobby, current_player_id);
    board.render(ui, 300.0, sender);
}

pub fn render_lobby_placing_ui(ctx: &Context, lobby: &mut Lobby, placing_for_id: PlayerID, is_online: bool, sender: &mpsc::Sender<Message>) {
    let placing_player = lobby.players.get(&placing_for_id);
    let player_name = placing_player.map(|p| p.name.as_str()).unwrap_or("Unknown");
    let player_color = placing_player.map(|p| p.color).unwrap_or((128, 128, 128));

    render_lobby_top_panel(ctx, lobby, false, is_online, sender);

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.heading("Placing pawn for:");
                render_player_color_circle(ui, player_color, 8.0);
                ui.heading(player_name);
            });

            ui.label("Click on any board edge to place their pawn");
            ui.add_space(20.0);

            render_lobby_board(ui, lobby, placing_for_id, sender);
        });
    });

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.vertical(|ui| {
            render_player_list(ui, lobby, placing_for_id, Some(placing_for_id));

            // Only show debug tools for local lobbies
            if !is_online {
                ui.add_space(20.0);
                render_debug_tools(ui, lobby, true, sender);
            }

            ui.add_space(20.0);
            ui.separator();

            if ui.button("⬅ Back to Lobby").clicked() {
                send_ui_message(sender, Message::BackToMainMenu);
            }
        });
    });
}