# Development Roadmap

## Current Development Prioritie

### **Phase 1: Trail System Implementation** (1-2 weeks)
**Goal**: Implement the improved trail-based architecture from TRAILS.md

#### Tasks:
1. **Define Trail Data Structures**
   ```rust
   pub struct TrailSegment {
       pub board_pos: (usize, usize),
       pub entry_point: u8,
       pub exit_point: u8,
   }

   pub struct Trail {
       pub segments: Vec<TrailSegment>,
       pub start_pos: PlayerPosition,
       pub end_pos: PlayerPosition,
       pub completed: bool,
   }
   ```

2. **Update `traverse_from` to Return Trail**
   - Replace current `HashSet<(usize, usize)>` return with `Trail`
   - Maintain backward compatibility with wrapper function
   - Add trail intersection detection methods

3. **Implement Trail-Based Rendering**
   - Add `world_coordinates()` method to convert trails to screen coordinates
   - Update `BoardRenderer` to use trail data for path rendering
   - Add smooth animation support using trail data

**Acceptance Criteria**: Trails rendered with better accuracy, foundation for animations

---

### **Phase 2: Multiplayer Server** (2-3 weeks)
**Goal**: Add networked multiplayer support using tarpc RPC framework

#### Architecture Overview:
- **Server Module**: New `server/` crate for game server logic
- **RPC Service**: tarpc-based service for client-server communication
- **Client Updates**: Modify client to communicate with server instead of local game state

#### Tasks:
1. **Create Server Crate**
   ```
   server/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs          # Server binary
   │   ├── lib.rs           # Server library
   │   ├── service.rs       # tarpc service implementation
   │   ├── game_manager.rs  # Multi-game state management
   │   └── room.rs          # Game room/lobby management
   ```

2. **Define RPC Service Interface**
   ```rust
   #[tarpc::service]
   trait TsuroGameService {
       // Room management
       async fn create_room(room_name: String) -> Result<RoomId, GameError>;
       async fn join_room(room_id: RoomId, player_name: String) -> Result<PlayerId, GameError>;
       async fn leave_room(room_id: RoomId, player_id: PlayerId) -> Result<(), GameError>;

       // Game actions
       async fn place_tile(room_id: RoomId, player_id: PlayerId, tile: Tile, cell: CellCoord) -> Result<GameState, GameError>;
       async fn get_game_state(room_id: RoomId) -> Result<GameState, GameError>;

       // Real-time updates
       async fn subscribe_to_game_updates(room_id: RoomId) -> Result<GameUpdateStream, GameError>;
   }
   ```

3. **Server-Side Game Management**
   - Multi-room support with concurrent games
   - Player session management and authentication
   - Game state validation and synchronization
   - Spectator support

4. **Client-Side Integration**
   - Replace local `Game` state with `GameClient` that communicates via RPC
   - Handle network latency and connection issues
   - Implement optimistic updates for responsiveness
   - Add lobby/room selection UI

5. **Network Protocol Design**
   - Efficient serialization of game state updates
   - Delta updates instead of full state sync
   - Heartbeat/keepalive for connection management
   - Graceful handling of disconnections and reconnections

**Acceptance Criteria**:
- Multiple clients can join same game room
- Real-time synchronized gameplay across all clients
- Robust handling of network issues and player disconnections

---

### **Phase 3: Polish & Advanced Features** (Ongoing)
**Goal**: Code quality, type safety, and enhanced features

#### Tasks:
1. **Type System Improvements**
   - Convert `TileEndpoint` to enum with named directions
   - Add proper error types instead of string literals
   - Implement comprehensive validation

2. **Code Quality**
   - Remove unused imports and variables (many warnings currently)
   - Improve error messages and user feedback
   - Add comprehensive tests for game logic and RPC layer

3. **Remaining UI Enhancements**
   - Game over screen with winner announcement
   - Error messages for invalid moves
   - Player setup improvements (custom starting positions)
   - Player name customization

4. **Advanced Features**
   - Implement game replay system
   - Add AI opponents that can join multiplayer games
   - Tournament/ranking system
   - Spectator mode with game history
   - Custom game variants and rule modifications


## Code Organization: app.rs Refactoring

### Problem Analysis
`client-egui/src/app.rs` has grown to 674 lines with multiple responsibilities:
- Application state management (AppState enum, TemplateApp struct)
- Message handling and routing (~150 lines of match statements)
- UI rendering for 5 different screens (MainMenu, CreateLobbyForm, JoinLobbyForm, Lobby, Game)
- Lobby board rendering with spawn position logic
- Game UI with player cards and board display

This violates Single Responsibility Principle and makes the code hard to maintain, test, and navigate.

### Refactoring Strategies Considered

#### Strategy 1: Screen-Based Modules ⭐ **RECOMMENDED**
**Approach**: Split by UI screens/views with dedicated modules for each major screen.

```
client-egui/src/
├── app.rs              # Core app struct, state machine, message routing (150 lines)
├── screens/
│   ├── mod.rs          # Screen trait definition
│   ├── main_menu.rs    # Main menu rendering (30 lines)
│   ├── lobby_form.rs   # Create/Join lobby forms (80 lines)
│   ├── lobby.rs        # Lobby UI with board (150 lines)
│   └── game.rs         # Game UI (250 lines)
├── components/
│   ├── lobby_board.rs  # Extracted lobby board rendering (120 lines)
│   └── (existing: board_renderer, hand_renderer, player_card, tile_button)
└── (existing: lib.rs, main.rs, rendering.rs)
```

**Pros**:
- Natural separation by user flow
- Each screen is self-contained and testable
- Easy to locate code for specific screens
- Aligns with state machine (AppState enum)
- Can introduce `Screen` trait for consistency

**Cons**:
- Some screens share logic (forms have similar structure)
- May need shared utilities module

**Implementation Complexity**: Medium (3-4 hours)

---

#### Strategy 2: Feature-Based Modules
**Approach**: Organize by feature (lobby, game, menu).

```
client-egui/src/
├── app.rs              # App struct, state machine
├── lobby/
│   ├── mod.rs
│   ├── forms.rs        # Create/Join forms
│   ├── ui.rs           # Lobby UI
│   └── board.rs        # Lobby board rendering
├── game/
│   ├── mod.rs
│   ├── ui.rs           # Game UI
│   └── (use existing board_renderer, hand_renderer)
└── menu.rs             # Main menu
```

**Pros**:
- Clear feature boundaries
- Related code grouped together
- Good for larger features with multiple components

**Cons**:
- Lobby feature split across forms/ui/board may feel artificial
- Less clear mapping to state machine states

**Implementation Complexity**: Medium (3-4 hours)

---

#### Strategy 3: MVC-Style Separation
**Approach**: Separate rendering, state, and message handling.

```
client-egui/src/
├── app.rs              # Core app struct
├── state.rs            # AppState enum and transitions
├── messages.rs         # Message enum and handlers
├── views/
│   ├── main_menu.rs
│   ├── lobby_forms.rs
│   ├── lobby_view.rs
│   └── game_view.rs
└── controllers/
    ├── lobby_controller.rs
    └── game_controller.rs
```

**Pros**:
- Clear separation of concerns (MVC pattern)
- Message handlers separate from rendering
- State transitions isolated

**Cons**:
- More boilerplate and indirection
- egui's immediate mode makes MVC less natural
- May be over-engineered for current scale
- Higher cognitive overhead navigating between layers

**Implementation Complexity**: High (5-6 hours)

---

### Recommendation: Strategy 1 (Screen-Based Modules)

**Rationale**:
1. **Natural fit with state machine**: Each AppState maps to a screen module
2. **Balanced granularity**: Not too fine-grained (MVC) nor too coarse (single file)
3. **Egui-friendly**: Immediate mode GUI works well with screen-based organization
4. **Easy navigation**: Developer thinking "I need to fix the lobby UI" knows exactly where to go
5. **Incremental refactoring**: Can extract screens one at a time
6. **Testability**: Each screen can be tested independently with mock senders

**Optional Enhancement**: Add `Screen` trait for consistency:
```rust
pub trait Screen {
    fn render(&self, ctx: &Context, sender: &mpsc::Sender<Message>);
}
```

### Implementation Priority
1. Extract `lobby_board.rs` first (most complex, already isolated)
2. Extract forms (similar structure, easy wins)
3. Extract game and lobby UI
4. Extract main menu (simplest)

---

## Form UX Improvements

### Auto-focus and Submit on Enter

**Requirements**:
- Focus first input field when entering form screen
- Submit form when pressing Enter in any field
- Only enable submit if validation passes

**Implementation**:
```rust
// In render_create_lobby_form:
let lobby_name_response = ui.text_edit_singleline(lobby_name);
if ui.input(|i| i.key_pressed(egui::Key::Enter)) && can_create {
    sender.send(Message::CreateAndJoinLobby(lobby_name.clone(), player_name.clone())).ok();
}

// Request focus on first render (track with state flag)
if is_first_render {
    lobby_name_response.request_focus();
}
```

**Details**:
- Track whether form was just entered (add `form_just_opened` flag)
- Use `request_focus()` on first text field
- Use `ui.input(|i| i.key_pressed(egui::Key::Enter))` for submit
- Ensure validation runs before submit

---

## Technical Debt Log

- `common/src/lib.rs:14` - Rename TileEndpoint references to "entry point"
- `common/src/board.rs:14` - Convert TileEndpoint from usize to enum
- Trail system - Current tile-based approach should be replaced with TRAILS.md design
- **NEW**: `client-egui/src/app.rs` - 674 lines, needs screen-based module refactoring
