# Tsurust Alpha — Setup & How It Works

This is the operations guide for the first alpha distribution channel: a client
hosted on **GitHub Pages** that players point at a **server one of them runs**,
reached over a free **tunnel**. No cloud server, no cost, no accounts.

- **Pitch this to testers:** [ALPHA_PLAYTEST.md](ALPHA_PLAYTEST.md)
- **Release scope & checklists:** [ALPHA_RELEASE.md](ALPHA_RELEASE.md)
- **Deep deploy reference:** [../DEPLOYMENT.md](../DEPLOYMENT.md)

The live client: **https://gallowstree.github.io/tsurust/trial/**

---

## The idea in one picture

```
Player's browser (Chrome/Firefox)
  │  loads the WASM client from  https://…github.io/tsurust/trial/   ← GitHub Pages (static, HTTPS)
  │  reads ?server= from the URL → opens  wss://<name>.trycloudflare.com
  ▼
Cloudflare edge            ← terminates TLS, forwards to the tunnel
  ▼
cloudflared (on the HOST's machine)   ← started by host-game.sh
  │  proxies to  ws://127.0.0.1:8080
  ▼
Tsurust server (host's machine)       ← runs the actual game, joins players to lobbies
```

Everyone in a game connects to **one** server — whoever hosts. Any player can be
the host.

## Why a tunnel is required

GitHub Pages serves the client over **HTTPS**, and browsers only allow an HTTPS
page to open a **secure** (`wss://`) WebSocket. A plain `ws://` to a public or
LAN address is blocked as *mixed content* — the sole exception is
`ws://127.0.0.1` (the visitor's own machine), which is why the localhost default
only helps the host test locally.

So the host needs a public `wss://` address. A tunnel
([cloudflared](https://github.com/cloudflare/cloudflared) or `ngrok`) provides
one for free, with a valid TLS certificate, and without any router/firewall
port-forwarding — the tunnel dials **out** from the host to the edge, and return
traffic rides that connection back in.

## Hosting a game (the host)

**Requirements:** the Rust toolchain (`cargo`) and one of `cloudflared`
(recommended) or `ngrok`.

```bash
brew install cloudflared          # one time
./host-game.sh
```

`host-game.sh` will:
1. reuse a server already listening on `:8080`, or start `cargo run --release --bin server`;
2. open a cloudflared tunnel to it;
3. print an invite link like
   `https://gallowstree.github.io/tsurust/trial/?server=wss://<name>.trycloudflare.com`.

Share that link. Keep the terminal open for the whole session — **Ctrl-C ends the
game and closes the tunnel.** Overrides:

```bash
PORT=9000 ./host-game.sh                                  # different server port
PAGES_URL=https://you.github.io/tsurust/ ./host-game.sh   # your own Pages deploy
```

Doing it by hand:

```bash
cargo run --release --bin server                     # ws://127.0.0.1:8080
cloudflared tunnel --url http://localhost:8080       # → https://<name>.trycloudflare.com
# share: https://gallowstree.github.io/tsurust/trial/?server=wss://<name>.trycloudflare.com
```

## Joining a game (players)

1. Open the host's invite link in **Chrome or Firefox** (not Safari — it blocks
   `ws://localhost` and is stricter about mixed content).
2. Create or join a lobby and play. No install.

Arrived without a link, or want to switch servers? Paste the host's `wss://…`
address into the **Server** field on the main menu — it does the same thing as
the `?server=` param.

## How the client picks its server (resolution order)

1. `?server=<url>` query param — the invite link,
2. the in-app **Server** field,
3. a `wsServerUrl` baked into the page at build time (`WS_SERVER_URL`),
4. `ws://127.0.0.1:8080` fallback — the main menu shows a **warning** when this
   is in effect, because on the web it only reaches the visitor's own machine.

Because resolution is at runtime, one static Pages build serves every host — no
rebuild to change servers.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Menu warns "Pointed at your own machine (127.0.0.1)" | Opened the bare Pages URL with no `?server=` | Use the host's full invite link, or paste the `wss://` into the Server field |
| "Couldn't connect to server" | Host's server or tunnel isn't running, or the tunnel URL changed | Host re-runs `host-game.sh` and reshares the new link |
| Works in Chrome, fails in Safari | Safari blocks the socket | Use Chrome or Firefox |
| Link worked yesterday, dead today | Quick-tunnel URLs are **ephemeral** — they change every run | Host reshares the current link each session (or set up a named tunnel — see below) |
| Everyone dropped at once | Host closed the terminal / laptop slept | Host restarts; games are in-memory and don't survive a server restart |
| A player froze mid-game | A dropped socket is **fail-closed** — no reconnection yet (`proposals/004`) | That player rejoins a fresh lobby |

## Making the invite link stable (optional)

The free quick tunnel's URL changes every run. For a fixed address, the free
upgrade is a **cloudflared Named Tunnel** (a free Cloudflare account + a domain
routed through Cloudflare) → a permanent `wss://tsuro.yourdomain.com`. `ngrok`
paid plans offer reserved domains too. Neither is needed for casual playtests.
