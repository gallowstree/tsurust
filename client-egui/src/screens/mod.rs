use crate::app::Message;
use eframe::egui::Context;
use std::sync::mpsc;

pub mod game;
pub mod lobby;
pub mod lobby_forms;
pub mod main_menu;
pub mod replay_viewer;

/// Trait for UI screens that can be rendered
/// Placeholder trait for future screen abstraction
#[allow(dead_code)]
pub trait Screen {
    /// Render the screen to the given context
    fn render(&mut self, ctx: &Context, sender: &mpsc::Sender<Message>);
}
