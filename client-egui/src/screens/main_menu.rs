use std::sync::mpsc;

use eframe::egui::{self, Context};

use crate::app::Message;
use crate::messaging::send_message;

pub fn render(ctx: &Context, sender: &mpsc::Sender<Message>) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("🐉 Tsurust");
            ui.add_space(20.0);
            ui.label("Year of the Dragon of Wood");
            ui.add_space(50.0);

            let button_width = 200.0;

            if ui.add_sized([button_width, 30.0], egui::Button::new("🌐 Create Online Lobby")).clicked() {
                send_message(sender, Message::ShowCreateLobbyForm);
            }

            ui.add_space(10.0);

            if ui.add_sized([button_width, 30.0], egui::Button::new("🔗 Join Online Lobby")).clicked() {
                send_message(sender, Message::ShowJoinLobbyForm);
            }

            ui.add_space(10.0);

            if ui.add_sized([button_width, 30.0], egui::Button::new("🏠 Local Game")).clicked() {
                send_message(sender, Message::StartLobby);
            }

            ui.add_space(10.0);

            if ui.add_sized([button_width, 30.0], egui::Button::new("🎮 Sample Game")).clicked() {
                send_message(sender, Message::StartSampleGame);
            }
        });
    });
}