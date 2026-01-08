use tsurust_common::board::Player;
use tsurust_common::game::{Game, GameExport};

/// Playback status for replay viewer
#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackStatus {
    Paused,
    Playing,
    Finished,
}

/// State management for replay playback controls
#[derive(Debug, Clone)]
pub struct ReplayState {
    pub export: GameExport,
    pub current_move_index: usize,
    pub playback_status: PlaybackStatus,
    pub playback_speed: f32,
    pub last_step_time: Option<f64>, // Timestamp in seconds (works on all platforms including WASM)
}

impl ReplayState {
    /// Create a new replay state from an exported game
    pub fn new(export: GameExport) -> Self {
        Self {
            export,
            current_move_index: 0,
            playback_status: PlaybackStatus::Paused,
            playback_speed: 1.0,
            last_step_time: None,
        }
    }

    /// Reconstruct the game state at the current move index
    pub fn current_game_state(&self) -> Game {
        let initial_players = self.extract_initial_players();
        let moves = &self.export.game_state.board.history[0..self.current_move_index];

        Game::rebuild_from_history(initial_players, moves)
            .expect("Failed to rebuild game state from history")
    }

    /// Extract initial player positions from the export
    fn extract_initial_players(&self) -> Vec<Player> {
        self.export.game_state.players.iter()
            .map(|p| Player {
                id: p.id,
                name: p.name.clone(),
                pos: self.get_initial_position(p.id),
                alive: true,
                has_moved: false,
                color: p.color,
            })
            .collect()
    }

    /// Get the starting position for a player from their trail
    fn get_initial_position(&self, player_id: tsurust_common::board::PlayerID) -> tsurust_common::board::PlayerPos {
        self.export.game_state.player_trails
            .get(&player_id)
            .map(|trail| trail.start_pos)
            .expect("Player should have a trail")
    }

    /// Check if we can step forward
    pub fn can_step_forward(&self) -> bool {
        self.current_move_index < self.export.game_state.board.history.len()
    }

    /// Check if we can step backward
    pub fn can_step_backward(&self) -> bool {
        self.current_move_index > 0
    }

    /// Step forward one move
    pub fn step_forward(&mut self) -> Option<Game> {
        if self.can_step_forward() {
            self.current_move_index += 1;
            Some(self.current_game_state())
        } else {
            None
        }
    }

    /// Step backward one move
    pub fn step_backward(&mut self) -> Option<Game> {
        if self.can_step_backward() {
            self.current_move_index -= 1;
            Some(self.current_game_state())
        } else {
            None
        }
    }

    /// Jump to a specific move index
    pub fn set_move_index(&mut self, index: usize) -> Option<Game> {
        let max_index = self.export.game_state.board.history.len();
        if index <= max_index {
            self.current_move_index = index;
            Some(self.current_game_state())
        } else {
            None
        }
    }

    /// Update for auto-advance when playing
    /// Call this every frame to handle automatic playback
    pub fn update(&mut self, ctx: &egui::Context) -> Option<Game> {
        if self.playback_status != PlaybackStatus::Playing {
            return None;
        }

        let current_time = ctx.input(|i| i.time);

        if let Some(last_time) = self.last_step_time {
            let elapsed = (current_time - last_time) as f32;
            let step_interval = 1.0 / self.playback_speed;

            if elapsed >= step_interval {
                self.last_step_time = Some(current_time);
                ctx.request_repaint(); // Ensure continuous updates

                if self.can_step_forward() {
                    return self.step_forward();
                } else {
                    self.playback_status = PlaybackStatus::Finished;
                }
            } else {
                ctx.request_repaint(); // Keep checking
            }
        } else {
            // First update after starting playback
            self.last_step_time = Some(current_time);
            ctx.request_repaint();
        }

        None
    }

    /// Start playback
    pub fn play(&mut self) {
        self.playback_status = PlaybackStatus::Playing;
        self.last_step_time = None; // Will be initialized on first update
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playback_status = PlaybackStatus::Paused;
    }

    /// Set playback speed (moves per second)
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed;
    }

    /// Jump to the start
    pub fn jump_to_start(&mut self) -> Option<Game> {
        self.set_move_index(0)
    }

    /// Jump to the end
    pub fn jump_to_end(&mut self) -> Option<Game> {
        let end_index = self.export.game_state.board.history.len();
        self.set_move_index(end_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tsurust_common::board::{CellCoord, PlayerPos, Tile, Segment};
    use tsurust_common::game::{GameMetadata, GameMode};

    fn create_test_export() -> GameExport {
        let players = vec![
            Player {
                id: 1,
                name: "Player 1".to_string(),
                pos: PlayerPos { cell: CellCoord { row: 1, col: 1 }, endpoint: 0 },
                alive: true,
                has_moved: false,
                color: (255, 0, 0),
            },
        ];
        let mut game = Game::new(players);

        // Make a few moves
        for i in 0..3 {
            let tile = Tile {
                segments: [
                    Segment { a: 0, b: 1 },
                    Segment { a: 2, b: 3 },
                    Segment { a: 4, b: 5 },
                    Segment { a: 6, b: 7 },
                ]
            };
            game.hands.get_mut(&1).unwrap().push(tile);
            let mov = tsurust_common::board::Move {
                player_id: 1,
                cell: CellCoord { row: i + 1, col: i + 1 },
                tile,
            };
            game.perform_move(mov).ok();
        }

        let metadata = GameMetadata {
            game_mode: GameMode::Local,
            room_id: None,
            room_name: Some("Test".to_string()),
            completed: false,
            winner_id: None,
            total_turns: game.board.history.len(),
            player_names: vec![(1, "Player 1".to_string())],
        };

        game.export(metadata, None)
    }

    #[test]
    fn test_replay_step_forward_backward() {
        let export = create_test_export();
        let mut replay_state = ReplayState::new(export);

        // Initial state
        assert_eq!(replay_state.current_move_index, 0);
        assert!(replay_state.can_step_forward());
        assert!(!replay_state.can_step_backward());

        // Step forward
        replay_state.step_forward();
        assert_eq!(replay_state.current_move_index, 1);
        assert!(replay_state.can_step_backward());
        assert!(replay_state.can_step_forward());

        // Step backward
        replay_state.step_backward();
        assert_eq!(replay_state.current_move_index, 0);
        assert!(!replay_state.can_step_backward());
    }

    #[test]
    fn test_replay_jump_to_move() {
        let export = create_test_export();
        let mut replay_state = ReplayState::new(export);

        // Jump to middle
        replay_state.set_move_index(2);
        assert_eq!(replay_state.current_move_index, 2);

        // Jump to end
        let end_index = replay_state.export.game_state.board.history.len();
        replay_state.set_move_index(end_index);
        assert_eq!(replay_state.current_move_index, end_index);
        assert!(!replay_state.can_step_forward());
    }

    #[test]
    fn test_replay_speed_control() {
        let export = create_test_export();
        let mut replay_state = ReplayState::new(export);

        replay_state.set_speed(2.0);
        assert_eq!(replay_state.playback_speed, 2.0);

        replay_state.set_speed(0.5);
        assert_eq!(replay_state.playback_speed, 0.5);
    }

    #[test]
    fn test_replay_playback_status() {
        let export = create_test_export();
        let mut replay_state = ReplayState::new(export);

        assert_eq!(replay_state.playback_status, PlaybackStatus::Paused);

        replay_state.play();
        assert_eq!(replay_state.playback_status, PlaybackStatus::Playing);

        replay_state.pause();
        assert_eq!(replay_state.playback_status, PlaybackStatus::Paused);
    }
}
