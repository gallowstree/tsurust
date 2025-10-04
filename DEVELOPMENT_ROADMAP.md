# Development Roadmap

## Current Development Prioritie

### **Phase 1: Trail System & Game Flow Polish**

#### Remaining Tasks:
1. **Animation System**
   - Add smooth animation when player follows trail after tile placement
   - Implement configurable animation speed
   - Add visual feedback during player movement

2. **Game Flow Improvements**
   - Add game over detection and winner announcement
   - Implement proper turn management UI feedback
   - Add invalid move error messages

---

### **Phase 2: Multiplayer Server** (2-3 weeks)
**Goal**: Add networked multiplayer support using WebSockets

#### Architecture Overview:
- **Server Module**: New `server/` crate for game server logic
- **WebSocket Communication**: Message-based protocol for client-server communication
- **Client Updates**: Modify client to communicate with server instead of local game state

#### Tasks:
1. **Create Server Crate**
   ```
   server/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs          # Server binary (tokio-websockets server)
   │   ├── lib.rs           # Server library
   │   ├── handler.rs       # WebSocket message handler
   │   ├── game_manager.rs  # Multi-game state management
   │   └── room.rs          # Game room/lobby management
   ```

2. **Define WebSocket Message Protocol**
   ```rust
   // Client -> Server messages
   enum ClientMessage {
       CreateRoom { room_name: String },
       JoinRoom { room_id: RoomId, player_name: String },
       LeaveRoom { room_id: RoomId },
       PlaceTile { room_id: RoomId, tile: Tile, cell: CellCoord },
       GetGameState { room_id: RoomId },
   }

   // Server -> Client messages
   enum ServerMessage {
       RoomCreated { room_id: RoomId },
       PlayerJoined { player_id: PlayerId },
       GameStateUpdate { state: GameState },
       Error { message: String },
       PlayerDisconnected { player_id: PlayerId },
   }
   ```

3. **Server-Side Game Management**
   - Multi-room support with concurrent games
   - Player session management via WebSocket connections
   - Game state validation and synchronization
   - Spectator support

4. **Client-Side Integration**
   - WebSocket client integration (ewebsock for egui compatibility)
   - Replace local `Game` state with messages to/from server
   - Handle network latency and connection issues
   - Implement optimistic updates for responsiveness
   - Add lobby/room selection UI

5. **Network Protocol Design**
   - JSON serialization for game state (serde)
   - Delta updates instead of full state sync
   - Heartbeat/ping-pong for connection management
   - Graceful handling of disconnections and reconnections

**Acceptance Criteria**:
- Multiple clients can join same game room
- Real-time synchronized gameplay across all clients
- Robust handling of network issues and player disconnections

---

### **Phase 3: Polish & Advanced Features** (Ongoing)
**Goal**: Code quality, type safety, and enhanced features

#### Tasks:
1. **Type System Improvements**
   - Convert `TileEndpoint` to enum with named directions
   - Add proper error types instead of string literals
   - Implement comprehensive validation

2. **Code Quality**
   - Remove unused imports and variables (many warnings currently)
   - Improve error messages and user feedback
   - Add comprehensive tests for game logic and RPC layer

3. **Remaining UI Enhancements**
   - Game over screen with winner announcement
   - Error messages for invalid moves
   - Player setup improvements (custom starting positions)
   - Player name customization

4. **Advanced Features**
   - Implement game replay system
   - Add AI opponents that can join multiplayer games
   - Tournament/ranking system
   - Spectator mode with game history
   - Custom game variants and rule modifications



## Technical Debt Log

- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Trail system - Current tile-based approach should be replaced with TRAILS.md design
- Unicode glyph rendering - Consider runtime detection if more rendering issues occur
