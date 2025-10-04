use eframe::egui::Context;
use std::sync::mpsc;
use crate::app::Message;

pub mod main_menu;
pub mod lobby_forms;
pub mod lobby;
pub mod game;

/// Trait for UI screens that can be rendered
pub trait Screen {
    /// Render the screen to the given context
    fn render(&mut self, ctx: &Context, sender: &mpsc::Sender<Message>);
}