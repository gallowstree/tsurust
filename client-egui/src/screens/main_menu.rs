use eframe::egui::{self, Context};
use std::sync::mpsc;
use crate::app::Message;

pub fn render(ctx: &Context, sender: &mpsc::Sender<Message>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("🐉 Tsurust");
            ui.add_space(20.0);
            ui.label("Year of the Dragon of Wood");
            ui.add_space(50.0);

            let button_width = 200.0;

            if ui.add_sized([button_width, 30.0], egui::Button::new("➕ Create Lobby")).clicked() {
                sender.send(Message::ShowCreateLobbyForm).expect("Failed to send message");
            }

            ui.add_space(10.0);

            if ui.add_sized([button_width, 30.0], egui::Button::new("🔗 Join Lobby")).clicked() {
                sender.send(Message::ShowJoinLobbyForm).expect("Failed to send message");
            }

            ui.add_space(10.0);

            if ui.add_sized([button_width, 30.0], egui::Button::new("🎮 Sample Game")).clicked() {
                sender.send(Message::StartSampleGame).expect("Failed to send message");
            }
        });
    });
}