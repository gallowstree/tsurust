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

2. **Server Authority & Rule Enforcement** (ship-blockers — see proposals/005)
   - Enforce the core placement rule in `Game::perform_move`: `mov.cell` must be
     the current player's pawn cell (currently only the honest client enforces
     this — cheat/desync vector for any hand-rolled WS client)
   - Bind connections to identities in `handler.rs`: validate client-supplied
     `player_id` against the connection's tracked player for
     `PlaceTile`/`PlacePawn`/`LeaveRoom` (impersonation is trivial today)
   - Add phase guards: reject `JoinRoom` after game start (ghost player with no
     hand wedges turn rotation) and `PlaceTile` during the lobby phase
   - Add `Game::eliminate_player(id)` in `common` and use it for in-game
     disconnects: fixes turn advancing to first-alive instead of next-in-rotation,
     hand not returned to deck, missing stats, and missing win-check

3. **Server-Side Improvements**
   - Add room cleanup for abandoned games (timeout-based, currently immediate on last player leaving)
   - Add proper error handling and validation (typed errors instead of `String`/`&'static str`)
   - Remove hand-content debug prints from server logs (`room.rs`, `handler.rs`
     dump every player's hidden hand each turn); migrate the rest to `tracing`
   - Decide hidden-information policy: `GameStateUpdate` broadcasts all hands to
     every client (wire-level leak; `Game::export` already has per-perspective
     filtering, the live protocol does not) — per-connection filtered sends or
     an explicit open-hands ruling
   - Refactor `GameRoom`'s dual state (placeholder `Game` + `Option<Lobby>`,
     player lists kept in sync by hand) into `enum RoomPhase { Lobby, Playing }`

4. **Testing & Polish**
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
- `client-egui/src/app.rs` - unconditional `request_repaint()` every frame while
  a `game_client` exists burns CPU at max FPS; use `ewebsock::connect_with_wakeup`
- Client tile rotations snap back on every `GameStateUpdate` (rotation mutates
  the client's copy of server state); keep a client-side rotation overlay
- Dragon tile is half-built: `Game.dragon` is never assigned, `dragon_turns` and
  the PlayerCard dragon flag are dead paths — implement the Tsuro dragon rule or
  remove the field
- Head-on pawn collisions (both die in real Tsuro) are not implemented — document
  as a house rule or implement
- `server/src/server.rs::create_room` - room-ID uniqueness checked under a read
  lock but inserted under a later write lock; a colliding ID silently replaces an
  existing room (use the entry API under one write lock)
