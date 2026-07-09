# 🐉 Play the Tsurust Alpha

**Tsurust** is a from-scratch Rust build of the tile-laying board game
**[Tsuro](https://en.wikipedia.org/wiki/Tsuro)** — lay a path, follow it, be the
last dragon standing. It runs **right in your browser**, and we'd love your help
shaking out the first online-multiplayer alpha.

No account. No download. No cost. You just click a link.

---

## Play in 30 seconds (guest)

1. A host shares a link that looks like this:
   `https://gallowstree.github.io/tsurust/trial/?server=wss://something.trycloudflare.com`
2. Open it in **Chrome or Firefox** (⚠️ **not Safari** — it won't connect).
3. Enter a name, **create or join a lobby**, pick a starting spot, and play.

That's it. You're playing across the internet against friends.

## Want to host a game yourself?

Any player can host — the game server runs on your machine and friends connect to
it. You need [Rust](https://rustup.rs) and one free tool:

```bash
brew install cloudflared     # one-time (macOS; see cloudflared docs for other OSes)
git clone https://github.com/gallowstree/tsurust && cd tsurust
./host-game.sh               # prints an invite link — share it, keep the terminal open
```

Details and troubleshooting: **[ALPHA_SETUP.md](ALPHA_SETUP.md)**.

## What to try (and where it might break)

We're most interested in the **online path** — lobbies, joining, and a full game
between real people. Please poke at:

- **Joining:** the invite link, the public-lobby browser, joining by code, and a
  private lobby.
- **A full game:** place tiles, rotate them, watch pawns move along paths, play
  until someone wins.
- **Spectating:** watch a public game you're not in.
- **Edge cases:** two people on the same lobby, someone closing their tab
  mid-game, a slow connection, refreshing the page.
- **Replays:** export a finished game and load it back.

## Known rough edges (you don't need to report these)

- **No reconnection yet.** If your connection blips, you're dropped from the game
  and rejoin fresh. (This is the top thing we're deciding whether to build.)
- **The invite link changes** each time the host restarts — grab the latest one.
- **Games live in memory** — if the host restarts the server, in-progress games
  are gone.
- **Safari** isn't supported; use Chrome or Firefox.
- **Head-on pawn collisions** don't kill both pawns yet (a rules detail under
  discussion).

## How to give feedback

Tell us — a sentence is plenty:

- **What you did** (e.g. "joined by code, then created a 3-player lobby").
- **What you expected vs. what happened.**
- **Browser + OS**, and the **invite link / lobby code** if relevant.
- A **screenshot** if it's visual.
- Anything that felt **confusing, slow, or fun**.

Send to: _<add your channel — GitHub issue, Discord, email>_.

Thank you for playing — every game you break helps. 🎲

---

### Copy-paste invite (for the host to send)

> 🐉 Want to play some **Tsuro** online? It's a board game where you lay tiles to
> build a path and try to survive longest. Runs in your browser — no download.
>
> 1. Open this in **Chrome or Firefox** (not Safari): `<paste your host-game.sh link>`
> 2. Type a name and join my lobby.
>
> Heads-up: it's an early alpha, so expect a few rough edges — and if it says
> it can't connect, ping me, my server might've restarted.
