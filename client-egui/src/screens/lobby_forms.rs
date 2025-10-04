use eframe::egui::{self, Context};
use std::sync::mpsc;
use crate::app::Message;

pub fn render_create_lobby_form(ctx: &Context, lobby_name: &mut String, player_name: &mut String, sender: &mpsc::Sender<Message>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Create Lobby");
            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.label("Lobby Name:");
                let lobby_name_response = ui.text_edit_singleline(lobby_name);
                // Auto-focus on first render
                if lobby_name.is_empty() && player_name.is_empty() {
                    lobby_name_response.request_focus();
                }
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Your Name:");
                ui.text_edit_singleline(player_name);
            });
            ui.add_space(30.0);

            let can_create = !lobby_name.trim().is_empty() && !player_name.trim().is_empty();

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_create {
                sender.send(Message::CreateAndJoinLobby(
                    lobby_name.clone(),
                    player_name.clone()
                )).expect("Failed to send message");
            }

            ui.horizontal(|ui| {
                if ui.add_enabled(can_create, egui::Button::new("Create & Join")).clicked() {
                    sender.send(Message::CreateAndJoinLobby(
                        lobby_name.clone(),
                        player_name.clone()
                    )).expect("Failed to send message");
                }

                if ui.button("Back").clicked() {
                    sender.send(Message::BackToMainMenu).expect("Failed to send message");
                }
            });
        });
    });
}

pub fn render_join_lobby_form(ctx: &Context, lobby_id: &mut String, player_name: &mut String, sender: &mpsc::Sender<Message>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Join Lobby");
            ui.add_space(40.0);

            ui.horizontal(|ui| {
                ui.label("Lobby ID:");
                let lobby_id_response = ui.text_edit_singleline(lobby_id);
                // Auto-focus on first render
                if lobby_id.is_empty() && player_name.is_empty() {
                    lobby_id_response.request_focus();
                }
            });
            ui.label("(4-character code)");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Your Name:");
                ui.text_edit_singleline(player_name);
            });
            ui.add_space(30.0);

            let can_join = lobby_id.trim().len() == 4 && !player_name.trim().is_empty();

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_join {
                sender.send(Message::JoinLobbyWithId(
                    lobby_id.clone(),
                    player_name.clone()
                )).expect("Failed to send message");
            }

            ui.horizontal(|ui| {
                if ui.add_enabled(can_join, egui::Button::new("Join")).clicked() {
                    sender.send(Message::JoinLobbyWithId(
                        lobby_id.clone(),
                        player_name.clone()
                    )).expect("Failed to send message");
                }

                if ui.button("Back").clicked() {
                    sender.send(Message::BackToMainMenu).expect("Failed to send message");
                }
            });
        });
    });
}