use eframe::egui::{self, Context};
use std::sync::mpsc;
use crate::app::Message;
use crate::components::LobbyBoard;
use tsurust_common::board::PlayerID;
use tsurust_common::lobby::Lobby;

pub fn render_lobby_ui(ctx: &Context, lobby: &mut Lobby, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.heading(format!("Lobby: {}", lobby.name));
                ui.separator();
                ui.label(format!("Room ID: {}", lobby.id));
                ui.separator();
                ui.label(format!("Players: {}/{}", lobby.players.len(), lobby.max_players));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if lobby.can_start() {
                        if ui.button("🚀 Start Game").clicked() {
                            if let Err(e) = sender.send(Message::StartGameFromLobby) {
                                eprintln!("Failed to send StartGameFromLobby message: {}", e);
                            }
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new("⏳ Waiting for players..."));
                    }
                });
            });
        });

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

            for (player_id, lobby_player) in &lobby.players {
                ui.horizontal(|ui| {
                    let player_color = egui::Color32::from_rgb(
                        lobby_player.color.0,
                        lobby_player.color.1,
                        lobby_player.color.2
                    );

                    let circle_center = ui.cursor().min + egui::Vec2::new(12.0, 12.0);
                    ui.painter().circle_filled(circle_center, 8.0, player_color);
                    ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));

                    ui.add_space(20.0);
                    ui.label(&lobby_player.name);

                    if lobby_player.spawn_position.is_some() {
                        ui.label("✔ Ready");
                    } else {
                        ui.label("⏳ Placing pawn...");
                    }

                    if *player_id == current_player_id {
                        ui.label("(You)");
                    }
                });
                ui.add_space(5.0);
            }

            ui.add_space(20.0);
            ui.separator();

            if lobby.players.len() < lobby.max_players {
                ui.label("Waiting for more players to join...");
            }

            ui.add_space(20.0);
            ui.separator();
            ui.heading("Debug Tools");
            ui.add_space(10.0);

            if ui.button("➕ Add Test Player").clicked() {
                sender.send(Message::DebugAddPlayer).expect("Failed to send message");
            }

            ui.add_space(10.0);
            ui.label("Place pawn for:");
            for (player_id, lobby_player) in &lobby.players {
                if lobby_player.spawn_position.is_none() {
                    let player_color = egui::Color32::from_rgb(
                        lobby_player.color.0,
                        lobby_player.color.1,
                        lobby_player.color.2
                    );

                    ui.horizontal(|ui| {
                        let circle_center = ui.cursor().min + egui::Vec2::new(8.0, 8.0);
                        ui.painter().circle_filled(circle_center, 6.0, player_color);
                        ui.add_space(16.0);

                        if ui.button(&lobby_player.name).clicked() {
                            sender.send(Message::DebugPlacePawn(*player_id)).expect("Failed to send message");
                        }
                    });
                }
            }
        });
    });
}

fn render_lobby_board(ui: &mut egui::Ui, lobby: &Lobby, current_player_id: PlayerID, sender: &mpsc::Sender<Message>) {
    let board = LobbyBoard::new(lobby, current_player_id);
    board.render(ui, 300.0, sender);
}

pub fn render_lobby_placing_ui(ctx: &Context, lobby: &mut Lobby, placing_for_id: PlayerID, sender: &mpsc::Sender<Message>) {
    let placing_player = lobby.players.get(&placing_for_id);
    let player_name = placing_player.map(|p| p.name.as_str()).unwrap_or("Unknown");
    let player_color = placing_player.map(|p| p.color).unwrap_or((128, 128, 128));

    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.heading(format!("Lobby: {}", lobby.name));
                ui.separator();
                ui.label(format!("Room ID: {}", lobby.id));
                ui.separator();
                ui.label(format!("Players: {}/{}", lobby.players.len(), lobby.max_players));
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            let player_color_ui = egui::Color32::from_rgb(player_color.0, player_color.1, player_color.2);

            ui.horizontal(|ui| {
                ui.heading("Placing pawn for:");
                let circle_center = ui.cursor().min + egui::Vec2::new(8.0, 12.0);
                ui.painter().circle_filled(circle_center, 8.0, player_color_ui);
                ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));
                ui.add_space(20.0);
                ui.heading(player_name);
            });

            ui.label("Click on any board edge to place their pawn");
            ui.add_space(20.0);

            render_lobby_board(ui, lobby, placing_for_id, sender);
        });
    });

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.vertical(|ui| {

            for (player_id, lobby_player) in &lobby.players {
                ui.horizontal(|ui| {
                    let player_color = egui::Color32::from_rgb(
                        lobby_player.color.0,
                        lobby_player.color.1,
                        lobby_player.color.2
                    );

                    let circle_center = ui.cursor().min + egui::Vec2::new(12.0, 12.0);
                    ui.painter().circle_filled(circle_center, 8.0, player_color);
                    ui.painter().circle_stroke(circle_center, 8.0, (1.0, egui::Color32::WHITE));

                    ui.add_space(20.0);
                    ui.label(&lobby_player.name);

                    if lobby_player.spawn_position.is_some() {
                        ui.label("✔ Ready");
                    } else if *player_id == placing_for_id {
                        ui.label("👈 Placing now...");
                    } else {
                        ui.label("⏳ Waiting...");
                    }
                });
                ui.add_space(5.0);
            }

            ui.add_space(20.0);
            ui.separator();
            ui.heading("Debug Tools");
            ui.add_space(10.0);

            if ui.button("➕ Add Test Player").clicked() {
                sender.send(Message::DebugAddPlayer).expect("Failed to send message");
            }

            ui.add_space(10.0);
            ui.label("Switch Player:");

            ui.horizontal(|ui| {
                if ui.button("⬅ Previous").clicked() {
                    sender.send(Message::DebugCyclePlayer(false)).expect("Failed to send message");
                }
                if ui.button("Next ➡").clicked() {
                    sender.send(Message::DebugCyclePlayer(true)).expect("Failed to send message");
                }
            });

            ui.add_space(20.0);
            ui.separator();

            if ui.button("⬅ Back to Lobby").clicked() {
                sender.send(Message::BackToMainMenu).expect("Failed to send message");
            }
        });
    });
}