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
   - Migrate server `println!`/`eprintln!` logging to `tracing` with leveled
     events and room/player/connection fields
   - Per-connection redacted game views, when online play grows beyond trusted
     groups (decision + design in CLIENT_SERVER.md "Trust Model & Hidden
     Information"): `Game::view_for` in common, redaction at the connection
     boundary in `handler.rs`, `hand_counts`/`deck_count` protocol fields,
     reworked state-sync test invariants
   - Refactor `GameRoom`'s dual state (placeholder `Game` + `Option<Lobby>`,
     player lists kept in sync by hand) into `enum RoomPhase { Lobby, Playing }`

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
- `client-egui/src/replay_state.rs` - `current_game_state()` panics via `.expect()`
  on exports whose turn order was force-advanced by the server (disconnects);
  also rebuilds each step with a freshly shuffled deck so mid-replay hands are
  noise. Propagate the error to a toast; consider hiding hands in replay.
- `common/src/game.rs::update_players_and_trails` - stats undercount multi-tile
  moves: `path_length` +1 per move regardless of cells traversed, and
  `cells_visited` records only the final cell (iterate `trail.segments` instead)
- `server/src/handler.rs` - heartbeat pings every `PING_TIMEOUT` (10s) after the
  first cycle instead of `PING_INTERVAL` (30s); the interval swap never reverts
- Client tile rotations snap back on every `GameStateUpdate` (rotation mutates
  the client's copy of server state); keep a client-side rotation overlay
- Head-on pawn collisions (both die in real Tsuro) are not implemented — document
  as a house rule or implement
- Real-Tsuro fidelity variant (draw one tile per turn + dragon-tile queue) — the
  current refill-to-3 rule is a deliberate house variant; if fidelity becomes a
  goal, implement both together as one feature (Phase 3, custom game variants)
- `server/src/server.rs::create_room` - room-ID uniqueness checked under a read
  lock but inserted under a later write lock; a colliding ID silently replaces an
  existing room (use the entry API under one write lock)
