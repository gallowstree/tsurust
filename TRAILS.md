# Trail-Based Path Tracking Implementation

## Current Problem

The current `traverse_from` function in `common/src/game.rs` only tracks which tiles a player has passed through, but doesn't capture the complete path information. This limits our ability to:

- Render visual trails showing player movement paths
- Animate player movement along their trails
- Perform accurate collision detection between player paths
- Calculate trail intersections and overlaps
- Implement trail-based game features

## Proposed Solution: Full Trail Tracking

Instead of just tracking visited tiles, `traverse_from` should return a complete `Trail` structure that captures the full path geometry.

### Trail Data Structure

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TrailSegment {
    pub board_pos: (usize, usize),     // Board cell position
    pub entry_point: u8,               // Entry point into this tile (0-7)
    pub exit_point: u8,                // Exit point from this tile (0-7)
    // Note: No tile reference needed - this is purely topological data
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trail {
    pub segments: Vec<TrailSegment>,
    pub start_pos: PlayerPosition,
    pub end_pos: PlayerPosition,
    pub completed: bool,               // true if trail ends at board edge or collision
    pub collision_point: Option<usize>, // index in segments where collision occurred
}

impl Trail {
    pub fn length(&self) -> usize {
        self.segments.len()
    }

    pub fn intersects_with(&self, other: &Trail) -> Vec<(usize, usize)> {
        // Returns pairs of segment indices where trails intersect
    }

    pub fn world_coordinates(&self, board: &Board) -> Vec<(f32, f32)> {
        // Convert trail to world coordinates for rendering
        // Takes board reference to look up tile geometry at each position
    }

    pub fn animate_position(&self, progress: f32) -> (f32, f32) {
        // Get interpolated position along trail for animation
    }
}
```

### Updated traverse_from Function

```rust
impl GameState {
    pub fn traverse_from(&self, start_pos: PlayerPosition) -> Trail {
        let mut trail = Trail {
            segments: Vec::new(),
            start_pos,
            end_pos: start_pos,
            completed: false,
            collision_point: None,
        };

        let mut current_pos = start_pos;
        let mut visited_positions = HashSet::new();

        loop {
            // Check if we've been here before (infinite loop detection)
            if visited_positions.contains(&current_pos) {
                trail.collision_point = Some(trail.segments.len());
                trail.completed = true;
                break;
            }
            visited_positions.insert(current_pos);

            // Check if we're at the board edge
            if self.is_edge_position(&current_pos) {
                trail.completed = true;
                break;
            }

            // Get the tile at current position
            let tile = match self.board.get_tile(current_pos.cell) {
                Some(tile) => tile,
                None => {
                    trail.completed = true;
                    break;
                }
            };

            // Find the exit point from this tile
            let exit_point = tile.get_connected_point(current_pos.entry_point);

            // Create trail segment
            let segment = TrailSegment {
                board_pos: current_pos.cell,
                entry_point: current_pos.entry_point,
                exit_point,
            };
            trail.segments.push(segment);

            // Calculate next position
            let next_pos = self.calculate_next_position(current_pos.cell, exit_point);
            match next_pos {
                Some(pos) => {
                    current_pos = pos;
                    trail.end_pos = pos;
                }
                None => {
                    trail.completed = true;
                    break;
                }
            }
        }

        trail
    }
}
```

## Benefits of Trail-Based Implementation

### 1. Visual Trail Rendering
```rust
impl BoardRenderer {
    pub fn render_trail(&mut self, ui: &mut egui::Ui, trail: &Trail, board: &Board, color: Color32) {
        let coords = trail.world_coordinates(board);
        for i in 0..coords.len().saturating_sub(1) {
            ui.painter().line_segment(
                [coords[i].into(), coords[i + 1].into()],
                Stroke::new(2.0, color)
            );
        }
    }

    fn tile_entry_point_to_world(&self, board_pos: (usize, usize), entry_point: u8) -> (f32, f32) {
        // Convert tile position + entry point to world coordinates
        // Uses existing tile rendering math to map entry points to pixel positions
    }
}
```

### 2. Player Movement Animation
```rust
impl Player {
    pub fn animate_along_trail(&mut self, trail: &Trail, dt: f32) {
        self.animation_progress += dt * ANIMATION_SPEED;
        if self.animation_progress >= 1.0 {
            self.position = trail.end_pos;
            self.animation_progress = 0.0;
        } else {
            let world_pos = trail.animate_position(self.animation_progress);
            self.visual_position = world_pos;
        }
    }
}
```

### 3. Collision Detection
```rust
impl GameState {
    pub fn check_trail_collisions(&self, new_trail: &Trail) -> Vec<CollisionInfo> {
        let mut collisions = Vec::new();

        for (player_id, player) in &self.players {
            if let Some(player_trail) = &player.current_trail {
                let intersections = new_trail.intersects_with(player_trail);
                for (seg1, seg2) in intersections {
                    collisions.push(CollisionInfo {
                        player_id: *player_id,
                        intersection_point: (seg1, seg2),
                    });
                }
            }
        }

        collisions
    }
}
```

### 4. Game State Integration
```rust
pub struct Player {
    pub position: PlayerPosition,
    pub visual_position: (f32, f32),    // For smooth animation
    pub current_trail: Option<Trail>,   // Trail from last move
    pub trail_history: Vec<Trail>,      // All previous trails
    pub alive: bool,
    pub animation_progress: f32,
}
```

## Implementation Strategy

### Phase 1: Core Trail Structure
1. Define `Trail` and `TrailSegment` structs in `common/src/game.rs`
2. Implement basic trail methods (`length`, `intersects_with`)
3. Update `traverse_from` to return `Trail` instead of `HashSet<(usize, usize)>`
4. Implement `world_coordinates` method in `BoardRenderer` (rendering-specific)

### Phase 2: Game Integration
1. Add `current_trail` field to `Player` struct
2. Update game logic to store trails when players move
3. Modify collision detection to use trail intersections

### Phase 3: Visual Implementation
1. Add trail rendering to `BoardRenderer`
2. Implement player animation along trails
3. Add trail visualization options (colors, opacity, etc.)

### Phase 4: Advanced Features
1. Trail-based game mechanics (if desired)
2. Trail analytics and statistics
3. Trail-based AI decision making

## Design Principles

- **Separation of Concerns**: Trail data is purely topological (positions + entry/exit points)
- **Rendering Independence**: Visual coordinate conversion happens in the rendering layer
- **Clean Data Model**: No tile references in trail segments - look up tiles when needed

## Compatibility Considerations

- The existing `traverse_from` interface can be maintained by adding a wrapper that extracts just the visited cells
- Current game logic that depends on visited cells can be gradually migrated
- Trail data can be optional initially to maintain backward compatibility

## Performance Notes

- Trails should be cached and only recalculated when the board changes
- For rendering, trail coordinates can be pre-computed and stored
- Trail intersection calculations should use spatial indexing for large numbers of trails