# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Tsurust is a Rust implementation of the Tsuro board game with an egui-based GUI client and WebSocket multiplayer server. The project is structured as a Cargo workspace with three main crates:

- `common/`: Core game logic, data structures, game rules, and networking protocol
- `client-egui/`: GUI client using egui with hand-drawn rendering primitives (native and WASM)
- `server/`: WebSocket server for online multiplayer

## Build and Run Commands

### Development
- `cargo run --bin client-egui_bin` - Run the GUI client
- `cargo run --bin server` - Run the WebSocket server
- `cargo check --workspace` - Check compilation across all crates
- `cargo test --workspace` - Run all tests (70+ tests across all crates)
- `cargo build --workspace` - Build all crates

### Debugging
- `RUST_LOG=debug cargo run --bin client-egui_bin` - Run with debug logging

## Architecture

### Core Game Components (`common/`)
- **Board**: 6x6 grid where tiles are placed
- **Tiles**: Represented as 4 segments connecting 8 entry points (numbered 0-7 counterclockwise)
- **Players**: Have position (cell + entry point) and alive status
- **Game State**: Manages deck, hands, board history, and turn logic
- **Move**: Represents tile placement by a player

### UI Components (`client-egui/`)
- **TemplateApp**: Main app state with mpsc channels for event handling
- **BoardRenderer**: Renders the 6x6 game board with placed tiles and player positions
- **HandRenderer**: Displays player's tiles in a scrollable side panel
- **TileButton**: Interactive tile widget with rotation (left/right click) and selection

### Communication Pattern
The UI uses mpsc channels for event passing. TileButton components send messages to the main app loop, which processes them and updates game state.

## Key Technical Details

### Tile System
- Each tile has 8 entry points numbered 0-7 (counterclockwise from bottom-left)
- Tiles are defined by 4 segments (pairs of connected entry points)
- Tiles can be rotated clockwise/counterclockwise
- Tile rendering uses custom drawing primitives (no sprites)

### Coordinate System
- Board: 6x6 grid with (0,0) at top-left
- Entry points on each tile: 0-1 (bottom), 2-3 (right), 4-5 (top), 6-7 (left)

### Game Flow
- Players start with 3 tiles in hand
- On each turn: place a tile, move along paths, refill hand
- Players eliminated when they reach board edge

### Statistics System
The game tracks comprehensive player statistics during gameplay:
- **Turns Survived**: Number of turns before elimination
- **Tiles Placed**: Total tiles placed during the game
- **Path Length**: Total cells traversed (including revisits)
- **Trail Distance**: Actual geometric distance traveled along tile paths, calculated using Euclidean distance between entry/exit points on a 3×3 coordinate grid
- **Dragon Turns**: Number of turns holding the dragon tile
- **Players Eliminated**: Count of other players eliminated by this player
- **Unique Tiles Visited**: Number of distinct board cells visited
- **Board Coverage**: Percentage of the 6×6 board explored
- **Max Revisits**: Highest visit count to any single cell
- **Efficiency**: Path length divided by tiles placed

All statistics are automatically included in game exports for replay analysis.

## Development Practices

- **Code Organization**: Move code between `common/` and `client-egui/` as it makes sense
- **Simplicity**: When in doubt, keep it simple and refactor to simplify further
- **Error Handling**: Always use `.expect("descriptive message")` instead of `.unwrap()` to provide context when panics occur
- **Testing**: Add tests to `common/` crate for game logic validation
- **Rendering**: Continue using hand-drawn primitives rather than adding sprite assets

## Documentation Policy ⚠️ IMPORTANT

**DEVELOPMENT_ROADMAP.md is for TODO items only, NOT status updates**

❌ **NEVER add to roadmap**:
- "✅ COMPLETE" sections
- "Successfully implemented X" reports
- Lists of what was done
- Status updates or progress reports
- Any checkmarks, completion markers, or achievement lists

✅ **Roadmap should only contain**:
- Active TODO items that need work
- Technical debt that needs addressing
- Future phases and planned work

**Where completed work belongs**:
- Git commit messages (the ONLY place for "what was done")
- Code comments (for implementation details)
- Nowhere else

**Enforcement**: Before modifying DEVELOPMENT_ROADMAP.md, ask yourself: "Am I about to add a status update or completion report?" If yes, STOP and don't add it.

## Current Status

The game is fully playable with both local and online multiplayer modes:
- ✅ Core game logic and rules implementation
- ✅ Local multiplayer with lobby system
- ✅ Online multiplayer via WebSocket server
- ✅ Native (desktop) and WASM (browser) support
- ✅ Comprehensive statistics tracking and display
- ✅ Game export/import with JSON serialization
- ✅ Replay system with playback controls
- 🚧 Server improvements (disconnect handling, reconnection)
- 🚧 UI polish and animations

See DEVELOPMENT_ROADMAP.md for detailed current priorities and technical debt.