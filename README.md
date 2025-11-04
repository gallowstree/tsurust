# Tsurust

A multiplayer implementation of the [Tsuro](https://en.wikipedia.org/wiki/Tsuro) board game in Rust, featuring both local and online multiplayer gameplay.

## Features

- **Local Multiplayer**: Play offline with multiple players on the same machine
- **Online Multiplayer**: Host or join games via WebSocket server
- **Server With Lobby System**: Create rooms, invite players, and configure starting positions

## Architecture

The project is organized as a Cargo workspace with three main crates:

- **`common/`** - Core game logic, board state, and networking protocol
- **`client-egui/`** - GUI client built with [egui](https://github.com/emilk/egui)
- **`server/`** - WebSocket server for online multiplayer

## Getting Started

### Prerequisites

- Rust toolchain (install from [rustup.rs](https://rustup.rs/))

### Building

```bash
# Build all crates
cargo build --workspace

# Build with optimizations (faster runtime)
cargo build --workspace --release
```

### Running

**Local Game (Offline Multiplayer)**
```bash
cargo run --bin client-egui_bin
```
Click "Local Game" in the main menu to start an offline multiplayer session.

**Online Multiplayer**

1. Start the server:
```bash
cargo run --bin server
```

2. Launch client(s):
```bash
cargo run --bin client-egui_bin
```

3. In each client:
   - Click "Host Game" (first player) or "Join Game" (other players)
   - Enter the lobby code
   - Place starting pawns
   - Click "Start Game"

### Testing

```bash
# Run all tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test <test_name>
```

## Game Rules

Tsuro is a path-building board game where players navigate a 6x6 grid by placing tiles that create winding paths. Each tile has 8 entry points connected by 4 path segments.

**Objective**: Be the last player remaining on the board.

**Gameplay**:
1. Players start at the board edge with 3 tiles in hand
2. On your turn, place a tile at your current position
3. Your pawn follows the path automatically
4. If you reach the board edge or collide with another player, you're eliminated
5. Draw a new tile to maintain 3 tiles in hand

## Project Structure

```
tsurust/
├── common/           # Shared game logic
│   ├── src/
│   │   ├── board.rs      # Board state, tiles, players
│   │   ├── game.rs       # Game state management
│   │   ├── deck.rs       # Tile deck
│   │   ├── lobby.rs      # Multiplayer lobby
│   │   ├── protocol.rs   # Network protocol messages
│   │   └── trail.rs      # Player trail visualization
│   └── tests/            # Integration tests
│
├── client-egui/      # GUI client
│   └── src/
│       ├── app.rs           # Main application state
│       ├── rendering.rs     # Custom tile rendering
│       ├── board_renderer.rs # Game board display
│       ├── tile_button.rs   # Interactive tile UI
│       └── screens/         # UI screens (menu, lobby, game)
│
└── server/           # WebSocket server
    └── src/
        ├── main.rs       # Server entry point
        ├── server.rs     # Server state management
        ├── room.rs       # Game room logic
        └── handler.rs    # WebSocket connection handling
```

## Development

### Code Guidelines

- **Error Handling**: Always use `.expect("descriptive message")` instead of `.unwrap()`
- **Documentation**: Update `DEVELOPMENT_ROADMAP.md` for TODOs (not completion reports)

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run --bin client-egui_bin
RUST_LOG=debug cargo run --bin server
```

## Roadmap

See `DEVELOPMENT_ROADMAP.md` for detailed development plans and technical debt tracking.

Current priorities:
- Server-side improvements (disconnect handling, room cleanup)
- WASM compilation for browser deployment
- Comprehensive integration tests
- Animation system for player movement

## Contributing

This is a personal project, but feedback and suggestions are welcome! Feel free to open issues for bugs or feature requests.

## License

This project is available for educational and personal use. Tsuro is a registered trademark of Calliope Games.

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) immediate mode GUI framework
- WebSocket server powered by [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
