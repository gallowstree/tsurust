# Deployment

Two artifacts ship independently: the **WebSocket server** (a long-running
process) and the **WASM client** (static files).

The client picks its server URL **at runtime**, so one static build serves any
number of servers. Resolution order:

1. a `?server=<url>` query param — the shareable-invite path,
2. the in-app **Server** field on the main menu (paste/edit at runtime),
3. a `wsServerUrl` baked into `index.html` at build time (`WS_SERVER_URL`),
4. `ws://127.0.0.1:8080` as a last resort (the UI warns when this is in effect).

This means you don't have to rebuild to change servers — see
[Alpha: friends host the server](#alpha-friends-host-the-server).

## Server (Docker)

`server/Dockerfile` builds a small static image. Config is environment-only:

- `HOST` — bind address (default `0.0.0.0` in the container).
- `PORT` — listen port (default `8080`).

```bash
docker build -f server/Dockerfile -t tsurust-server .
docker run -d --name tsurust-server -p 8080:8080 tsurust-server
```

`docker-compose.yml` brings up the server and an nginx-served client together
(server on 8080, client on 8081 → 80) for a full local stack:

```bash
docker-compose up --build
```

**Behind TLS.** Browsers on an `https://` page can only open `wss://` sockets, so
production needs a reverse proxy terminating TLS and forwarding the WebSocket
upgrade. The critical headers:

```nginx
location / {
    proxy_pass http://tsurust-server:8080;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
}
```

## Client (WASM)

The client builds to WASM with [Trunk](https://trunkrs.dev) (`client-egui/Trunk.toml`,
output `dist/`). A `WS_SERVER_URL` build-time env var bakes a **default** server
into `index.html`, but it's only the fallback — `?server=` and the in-app field
override it at runtime (see the resolution order above), so baking one is
optional:

```bash
cd client-egui
WS_SERVER_URL=wss://your-server.example trunk build --release   # optional default
# → client-egui/dist/  (static files: index.html, .js, .wasm)
```

Serve `dist/` from any static host. Note the `.wasm` files must be sent with the
`application/wasm` MIME type. The nginx image (`client-egui/Dockerfile` +
`nginx.conf`) handles this; so does GitHub Pages.

### GitHub Pages (the wired-up path)

`.github/workflows/ci.yml` has a `deploy-pages` job that, on push to `master`,
builds the client with Trunk and publishes `dist/` to the `gh-pages` branch. It
reads the server URL from the repository variable `WS_SERVER_URL` — set it under
**Settings → Secrets and variables → Actions → Variables**, then enable Pages
from the `gh-pages` branch.

> ⚠️ **`WS_SERVER_URL` must be a full URL *with* scheme** — `wss://host:port`,
> not `host:port`. The CI `sed`-substitutes it verbatim into `index.html`, so a
> value missing the `wss://` (or a truncated one) produces a dead `wsServerUrl`
> that no browser can dial. If you only ever host via invite links, you can
> leave this variable **unset** — the runtime `?server=` param takes over and
> the baked value is irrelevant.

## Alpha: friends host the server

The first alpha distribution channel needs **no hosted server**: the client
lives on GitHub Pages, and whoever wants to play a game runs the server on their
own machine and shares an invite link. Any friend can be the host.

The one catch is TLS. Pages is served over HTTPS, and a browser will only open a
**`wss://`** (secure) socket from an HTTPS page — a plain `ws://` to a public or
LAN address is blocked as mixed content. The only exception is `ws://127.0.0.1`
(your own machine), which is why the localhost default is a dead-end for
everyone but the host. So the host needs a public `wss://` address; the easiest
way to get one for free — without TLS certs — is a tunnel.

**Host workflow** (one command):

```bash
./host-game.sh
```

It runs the server, opens a [cloudflared](https://github.com/cloudflare/cloudflared)
(or ngrok) tunnel to it, and prints a link like:

```
https://gallowstree.github.io/tsurust/trial/?server=wss://abc123.trycloudflare.com
```

Friends open that link in **Chrome or Firefox** (not Safari — it blocks
`ws://localhost` and is stricter about mixed content) and join the lobby. No
install on their end. Ctrl-C ends the game and closes the tunnel.

Doing it by hand instead of the script:

```bash
cargo run --release --bin server                    # local ws://127.0.0.1:8080
cloudflared tunnel --url http://localhost:8080      # → https://<name>.trycloudflare.com
# share: <pages-url>?server=wss://<name>.trycloudflare.com
```

Friends can also skip the link and paste the `wss://…` address straight into the
**Server** field on the main menu.

## Notes

- The Dockerfiles run as non-root and use multi-stage builds (server image
  ~50–100 MB, client ~20–30 MB).
- The server keeps all game state in memory, so horizontal scaling would need
  sticky sessions or shared state — out of scope while it's single-instance.
