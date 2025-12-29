use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

use tsurust_common::board::{Move, PlayerID, PlayerPos};
use tsurust_common::protocol::{ClientMessage, RoomId, ServerMessage};

pub struct GameClient {
    ws_sender: WsSender,
    ws_receiver: WsReceiver,
    pub connected: bool,
}

impl GameClient {
    pub fn connect(url: &str) -> Result<Self, String> {
        let options = ewebsock::Options::default();
        let (ws_sender, ws_receiver) = ewebsock::connect(url, options)
            .map_err(|e| format!("Failed to connect to {}: {}", url, e))?;

        Ok(Self {
            ws_sender,
            ws_receiver,
            connected: true,
        })
    }

    pub fn send(&mut self, msg: ClientMessage) {
        let json = serde_json::to_string(&msg)
            .expect("Failed to serialize client message");
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

    pub fn try_recv(&self) -> Option<ServerMessage> {
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
                }
                WsEvent::Message(WsMessage::Text(json)) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::log_1(&format!("[WS_CLIENT] Received message: {}", json).into());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("[WS_CLIENT] Received message: {}", json);
                    }
                    match serde_json::from_str(&json) {
                        Ok(msg) => {
                            return Some(msg)
                        },
                        Err(e) => {
                            #[cfg(target_arch = "wasm32")]
                            {
                                web_sys::console::error_1(&format!("Failed to parse server message: {}\nRaw: {}", e, json).into());
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
                }
                _ => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        web_sys::console::log_1(&"[WS_CLIENT] Other event type".into());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        println!("[WS_CLIENT] Other event type");
                    }
                }
            }
        }
        None
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
        self.send(ClientMessage::LeaveRoom {
            room_id,
            player_id,
        });
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
