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
   - Per-connection redacted game views, when online play grows beyond trusted
     groups (decision + design in CLIENT_SERVER.md "Trust Model & Hidden
     Information"): `Game::view_for` in common, redaction at the connection
     boundary in `handler.rs`, `hand_counts`/`deck_count` protocol fields,
     reworked state-sync test invariants

3. **Testing & Polish**
   - Test network latency and disconnections
   - Session-resume reconnection (proposals/004, Option A) — only if disconnect
     telemetry or user reports justify it; fail-closed handling (Option B) is in place

---

### **Phase 3: Polish & Advanced Features** (Future)
**Goal**: Code quality, type safety, and enhanced features

1. **Type System Improvements**
   - Convert `TileEndpoint` to enum with named directions

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
- Head-on pawn collisions (both die in real Tsuro) are not implemented — document
  as a house rule or implement
- Real-Tsuro fidelity variant (draw one tile per turn + dragon-tile queue) — the
  current refill-to-3 rule is a deliberate house variant; if fidelity becomes a
  goal, implement both together as one feature (Phase 3, custom game variants)
