use std::sync::mpsc;

use tsurust_common::protocol::ClientMessage;

use crate::app::Message;

/// Helper to send a UI message and log errors instead of panicking.
/// Returns true if the message was sent successfully.
pub fn send_ui_message(sender: &mpsc::Sender<Message>, message: Message) -> bool {
    if let Err(e) = sender.send(message) {
        eprintln!("Failed to send UI message: {}", e);
        false
    } else {
        true
    }
}

/// Helper to send a server message via the UI message channel.
/// This wraps the ClientMessage in a SendToServer variant and sends it through mpsc.
/// Returns true if the message was sent successfully.
pub fn send_server_message(sender: &mpsc::Sender<Message>, message: ClientMessage) -> bool {
    send_ui_message(sender, Message::SendToServer(message))
}
