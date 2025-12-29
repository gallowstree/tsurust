use std::sync::mpsc;

use eframe::egui;

use tsurust_common::board::{PlayerID, PlayerPos};
use tsurust_common::lobby::Lobby;

use crate::app::Message;
use crate::messaging::send_ui_message;

/// Component for rendering the lobby board with spawn positions
pub struct LobbyBoard<'a> {
    lobby: &'a Lobby,
    #[allow(dead_code)]
    current_player_id: PlayerID,
}

impl<'a> LobbyBoard<'a> {
    pub fn new(lobby: &'a Lobby, current_player_id: PlayerID) -> Self {
        Self {
            lobby,
            current_player_id,
        }
    }

    // TODO (low): use existing board rendering code and add support in there for spawns, etc.
    /// Render the lobby board in the given UI with the specified size
    pub fn render(&self, ui: &mut egui::Ui, board_size: f32, sender: &mpsc::Sender<Message>) {
        let (rect, _response) = ui.allocate_exact_size(egui::Vec2::splat(board_size), egui::Sense::hover());

        // Draw board grid
        self.draw_grid(ui, rect, board_size);

        // Draw spawn positions and handle interaction
        self.draw_spawn_positions(ui, rect, board_size, sender);
    }

    fn draw_grid(&self, ui: &mut egui::Ui, rect: egui::Rect, board_size: f32) {
        // Draw outer border
        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_GRAY));

        let cell_size = board_size / 6.0;

        // Draw grid lines
        for i in 1..6 {
            let x = rect.min.x + i as f32 * cell_size;
            let y = rect.min.y + i as f32 * cell_size;

            // Vertical lines
            ui.painter().line_segment(
                [egui::Pos2::new(x, rect.min.y), egui::Pos2::new(x, rect.max.y)],
                egui::Stroke::new(1.0, egui::Color32::GRAY)
            );

            // Horizontal lines
            ui.painter().line_segment(
                [egui::Pos2::new(rect.min.x, y), egui::Pos2::new(rect.max.x, y)],
                egui::Stroke::new(1.0, egui::Color32::GRAY)
            );
        }
    }

    fn draw_spawn_positions(&self, ui: &mut egui::Ui, rect: egui::Rect, board_size: f32, sender: &mpsc::Sender<Message>) {
        let cell_size = board_size / 6.0;

        // Draw available spawn positions on outer border only
        for row in 0..6 {
            for col in 0..6 {
                // Only edge cells
                if row == 0 || row == 5 || col == 0 || col == 5 {
                    for endpoint in 0..8 {
                        // Only render endpoints that are on the actual outer border
                        if !self.is_outer_endpoint(row, col, endpoint) {
                            continue;
                        }

                        let pos = PlayerPos::new(row, col, endpoint);
                        let spawn_center = self.get_endpoint_pos(rect, cell_size, row, col, endpoint);

                        // Check if position is already taken
                        let is_taken = self.lobby.players.values().any(|p| p.spawn_position == Some(pos));

                        if is_taken {
                            self.draw_occupied_spawn(ui, spawn_center, pos);
                        } else {
                            self.draw_available_spawn(ui, spawn_center, pos, sender);
                        }
                    }
                }
            }
        }
    }

    fn is_outer_endpoint(&self, row: usize, col: usize, endpoint: usize) -> bool {
        match (row, col, endpoint) {
            // Top edge cells (row 0): only endpoints 4-5 (top edge)
            (0, _, 4..=5) => true,
            // Bottom edge cells (row 5): only endpoints 0-1 (bottom edge)
            (5, _, 0..=1) => true,
            // Left edge cells (col 0): only endpoints 6-7 (left edge)
            (_, 0, 6..=7) => true,
            // Right edge cells (col 5): only endpoints 2-3 (right edge)
            (_, 5, 2..=3) => true,
            _ => false,
        }
    }

    fn get_endpoint_pos(&self, rect: egui::Rect, cell_size: f32, row: usize, col: usize, endpoint: usize) -> egui::Pos2 {
        let cell_min = rect.min + egui::Vec2::new(col as f32 * cell_size, row as f32 * cell_size);
        let half_cell = cell_size / 2.0;

        // Endpoint positions (counterclockwise from bottom-left):
        // 0-1: bottom, 2-3: right, 4-5: top, 6-7: left
        match endpoint {
            0 => cell_min + egui::Vec2::new(half_cell * 0.5, cell_size),     // bottom-left
            1 => cell_min + egui::Vec2::new(half_cell * 1.5, cell_size),     // bottom-right
            2 => cell_min + egui::Vec2::new(cell_size, half_cell * 1.5),     // right-bottom
            3 => cell_min + egui::Vec2::new(cell_size, half_cell * 0.5),     // right-top
            4 => cell_min + egui::Vec2::new(half_cell * 1.5, 0.0),           // top-right
            5 => cell_min + egui::Vec2::new(half_cell * 0.5, 0.0),           // top-left
            6 => cell_min + egui::Vec2::new(0.0, half_cell * 0.5),           // left-top
            7 => cell_min + egui::Vec2::new(0.0, half_cell * 1.5),           // left-bottom
            _ => cell_min + egui::Vec2::new(half_cell, half_cell),
        }
    }

    fn draw_occupied_spawn(&self, ui: &mut egui::Ui, spawn_center: egui::Pos2, pos: PlayerPos) {
        // Draw taken position with player's color
        if let Some(lobby_player) = self.lobby.players.values().find(|p| p.spawn_position == Some(pos)) {
            let player_color = egui::Color32::from_rgb(
                lobby_player.color.0,
                lobby_player.color.1,
                lobby_player.color.2
            );
            ui.painter().circle_filled(spawn_center, 6.0, player_color);
            ui.painter().circle_stroke(spawn_center, 6.0, (2.0, egui::Color32::WHITE));
        }
    }

    fn draw_available_spawn(&self, ui: &mut egui::Ui, spawn_center: egui::Pos2, pos: PlayerPos, sender: &mpsc::Sender<Message>) {
        // Draw available spawn as clickable spot
        let spawn_response = ui.allocate_rect(
            egui::Rect::from_center_size(spawn_center, egui::Vec2::splat(12.0)),
            egui::Sense::click()
        );

        let color = if spawn_response.hovered() {
            egui::Color32::from_rgb(100, 150, 255)
        } else {
            egui::Color32::from_rgba_premultiplied(150, 150, 150, 100)
        };

        ui.painter().circle_filled(spawn_center, 4.0, color);

        if spawn_response.clicked() {
            #[cfg(target_arch = "wasm32")]
            {
                web_sys::console::log_1(&format!("Pawn placement clicked at position: {:?}", pos).into());
            }
            send_ui_message(sender, Message::PlacePawn(pos));
        }
    }
}