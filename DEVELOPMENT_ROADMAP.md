# Development Roadmap & Architecture Review

## Current Architecture Assessment

### ✅ Strengths
- **Clean separation of concerns**: Game logic properly isolated in `common/`, UI in `client-egui/`
- **Sound data modeling**: Tiles as 4 segments with 8 endpoints, proper position tracking with `PlayerPos`
- **Functional UI patterns**: Proper Widget trait implementations, message passing with mpsc channels
- **Interactive tile handling**: TileButton supports rotation (left/right click to rotate tiles)
- **Custom rendering**: Hand-drawn primitives work well for the game's aesthetic

### ⚠️ Critical Issues Requiring Immediate Attention

#### **HIGH PRIORITY**

1. **Incomplete Game Loop** (`common/src/game.rs:35-48`)
   - `perform_move()` is mostly commented out
   - No player validation or turn management
   - Missing hand refilling logic
   - **Impact**: Game is not actually playable

2. **No Tile Placement Mechanism**
   - TileButton only sends generic `Message::Clicked`
   - No way to communicate which tile was selected
   - No mechanism to place tiles from hand onto board
   - **Impact**: Core gameplay mechanic is missing

3. **Player Movement System Incomplete**
   - `update_players()`, `fill_hands()`, `complete_turn()` are empty stubs
   - No path traversal after tile placement
   - **Impact**: Players don't move, game doesn't progress

#### **MEDIUM PRIORITY**

4. **Limited Message System**
   - Only `Message::Clicked` exists
   - Need messages for tile selection, placement, rotation
   - **Impact**: UI can't communicate detailed actions

5. **Type Safety Issues**
   - `TileEndpoint` is `usize` but should be enum (noted in comments)
   - Missing validation and error types
   - **Impact**: Runtime errors, unclear intent

6. **Error Handling**
   - Hardcoded error messages ("fuck the client" in deduct_tile_from_hand)
   - Missing proper validation
   - **Impact**: Poor developer experience, unclear failures

## Prioritized Development Roadmap

### **Phase 1: Core Game Loop** (1-2 weeks)
**Goal**: Make the game actually playable with basic tile placement

#### Tasks:
1. **Expand Message Types**
   ```rust
   pub enum Message {
       TileSelected(usize),  // index in hand
       TilePlaced(Tile, CellCoord),
       TileRotated(usize, bool), // index, clockwise
   }
   ```

2. **Complete `perform_move()` Function**
   - Add player turn validation
   - Implement tile deduction from hand
   - Add basic move validation (valid placement)
   - Ensure tile is actually placed on board

3. **Implement Tile Placement UI Flow**
   - TileButton sends `TileSelected` on center click
   - Add board click handling for placement
   - Connect hand selection to board placement

4. **Add Basic Turn Management**
   - Track current player
   - Enforce turn order
   - Prevent invalid moves

**Acceptance Criteria**: Player can select tile from hand, place it on board, and turn advances

---

### **Phase 2: Player Movement** (1 week)
**Goal**: Players move along paths after tile placement

#### Tasks:
1. **Complete `update_players()` Method**
   - Implement path traversal logic using placed tiles
   - Update player positions after each move
   - Handle edge detection for elimination

2. **Implement `fill_hands()` Method**
   - Maintain 3 tiles per player
   - Handle deck depletion
   - Return eliminated player tiles to deck

3. **Add Player Elimination Logic**
   - Detect when players reach board edge
   - Remove eliminated players from active play
   - Return their tiles to deck

**Acceptance Criteria**: Players move correctly along tile paths and are eliminated at edges

---

### **Phase 3: Complete Game Flow** (1 week)
**Goal**: Full game with win/lose conditions and proper state management

#### Tasks:
1. **Game State Management**
   - Add game phases (setup, playing, ended)
   - Implement win conditions (last player standing)
   - Add game over detection

2. **Complete Turn Cycling**
   - Handle player elimination in turn order
   - Skip eliminated players
   - End game when only one player remains

3. **Enhanced Validation**
   - Prevent placing tiles on occupied cells
   - Validate tile placement legality
   - Add proper error reporting

**Acceptance Criteria**: Complete games can be played from start to finish with proper winner determination

---

### **Phase 4: Multiplayer Server** (2-3 weeks)
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

### **Phase 5: Polish & Advanced Features** (Ongoing)
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

## Immediate Next Steps

### Week 1 Priority:
1. **Expand Message enum** with `TileSelected`, `TilePlaced`, `TileRotated`
2. **Modify TileButton** to send tile index on center click
3. **Add board click handling** in BoardRenderer
4. **Implement basic tile placement** in `perform_move()`

### Success Metrics:
- [ ] Can select tile from hand (visual feedback)
- [ ] Can place selected tile on empty board cell
- [ ] Turn advances to next player after placement
- [ ] Basic game loop functions without crashes

## Technical Debt Log

- `common/src/game.rs:53` - Replace "fuck the client" with proper error type
- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- Multiple files - Remove unused imports (warnings in cargo build)
- `client-egui/src/app.rs:37` - Handle unused variable `t` from append operation
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum

## Notes

- Project builds successfully with warnings
- 7 tests currently pass in common crate
- UI framework (egui) is working well
- Message passing architecture is sound but underutilized
- Focus on game logic completion before adding new features