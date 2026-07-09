# Tsurust — Alpha v0.1.0

**Status:** Alpha (first public playtest)
**Date:** 2026-07-09
**Client:** https://gallowstree.github.io/tsurust/trial/
**Distribution:** GitHub Pages client + host-run server over a free tunnel
(see [ALPHA_SETUP.md](ALPHA_SETUP.md))

The goal of this alpha is a single question: **can real people, on different
machines, find each other and play a full game of Tsuro in the browser?**
Everything here serves that, and nothing more is promised.

---

## What's in it

- **Play in the browser** — WASM client, no install (Chrome/Firefox).
- **Online multiplayer** — create/join lobbies (public or private), a public
  lobby browser, join-by-code, and live **spectating**.
- **Local play** — hot-seat multiplayer and a sample game on one machine.
- **Full game loop** — tile placement, rotation, path movement, elimination,
  win detection, and end-of-game **statistics**.
- **Replays** — export a finished game to JSON and load it back.
- **Runtime server selection** — `?server=` invite links and an in-app Server
  field, so one static build works for any host.
- **Zero-cost hosting path** — `host-game.sh` runs the server and a free
  cloudflared/ngrok tunnel and prints an invite link.

## What's explicitly *not* in this alpha

- **No reconnection.** A dropped socket is fail-closed; the player rejoins fresh
  (`proposals/004`).
- **No persistence.** Server state is in memory; a server restart ends live games.
- **No hosted/always-on server.** A player hosts per session; the invite link is
  ephemeral (changes each run of the tunnel).
- **No accounts, matchmaking, AI opponents, or ranking.**
- **Not Safari-compatible** (mixed-content restrictions).
- **Head-on pawn collisions** don't kill both pawns (rules detail, under review).

## How to run it

- **Play:** open a host's invite link in Chrome/Firefox → join a lobby.
- **Host:** `./host-game.sh` → share the printed link.
- **Full detail & troubleshooting:** [ALPHA_SETUP.md](ALPHA_SETUP.md)
- **Invite testers:** [ALPHA_PLAYTEST.md](ALPHA_PLAYTEST.md)

---

## Release checklist

### A. Engineering readiness

- [x] Runtime `?server=` resolution + in-app Server field (unit-tested)
- [x] Loud fallback warning when pointed at `127.0.0.1` on the web
- [x] `host-game.sh` verified end-to-end: tunnel up, real `wss://` WebSocket
      reaches the local server (server logged the connection)
- [x] `cargo test --workspace` green (incl. headless UI e2e)
- [x] `/trial/` alpha build deployed to Pages and serving live
- [x] CI builds & republishes `/trial/` on every push (durable channel)
- [x] Docs: setup, playtest pitch, this release note
- [ ] Commit the working branch and push to `master` (activates the durable CI
      `/trial/` deploy; until then the live `/trial/` is a manual preview)
- [ ] **Fix the `WS_SERVER_URL` repo variable** — it is currently mangled
      (`llename.com`), which breaks the root deploy at `/tsurust/`. Either set it
      to a real `wss://…` or clear it. The alpha at `/trial/` is unaffected
      (runtime `?server=` overrides it), but the bare root URL stays broken until
      this is done.

### B. Before you announce (go / no-go)

- [ ] Open the live invite link on a **second device / network** and complete a
      real 2-player game
- [ ] Confirm the flow in both **Chrome and Firefox**
- [ ] Confirm the **Safari** failure is graceful (clear "can't connect", not a
      blank screen)
- [ ] Pick and write the **feedback channel** into
      [ALPHA_PLAYTEST.md](ALPHA_PLAYTEST.md) (GitHub issue / Discord / email)
- [ ] Decide the **host** for launch sessions (who runs `host-game.sh`, and when)

### C. Per-session host checklist

- [ ] `git pull` (latest client/server)
- [ ] `./host-game.sh` — wait for the invite link
- [ ] Open the link yourself once to confirm it connects
- [ ] Share the link; note it's **this session only**
- [ ] Keep the terminal open and the machine awake for the whole session
- [ ] Ctrl-C when done (closes the tunnel)

### D. Tester quick checklist (hand to testers)

- [ ] Opened the link in Chrome/Firefox and joined a lobby
- [ ] Played a full game to a win/elimination
- [ ] Tried the public lobby browser and/or join-by-code
- [ ] Spectated a game
- [ ] Tried one "break it" case (refresh mid-game, close a tab, slow network)
- [ ] Sent one line of feedback (what you did, expected vs. actual, browser/OS)

## Post-alpha follow-ups (from what this alpha teaches)

- **Reconnection** (`proposals/004`) — build only if drop reports justify it.
- **Stable invite URL** — a free cloudflared **named tunnel**, or a small
  always-on hosted server, to remove the "link changed" friction.
- **In-game chat** (`proposals/005`) if testers want to coordinate in-app.
- Revisit head-on collision rules based on player reactions.
