# 005 — Code Review Findings (2026-07-03)

Full-codebase review of branch `test-ci` at commit `711a02f`, focused on
correctness, simplicity, and shippability. All of `common`, `server`, and the
client app/networking layers were read in full; the workspace test suite was
green at the time of review. Line numbers refer to `711a02f` and will drift —
function names are the stable reference.

**Verdict:** the crate split is right (`common` owns rules + protocol; server
and client are thin), the lobby module is exemplary, protocol tests target the
right historical bug class, and CI is solid. The gap between "playable with
friends" and "shippable" is almost entirely **server-side rule enforcement** —
the engine and server trust the client in ways a single honest client hides.

## Correctness findings (ordered by severity)

### 1. Core placement rule not enforced anywhere authoritative
`Game::perform_move` (`common/src/game.rs`) checks turn, liveness,
cell-occupancy, and tile-in-hand — but never that `mov.cell` is the cell the
player's pawn occupies. The honest client computes `player_cell` itself
(`app.rs::handle_tile_placed`), and `GameRoom::place_tile` adds nothing. Any
hand-rolled WebSocket client can drop tiles on arbitrary empty cells,
including onto an opponent's cell to force them off the board.

**Fix:** one comparison in `perform_move` (cell must equal the current
player's `pos.cell`) plus a test. Fixing it in `common` covers local play,
online play, and replays at once.

### 2. Connections not bound to player identities
`handle_connection` (`server/src/handler.rs`) tracks `current_player_id` per
connection but never uses it: `PlaceTile`, `PlacePawn`, and `LeaveRoom` trust
the client-supplied `player_id`. Anyone in a room can move for whoever's turn
it is, place other players' pawns, or force-disconnect them.

**Fix:** validate `player_id == current_player_id` at the handler boundary
(~10 lines, `String` errors are fine; does not need the typed-error work).

### 3. Every client receives every hand
`GameStateUpdate` broadcasts the entire `Game` — `hands` included — to the
whole room (`room.rs::place_tile`). The UI renders only your own hand, but the
wire leaks all hidden information; online play is cheatable with devtools.
`Game::export` already implements per-perspective filtering; the live protocol
doesn't. The per-room `broadcast::Sender` can't send different payloads per
player, so the fix is either per-connection filtered sends or an explicit
"open information online" decision. **Decision needed** before promoting
online play beyond trusted friends.

### 4. Room phase not checked on `JoinRoom` or `PlaceTile`
After `start_game`, `lobby` is `None`, but `join_room`
(`server/src/server.rs`) still pushes a new player into `game.players` —
alive, no hand, no stats. Turn rotation eventually selects this ghost; they
cannot move (`deduct_tile_from_hand`'s `.expect` panics the connection task if
they try) and the game wedges. Symmetrically, `PlaceTile` is not rejected
during the lobby phase, so moves can be made against the placeholder game.

**Fix:** two phase guards plus negative-path tests.

### 5. Disconnect-during-game diverges from real elimination
`GameRoom::handle_disconnect` marks the player dead by poking `game.players`
directly, then — if it was their turn — hands the turn to the *first* alive
player in the vec, not the next in rotation. It also skips everything normal
elimination does: hand not returned to the deck, `elimination_turn` stats not
recorded, no win-check (a 2-player game where one side disconnects should end
as a win, not idle).

**Fix:** add `Game::eliminate_player(id)` in `common` reusing the existing
elimination + `next_active_player_id` logic; have the server call that instead
of reaching through the authority boundary.

### 6. Replays of online games can panic
`ReplayState::current_game_state` (`client-egui/src/replay_state.rs`) uses
`.expect()` on `rebuild_from_history`, which replays moves through
`perform_move` with full turn validation. Any export from a game where the
server force-advanced the turn (finding 5) will not replay — a user-facing
crash on a legitimate file. Also, each step calls `Game::new`, which shuffles
a fresh random deck, so hands shown mid-replay are noise.

**Fix:** propagate the error into a toast; consider replay rendering ignoring
hands entirely.

### 7. Heartbeat cadence doesn't match its constants
After the first ping, `ping_interval` is permanently replaced with the 10s
`PING_TIMEOUT` interval (`handler.rs`) and never restored to the 30s
`PING_INTERVAL` — steady-state, the server pings every 10s. Harmless today,
but the constants lie. A `sleep`-with-deadline formulation would be simpler
than swapping intervals.

### 8. Stats don't match their documented meaning
In `update_players_and_trails` (`common/src/game.rs`), `path_length`
increments by 1 per move even when the trail traversed several tiles, and
`cells_visited` records only the final cell — so path length, unique-cells,
board coverage, and max-revisits all undercount multi-tile slides. The trail's
`segments` already contain exactly the per-cell data; iterate it.

## Simplicity & architecture

- **Placeholder `Game` during lobby phase** is the biggest bug generator:
  `GameRoom` carries a fake one-player `Game` from creation, and
  joins/disconnects must mutate `game.players` *and* `lobby.players` in sync.
  Findings 4 and 5 both grow from this. `enum RoomPhase { Lobby(Lobby),
  Playing(Game) }` makes the illegal states unrepresentable.
- **Continuous repaint while online:** `update()` calls
  `ctx.request_repaint()` every frame whenever a `game_client` exists —
  max-FPS CPU burn for the whole session.
  `ewebsock::connect_with_wakeup(url, options, move || ctx.request_repaint())`
  is the idiomatic fix.
- **Hand dumps in server logs:** `room.rs` / `handler.rs` print every player's
  hidden hand on every turn. Remove now; full `tracing` migration can follow.
- **Dragon is half-built:** `Game.dragon` is never assigned, so `dragon_turns`
  and the PlayerCard dragon flag are dead paths, while `fill_hands` refills
  everyone to 3 (real Tsuro draws one per turn and needs the dragon queue when
  the deck empties). Implement the rule or delete the field.
- **Head-on pawn collisions** (both die in real Tsuro) are not implemented; if
  intentional, document the house rule.
- **Client tile rotations snap back** on every `GameStateUpdate` because
  rotation mutates the client's copy of server state. A client-side rotation
  overlay per hand slot would survive updates.
- Small: `TemplateApp` is the eframe template leftover name; `sender`/
  `receiver` are `Option`s that are always `Some`; `next_connection_id` could
  be an `AtomicUsize`; `create_room` checks ID uniqueness under a read lock
  but inserts under a later write lock (colliding ID silently replaces a room
  — use the entry API under one write lock); `calculate_segment_distance`
  duplicates an 8-arm match that should be one `endpoint_pos()` called twice.
  The `board.rs` in-code question: yes, `MIN`/`MAX` are consts and legal in
  match patterns — `(0 | 1, MAX, col) => PlayerPos::new(MAX, col, tile_exit)`.

## What's notably good

`traverse_from` loop detection with real cycle tests; rotation-invariant
`is_same_tile` used consistently; `normalize_lobby_id` input hygiene; the
documented fail-closed reconnect decision (proposals/004 Option B) with the
first-reason-wins guard in `mark_disconnected`; the `pending()` trick
preventing busy-spin in the select loop; CI with fmt, clippy `-D warnings`,
audit with a documented ignore, and a cross-OS test matrix.

## Suggested order of attack

1. **Server correctness trio** — placement-cell rule in `perform_move`,
   connection-identity guard, phase guards on join/place. Small, testable
   with the existing integration harness; together they make the server
   actually authoritative. Ship-blocker set.
2. **`Game::eliminate_player`** shared by elimination and disconnect.
3. **Hand-dump log removal + repaint wakeup** — two cheap wins.
4. **Decide** hidden-information policy (finding 3) and dragon/collision
   rules — decisions first, then small code.
5. **`RoomPhase` refactor** when next touching the server; replay-panic and
   stats fixes as polish.
