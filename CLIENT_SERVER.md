# Client-Server Architecture for Tsurust Multiplayer

## Overview

Tsurust uses **WebSockets** for all client-server communication. WebSockets provide bidirectional, real-time messaging which is ideal for multiplayer games where:
1. Clients send commands (tile placement) to the server
2. Server broadcasts game state updates to all clients in real-time
3. No polling needed - server pushes updates immediately

## Technology Stack

- **Server**: `tokio-websockets` - High-performance WebSocket server with SIMD acceleration
- **Client**: `ewebsock` - WebSocket client that compiles to both native and WASM (egui-compatible)

## Implementation Decisions

1. **RoomId Type**: `String` (human-readable, easy to share)
2. **PlayerID**: Use existing `PlayerID` from `tsurust_common::board` if available, otherwise define in protocol
3. **Serialization**: Add `Serialize`/`Deserialize` derives to `Game`, `Move`, `Tile` in `common/`
4. **Authentication**: Not implemented initially - add in future iteration
5. **Server Port**: Hardcoded to `8080`

## Architecture

```
Client (ewebsock)              Server (tokio-websockets)
     │                                │
     ├─ WebSocket Connection :8080 ───┤
     │                                │
     │  ──── ClientMessage ──────>    │
     │  (PlaceTile, JoinRoom, etc)    │
     │                                │
     │  <──── ServerMessage ──────    │
     │  (GameStateUpdate, etc)        │
```

## Message Protocol

All messages are JSON-serialized using `serde`.

### Client → Server Messages

```rust
#[derive(Serialize, Deserialize)]
enum ClientMessage {
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
    },
    PlaceTile {
        room_id: RoomId,
        player_id: PlayerId,
        mov: Move,
    },
    GetGameState {
        room_id: RoomId,
    },
}
```

### Server → Client Messages

```rust
#[derive(Serialize, Deserialize)]
enum ServerMessage {
    RoomCreated {
        room_id: RoomId,
        player_id: PlayerId,
    },
    PlayerJoined {
        room_id: RoomId,
        player_id: PlayerId,
        player_name: String,
    },
    PlayerLeft {
        room_id: RoomId,
        player_id: PlayerId,
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
```

## Server Implementation

### Room Management

```rust
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use std::sync::Arc;

struct GameRoom {
    id: RoomId,
    game: Game,
    // Broadcast channel for pushing updates to all connected clients
    update_tx: broadcast::Sender<ServerMessage>,
}

struct GameServer {
    rooms: Arc<RwLock<HashMap<RoomId, GameRoom>>>,
    // Map of WebSocket connections to their room
    connections: Arc<RwLock<HashMap<ConnectionId, RoomId>>>,
}

impl GameRoom {
    pub async fn place_tile(&mut self, player_id: PlayerId, mov: Move) -> Result<TurnResult, GameError> {
        // Validate and perform move
        let result = self.game.perform_move(mov)?;

        // Broadcast update to all clients in this room
        let update = ServerMessage::TurnCompleted {
            room_id: self.id.clone(),
            result: result.clone(),
        };
        let _ = self.update_tx.send(update);

        // Also send full state update
        let state_update = ServerMessage::GameStateUpdate {
            room_id: self.id.clone(),
            state: self.game.clone(),
        };
        let _ = self.update_tx.send(state_update);

        Ok(result)
    }
}
```

### WebSocket Handler

```rust
use tokio_websockets::{ServerBuilder, Message};

async fn handle_connection(
    mut ws: WebSocket,
    connection_id: ConnectionId,
    server: Arc<GameServer>,
) {
    let mut update_rx: Option<broadcast::Receiver<ServerMessage>> = None;

    loop {
        tokio::select! {
            // Receive messages from client
            Some(Ok(msg)) = ws.next() => {
                if let Message::Text(text) = msg {
                    let client_msg: ClientMessage = serde_json::from_str(&text)?;

                    match client_msg {
                        ClientMessage::JoinRoom { room_id, player_name } => {
                            // Subscribe to room updates
                            let rooms = server.rooms.read().await;
                            if let Some(room) = rooms.get(&room_id) {
                                update_rx = Some(room.update_tx.subscribe());
                            }
                        }
                        ClientMessage::PlaceTile { room_id, player_id, mov } => {
                            let mut rooms = server.rooms.write().await;
                            if let Some(room) = rooms.get_mut(&room_id) {
                                let _ = room.place_tile(player_id, mov).await;
                            }
                        }
                        // ... handle other messages
                    }
                }
            }

            // Forward room updates to this client
            Some(Ok(update)) = async {
                match &mut update_rx {
                    Some(rx) => rx.recv().await.ok(),
                    None => None,
                }
            } => {
                let json = serde_json::to_string(&update)?;
                ws.send(Message::text(json)).await?;
            }
        }
    }
}
```

## Client Implementation

### Connection Setup

```rust
use ewebsock::{WsMessage, WsEvent};

struct GameClient {
    ws_sender: ewebsock::WsSender,
    ws_receiver: ewebsock::WsReceiver,
}

impl GameClient {
    pub fn connect(url: &str) -> Result<Self, ewebsock::Error> {
        let options = ewebsock::Options::default();
        let (ws_sender, ws_receiver) = ewebsock::connect(url, options)?;

        Ok(Self {
            ws_sender,
            ws_receiver,
        })
    }

    pub fn send(&mut self, msg: ClientMessage) {
        let json = serde_json::to_string(&msg).unwrap();
        self.ws_sender.send(WsMessage::Text(json));
    }

    pub fn try_recv(&self) -> Option<ServerMessage> {
        while let Some(event) = self.ws_receiver.try_recv() {
            match event {
                WsEvent::Message(WsMessage::Text(json)) => {
                    if let Ok(msg) = serde_json::from_str(&json) {
                        return Some(msg);
                    }
                }
                WsEvent::Error(e) => {
                    eprintln!("WebSocket error: {:?}", e);
                }
                WsEvent::Closed => {
                    eprintln!("WebSocket closed");
                }
                _ => {}
            }
        }
        None
    }
}
```

### Integration with egui

```rust
impl TemplateApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for server updates
        while let Some(server_msg) = self.game_client.try_recv() {
            match server_msg {
                ServerMessage::GameStateUpdate { state, .. } => {
                    self.game = state;
                }
                ServerMessage::TurnCompleted { result, .. } => {
                    // Handle turn result, show animations, etc.
                }
                ServerMessage::Error { message } => {
                    self.error_message = Some(message);
                }
                // ... handle other messages
            }
        }

        // Request repaint if waiting for updates
        if self.waiting_for_server {
            ctx.request_repaint();
        }

        // ... rest of UI code
    }
}
```

## Key Design Decisions

### Why WebSockets?

1. **Bidirectional**: Both client and server can initiate messages
2. **Real-time**: Instant push notifications, no polling
3. **Simple**: Single connection for all communication
4. **Widely supported**: Works in browsers (for future WASM build) and native
5. **Efficient**: Less overhead than HTTP polling

### Message Format: JSON

- **Human-readable**: Easy to debug
- **Flexible**: Easy to add new fields
- **Well-supported**: serde integration
- **Trade-off**: Larger than binary formats (acceptable for this game's data volume)

### Broadcast Channels

Using `tokio::sync::broadcast` for room updates:
- **One-to-many**: Server sends once, all clients receive
- **Non-blocking**: Slow clients don't block fast ones
- **Backpressure**: Lagging clients get errors and can resync

## Connection Lifecycle

### 1. Client Connects
```
Client → Server: WebSocket connection established
```

### 2. Join Room
```
Client → Server: JoinRoom { room_id, player_name }
Server → All clients in room: PlayerJoined { player_id, player_name }
Server → Client: GameStateUpdate { state }
```

### 3. Gameplay
```
Client → Server: PlaceTile { player_id, mov }
Server validates and updates game state
Server → All clients in room: TurnCompleted { result }
Server → All clients in room: GameStateUpdate { state }
```

### 4. Disconnection Handling
```
Client disconnects (network issue, tab closed, etc.)
Server detects closed connection
Server → All clients in room: PlayerLeft { player_id }
```

### 5. Reconnection
```
Client reconnects
Client → Server: JoinRoom with same player_id (if saved)
Server → Client: GameStateUpdate { state }  // Full resync
```

## Error Handling

### Client-Side
- Reconnect with exponential backoff on disconnect
- Request full state sync after reconnection
- Show error UI when connection lost

### Server-Side
- Validate all moves before applying
- Send `ServerMessage::Error` for invalid requests
- Clean up disconnected clients from rooms
- Prevent abandoned games from consuming memory

## Security Considerations

### Future Enhancements
1. **Authentication**: Add player tokens/sessions
2. **Rate Limiting**: Prevent message spam
3. **Input Validation**: Verify all client messages
4. **TLS**: Use `wss://` for encrypted connections

## Performance Considerations

### Message Size
- Full `Game` state ~1-5KB JSON
- Typical `TurnCompleted` ~500 bytes
- Target: <100ms latency for local networks

### Scalability
- Current design: Single server, multiple rooms
- Future: Horizontal scaling with room sharding
- Each room is independent, easy to distribute

## Testing Strategy

### Unit Tests
- Test message serialization/deserialization
- Mock WebSocket for game logic tests

### Integration Tests
- Spin up test server
- Connect multiple test clients
- Verify state synchronization

### Manual Testing
- Multiple browser tabs/windows
- Network latency simulation
- Disconnect/reconnect scenarios
