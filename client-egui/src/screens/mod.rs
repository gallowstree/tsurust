use eframe::egui::Context;
use std::sync::mpsc;
use crate::app::Message;

/// Trait for UI screens that can be rendered
pub trait Screen {
    /// Render the screen to the given context
    fn render(&mut self, ctx: &Context, sender: &mpsc::Sender<Message>);
}