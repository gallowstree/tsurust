use std::sync::mpsc;

use crate::app::Message;

/// Helper to send a message and log errors instead of panicking.
/// Returns true if the message was sent successfully.
pub fn send_message(sender: &mpsc::Sender<Message>, message: Message) -> bool {
    if let Err(e) = sender.send(message) {
        eprintln!("Failed to send message: {}", e);
        false
    } else {
        true
    }
}
