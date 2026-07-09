# Proposal 005: Chat Apps as a Game Transport (Telegram & WhatsApp)

**Status:** Decided (v3) — Telegram first; **Phase 0 (engine rule + turn
timer) is implemented**; Mini App promoted to Phase 1; chat-native play is
DM-only in its first cut; forced moves pick uniformly at random.
**Author:** Claude
**Date:** 2026-07-09
**Estimated Effort:** Phase 1 (Mini App): ~2-3 days · Phase 2 (chat-native):
~4-5 days · Phase 3 (animations): ~3-4 days · Phase 4 (tunnel): ~5-7 days

---

## Summary

Add chat apps as a transport for online games: a **gateway** service bridges a
chat platform (Telegram, WhatsApp) to the existing WebSocket server, so people
can play Tsuro from a chat thread. Each turn's result is rendered server-side
as a short animation and posted as the board update. For players using the
real client, the gateway also supports a **tunnel mode**: the normal
`ClientMessage`/`ServerMessage` protocol rides inside chat messages, encrypted
to per-player public keys, so payloads are opaque to the platform and to
anyone reading the chat.

Two central decisions:

1. **The gateway is a client of the existing server, not a new server mode.**
   It presents itself upstream as one WebSocket connection per chat player.
   `common/`, `server/`, and the game rules do not change; the per-connection
   redaction boundary (`ServerMessage::redacted_for`,
   `common/src/protocol.rs`) keeps doing its job unmodified.
2. **Platform choice is a trait, and Telegram goes first.** Telegram's Bot API
   is free, official, group-capable, and runnable from a laptop with no public
   endpoint; every WhatsApp option is worse on at least two axes (§4). The
   gateway is written against a `ChatTransport` trait so WhatsApp (or anything
   else) is an additional impl, not a rewrite.

A prerequisite that stands on its own (§8, **now implemented**): a
server-side **turn timer** with a rules-correct forced move — which first
required the engine to enforce the real Tsuro rule that a player may not play
a self-eliminating tile while a survivable option exists (`perform_move` did
not check this before).

---

## 1. Motivation

- Turn-based Tsuro is a natural correspondence game. Chat latency (seconds
  per hop) is unacceptable for realtime play but ideal for play-by-post with
  friends — the group chat *is* the lobby.
- A move in Tsuro is tiny: the placement cell is forced (the cell your pawn
  faces), so a complete move is just *which of ≤3 hand tiles* × *which of 4
  rotations* — e.g. `B2`, or two button taps. Chat is a genuinely good input
  device here, not a compromised one.
- Board updates as turn animations make the group thread a readable game
  record. The data already exists: `Game.current_turn_trails`
  (`common/src/game.rs:145`) was added precisely for per-turn animation.

## 2. Non-goals

- Realtime spectating over chat (latency makes it meaningless; the group
  thread already serves this role).
- Anti-cheat via cryptography. The server's `perform_move` validation remains
  the sole authority. Encryption (§7) protects payloads from *third parties*,
  not from the keyholder — a player can always read their own hand, which is
  fine, because it is their hand.

---

## 3. Architecture

```
                          ┌───────────────────────────── gateway-chat ──┐
 chat network             │                                             │
┌──────────────┐ Bot API/ ┌──────────────┐  1 WS conn per player   ┌───┴────┐
│ player phones │◄───────►│ ChatTransport │◄─┐                     │        │
│ + group chat  │ webhooks │ (trait)      │  │  ┌───────────┐  WS  │ server │
└──────────────┘          └──────────────┘  ├─►│ session map│◄────►│ (as-is)│
                                            │  └───────────┘      │        │
                          ┌──────────────┐  │  ┌───────────┐  WS  │        │
                          │ renderer     │◄─┘  │ "camera"  │◄────►│        │
                          │ (turn→mp4)   │     │ spectator │      └────────┘
                          └──────────────┘     └───────────┘
```

### 3.1 The gateway is N virtual clients

For every chat participant the gateway opens a dedicated upstream WebSocket
connection and speaks the ordinary protocol (`CreateRoom`, `JoinRoom`,
`PlaceTile`, …). Consequences:

- **Zero server changes.** Rooms, lobbies, spectators, redaction — all reused.
- **Hidden information stays correct by construction.** Each virtual client
  receives a `Game` already redacted for that player, so a player's hand
  render can only ever contain their own tiles.
- **Mixed tables work for free.** A room can hold two egui clients and two
  chat players; the server cannot tell the difference.

### 3.2 The camera is a spectator

Per room, the gateway opens one extra upstream connection via `SpectateRoom` —
the *camera*. All public media (the turn animation posted to the group) is
rendered **only from the camera's state**. A spectator view contains no hands
and no deck order (`Game::view_for(None)`), so a renderer bug cannot leak
hidden information into shared media. Private media (a player's hand image) is
rendered from that player's own virtual-client state, which contains only
their hand.

### 3.3 Crate layout

```
gateway-chat/                new workspace member (binary)
  src/main.rs                config, update loop, upstream WS pool
  src/transport/mod.rs       ChatTransport trait
  src/transport/telegram.rs  Bot API impl (long polling; teloxide or raw reqwest)
  src/transport/wa_cloud.rs  (later) Meta Cloud API impl
  src/transport/wa_linked.rs (later) linked-device sidecar impl
  src/session.rs             chat user id ↔ {upstream conn, player_id, room_id}
  src/commands.rs            Mode A input: commands, "B2", button callbacks
  src/tunnel.rs              Mode B envelope routing (§7)
  src/render.rs              turn state → frames → mp4/png (§6)
common/src/envelope.rs       encrypted envelope + keys, behind a feature flag
client-egui/src/chat_tunnel.rs  Mode B client backend, same interface as ws_client
```

`gateway-chat` depends on `client-egui` (a lib crate already, as the kittest
tests show) to reuse `BoardRenderer` and `rendering.rs` geometry. It drags in
egui + wgpu; acceptable inside the workspace.

---

## 4. Platform choice

"How does the bot reach the chat network at all?" Each platform offers
different doors, and they differ enormously in friction. The trait absorbs
the differences; this section is the decision input.

| | **Telegram Bot API** | **WhatsApp Cloud API** (official) | **WhatsApp linked device** (unofficial) |
|---|---|---|---|
| What it is | Official free bot platform; register with @BotFather in minutes | Meta-hosted business messaging API: HTTPS + webhooks | Software impersonating a companion phone (whatsmeow/Baileys) on a normal number |
| Legitimacy | First-class, sanctioned | Sanctioned; business verification required | **Violates WhatsApp ToS — ban risk** |
| Cost | Free | Per-conversation pricing | Free |
| Server needs | **None** — long polling works from a laptop behind NAT | Public HTTPS endpoint for webhooks | Public endpoint or persistent sidecar |
| Groups | Full support | Limited/recent | Full support |
| Rich input | Inline keyboards (callback buttons) — ideal for tile/rotation picks | 3-button / 10-row list messages | Text + limited buttons |
| Media | `sendAnimation` (looping MP4), photos, documents ≤50 MB, **in-place message editing** | image/video/document, no editing | Full |
| Messaging users first | One-time: user must `/start` the bot once, then unrestricted | Rolling 24 h window; outside it only pre-approved templates | Unrestricted |
| Platform reads content? | **Yes** — cloud chats are not E2E (secret chats exclude bots) | **Yes** — E2E terminates at Meta's endpoint | No — true E2E to the sidecar |
| Custom clients (for tunnel mode) | **Officially allowed** via MTProto; pure-Rust `grammers` crate | n/a | ToS-violating |

**Recommendation: Telegram first.** It wins on cost, setup friction,
deployability (no public endpoint!), groups, input UX, and message editing;
its one weakness — the platform can read cloud-chat content — is precisely
what the tunnel's envelope encryption addresses (§7), and its officially
sanctioned custom-client story (MTProto/`grammers`) makes tunnel mode
ToS-clean, which WhatsApp cannot offer at all. The WhatsApp impls remain
future `ChatTransport` implementations for reaching people who won't install
another app.

Two Telegram-specific opportunities worth noting:

- **In-place board message.** `editMessageMedia` lets the group keep one
  pinned "current board" message that the gateway updates every turn, with
  per-turn animations posted beneath — a much cleaner thread than an
  ever-growing image pile.
- **Mini App shortcut.** Telegram bots can launch full web apps in-chat. The
  WASM build of `client-egui` already exists and already speaks WebSocket —
  wrapping its URL as a Telegram Mini App gives chat users the *real* client
  (bot handles invites and turn nudges; the game itself runs in the actual
  UI, over its normal WS connection). This coexists with chat-native play and
  may be the highest value-per-effort item in the whole proposal.

---

## 5. Two operating modes

**Mode A — chat-native.** Humans play entirely from the chat app: buttons or
text in, rendered images out. No keys, no client install. This is the default
and the bulk of the value.

**Mode B — encrypted tunnel.** The egui client runs normally but its transport
is the chat network instead of a direct WebSocket: the same JSON protocol,
framed in encrypted envelopes, carried as chat messages between the player's
account and the bot. §7.

An honest boundary between them: **content a human must read cannot be
encrypted.** In Mode A the privacy story is DM separation (which mirrors
`redacted_for`: hand → DM, board → group) plus the platform's transport
encryption. The public-key layer applies only to Mode B's machine-to-machine
payloads.

### Mode A message flows

**Lobby**

```
Alice → bot   /new Friday Tsuro
bot  → Alice  Room ABCD created. Invite: t.me/<bot>?start=join_ABCD
Bob  → bot    (taps invite link)
bot  → group  Bob joined (2/8).
```

Gateway actions: open a virtual client per sender, `CreateRoom`/`JoinRoom`,
relay `LobbyStateUpdate`s as short texts.

**Pawn placement.** The bot sends a PNG of the empty board with border entry
slots numbered; the player picks via inline keyboard (or replies `spawn 14` —
internally the `r0c2e4`-style position the UI tests already use).

**A turn**

1. Server sends the current player's virtual client a redacted
   `GameStateUpdate`. Gateway renders their **hand PNG** — tiles labeled
   A/B/C, each shown in its 4 rotations — and DMs it with an inline keyboard:
   tile pick (≤3 buttons), then rotation pick (4 buttons). Text replies like
   `B2` also parse.
2. The pick resolves tile B from the redacted hand, applies the rotations
   (`Tile` rotation already exists), builds a `Move` for the forced cell, and
   sends `PlaceTile`. A `ServerMessage::Error` is relayed back as text.
3. On `TurnCompleted` + the camera's `GameStateUpdate`: renderer produces the
   **turn animation** (§6). Gateway posts it to the group and updates the
   pinned board message, with a caption: *"Turn 12 — Bob placed a tile. Carol
   eliminated. Alice to move."* (`TurnResult` carries `turn_number`,
   `next_player`, `eliminated`.)
4. On `PlayerWins`/`Extinction`: final animation, a **stats card PNG** (the
   stats system already computes everything), and the game **export JSON as a
   document attachment** — anyone in the chat can load it in the desktop
   client's replay viewer. The thread becomes a self-contained game archive.

**Input hygiene**

- **Dedup:** every inbound message/callback id goes into a processed-set
  before acting; replies are idempotent per id.
- **Ordering:** not guaranteed by the platform, but Mode A is human-paced and
  the server already rejects out-of-turn or invalid moves — the gateway just
  relays the error.
- **Nudges:** plain "your turn" messages, escalating per the turn timer (§8).

---

## 6. Rendering turn animations

**Data.** Everything needed is already on the wire: `board.history` (tile
placements), `current_turn_trails` (this turn's per-player movement, added for
animation), player positions, and `TurnResult` (eliminations/win). The
camera's redacted state is the sole input for public renders (§3.2).

**Pipeline.**

1. **Frames.** A minimal headless egui harness — the `egui_kittest` + wgpu
   setup proven in `client-egui/tests/visual_dump.rs` — draws *only* the board
   via `BoardRenderer`, stepped through the turn timeline at fixed timesteps:
   tile drop-in, pawns traversing the new trail segments, an elimination
   flash if any. ~36 frames at 12 fps ≈ 3 s, 480×480. On a headless server,
   wgpu runs on a software adapter (lavapipe/llvmpipe), the same trick CI
   uses for these tests.
2. **Encode.** Frames piped as rawvideo to an `ffmpeg` subprocess → H.264
   MP4. Telegram's `sendAnimation` takes MP4 directly; WhatsApp's "GIF" *is*
   an MP4 with a gif-like flag (actual `image/gif` isn't an accepted Cloud
   API media type) — so MP4 is the native target on both, not a compromise.
   Fallbacks, in order: animated WebP/WebM sticker if ffmpeg is absent;
   static PNG of the final position (pure Rust via the already-present
   `image` crate) as the guaranteed floor.
3. **Publish.** Upload once, reuse the returned file/media id for every
   recipient and for the pinned-message edit.

Static renders reuse the same harness: hand images, spawn-picker board, the
end-of-game stats card.

---

## 7. Mode B: the encrypted tunnel

### What it is

`client-egui` gains a second transport backend (`chat_tunnel.rs`) implementing
the same interface as `ws_client.rs`. Outbound `ClientMessage`s are
serialized, encrypted, and sent as chat messages from the *player's own
account* to the bot; inbound `ServerMessage`s arrive the same way in reverse.
On Telegram the client logs in as the player via MTProto (`grammers`, pure
Rust, officially permitted); on WhatsApp this requires a ToS-violating
linked-device session — another reason Telegram leads. The chat thread
visibly fills with ciphertext blobs — which is the point: the protocol is not
inspectable or scriptable from the chat.

### Keys and pairing

- Client generates an **Ed25519** signing keypair and an **X25519** KEM
  keypair on first run; the gateway has its own static pair, published with
  the bot's contact info.
- Pairing is TOFU with a manual check: the client shows a short fingerprint;
  the player sends `/pair <fingerprint>` to the bot from their account; the
  gateway pins `chat user id ↔ pubkeys`. Key changes require re-pairing (and
  are surfaced loudly).

### Envelope

```rust
struct Envelope {
    v: u8,              // format version
    room_id: RoomId,
    seq: u64,           // per (sender, room), strictly increasing
    ct: Vec<u8>,        // HPKE-sealed protocol JSON
    sig: [u8; 64],      // Ed25519 over (v, room_id, seq, ct)
}
```

- **Encryption:** HPKE (X25519-HKDF-SHA256 + ChaCha20-Poly1305; `hpke` crate)
  sealed to the recipient's KEM key, with `(room_id, seq, direction)` as AAD.
  `crypto_box` is an acceptable simpler substitute.
- **Replay/reorder:** receiver keeps a per-sender seq high-water mark with a
  small out-of-order window; violations are dropped, closing replay of old
  moves.
- **Encoding:** postcard/CBOR → base64 in the message body for small control
  messages; anything larger ships as a binary document attachment. Telegram's
  4096-char text limit makes attachments the common path for state updates
  (WhatsApp's ~65 K limit is roomier, same split logic applies).
- Lives in `common/src/envelope.rs` behind a `tunnel-crypto` feature so client
  (incl. WASM) and gateway share one implementation.

### What it actually buys — threat model

| Adversary | Without envelope | With envelope |
|---|---|---|
| The platform (Telegram cloud chats; Meta's Cloud API endpoint) | Reads all payloads | Sees ciphertext only — restores true end-to-end |
| Someone reading the phone / chat backups / synced devices | Full game history incl. hand deliveries | Ciphertext |
| Chat scraper / casual protocol botting | Trivial | Must extract keys from a client install — discouraged, not prevented |
| Message injection from a compromised account | Could submit moves as the player | Fails signature check — moves require the client's signing key, not just the account |
| The player themselves | n/a | **Unaffected.** They hold their key; server-side validation remains the anti-cheat. |

### What it does not buy

Traffic analysis (message timing/size reveals whose turn it is — so does the
group chat), platform metadata, or protection in Mode A, where content is
human-readable by design.

---

## 8. Turn timer and forced moves — ✅ implemented

Transport-independent server feature; correspondence games stall without it,
and the egui client benefits equally (a room-configurable clock for live
games). It has an engine prerequisite that is a rules-correctness fix in its
own right. Everything in this section is now in the tree (engine rule in
`common/src/game.rs`, timer in `server/src/room.rs` + `server.rs`, client
countdown/form/warnings in `client-egui`); details below are kept as the
design-of-record.

### 8.1 Engine prerequisite: the forced-suicide rule

Tsuro's rules: **a player may not play a tile that eliminates their own pawn
unless every playable option does.** `perform_move`
(`common/src/game.rs:243`) does not enforce this today — it validates turn,
identity, cell, occupancy, and tile-in-hand, then accepts any placement.

Additions to `common/`:

- `Game::simulate_move(&self, mov) -> Result<Vec<PlayerID>, MoveError>` —
  clone the game, run the real `perform_move` on the clone, return the
  eliminated list. Cloning reuses *all* engine logic (traversal, edge
  elimination in `update_players_and_trails`) instead of duplicating it, so
  it can never disagree with the engine. Cost: ≤12 clones of a small struct.
- `Game::survivable_moves(&self, player_id) -> Vec<Move>` — enumerate hand
  tiles × 4 rotations at the forced cell (deduped via the existing
  rotation-invariant `is_same_tile`), keep those whose simulation does not
  eliminate the mover.
- New validation in `perform_move`: if the proposed move self-eliminates and
  `survivable_moves` is non-empty → `MoveError::ForcedSuicide` ("that move
  eliminates you and you have a surviving option"). The egui client should
  also grey these tiles/rotations out; chat players get the error text
  relayed.

### 8.2 The timer

- **Configuration:** `turn_timer: Option<Duration>` on the lobby (a
  `CreateRoom` option surfaced in the lobby form and in `/new`). Live games
  might use 60 s; correspondence games 24–48 h; `None` disables.
- **Mechanics:** server-side, room-scoped. A tokio task per room arms on
  every turn advance and cancels/rearms on `TurnCompleted`. Server-side
  because the server is authoritative and the timer must work when the
  player's client is gone.
- **On expiry — forced move, rules-correct:** pick uniformly at random from
  `survivable_moves(current_player)`; if it is empty, from all playable moves
  (the rules require you to play even when every option kills you —
  elimination then follows naturally). Applied through the ordinary
  `perform_move`, so stats, trails, animation data, and broadcasts all
  behave as if the player had moved.
- **Forfeit alternative:** per-room option `on_timeout: ForceMove | Forfeit`
  (`eliminate_player` already implements forfeit bookkeeping). Default
  `ForceMove` — it keeps games moving without punishing a flaky connection
  with death.
- **Pawn-placement phase:** same deadline; on expiry pick a random free
  border position (or forfeit, per the same option).

### 8.3 Protocol additions (backward compatible)

- `GameStateUpdate`/`GameStarted` gain `turn_deadline_secs: Option<u64>`
  (serde-default `None`, like the `visibility` precedent) — remaining time at
  send, so clients tick locally and clock skew doesn't matter. The egui
  client renders a countdown; the gateway schedules nudges from it
  ("⏰ 6 h left, Alice").
- `TurnCompleted` gains `auto_played: bool` (serde-default `false`) so
  clients and chat captions can say *"Bob's turn was auto-played (timer)"*.

---

## 9. Persistence and failure modes

The gateway keeps a small store (sqlite or sled): chat user id ↔ player
identity, room membership, pinned pubkeys, seq windows, processed message
ids, uploaded media ids. All of it is rebuildable except pinned keys and
identity mappings.

**Gateway restart is currently fatal to its games**: the server has no
session resume (Proposal 004 shipped Option B, fail-closed), so the gateway's
virtual clients cannot rejoin as themselves. Mitigation order:

1. Accept it for Phase 1 (same failure mode the egui client has today).
2. If it hurts, the gateway is the first real consumer of 004's **Option A**
   (session tokens + grace period) — one implementation would serve both the
   egui client and the gateway, strengthening the case for doing it properly.

---

## 10. Phasing

**Phase 0 — engine + timer. ✅ Done.** `simulate_move`, `survivable_moves`,
the `ForcedSuicide` validation, lobby timer setting, server timer task,
protocol fields, egui countdown + fatal-rotation warnings. Ships value to
existing clients on its own.

**Phase 1 — Mini App (highest priority).** The shortest path to friends
playing from their phones: host the existing WASM client over HTTPS, register
it as a Telegram Mini App, and stand up a *slim* bot — /new and invite
deep-links (`t.me/<bot>?startapp=<room>` passing the room code into the app),
plus a camera spectator connection per room for "your turn" nudges. No
rendering pipeline, no command parser; the game runs in the real client over
its normal WebSocket. *~2-3 days.*

**Phase 2 — chat-native, DM-only.** Gateway grows the `ChatTransport`
Telegram impl proper (long polling), session map, command parser, virtual
clients, static PNG board per turn, hand PNG + inline keyboards — all via
DMs (decided: no group thread in the first cut; DM-broadcast matches the
redaction model). Full lobby→game→stats flow. *~4-5 days.*

**Phase 3 — alive.** Turn animations (frames→MP4 via `sendAnimation`), group
thread + pinned in-place board message, timer-linked nudges, stats card +
replay-export attachment. *~3-4 days.*

**Phase 4 — tunnel.** `common/src/envelope.rs`, pairing flow,
`chat_tunnel.rs` client backend via MTProto/`grammers`, gateway envelope
routing. *~5-7 days.*

**Later, if demanded:** WhatsApp `ChatTransport` impls (Cloud API and/or
linked-device sidecar), accepting the constraints in §4.

---

## 11. Decisions and open questions

Decided:

1. **Telegram first**; WhatsApp remains a future `ChatTransport` impl (§4).
2. **Mini App is the top priority** after Phase 0 — it reuses the finished
   WASM client and is the fastest route to phones.
3. **DM-only** for the first chat-native cut; groups + pinned board arrive
   with the animation phase.
4. **Forced moves are uniformly random** among survivable placements (any
   placement when none survive) — simple and unexploitable.

Still open:

1. **One bot, many games?** Yes by design (room_id routes everything), but
   per-group rate limits (~20 msg/min) may cap very chatty games; measure in
   Phase 2.
2. **Mini App hosting.** The WASM build needs an HTTPS origin and a reachable
   WS endpoint (Telegram requires HTTPS for Mini Apps); decide where both
   live before Phase 1.
