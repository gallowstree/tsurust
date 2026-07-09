use std::sync::mpsc;

use eframe::egui;

use tsurust_common::lobby::Visibility;
use tsurust_common::protocol::LobbyListing;

use crate::app::{JoinScreenRequest, Message};
use crate::messaging::send_ui_message;

/// Width of the centered form column.
const FORM_WIDTH: f32 = 420.0;

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

/// The turn-timer presets offered by the create form.
const TURN_TIMER_OPTIONS: [(Option<u64>, &str); 5] = [
    (None, "No timer"),
    (Some(30), "30 seconds"),
    (Some(60), "1 minute"),
    (Some(180), "3 minutes"),
    (Some(86_400), "24 hours (correspondence)"),
];

fn turn_timer_label(value: Option<u64>) -> &'static str {
    TURN_TIMER_OPTIONS
        .iter()
        .find(|(v, _)| *v == value)
        .map(|(_, label)| *label)
        .unwrap_or("Custom")
}

pub fn render_create_lobby_form(
    ui: &mut egui::Ui,
    lobby_name: &mut String,
    player_name: &mut String,
    public: &mut bool,
    turn_timer_secs: &mut Option<u64>,
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
            ui.add_space(12.0);

            ui.checkbox(public, "Public lobby (listed for anyone to join)");
            ui.label(
                egui::RichText::new(if *public {
                    "Anyone browsing the server can join or watch."
                } else {
                    "Unlisted: players need the 4-character room code."
                })
                .weak()
                .small(),
            );
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                ui.label("Turn timer:");
                egui::ComboBox::from_id_salt("turn_timer_combo")
                    .selected_text(turn_timer_label(*turn_timer_secs))
                    .show_ui(ui, |ui| {
                        for (value, label) in TURN_TIMER_OPTIONS {
                            ui.selectable_value(turn_timer_secs, value, label);
                        }
                    });
            });
            if turn_timer_secs.is_some() {
                ui.label(
                    egui::RichText::new(
                        "When a player's clock runs out, the server plays a random \
                         surviving tile for them.",
                    )
                    .weak()
                    .small(),
                );
            }
            ui.add_space(20.0);

            let can_create = !lobby_name.trim().is_empty() && !player_name.trim().is_empty();
            let visibility = if *public {
                Visibility::Public
            } else {
                Visibility::Private
            };

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_create {
                send_ui_message(
                    sender,
                    Message::CreateAndJoinLobby(
                        lobby_name.clone(),
                        player_name.clone(),
                        visibility,
                        *turn_timer_secs,
                    ),
                );
            }

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_create, egui::Button::new("Create & Join"))
                    .clicked()
                {
                    send_ui_message(
                        sender,
                        Message::CreateAndJoinLobby(
                            lobby_name.clone(),
                            player_name.clone(),
                            visibility,
                            *turn_timer_secs,
                        ),
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
    pending: Option<&JoinScreenRequest>,
    lobbies: &[LobbyListing],
    sender: &mpsc::Sender<Message>,
) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("Join Lobby");
            ui.add_space(30.0);
        });

        centered_column(ui, |ui| {
            let idle = pending.is_none();

            // The player name applies to both paths (browser and code).
            egui::Grid::new("join_name_form")
                .num_columns(2)
                .spacing([8.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Your Name:");
                    ui.text_edit_singleline(player_name);
                    ui.end_row();
                });
            let has_name = !player_name.trim().is_empty();

            ui.add_space(20.0);

            // --- Public lobby browser ---
            ui.horizontal(|ui| {
                ui.strong("Public lobbies");
                if ui
                    .add_enabled(idle, egui::Button::new("🔄 Refresh"))
                    .clicked()
                {
                    send_ui_message(sender, Message::RefreshLobbies);
                }
            });
            ui.add_space(6.0);

            if lobbies.is_empty() {
                ui.label(
                    egui::RichText::new("No public lobbies right now.")
                        .weak()
                        .italics(),
                );
            } else {
                egui::Grid::new("public_lobby_browser")
                    .num_columns(3)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        for listing in lobbies {
                            ui.label(&listing.name);
                            if listing.in_progress {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "In game · {} players",
                                        listing.player_count
                                    ))
                                    .weak(),
                                );
                                let button = ui.add_enabled(idle, egui::Button::new("👁 Spectate"));
                                // Row-scoped accessible label (also disambiguates
                                // rows for screen readers).
                                button.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        format!("Spectate {}", listing.name),
                                    )
                                });
                                if button.clicked() {
                                    send_ui_message(
                                        sender,
                                        Message::SpectateLobby(
                                            listing.room_id.clone(),
                                            listing.name.clone(),
                                        ),
                                    );
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}/{} players",
                                        listing.player_count, listing.max_players
                                    ))
                                    .weak(),
                                );
                                let full = listing.player_count >= listing.max_players;
                                let can_join = idle && has_name && !full;
                                let button = ui.add_enabled(can_join, egui::Button::new("Join"));
                                // Row-scoped accessible label (also disambiguates
                                // rows for screen readers).
                                button.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        format!("Join {}", listing.name),
                                    )
                                });
                                if full {
                                    button.on_hover_text("This lobby is full");
                                } else if !has_name {
                                    button.on_hover_text("Enter your name first");
                                } else if button.clicked() {
                                    send_ui_message(
                                        sender,
                                        Message::JoinLobbyWithId(
                                            listing.room_id.clone(),
                                            player_name.clone(),
                                        ),
                                    );
                                }
                            }
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(12.0);

            // --- Private lobby, by room code ---
            ui.strong("Private lobby");
            ui.add_space(6.0);
            egui::Grid::new("join_lobby_form")
                .num_columns(2)
                .spacing([8.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Room code:");
                    ui.text_edit_singleline(lobby_id);
                    ui.end_row();

                    ui.label("");
                    ui.label(egui::RichText::new("(4-character code)").weak().small());
                    ui.end_row();
                });
            ui.add_space(12.0);

            let can_join_by_code = lobby_id.trim().len() == 4 && has_name && idle;

            // Submit on Enter key
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_join_by_code {
                send_ui_message(
                    sender,
                    Message::JoinLobbyWithId(lobby_id.clone(), player_name.clone()),
                );
            }

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_join_by_code, egui::Button::new("Join by code"))
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

                // Loading state while a join/spectate request is in flight.
                if let Some(request) = pending {
                    ui.add_space(12.0);
                    ui.spinner();
                    match request {
                        JoinScreenRequest::Join => {
                            ui.label("Joining…");
                        }
                        JoinScreenRequest::Spectate { room_name, .. } => {
                            ui.label(format!("Connecting to spectate {room_name}…"));
                        }
                    }
                }
            });
        });
    });
}
