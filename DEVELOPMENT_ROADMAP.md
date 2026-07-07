# Development Roadmap

Active TODOs and known technical debt only. Completed work lives in git history,
not here (see the documentation policy in CLAUDE.md).

## Planned work

### Networking
- Session-resume reconnection (`proposals/004`, Option A) — only if disconnect
  telemetry or user reports justify it; the current behavior is fail-closed.
- Exercise network latency and flaky-connection handling.

### Features
- AI opponents that can join multiplayer games.
- Tournament / ranking system.
- Spectator game-history scrubbing (step back through moves while watching live).
- Custom game variants and rule modifications.

## Technical debt

- Convert `TileEndpoint` from `usize` to an enum with named directions, and
  rename lingering "TileEndpoint" references to "entry point"
  (`common/src/board.rs`, `common/src/lib.rs`).
- Unicode glyph rendering — consider runtime detection if more rendering issues
  surface.
- Head-on pawn collisions (both pawns die in real Tsuro) are not implemented —
  decide: bless as a house rule, or implement.
- Real-Tsuro fidelity variant (draw one tile per turn + dragon-tile queue). The
  current refill-to-3 rule is a deliberate house variant; if fidelity becomes a
  goal, implement both together as one feature (with custom game variants).
