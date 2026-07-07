use crate::app::Message;
use crate::ws_client::ConnectionStatus;
use eframe::egui;
use eframe::egui::Context;
use std::sync::mpsc;

pub mod game;
pub mod lobby;
pub mod lobby_forms;
pub mod main_menu;
pub mod replay_viewer;

/// Small status chip showing the WebSocket connection state, for the top
/// panel of online screens.
pub fn connection_chip(ui: &mut egui::Ui, status: &ConnectionStatus) {
    let (color, label) = match status {
        ConnectionStatus::Connecting => (egui::Color32::from_rgb(230, 190, 80), "Connecting…"),
        ConnectionStatus::Connected => (egui::Color32::from_rgb(90, 200, 120), "Connected"),
        ConnectionStatus::Disconnected { .. } => {
            (egui::Color32::from_rgb(220, 90, 90), "Disconnected")
        }
    };
    ui.colored_label(color, format!("● {label}"));
}

/// Trait for UI screens that can be rendered
/// Placeholder trait for future screen abstraction
#[allow(dead_code)]
pub trait Screen {
    /// Render the screen to the given context
    fn render(&mut self, ctx: &Context, sender: &mpsc::Sender<Message>);
}
