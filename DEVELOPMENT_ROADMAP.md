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
   - Add room cleanup for abandoned games (timeout-based, currently immediate on last player leaving)
   - Add proper error handling and validation (typed errors instead of `String`/`&'static str`)

3. **Testing & Polish**
   - Test network latency and disconnections
   - Session-resume reconnection (proposals/004, Option A) — only if disconnect
     telemetry or user reports justify it; fail-closed handling (Option B) is in place

---

### **Phase 3: Polish & Advanced Features** (Future)
**Goal**: Code quality, type safety, and enhanced features

#### High Priority:
1. **UI Enhancements**
   - Visual error messages for invalid moves
   - Connection status indicators
   - Loading states for network operations
   - Improve animation timing configurability
   - URL-based room joining: navigating to `http://hostname:port/<roomID>` should attempt to join that room

#### Lower Priority:
1. **Type System Improvements**
   - Convert `TileEndpoint` to enum with named directions
   - Add proper error types instead of string literals
   - Implement comprehensive validation

2. **Advanced Features**
   - Add AI opponents that can join multiplayer games
   - Tournament/ranking system
   - Spectator mode with game history
   - Custom game variants and rule modifications



## Technical Debt Log

### Medium Priority
- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Unicode glyph rendering - Consider runtime detection if more rendering issues occur
