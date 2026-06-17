# Proposal 004: WebSocket Reconnection Strategy

**Status:** Draft
**Author:** Claude
**Date:** 2026-05-07
**Estimated Effort:** Option A: 4-5 days · Option B: 0.5 day

---

## Summary

The client-egui WebSocket client currently has an in-flight reconnection
implementation (`client-egui/src/ws_client.rs:58-78`, `app.rs:399-419`) that is
buggy: after a successful socket reconnect, the server has lost all session
state and the client never re-establishes its identity. The user *appears*
reconnected but is silently severed from their game.

This proposal explores two ways to fix this:

- **Option A — Session resume.** Reconnect transparently. Add a session token
  to the protocol and a server-side grace period so a player can rejoin their
  in-progress game.
- **Option B — Fail-closed.** Treat any disconnect as fatal. Drop the
  `tick()` / `ConnectionStatus::Reconnecting` machinery, surface a clear
  error, and route the user back to the main menu.

Recommendation: **Option B for now**, with Option A scheduled if/when we have
data showing it matters. Justification in §6.

---

## 1. Motivation

### 1.1 Current bugs

The reconnection code in `ws_client.rs` has three concrete defects:

1. **Identity is not re-established after reconnect.** When the socket reopens
   (`ws_client.rs:111-129`), the client flips `connected = true` and flushes
   any queued messages, but those messages reference a `player_id` and
   `room_id` that no longer exist server-side — `handle_connection`'s local
   `current_room` / `current_player_id` (`server/src/handler.rs:24-25`) went
   out of scope when the previous connection died, and the room itself has
   already mutated (`server/src/room.rs:120-176` removes the player from the
   lobby or marks them eliminated).

2. **No connect-attempt timeout.** `tick()` (`ws_client.rs:58-78`) only fires
   while `status == Reconnecting`. If `ewebsock::connect()` returns `Ok` but
   the socket never reaches `WsEvent::Opened` (server unreachable, slow
   handshake), the state sits in `Connecting` forever — no further retry.

3. **Redundant state.** `self.reconnect_attempt` (line 36) and
   `ConnectionStatus::Reconnecting { attempt }` (line 27) track the same
   counter twice. They can drift.

In addition, `pending_messages` (line 34) grows unbounded during long
disconnects.

### 1.2 Server-side reality

The server (`server/src/server.rs`, `room.rs`) has **no concept of a session**.
A `ConnectionId` is a per-socket integer (`server.rs:14`), not a player
identity. The `(room, player_id)` association lives only on the
`handle_connection` task's stack (`handler.rs:23-28`). When the socket dies:

- **In lobby phase** (`room.rs:121-136`): the player is fully removed from
  both the lobby and the game player list.
- **In game phase** (`room.rs:137-167`): the player is marked
  `alive = false`, the turn advances, and a `PlayerLeft` broadcast goes out.

The protocol has no resume path: `ClientMessage::JoinRoom`
(`common/src/protocol.rs:15-18`) carries only `room_id` and `player_name`.

So even *if* the client kept its identity, the server has already destroyed
or eliminated the player by the time the socket reopens.

### 1.3 Why this matters

Today, network glitches in the WASM client produce silent failures: the user
sees a banner, the banner disappears, and then no input does anything.
Whichever direction we go, that needs to stop.

---

## 2. Constraints

- **WASM target.** `std::time::Instant` is unavailable in browsers — current
  code uses `web_time::Instant`. Any solution must compile for both targets.
- **No server-side persistence.** Rooms are in-memory (`server.rs:17`). A
  server restart wipes everything; reconnection cannot survive that.
- **Existing 81 tests must keep passing.** Especially the seven server
  integration tests in `server/src/integration_tests.rs` that already cover
  disconnect cleanup.
- **Hand-drawn rendering, simple egui app.** No need to over-engineer; this
  is a hobby-scale game with pickup multiplayer.

---

## 3. Option A — Session Resume

### 3.1 Overview

Give every connection a server-issued session token. On disconnect, the
server holds the player's slot for a grace period instead of eliminating
them. The client transparently reconnects and replays the token to re-bind
to the same `(room_id, player_id)`.

### 3.2 Protocol changes

```rust
// common/src/protocol.rs

pub type SessionToken = String;  // opaque, server-generated

#[derive(Serialize, Deserialize)]
pub enum ClientMessage {
    // ... existing variants
    Resume {
        session_token: SessionToken,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ServerMessage {
    // ... existing variants

    // Sent immediately after RoomCreated / PlayerJoined.
    SessionEstablished {
        session_token: SessionToken,
        room_id: RoomId,
        player_id: PlayerID,
    },

    // Sent in response to Resume.
    SessionResumed {
        room_id: RoomId,
        player_id: PlayerID,
        // Followed by a normal LobbyStateUpdate or GameStateUpdate.
    },

    SessionExpired,  // grace period elapsed; client should fall back to main menu
}
```

### 3.3 Server-side changes

```rust
// server/src/session.rs (new)

pub struct Session {
    pub room_id: RoomId,
    pub player_id: PlayerID,
    pub disconnected_at: Option<Instant>,
}

pub struct SessionStore {
    by_token: HashMap<SessionToken, Session>,
}
```

In `GameServer`:

```rust
pub struct GameServer {
    pub rooms: Arc<RwLock<HashMap<RoomId, GameRoom>>>,
    pub sessions: Arc<RwLock<SessionStore>>,
    // ...
}
```

Behavior changes:

- **`create_room` / `join_room`** mint a token, return `SessionEstablished`.
- **`handle_disconnect`** in `server.rs:150` no longer immediately removes /
  eliminates. Instead it marks `disconnected_at = Some(Instant::now())` on
  the session and lets a background task sweep stale sessions after
  `RECONNECT_GRACE` (suggested: 30s lobby phase, 60s game phase).
- **`Resume`** handler looks up the token, atomically clears
  `disconnected_at`, sets `current_room` / `current_player_id` on the
  handler stack, sends `SessionResumed` + current `LobbyStateUpdate` /
  `GameStateUpdate`.
- **In-game turn handling:** if the disconnected player's turn comes up
  while they're in grace, skip them (or pause the room — design decision).

### 3.4 Client-side changes

```rust
// client-egui/src/ws_client.rs

pub struct GameClient {
    // ...
    session_token: Option<SessionToken>,
}

// In tick(): after socket reopens, if we have a token, send Resume first.
// Drain pending_messages only after SessionResumed arrives.
```

The reconnection banner (`app.rs:402-419`) stays, but transitions through
`Reconnecting → Connecting → Resuming → Connected`.

### 3.5 Edge cases (must be tested)

| Case | Expected behavior |
|------|-------------------|
| Reconnect within grace | `SessionResumed`, game state catches up |
| Reconnect after grace | `SessionExpired`, route to main menu |
| Server restart mid-disconnect | Resume fails (no token), fall back gracefully |
| Two clients with the same token | Reject the second; first wins |
| Resume in lobby phase | Player still in lobby, no-op state-wise |
| Resume mid-game on your turn | Hand and board state must match what server has |
| Token leaked / forged | Constant-time compare; treat unknown tokens as fatal |

### 3.6 Test plan (additions)

New tests in `server/src/integration_tests.rs`:

- `test_resume_within_grace_restores_player`
- `test_resume_after_grace_returns_session_expired`
- `test_disconnect_in_grace_does_not_eliminate_player`
- `test_resume_with_unknown_token_fails`
- `test_grace_sweep_removes_player_after_timeout`
- `test_turn_skips_disconnected_player_in_grace`

New unit tests in `server/src/session.rs`:

- Token generation uniqueness
- Sweep logic with mocked clock

Client-side: pull `tick()` / `backoff()` into a testable struct that
doesn't own real `WsSender`/`WsReceiver` (trait-bound or split the timer
logic into a pure-function module).

### 3.7 Cost

- ~250 LOC server, ~80 LOC client, ~30 LOC protocol
- Roughly 8 new tests
- Real risk: turn-skipping logic during grace period interacts non-trivially
  with game state and broadcast ordering. This is where bugs will hide.

---

## 4. Option B — Fail-Closed

### 4.1 Overview

Disconnect = game over for this client. No reconnection, no tokens, no
grace period. Show a clear error, stop trying, and route the user back to
the main menu where they can rejoin manually if the room still exists.

### 4.2 Client-side changes

In `ws_client.rs`:

- Delete `tick()`, `ConnectionStatus::Reconnecting`, `backoff()`,
  `reconnect_attempt`, `on_disconnect()`.
- Replace `ConnectionStatus` with a flag-style enum:

```rust
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected { reason: String },
}
```

- On `WsEvent::Closed` / `WsEvent::Error`: transition to `Disconnected`,
  drop `pending_messages`.

In `app.rs`:

- Replace the reconnection banner with a modal-style "Disconnected" panel
  containing the reason and a "Return to menu" button.
- On click: drop `game_client`, reset `screen` to `MainMenu`, clear lobby
  / game state.

Server: **no changes.** The existing `handle_disconnect`
(`server.rs:150-160`) already does the right thing — the client just
acknowledges it instead of pretending otherwise.

### 4.3 UX

```
┌─────────────────────────────────────────────┐
│  ⚠  Disconnected from server                │
│                                             │
│  Connection lost: <reason>                  │
│                                             │
│  Your game has ended.                       │
│                                             │
│           [ Return to menu ]                │
└─────────────────────────────────────────────┘
```

### 4.4 Test plan (additions)

Existing integration tests already cover server-side disconnect behavior
(`test_disconnect_eliminates_player_and_advances_turn`,
`test_last_player_disconnect_removes_room`). New work:

- `client-egui` unit test: `GameClient::try_recv` on `WsEvent::Closed`
  produces `ConnectionStatus::Disconnected`. Requires the same
  testability refactor as Option A (decouple from `WsSender`/`WsReceiver`).
- Manual smoke test: kill server mid-game, verify modal appears, button
  routes to menu.

### 4.5 Cost

- Net code reduction: ~80 LOC removed, ~40 LOC added
- 1-2 new tests
- Risk: near zero. We're collapsing buggy machinery into a known-good
  no-op.

---

## 5. Comparison

| Dimension | Option A: Resume | Option B: Fail-closed |
|-----------|------------------|------------------------|
| User experience on flaky wifi | Seamless | Game over, must rejoin |
| Lines of code added | ~360 | ~40 (and ~80 removed) |
| Protocol changes | 3 new messages, 1 new type | None |
| Server state | Persistent session store + sweeper | None |
| Test surface | ~8 new integration tests + unit tests | 1-2 new tests |
| Failure modes | Token leakage, sweep races, turn-skip bugs, server restart | None new |
| Maintenance burden | Ongoing (sessions are stateful) | Zero |
| Time to ship | 4-5 days | < 1 day |

---

## 6. Recommendation: Option B (for now)

Reasons:

1. **No data justifies A yet.** This is a casual board game played in short
   sessions. We don't have telemetry showing meaningful disconnect rates or
   user complaints about lost games. Building a session store on speculation
   is premature.

2. **The current code is shipping bugs.** Whatever we do, the in-flight
   reconnection logic needs to come out — it produces a worse UX than
   honest failure. Option B is the minimum change to stop the bleeding.

3. **B doesn't preclude A.** The protocol and server stay clean, so adding
   sessions later is purely additive. We'd add `Resume` / `SessionEstablished`
   without breaking existing clients.

4. **The hard part of A is correctness during grace,** not the plumbing.
   Turn-skipping mid-game, state reconciliation on resume, and the sweep
   race are all subtle. Doing this work before we know we need it is
   exactly the "premature abstraction" CLAUDE.md warns against.

### 6.1 What to do now (Option B implementation order)

1. Tear out `tick()`, `ConnectionStatus::Reconnecting`, `backoff()`,
   `reconnect_attempt`, `on_disconnect()` in `ws_client.rs`.
2. Add `ConnectionStatus::Disconnected { reason }`.
3. Replace banner in `app.rs` with the modal + "Return to menu" button.
4. Add the small client-side unit test for the disconnect transition.
5. Manual smoke test against `cargo run --bin server`.

### 6.2 When to revisit (trigger for Option A)

- We add a public deployment with telemetry showing > X% of games end in
  disconnect rather than a winner.
- We get user reports of "lost my game when wifi blipped."
- We add tournament / ranked play where game integrity matters more.

If any of those hit, re-open this proposal and implement A.

---

## 7. Open questions

- For Option A: should grace-period turn behavior **skip** or **pause**?
  Skipping is simpler; pausing is more "fair" but lets one player hold up
  the room. Default: skip with a 60s timeout, mark eliminated on timeout.
- For Option B: do we want any short auto-reconnect on the *initial*
  handshake (i.e. server is briefly slow to come up)? Probably no — keep
  it dumb.
- Should `pending_messages` exist at all in Option B? Once we're
  disconnected, queuing is a lie. Drop it.

---

## 8. References

- Current ws_client (broken): `client-egui/src/ws_client.rs`
- Server disconnect path: `server/src/server.rs:150-160`,
  `server/src/room.rs:120-176`
- Protocol: `common/src/protocol.rs`
- Existing disconnect tests: `server/src/integration_tests.rs:414-486`
- Heartbeat (related, already merged): commit `289d155`
