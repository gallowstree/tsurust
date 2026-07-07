# Deployment

Two artifacts ship independently: the **WebSocket server** (a long-running
process) and the **WASM client** (static files). Nothing is hosted today — this
documents the paths the repo is wired for. The client's server URL is baked in
at build time via `WS_SERVER_URL`, so point it at wherever the server lives
before you build.

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
output `dist/`). The server URL is read from the `WS_SERVER_URL` build-time env
var (falling back to `ws://localhost:8080`):

```bash
cd client-egui
WS_SERVER_URL=wss://your-server.example trunk build --release
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

## Notes

- The Dockerfiles run as non-root and use multi-stage builds (server image
  ~50–100 MB, client ~20–30 MB).
- A second, dormant workflow (`deploy-pages.yml`) targets a `production` branch
  that doesn't exist; `ci.yml` is the live deploy path. Reconcile to one before
  relying on Pages deploys.
- The server keeps all game state in memory, so horizontal scaling would need
  sticky sessions or shared state — out of scope while it's single-instance.
