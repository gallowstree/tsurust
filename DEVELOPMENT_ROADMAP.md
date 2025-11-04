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

3. **Testing & Polish**
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
  - TODO: Add tests that serialize/deserialize all protocol messages

### Medium Priority
- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Trail system - Current tile-based approach should be replaced with TRAILS.md design
- Unicode glyph rendering - Consider runtime detection if more rendering issues occur
