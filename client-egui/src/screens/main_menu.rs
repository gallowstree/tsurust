use std::sync::mpsc;

use eframe::egui;

use crate::app::{LocalServerStatus, Message, DEFAULT_WS_URL};
use crate::messaging::send_ui_message;

pub fn render(
    ui: &mut egui::Ui,
    server_status: &LocalServerStatus,
    server_url: &mut String,
    sender: &mpsc::Sender<Message>,
) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("🐉 Tsurust");
            ui.add_space(20.0);
            ui.label("Year of the Dragon of Wood");
            ui.add_space(30.0);

            let button_width = 200.0;

            // Server address for online play. Friends paste a host's wss:// URL
            // here, or arrive via a `?server=…` invite link that prefills it.
            ui.group(|ui| {
                ui.set_max_width(360.0);
                ui.horizontal(|ui| {
                    ui.label("Server:");
                    ui.add(
                        egui::TextEdit::singleline(server_url)
                            .hint_text("wss://host…")
                            .desired_width(260.0),
                    );
                });

                // Loud fallback: warn when the effective URL can't reach a shared
                // host, instead of silently pointing at 127.0.0.1.
                let trimmed = server_url.trim();
                if trimmed.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 80, 80),
                        "⚠ No server set — paste a host's wss:// URL to play online.",
                    );
                } else if cfg!(target_arch = "wasm32") && trimmed == DEFAULT_WS_URL {
                    ui.colored_label(
                        egui::Color32::from_rgb(210, 150, 0),
                        "⚠ Pointed at your own machine (127.0.0.1). Paste a host's \
                         wss:// URL, or open a shared invite link.",
                    );
                }
            });

            ui.add_space(20.0);

            if ui
                .add_sized(
                    [button_width, 30.0],
                    egui::Button::new("🌐 Create Online Lobby"),
                )
                .clicked()
            {
                send_ui_message(sender, Message::ShowCreateLobbyForm);
            }

            ui.add_space(10.0);

            if ui
                .add_sized(
                    [button_width, 30.0],
                    egui::Button::new("🔗 Join Online Lobby"),
                )
                .clicked()
            {
                send_ui_message(sender, Message::ShowJoinLobbyForm);
            }

            ui.add_space(10.0);

            if ui
                .add_sized(
                    [button_width, 30.0],
                    egui::Button::new("🖥️ Start Local Server"),
                )
                .clicked()
            {
                send_ui_message(sender, Message::StartLocalServer);
            }

            // Show server status feedback
            match server_status {
                LocalServerStatus::Running(pid) => {
                    ui.label(format!("✅ Server running (PID: {})", pid));
                }
                LocalServerStatus::Failed(error) => {
                    ui.colored_label(egui::Color32::RED, format!("❌ Server failed: {}", error));
                }
                LocalServerStatus::NotStarted => {}
            }

            ui.add_space(10.0);

            if ui
                .add_sized([button_width, 30.0], egui::Button::new("🏠 Local Game"))
                .clicked()
            {
                send_ui_message(sender, Message::StartLobby);
            }

            ui.add_space(10.0);

            if ui
                .add_sized([button_width, 30.0], egui::Button::new("🎮 Sample Game"))
                .clicked()
            {
                send_ui_message(sender, Message::StartSampleGame);
            }

            ui.add_space(10.0);

            if ui
                .add_sized([button_width, 30.0], egui::Button::new("📂 Load Replay"))
                .clicked()
            {
                send_ui_message(sender, Message::ImportReplay);
            }
        });
    });
}
