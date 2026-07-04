# Tsurust Implementation Plan

*Baseline: branch `test-ci` — a superset of all merged feature branches and +12 ahead
of `master`. All seven remote feature branches (`trails`, `with_unpushed`, `ui-polish`,
`lobby`, `client-server`, `wasm-support`, `dev/upgrade-libraries`) are fully merged into
`test-ci`; there is no unmerged work to recover. `gh-pages` is deploy output only.*

This plan was produced after verifying the actual state of each task in the code, not
just the roadmap. Status reflects what exists on `test-ci` as of 2026-06-17.

## Legend
- **Status:** ❌ net-new · ◑ partial (scaffolding exists) · ✅ done (close roadmap item)
- **Effort:** S (<½ day) · M (1–2 days) · L (multi-day)

---

## `common/` — core logic & protocol

### C1 — `TileEndpoint` enum — ◑ scaffolding noted · M
`pub type TileEndpoint = usize` (`board.rs:14`) already carries a comment prescribing
`enum { NE, NW, EN, ES, WN, WS, SW, SE }` in the current 0–7 order.
1. Define `#[repr(u8)]` enum with `Serialize/Deserialize`, preserving the 0–7 numeric
   order so the wire format stays compatible.
2. Add `from_u8`/`as_u8` (or `TryFrom`) helpers — renderer and trail math index by number.
3. Migrate `board.rs`, `trail.rs` (`entry_point`/`exit_point`), segment construction, and
   the index math in `client-egui/src/rendering.rs` / `board_renderer.rs`.
4. Confirm `protocol_tests.rs` round-trips unchanged.
- **Risk:** serialization-sensitive; round-trip tests are the guard.

### C2 — Typed errors — ❌ · M — foundation for S4
`common` returns `&'static str` (`perform_move -> Result<TurnResult, &'static str>`,
`deduct_tile_from_hand`); `server` re-wraps as `String` (7 sites).
1. Add a `MoveError`/`GameError` enum in `common` (not-your-turn, wrong player_id,
   illegal cell, tile-not-in-hand, game-over).
2. Change `perform_move`/`deduct_tile_from_hand` to the enum; update `game.rs` tests.
3. In `server`, wrap `GameError` + transport/validation cases; map to
   `ServerMessage::Error { message }` at the boundary (wire protocol unchanged).
- **Dep:** do before S4. Pairs with L1.

### C3 — Fix production `unwrap()` — ◑ trivial · S — **DONE in Phase A**
Only one production `unwrap()` existed: `board.rs:122`. Replaced with `.expect(...)`.

### C4 — Trail system — ✅ already implemented — **closed in Phase A**
`TRAILS.md` self-reports implemented; `Trail`/`TrailSegment`, `traverse_from()` full
trails, `player_trails`/`current_turn_trails`, and rendering all present. Stale debt line
removed from the roadmap. Remaining optional follow-ups (trail animation, collision-based
features) are not tracked unless wanted.

### C5 — Serialization-safety guard — ✅ downgraded/closed — **closed in Phase A**
`protocol_tests.rs` round-trips every message including int-keyed `HashMap`s (the
historical bug class). A compile-time proc-macro guard isn't worth it here; the item is
closed as "documented + test-enforced." High-priority debt entry removed from roadmap.

---

## `server/` — multiplayer

### S1 — Timeout-based room cleanup — ❌ · M — only real Phase-2 server gap
Rooms live in `Arc<RwLock<HashMap<RoomId, GameRoom>>>`; removed only synchronously in
`handle_disconnect` when `should_remove`. No reaping of idle/abandoned rooms.
1. Add `last_activity: Instant` to `GameRoom`, bumped on every handled message + join.
2. Spawn a `tokio::time::interval` background loop (~60s): write-lock, drop rooms idle
   beyond a configurable threshold.
3. Base idle on *no connected players* + grace (reuse `connections` map), not raw message
   silence — don't reap a live game waiting on a slow player.
4. Test: idle room reaped; active room survives.

### S2 — Structured logging — ❌ · S–M
29 raw `println!`/`eprintln!` in `server` (e.g. `[SERVER] Broadcasting...` in `room.rs`).
1. Add `tracing` + `tracing-subscriber`; init in `main.rs` with `RUST_LOG` filter.
2. Convert prints to leveled events with `room_id`/`player_id`/`connection_id` fields.
3. Keep client prints (37) out of scope for this task.

### S3 — 3+ client integration test — ◑ 2-player exists · M — highest bug-guard value
`integration_tests.rs` has 10 tests, all 2-player. Directly guards the recurring
multiplayer bugs in `HANDOFF.md`.
1. 3–4 player test: create → 3 joins → place pawns → start → multi-turn placement.
2. Assert state-sync: each broadcast `GameStateUpdate` agrees on `current_player_id`,
   board history, distinct hands (the "both see same tiles" bug).
3. Drive an elimination; assert turn order skips it and `TurnResult` variants are correct.
- **Dep:** none — can start immediately.

### S4 — Input validation hardening — ◑ · S–M
Turn/player checks exist in `room.rs::place_tile`; no systematic guard on malformed room
IDs / unknown players / wrong phase.
- After C2, add typed validation at the handler boundary + negative-path tests.
- **Dep:** after C2.

### S5 — Heartbeat — ✅ done — **closed in Phase A**
Ping/pong + pong-timeout disconnect implemented in `handler.rs:26-110`. Roadmap line
removed.

---

## `client-egui/` — GUI

### L1 — Surface server errors — ◑ status banner exists; error toast is a live TODO · S–M — recommended first
`ServerMessage::Error` → `eprintln!` only (`app.rs:581`); connect failure → literal
`// TODO: Display error message in UI` (`app.rs:727`). The `ConnectionStatus` disconnect
modal (`app.rs:431`) is a separate channel.
1. Add `last_error: Option<(String, Instant)>` (skip-serialize) to `TemplateApp`.
2. Set it in the `Error` arm and the connect-failure `Err(e)` arm.
3. Render a dismissible, auto-expiring toast/banner (reuse the cross-platform `Instant`).
- Consumes C2's better messages but doesn't depend on it.

### L2 — Connection-status indicator — ◑ disconnect modal only · S
Verified: `Disconnected` shows a modal (`app.rs:431`), but `Connecting`/`Connected` are
never surfaced.
- Add a persistent status chip (●) in online screens covering Connecting/Connected too.

### L3 — Loading states — ❌ · S–M
`waiting_for_server` exists in `OnlineGame` but no spinner/disabled-control treatment.
- Drive a busy indicator from `waiting_for_server` + `ConnectionStatus::Connecting`;
  disable action buttons while pending.

### L4 — URL-based room joining (WASM) — ❌ confirmed unimplemented · M
`main_wasm.rs` only sets up canvas; the only URL-adjacent code reads
`TSURUST_CONFIG.wsServerUrl` (server config) and `set_href` (downloads). No path/query
parsing, no auto-join.
1. Read `window.location` path/hash via `web_sys` at WASM startup.
2. If a room id is present, auto-issue `JoinRoom` after connect.
3. Optionally `history.pushState` the room id for shareable links.
4. Native: cfg-gated no-op.
- **Open UX question:** how does a URL-joiner supply their player name (prompt vs
  auto-generate)? Decide before building.

### L5 — Configurable animation timing — ◑ · S
Timing hard-coded in `board_renderer.rs`. Extract into an `AnimationConfig`; optional
settings slider. Low priority.

### L6 — WebSocket reconnection w/ backoff — deferred by design · L
Intentionally fail-closed (`ws_client.rs` + `proposals/004`, Option B). Pursue only if
disconnect telemetry/user reports justify it: session-resume token server-side, client
exponential backoff, server grace window before reaping the player. Do last.

---

## Recommended execution order

- **Phase A — close stale debt & quick wins (DONE):** C3 fix; C4 + C5 + S5 roadmap
  closures; L2 verified (stays open — needs status chip).
- **Phase B — multiplayer correctness (highest value):** S3 → L1 → S1.
- **Phase C — error-handling backbone:** C2 → S4 → S2.
- **Phase D — UX polish:** L3 → L4 (needs UX decision) → L5.
- **Phase E — only if justified:** C1 (serialization-sensitive) and L6 (reconnection).
