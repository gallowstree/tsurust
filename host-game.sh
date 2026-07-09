#!/usr/bin/env bash
#
# Host a Tsurust alpha game: run the WebSocket server locally, open a public
# HTTPS tunnel to it, and print an invite link friends open in their browser.
#
# Why a tunnel? The client is served from GitHub Pages over HTTPS, and browsers
# only allow an HTTPS page to open a *secure* (wss://) socket. A tunnel gives
# your local `ws://127.0.0.1:8080` server a public `wss://…` address for free,
# without setting up TLS certificates.
#
# Requires: cargo, and ONE of: cloudflared (recommended) or ngrok.
#   macOS:  brew install cloudflared   # or: brew install ngrok
#
# Usage:  ./host-game.sh
#   PORT=9000 ./host-game.sh                       # different server port
#   PAGES_URL=https://you.github.io/tsurust/ ./host-game.sh   # your own deploy
set -euo pipefail

PORT="${PORT:-8080}"
PAGES_URL="${PAGES_URL:-https://gallowstree.github.io/tsurust/trial/}"

SERVER_PID=""
TUNNEL_PID=""
cleanup() {
  [[ -n "$TUNNEL_PID" ]] && kill "$TUNNEL_PID" 2>/dev/null || true
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

listening() { lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; }

# 1. Server -----------------------------------------------------------------
if listening; then
  echo "▶ A server is already listening on :$PORT — using it."
else
  echo "▶ Starting Tsurust server on 127.0.0.1:$PORT …"
  PORT="$PORT" cargo run --release --bin server >/tmp/tsurust-server.log 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 120); do
    listening && break
    sleep 0.5
  done
  listening || { echo "✗ Server didn't come up. See /tmp/tsurust-server.log" >&2; exit 1; }
fi

# 2. Public HTTPS tunnel → wss:// -------------------------------------------
TUNNEL_LOG="$(mktemp)"
if command -v cloudflared >/dev/null 2>&1; then
  echo "▶ Opening cloudflared tunnel …"
  cloudflared tunnel --url "http://localhost:$PORT" >"$TUNNEL_LOG" 2>&1 &
  TUNNEL_PID=$!
  PATTERN='https://[a-z0-9-]+\.trycloudflare\.com'
elif command -v ngrok >/dev/null 2>&1; then
  echo "▶ Opening ngrok tunnel …"
  ngrok http "$PORT" --log stdout >"$TUNNEL_LOG" 2>&1 &
  TUNNEL_PID=$!
  PATTERN='https://[a-z0-9-]+\.ngrok[a-z0-9.-]*'
else
  echo "✗ Need cloudflared or ngrok. Install one:" >&2
  echo "    brew install cloudflared   # or: brew install ngrok" >&2
  exit 1
fi

PUBLIC=""
for _ in $(seq 1 120); do
  PUBLIC="$(grep -oE "$PATTERN" "$TUNNEL_LOG" | head -1 || true)"
  [[ -n "$PUBLIC" ]] && break
  sleep 0.5
done
if [[ -z "$PUBLIC" ]]; then
  echo "✗ Couldn't read the tunnel URL. Tunnel output:" >&2
  cat "$TUNNEL_LOG" >&2
  exit 1
fi

WSS="${PUBLIC/https:/wss:}"
LINK="${PAGES_URL}?server=${WSS}"

cat <<EOF

✅ Your game is live. Share this link with friends:

   $LINK

They open it in Chrome or Firefox — no install needed — and join your lobby.
Keep this terminal open; press Ctrl-C to end the game and close the tunnel.
EOF

wait "$TUNNEL_PID"
