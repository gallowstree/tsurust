use std::sync::mpsc;

use eframe::egui;

use crate::app::Message;
use crate::messaging::send_ui_message;

/// Width of the centered form column.
const FORM_WIDTH: f32 = 380.0;

/// Center a fixed-width column horizontally. Sub-uis (like a Grid of form
/// rows) span the full width, so `vertical_centered` alone can't center
/// them — it only centers single widgets.
fn centered_column(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let pad = ((ui.available_width() - FORM_WIDTH) / 2.0).max(0.0);
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_max_width(FORM_WIDTH);
            add_contents(ui);
        });
    });
}

pub fn render_create_lobby_form(
    ui: &mut egui::Ui,
    lobby_name: &mut String,
    player_name: &mut String,
    sender: &mpsc::Sender<Message>,
) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Create Lobby");
            ui.add_space(40.0);
        });

        centered_column(ui, |ui| {
            egui::Grid::new("create_lobby_form")
                .num_columns(2)
                .spacing([8.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Lobby Name:");
                    let lobby_name_response = ui.text_edit_singleline(lobby_name);
                    // Auto-focus on first render
                    if lobby_name.is_empty() && player_name.is_empty() {
                        lobby_name_response.request_focus();
                    }
                    ui.end_row();

                    ui.label("Your Name:");
                    ui.text_edit_singleline(player_name);
                    ui.end_row();
                });
            ui.add_space(30.0);

            let can_create = !lobby_name.trim().is_empty() && !player_name.trim().is_empty();

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_create {
                send_ui_message(
                    sender,
                    Message::CreateAndJoinLobby(lobby_name.clone(), player_name.clone()),
                );
            }

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_create, egui::Button::new("Create & Join"))
                    .clicked()
                {
                    send_ui_message(
                        sender,
                        Message::CreateAndJoinLobby(lobby_name.clone(), player_name.clone()),
                    );
                }

                if ui.button("Back").clicked() {
                    send_ui_message(sender, Message::BackToMainMenu);
                }
            });
        });
    });
}

pub fn render_join_lobby_form(
    ui: &mut egui::Ui,
    lobby_id: &mut String,
    player_name: &mut String,
    pending: bool,
    sender: &mpsc::Sender<Message>,
) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Join Lobby");
            ui.add_space(40.0);
        });

        centered_column(ui, |ui| {
            egui::Grid::new("join_lobby_form")
                .num_columns(2)
                .spacing([8.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Lobby ID:");
                    let lobby_id_response = ui.text_edit_singleline(lobby_id);
                    // Auto-focus on first render
                    if lobby_id.is_empty() && player_name.is_empty() {
                        lobby_id_response.request_focus();
                    }
                    ui.end_row();

                    ui.label("");
                    ui.label(egui::RichText::new("(4-character code)").weak().small());
                    ui.end_row();

                    ui.label("Your Name:");
                    ui.text_edit_singleline(player_name);
                    ui.end_row();
                });
            ui.add_space(30.0);

            let can_join = lobby_id.trim().len() == 4 && !player_name.trim().is_empty() && !pending;

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_join {
                send_ui_message(
                    sender,
                    Message::JoinLobbyWithId(lobby_id.clone(), player_name.clone()),
                );
            }

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_join, egui::Button::new("Join"))
                    .clicked()
                {
                    send_ui_message(
                        sender,
                        Message::JoinLobbyWithId(lobby_id.clone(), player_name.clone()),
                    );
                }

                if ui.button("Back").clicked() {
                    send_ui_message(sender, Message::BackToMainMenu);
                }

                // Loading state while the join request is in flight.
                if pending {
                    ui.add_space(12.0);
                    ui.spinner();
                    ui.label(format!("Joining room {}…", lobby_id.trim().to_uppercase()));
                }
            });
        });
    });
}
