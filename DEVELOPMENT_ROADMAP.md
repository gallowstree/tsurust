# Development Roadmap

## Current Development Priorities

### **Phase 1: Trail System Implementation** (1-2 weeks)
**Goal**: Implement the improved trail-based architecture from TRAILS.md

#### Tasks:
1. **Define Trail Data Structures**
   ```rust
   pub struct TrailSegment {
       pub board_pos: (usize, usize),
       pub entry_point: u8,
       pub exit_point: u8,
   }

   pub struct Trail {
       pub segments: Vec<TrailSegment>,
       pub start_pos: PlayerPosition,
       pub end_pos: PlayerPosition,
       pub completed: bool,
   }
   ```

2. **Update `traverse_from` to Return Trail**
   - Replace current `HashSet<(usize, usize)>` return with `Trail`
   - Maintain backward compatibility with wrapper function
   - Add trail intersection detection methods

3. **Implement Trail-Based Rendering**
   - Add `world_coordinates()` method to convert trails to screen coordinates
   - Update `BoardRenderer` to use trail data for path rendering
   - Add smooth animation support using trail data

**Acceptance Criteria**: Trails rendered with better accuracy, foundation for animations

---

### **Phase 2: Multiplayer Server** (2-3 weeks)
**Goal**: Add networked multiplayer support using tarpc RPC framework

#### Architecture Overview:
- **Server Module**: New `server/` crate for game server logic
- **RPC Service**: tarpc-based service for client-server communication
- **Client Updates**: Modify client to communicate with server instead of local game state

#### Tasks:
1. **Create Server Crate**
   ```
   server/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs          # Server binary
   │   ├── lib.rs           # Server library
   │   ├── service.rs       # tarpc service implementation
   │   ├── game_manager.rs  # Multi-game state management
   │   └── room.rs          # Game room/lobby management
   ```

2. **Define RPC Service Interface**
   ```rust
   #[tarpc::service]
   trait TsuroGameService {
       // Room management
       async fn create_room(room_name: String) -> Result<RoomId, GameError>;
       async fn join_room(room_id: RoomId, player_name: String) -> Result<PlayerId, GameError>;
       async fn leave_room(room_id: RoomId, player_id: PlayerId) -> Result<(), GameError>;

       // Game actions
       async fn place_tile(room_id: RoomId, player_id: PlayerId, tile: Tile, cell: CellCoord) -> Result<GameState, GameError>;
       async fn get_game_state(room_id: RoomId) -> Result<GameState, GameError>;

       // Real-time updates
       async fn subscribe_to_game_updates(room_id: RoomId) -> Result<GameUpdateStream, GameError>;
   }
   ```

3. **Server-Side Game Management**
   - Multi-room support with concurrent games
   - Player session management and authentication
   - Game state validation and synchronization
   - Spectator support

4. **Client-Side Integration**
   - Replace local `Game` state with `GameClient` that communicates via RPC
   - Handle network latency and connection issues
   - Implement optimistic updates for responsiveness
   - Add lobby/room selection UI

5. **Network Protocol Design**
   - Efficient serialization of game state updates
   - Delta updates instead of full state sync
   - Heartbeat/keepalive for connection management
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
