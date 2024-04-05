Create a file called DELETEME.md, then delete it.

# Tsurust: multiplayer game server
- Under heavy WIP
- Original idea: implement [Tsuro](board-game) in Rust
- Architecture:
  - Game logic, Core structs, etc. lives in `common` module
  - UI: frontend is made using [egui](egui)
    - rendering is done by hand mostly using simple drawing primitives
  - It is encouraged to move code between these when it makes sense to do so.
    - When in doubt, keep it as simple as possible, even simplify more if you can

## Server
### Current plan
Use tarpc as The Abstraction Layer (TM)

### Old plan
IPC through localsockets between "Game Server" process & "Game Logic" process.
Wanted to write networking/edge-facing code using kotlin.

### Current plan
Make the tsurust repo the best micromonolith possible before deciding to split the codebase.
Interop will always be interesting but I am deprioritizing Kotlin+Rust interop. More interested in other potential ideas.
