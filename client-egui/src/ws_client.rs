use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use serde::{Deserialize, Serialize};
use tsurust_common::board::{Move, PlayerID};
use tsurust_common::game::{Game, TurnResult};

pub type RoomId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    CreateRoom {
        room_name: String,
        creator_name: String,
    },
    JoinRoom {
        room_id: RoomId,
        player_name: String,
    },
    LeaveRoom {
        room_id: RoomId,
        player_id: PlayerID,
    },
    PlaceTile {
        room_id: RoomId,
        player_id: PlayerID,
        mov: Move,
    },
    GetGameState {
        room_id: RoomId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    RoomCreated {
        room_id: RoomId,
        player_id: PlayerID,
    },
    PlayerJoined {
        room_id: RoomId,
        player_id: PlayerID,
        player_name: String,
    },
    PlayerLeft {
        room_id: RoomId,
        player_id: PlayerID,
    },
    GameStateUpdate {
        room_id: RoomId,
        state: Game,
    },
    TurnCompleted {
        room_id: RoomId,
        result: TurnResult,
    },
    Error {
        message: String,
    },
}

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
        self.ws_sender.send(WsMessage::Text(json));
    }

    pub fn try_recv(&self) -> Option<ServerMessage> {
        while let Some(event) = self.ws_receiver.try_recv() {
            match event {
                WsEvent::Opened => {
                    println!("WebSocket connection opened");
                }
                WsEvent::Message(WsMessage::Text(json)) => {
                    match serde_json::from_str(&json) {
                        Ok(msg) => return Some(msg),
                        Err(e) => {
                            eprintln!("Failed to parse server message: {}", e);
                        }
                    }
                }
                WsEvent::Error(e) => {
                    eprintln!("WebSocket error: {}", e);
                }
                WsEvent::Closed => {
                    println!("WebSocket connection closed");
                    // Note: We don't set self.connected = false here because
                    // self is immutable. Caller should handle this.
                }
                _ => {}
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
}
