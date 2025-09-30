# Development Roadmap & Architecture Review

## Current Architecture Assessment

### ✅ Strengths
- **Clean separation of concerns**: Game logic properly isolated in `common/`, UI in `client-egui/`
- **Sound data modeling**: Tiles as 4 segments with 8 endpoints, proper position tracking with `PlayerPos`
- **Functional UI patterns**: Proper Widget trait implementations, message passing with mpsc channels
- **Interactive tile handling**: TileButton supports rotation (left/right click to rotate tiles)
- **Custom rendering**: Hand-drawn primitives work well for the game's aesthetic

### ✅ Recently Completed Major Systems

1. **Complete Game Loop Implementation** (`common/src/game.rs:47-79`)
   - ✅ Full `perform_move()` with validation, tile placement, and player movement
   - ✅ Turn management with proper player validation
   - ✅ Hand refilling logic implemented
   - ✅ Player elimination and win condition detection

2. **Full Tile Placement System**
   - ✅ `Message::TilePlaced` and `Message::TileRotated` implemented
   - ✅ Center-click to place tile at current player position
   - ✅ Left/right click to rotate tiles in hand
   - ✅ Complete UI flow from hand selection to board placement

3. **Player Movement and Trail System**
   - ✅ `update_players_and_trails()` with path traversal
   - ✅ `fill_hands()` maintains 3 tiles per player
   - ✅ `complete_turn()` advances turns and handles game end
   - ✅ Trail rendering system showing player paths

### 🎯 Current Status: FUNCTIONAL GAME

### ⚠️ Remaining Issues

#### **MEDIUM PRIORITY**

1. **Trail System Architecture Decision**
   - Current: Tile-based trail tracking with player/endpoint mapping
   - Proposed: Full trail data structure (see TRAILS.md)
   - **Impact**: Better animation, collision detection, and extensibility

2. **Type Safety Improvements**
   - `TileEndpoint` is `usize` but should be enum (noted in comments)
   - Some hardcoded error messages remain
   - **Impact**: Better developer experience and type safety

3. **UI Polish**
   - No visual indication of current player turn
   - No game status display (turns, eliminated players)
   - Limited feedback for invalid moves

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

### **Phase 2: UI Enhancements** (1 week)
**Goal**: Improve game status visibility and user experience

#### Tasks:
1. **Add Game Status Display**
   - Show current player turn with color indicator
   - Display eliminated players and turn count
   - Add game over screen with winner announcement

2. **Improve Visual Feedback**
   - Highlight valid placement positions
   - Show error messages for invalid moves
   - Add hover effects and selection indicators

3. **Player Setup Improvements**
   - Allow players to select starting positions during setup
   - Validate starting positions are on board edges
   - Add player name customization

**Acceptance Criteria**: Clear game state communication, better user experience

---

### **Phase 3: Multiplayer Server** (2-3 weeks)
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

### **Phase 4: Polish & Advanced Features** (Ongoing)
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

3. **Advanced Features**
   - Implement game replay system
   - Add AI opponents that can join multiplayer games
   - Tournament/ranking system
   - Spectator mode with game history
   - Custom game variants and rule modifications

## Current Game Status Assessment

### ✅ Fully Implemented Core Game:
- **Complete Game Loop**: Full tile placement, player movement, turn management
- **Player Management**: 4 players, elimination logic, win conditions
- **Trail Rendering**: Visual player paths with distinct colors
- **UI Integration**: TileButton rotation/placement, message system
- **Game Logic**: Deck management, hand refilling, edge detection

### 🎯 Current State: PLAYABLE TSURO GAME
The game is now functionally complete and playable from start to finish.

### Success Metrics:
- [x] Multiple players visible on board with different colors
- [x] Turn advances between players after tile placement
- [x] Players are eliminated when reaching board edges
- [x] Game declares winner when one player remains
- [x] Trail overlaps show correctly for multiple players
- [x] Complete game can be played from start to finish

### Next Iteration Focus:
1. **Trail System Redesign** - Implement TRAILS.md proposal for better architecture
2. **UI Polish** - Game status display, better visual feedback
3. **Code Quality** - Type safety improvements, error handling

## Technical Debt Log

- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- Multiple files - Remove unused imports (warnings in cargo build)
- `client-egui/src/app.rs:37` - Handle unused variable `t` from append operation
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Trail system - Current tile-based approach should be replaced with TRAILS.md design

## Trail System Evolution

### Current Implementation: Tile-Based Player Path Mapping ✅
**Status**: Currently implemented and working

**Approach**: Maintain a mapping during tile rendering:
```rust
tile_position -> Vec<(PlayerID, TileEndpoint)>
```

**Benefits Achieved**:
- ✅ Accurate Paths: Trails match tile segments exactly
- ✅ Real-time Accuracy: Trail rendering uses same primitives as tiles
- ✅ Multiple Player Support: Shows overlapping trails correctly

### Next Evolution: Full Trail Data Structure (TRAILS.md)
**Status**: Proposed improvement for better extensibility

**Key Improvements**:
- Better animation support with interpolated positions
- Trail intersection and collision detection
- Cleaner separation of topological vs. visual concerns
- Foundation for advanced features (replay, AI analysis)

**Migration Strategy**: Implement alongside current system, then gradually migrate

## Notes

- Project builds successfully with warnings
- 7 tests currently pass in common crate
- UI framework (egui) is working well
- Message passing architecture is sound but underutilized
- Focus on game logic completion before adding new features