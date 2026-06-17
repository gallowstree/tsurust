use std::sync::mpsc;

use eframe::egui::{self, Context};

use crate::app::{LocalServerStatus, Message};
use crate::messaging::send_ui_message;

pub fn render(ctx: &Context, server_status: &LocalServerStatus, sender: &mpsc::Sender<Message>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("🐉 Tsurust");
            ui.add_space(20.0);
            ui.label("Year of the Dragon of Wood");
            ui.add_space(50.0);

            let button_width = 200.0;

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
