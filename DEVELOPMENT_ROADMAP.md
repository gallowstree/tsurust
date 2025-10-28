# Development Roadmap

## Current Development Priority

### **Phase 2: Client-Server Integration** (Active)
**Goal**: Complete the multiplayer WebSocket implementation

#### Tasks:

1. **UI Improvements for Online/Offline Modes**
   - Distinguish local vs online game modes in main menu
   - Local Game button creates offline multiplayer lobby
   - Online lobby buttons clarified with "Online" prefix
   - Maintain sample game for quick testing

2. **Server-Side Improvements**
   - Implement player disconnect handling (`handler.rs:75`, `server.rs:116`)
   - Add room cleanup for abandoned games
   - Implement heartbeat/ping-pong for connection health
   - Add proper error handling and validation

3. **Client-Server State Sync**
   - Wire up `GameStateUpdate` handler in client (`app.rs:194`)
   - Implement actual game moves through server (currently local only)
   - Synchronize lobby state between clients
   - Handle server errors with user feedback

4. **Multiplayer Game Flow**
   - Send tile placement moves to server instead of handling locally
   - Receive and apply game state updates from server
   - Display other players' actions in real-time
   - Handle turn management across clients

5. **Testing & Polish**
   - **[CRITICAL]** Add integration tests for online multiplayer (tile placement, state sync)
   - **[CRITICAL]** Add serialization tests for all protocol messages (prevent JSON key errors)
   - Test multiple clients in same game
   - Test network latency and disconnections
   - Add reconnection logic with exponential backoff
   - Test lobby flow (create, join, start game)

---

### **Phase 3: Polish & Advanced Features** (Future)
**Goal**: Code quality, type safety, and enhanced features

#### High Priority:
1. **Animation System**
   - Add smooth animation when player follows trail after tile placement
   - Implement configurable animation speed
   - Add visual feedback during player movement

2. **UI Enhancements**
   - Game over screen with winner announcement UI
   - Visual error messages for invalid moves
   - Connection status indicators
   - Loading states for network operations

#### Lower Priority:
1. **Type System Improvements**
   - Convert `TileEndpoint` to enum with named directions
   - Add proper error types instead of string literals
   - Implement comprehensive validation

2. **Advanced Features**
   - Implement game replay system
   - Add AI opponents that can join multiplayer games
   - Tournament/ranking system
   - Spectator mode with game history
   - Custom game variants and rule modifications



## Technical Debt Log

### High Priority
- **Serialization safety** - Add compile-time checks to prevent non-string HashMap keys in serializable structs
  - Issue: `player_trails` and `tile_trails` fields broke online multiplayer due to JSON serialization failure
  - Fix: Marked as `#[serde(skip)]` but need better safeguards
  - TODO: Add tests that serialize/deserialize all protocol messages
- **Server error logging** - Improve error visibility in async handlers
  - Issue: Serialization errors were silently failing until explicit logging added
  - Fix: Always use `match` with explicit error logging, never `if let Ok(...)`
  - TODO: Add structured logging with levels (error, warn, info, debug)

### Medium Priority
- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Trail system - Current tile-based approach should be replaced with TRAILS.md design
- Unicode glyph rendering - Consider runtime detection if more rendering issues occur
- Remove debug logging from `server/src/handler.rs` and `server/src/room.rs` once online multiplayer is stable
