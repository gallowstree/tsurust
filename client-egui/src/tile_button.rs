use std::sync::mpsc;

use eframe::egui::{vec2, Frame, Rect, Sense, Widget};

use tsurust_common::board::*;

use crate::app::Message;
use crate::messaging::send_ui_message;
use crate::rendering::{paint_tile_with_rotation, paint_tile_button_hoverlay, paint_tile_button_hoverlay_with_highlight, tile_to_screen_transform};

pub struct TileButton {
    tile: Tile,
    index: usize,
    sender: mpsc::Sender<Message>,
    // Animation state
    rotation_progress: f32,  // 0.0 = no animation, 1.0 = animation complete
    target_rotation_steps: i32,  // How many 90° rotations to animate (positive = CW, negative = CCW)
}

impl TileButton {
    pub fn new(tile: Tile, index: usize, sender: mpsc::Sender<Message>) -> Self {
        Self {
            tile,
            index,
            sender,
            rotation_progress: 1.0,  // Start with no animation
            target_rotation_steps: 0,
        }
    }

    pub fn with_rotation_animation(mut self, clockwise: bool) -> Self {
        self.rotation_progress = 0.0;
        self.target_rotation_steps = if clockwise { 1 } else { -1 };
        self
    }
}

impl Widget for TileButton {
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(140.0, 140.0), Sense::click().union(Sense::hover()));

        // Handle animation
        let animation_duration = 0.5; // seconds - longer for visibility
        let is_animating = self.rotation_progress < 1.0;

        if is_animating {
            // Animate using egui's animation system
            let animation_id = response.id.with("rotation");

            // Animate from current progress toward 1.0
            let progress = ui.ctx().animate_value_with_time(
                animation_id,
                1.0,
                animation_duration
            );
            self.rotation_progress = progress;

            // Request repaint for next frame
            ui.ctx().request_repaint();
        }

        let to_screen = tile_to_screen_transform(rect);
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let pos = to_screen.inverse().transform_pos(pos);
                if pos.x < 1. {
                    // Left side clicked - rotate counterclockwise
                    send_ui_message(&self.sender, Message::TileRotated(self.index, false));
                } else if pos.x > 2. {
                    // Right side clicked - rotate clockwise
                    send_ui_message(&self.sender, Message::TileRotated(self.index, true));
                } else {
                    // Center clicked - place tile at current player position
                    send_ui_message(&self.sender, Message::TilePlaced(self.index));
                }
            }
        }

        Frame::canvas(ui.style())
            .show(ui, |ui| {
                let painter = ui.painter();
                let rect = response.rect;

                // Calculate current rotation angle for animation
                // The tile data has already been rotated, so we need to show it rotating FROM old TO new
                // This means we start at the NEGATIVE of the rotation and animate to 0
                let rotation_angle = if self.rotation_progress < 1.0 {
                    let target_angle = self.target_rotation_steps as f32 * std::f32::consts::FRAC_PI_2;

                    // Ease-out cubic for smooth deceleration
                    let eased_progress = 1.0 - (1.0 - self.rotation_progress).powi(3);

                    // Start at -target_angle (old position) and animate to 0 (new position)
                    -target_angle * (1.0 - eased_progress)
                } else {
                    0.0  // No rotation when animation is complete
                };

                // Draw tile with rotation
                paint_tile_with_rotation(
                    &self.tile,
                    Rect::from_center_size(rect.center(), vec2(139., 139.)),
                    painter,
                    rotation_angle,
                );

                // Draw hover overlay on top so rotation indicators are visible
                if response.hovered() {
                    if let Some(pos) = response.hover_pos() {
                        let pos = to_screen.inverse().transform_pos(pos);
                        if pos.x < 1. {
                            // Hovering over left rotation area - show both buttons, highlight left
                            paint_tile_button_hoverlay_with_highlight(rect, painter, Some(false));
                        } else if pos.x > 2. {
                            // Hovering over right rotation area - show both buttons, highlight right
                            paint_tile_button_hoverlay_with_highlight(rect, painter, Some(true));
                        } else {
                            // Hovering over center placement area - show both buttons, no highlight
                            paint_tile_button_hoverlay_with_highlight(rect, painter, None);
                        }
                    } else {
                        paint_tile_button_hoverlay(rect, painter);
                    }
                }
            });
        response
    }
}
