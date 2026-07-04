# HANDOFF — resume & next steps

**How to use:** open a fresh Claude Code session **in this repo** (so it loads this
project's `CLAUDE.md` + `.claude` memory), set the model to **Opus 4.8 at high/xhigh
effort**, and use the prompt below as the kickoff. This is a *resume*, not a cold
review — the first job is to make sense of the in-flight work before starting anything new.

## Recon snapshot (gathered 2026-06-17, before any work)

- **Workspace** (Rust, Cargo): `common` (board/deck/game/lobby/protocol/trail + tests),
  `server` (handler/room/server + integration/room/server tests), `client-egui`
  (egui; native + wasm builds; `screens/` + `components/` modules).
- **On branch `test-ci`** (tracks `origin/test-ci`, up to date — a remote exists).
- **~30 uncommitted changes — handle before new work:**
  - A `client-egui` UI refactor in progress (modified `app.rs`, renderers, and new
    `screens/{main_menu,lobby,lobby_forms}.rs` + `components/{lobby_board}.rs`) — likely
    the roadmap's "online/offline modes" UI task, but confirm.
  - **Deleted** `.github/workflows/{deploy-pages,rust}.yml` and built `client-egui/dist/`
    wasm+js artifacts — intentional cleanup or accidental? Decide.
  - **Untracked:** `proposals/004-websocket-reconnection.md` (a real design doc — keep/commit)
    and a stray `.DS_Store` (gitignore it).
- **Rich docs already exist:** `README.md`, `DEVELOPMENT_ROADMAP.md`, `CLIENT_SERVER.md`,
  `LOBBY.md`, `TRAILS.md`, `DEPLOYMENT.md`, `proposals/001-004`.
- **Recent history:** integration tests, WS heartbeat, disconnect handling, CI/CD, GitHub
  Pages deploy — so multiplayer transport is fairly built out.
- **Known bug to confirm:** the vault flagged a multiplayer bug (missing trail colors /
  failing to play tiles on a turn, "both players see the same tiles") as *fixed* — verify
  it actually holds.

## Kickoff prompt

> Resuming work on tsurust (Rust Tsuro: `common` / `server` / `client-egui` egui workspace).
> I'm on branch `test-ci` with a chunk of uncommitted work in flight. Use high reasoning
> effort. **Do not start new features until the in-flight work is triaged.**
>
> **Ground rules**
> - Read this repo's `CLAUDE.md`, `README.md`, and `DEVELOPMENT_ROADMAP.md` first, then the
>   relevant design docs (`CLIENT_SERVER.md`, `LOBBY.md`, `TRAILS.md`, `proposals/`).
> - A remote exists — don't push without asking. Commit in logical chunks. Don't discard
>   uncommitted work without showing me first.
>
> **Phase 0 — Triage the uncommitted WIP (do this before anything else)**
> - Summarize the working-tree diff. What is the `client-egui` refactor (`screens/`,
>   `components/`, `app.rs`) trying to do, and is it complete? Does the workspace build
>   (`cargo build --workspace`)?
> - Were the deleted `.github/workflows/*.yml` and `client-egui/dist/*` removals intentional?
>   Recommend: finish & commit, stash, or revert — and why. gitignore `.DS_Store`. Decide on
>   the untracked `proposals/004-websocket-reconnection.md`.
> - Land a clean, known baseline (committed or stashed) before Phase 2.
>
> **Phase 1 — Orient & baseline**
> - `cargo build --workspace` and `cargo test --workspace` (common: protocol/lobby tests;
>   server: integration/room/server tests). Report failures.
> - Run server + client locally (native; and the wasm path if relevant). Play a 2-player
>   game and **verify the previously-fixed multiplayer bug** (trail colors render; tiles
>   play correctly on a turn; players see distinct hands).
>
> **Phase 2 — Pick the milestone (surface options, then I choose)**
> Grounded in `DEVELOPMENT_ROADMAP.md`:
> - **Finish the online/offline UI refactor** already in flight (Phase 2, task 1) and commit it.
> - **Phase 2 [CRITICAL] tests:** integration tests for online multiplayer (tile placement,
>   state sync) + protocol-message serialization tests — these directly guard the multiplayer
>   bugs I keep hitting. (Strong default.)
> - **WebSocket reconnection** with exponential backoff (per `proposals/004`).
> - **AI opponents / single-player** (Phase 3 advanced) — lets me play without a second human.
> - **Release/deploy** polish (GitHub Pages + `DEPLOYMENT.md`).
> Recommend one, but ask me before committing to a direction.
>
> **Phase 3 — Plan & execute the chosen milestone**
> - Break it into tasks, implement, keep `cargo test --workspace` green, commit in chunks,
>   and update `DEVELOPMENT_ROADMAP.md` as you close items.
```

I left the **milestone decision to you** (your open question from the project tracker) — the prompt has the session lay out roadmap-grounded options and recommends the two **[CRITICAL]** test suites as the default, since they directly guard the multiplayer bugs you've been hitting.
