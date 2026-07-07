# Architecture

Tsurust is a Rust implementation of the Tsuro board game: a shared game engine,
a WebSocket server for online multiplayer, and an egui client that runs natively
and in the browser (WASM). This document is the map — it points at the code
rather than duplicating it, so it stays true as the code changes.

## Workspace

A three-crate Cargo workspace:

| Crate | What lives there |
|-------|------------------|
| `common/` | The game engine and the wire protocol. No I/O, no UI. Shared by both other crates. |
| `server/` | The WebSocket multiplayer server (`tokio` + `tokio-tungstenite`, `tracing` for logs). Binary: `server`. |
| `client-egui/` | The GUI client (`egui`/`eframe` 0.35, `ewebsock`). Binary: `client-egui_bin`; also builds to WASM via Trunk. |

Run commands and the full feature list are in [README.md](README.md); the current
TODO list is in [DEVELOPMENT_ROADMAP.md](DEVELOPMENT_ROADMAP.md).

## Game engine (`common/`)

- **Board** (`board.rs`) — a 6×6 grid. A tile is 4 segments connecting 8 entry
  points numbered 0–7 counter-clockwise (0–1 bottom, 2–3 right, 4–5 top, 6–7
  left). `Board::traverse_from` walks a pawn along the placed tiles until it
  leaves the last tile, following connections across cells.
- **Deck** (`deck.rs`) — the 35 distinct Tsuro tiles, shuffled; `new_empty()`
  backs redacted views.
- **Game** (`game.rs`) — the authoritative state: board history, players, each
  player's hand, per-player and per-turn `Trail`s (for rendering/animation),
  whose turn it is, and per-player `PlayerStats`. `perform_move` validates a
  placement (turn, identity, pawn cell, occupancy, tile-in-hand), moves every
  pawn the tile touches, records trails and stats, refills hands to 3, and
  advances the turn — returning a `TurnResult` (`TurnAdvanced` / `PlayerWins` /
  `Extinction`). `eliminate_player` handles a forfeit/disconnect with the same
  bookkeeping. `export` serializes a game (with per-perspective hand redaction)
  for replays.
- **Trails** (`trail.rs`) — purely topological path data (`Trail` /
  `TrailSegment`); geometry and drawing live in the client's `rendering.rs`.
- **Lobby** (`lobby.rs`) — the pre-game room: players, spawn positions, the
  `Visibility` (Public/Private) flag, room-id generation/normalization
  (`next_lobby_id`, `normalize_lobby_id`), and `to_game()` which freezes the
  lobby into a `Game` at start.
- **Protocol** (`protocol.rs`) — the `ClientMessage` / `ServerMessage` enums,
  JSON-serialized over the socket, plus `LobbyListing` for the public directory.
  See [Networking](#networking) below.

## Networking

All client↔server traffic is JSON `ClientMessage` / `ServerMessage` values over a
single WebSocket. The message set (see `protocol.rs` for exact shapes):

- **Client → server:** `CreateRoom { visibility }`, `JoinRoom`, `ListLobbies`,
  `SpectateRoom`, `LeaveRoom`, `PlacePawn`, `StartGame`, `PlaceTile`,
  `GetGameState`.
- **Server → client:** `RoomCreated`, `PlayerJoined`, `PlayerLeft`,
  `LobbyStateUpdate`, `LobbyList`, `PawnPlaced`, `GameStarted`,
  `GameStateUpdate`, `TurnCompleted`, `Error`.

`GameStarted` and `GameStateUpdate` are the only messages carrying secret state,
and they are redacted per recipient — see [Hidden information](#hidden-information).

## Server (`server/`)

- **`GameServer`** (`server.rs`) — owns all rooms behind an `RwLock`. Creates
  rooms, routes joins, serves the public lobby directory (`list_public_rooms`,
  public rooms only, joinable-first), and reaps idle rooms with no subscribers.
- **`GameRoom` / `RoomPhase`** (`room.rs`) — a room is either `Lobby(Lobby)` or
  `Playing(Game)`, never both, so a lobby has no game to poke and a live game has
  no lobby to re-join. Each room owns a `tokio::broadcast` channel carrying the
  **full** game state to every subscribed connection.
- **Connection task** (`handler.rs`) — one per socket. Parses client messages,
  enforces identity (`verify_sender` rejects any action whose claimed player id
  or room doesn't match what this connection was assigned), forwards room
  broadcasts to its client, and runs a ping/pong heartbeat. On disconnect it
  eliminates the player (fail-closed) and advances the turn.

### Hidden information

The board and tile *counts* are public; the tiles in a hand and the deck order
are not. Redaction happens once, per connection, on egress:

- `Game::view_for(viewer: Option<PlayerID>)` returns a copy with every hand
  except the viewer's emptied and the deck hidden. `None` (a spectator) sees no
  hands at all.
- The broadcast channel stays full and server-internal. Each connection task
  calls `ServerMessage::redacted_for(current_player_id)` on every outgoing
  `GameStateUpdate` / `GameStarted` (and the direct `SpectateRoom` /
  `GetGameState` responses), so tiles a client isn't entitled to see never reach
  the wire.
- Those messages also carry `hand_counts` and `deck_count`, computed from the
  full state *before* redaction, so the UI still shows opponents' tile totals.

The invariant clients agree on is board + turn + positions + **counts**; each
client's raw `hands` map holds only its own tiles. The integration tests assert
exactly this.

### Spectating & visibility

A room is Public (listed in the lobby directory, joinable/spectatable by anyone
on the server) or Private (reachable only by its room code). `SpectateRoom`
subscribes a connection with no player identity — the identity check
structurally rejects any move it might send, and it receives the fully-redacted
(no-hands) view. Private rooms and the code path keep working unchanged.

## Client (`client-egui/`)

- **`TemplateApp`** (`app.rs`) — the whole app. `eframe::App::ui` runs each
  frame; a `mpsc` channel carries UI events (button clicks, tile placements) into
  a single handler. State is an `AppState` enum: `MainMenu`, `CreateLobbyForm`,
  `JoinLobbyForm` (the public-lobby browser), `Lobby` / `LobbyPlacingFor`,
  `Game` (local, client-authoritative), `OnlineGame` (server-authoritative,
  holds `hand_counts` and an `is_spectator` flag), and `ReplayViewer`.
- **`GameClient` / `ConnectionStatus`** (`ws_client.rs`) — the WebSocket wrapper
  (native + WASM via `ewebsock`), polled each frame. Reconnection is
  **fail-closed** (`Disconnected { reason }` → toast / main menu); session-resume
  is deliberately deferred (see `proposals/004`).
- **Rendering** — everything is hand-drawn with egui primitives, no sprites:
  `board_renderer.rs`, `hand_renderer.rs`, `tile_button.rs` (rotate on
  left/right click), `player_card.rs`, `rendering.rs` (trail geometry),
  `stats_display.rs`. Screens live in `src/screens/`.
- **Replay** (`replay_state.rs`, `file_io.rs`) — export a game to JSON and step
  through it in the `ReplayViewer`.

Custom widgets publish accessibility labels (`WidgetInfo::labeled`) — spawn
spots, hand tiles, lobby-browser rows — which double as the handles the UI tests
drive.

## Testing

- **Unit** — game rules, protocol round-trips, redaction, and lobby logic in
  `common/` (`cargo test -p tsurust_common`).
- **Integration** — `server/src/integration_tests.rs` spins up a real server on
  an OS-assigned port and drives real WebSocket clients through full games
  (lobby → play → disconnect → spectate), asserting cross-client agreement and
  the redaction invariant.
- **End-to-end UI** — `client-egui/tests/ui_e2e.rs` drives the real app with
  `egui_kittest` against an in-process server (create/join/play, spectate,
  error handling, idle-repaint). `tests/visual_dump.rs` (ignored by default)
  renders each screen to PNG for manual inspection.

Run everything with `cargo test --workspace`. CI (`.github/workflows/ci.yml`)
adds fmt/clippy, `cargo audit`, a cross-OS test matrix, a WASM build, and Docker
image publishing.
