use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

use tsurust_common::board::{Move, PlayerID, PlayerPos};
use tsurust_common::protocol::{ClientMessage, RoomId, ServerMessage};

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    /// The socket closed or errored. The game is over for this client; there is
    /// no automatic reconnection (see proposals/004-websocket-reconnection.md,
    /// Option B). The UI surfaces this and routes back to the main menu.
    Disconnected {
        reason: String,
    },
}

pub struct GameClient {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    pub connected: bool,
    pending_messages: Vec<ClientMessage>,
    pub status: ConnectionStatus,
}

impl GameClient {
    /// Connect to the server. `wake_up` is invoked whenever a socket event
    /// arrives so the UI can request a repaint instead of polling every frame.
    pub fn connect(url: &str, wake_up: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let options = ewebsock::Options::default();
        let (ws_sender, ws_receiver) = ewebsock::connect_with_wakeup(url, options, wake_up)
            .map_err(|e| format!("Failed to connect to {}: {}", url, e))?;

        Ok(Self {
            ws_sender,
            ws_receiver,
            connected: false,
            pending_messages: Vec::new(),
            status: ConnectionStatus::Connecting,
        })
    }

    pub fn send(&mut self, msg: ClientMessage) {
        if !self.connected {
            #[cfg(target_arch = "wasm32")]
            {
                web_sys::console::log_1(
                    &format!("[WS_CLIENT] Queuing message (not connected yet): {:?}", msg).into(),
                );
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                println!("[WS_CLIENT] Queuing message (not connected yet): {:?}", msg);
            }
            self.pending_messages.push(msg);
            return;
        }

        let json = serde_json::to_string(&msg).expect("Failed to serialize client message");
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::console::log_1(&format!("[WS_CLIENT] Sending message: {}", json).into());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            println!("[WS_CLIENT] Sending message: {}", json);
        }
        self.ws_sender.send(WsMessage::Text(json));
    }

    pub fn try_recv(&mut self) -> Option<ServerMessage> {
        while let Some(event) = self.ws_receiver.try_recv() {
            match event {
                WsEvent::Opened => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::log_1(&"[WS_CLIENT] WebSocket connection opened".into());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("[WS_CLIENT] WebSocket connection opened");
                    }
                    self.connected = true;
                    self.status = ConnectionStatus::Connected;

                    // Send all queued messages
                    let pending = std::mem::take(&mut self.pending_messages);
                    for msg in pending {
                        self.send(msg);
                    }
                }
                WsEvent::Message(WsMessage::Text(json)) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::log_1(
                            &format!("[WS_CLIENT] Received message: {}", json).into(),
                        );
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("[WS_CLIENT] Received message: {}", json);
                    }
                    match serde_json::from_str(&json) {
                        Ok(msg) => return Some(msg),
                        Err(e) => {
                            #[cfg(target_arch = "wasm32")]
                            {
                                web_sys::console::error_1(
                                    &format!(
                                        "Failed to parse server message: {}\nRaw: {}",
                                        e, json
                                    )
                                    .into(),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                eprintln!("Failed to parse server message: {}", e);
                                eprintln!("Raw message: {}", json);
                            }
                        }
                    }
                }
                WsEvent::Error(e) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::error_1(&format!("WebSocket error: {}", e).into());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        eprintln!("WebSocket error: {}", e);
                    }
                    self.mark_disconnected(format!("connection error: {}", e));
                }
                WsEvent::Closed => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::warn_1(&"[WS_CLIENT] WebSocket connection closed".into());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("WebSocket connection closed");
                    }
                    self.mark_disconnected("connection closed".to_string());
                }
                _ => {}
            }
        }
        None
    }

    /// Mark the connection as permanently lost. Option B is fail-closed: there is
    /// no reconnection, so any queued messages are dropped and the UI is expected
    /// to route the user back to the main menu.
    fn mark_disconnected(&mut self, reason: String) {
        // Keep the first reason; a trailing Closed after an Error shouldn't clobber it.
        if matches!(self.status, ConnectionStatus::Disconnected { .. }) {
            return;
        }
        self.connected = false;
        self.pending_messages.clear();
        self.status = ConnectionStatus::Disconnected { reason };
    }

    pub fn create_room(&mut self, room_name: String, creator_name: String) {
        self.send(ClientMessage::CreateRoom {
            room_name,
            creator_name,
        });
    }

    pub fn join_room(&mut self, room_id: RoomId, player_name: String) {
        self.send(ClientMessage::JoinRoom {
            room_id,
            player_name,
        });
    }

    pub fn leave_room(&mut self, room_id: RoomId, player_id: PlayerID) {
        self.send(ClientMessage::LeaveRoom { room_id, player_id });
    }

    pub fn place_tile(&mut self, room_id: RoomId, player_id: PlayerID, mov: Move) {
        self.send(ClientMessage::PlaceTile {
            room_id,
            player_id,
            mov,
        });
    }

    pub fn get_game_state(&mut self, room_id: RoomId) {
        self.send(ClientMessage::GetGameState { room_id });
    }

    pub fn place_pawn(&mut self, room_id: RoomId, player_id: PlayerID, position: PlayerPos) {
        self.send(ClientMessage::PlacePawn {
            room_id,
            player_id,
            position,
        });
    }

    pub fn start_game(&mut self, room_id: RoomId) {
        self.send(ClientMessage::StartGame { room_id });
    }
}
