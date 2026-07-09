use std::sync::mpsc;

use eframe::egui::{vec2, Rect, Sense, Widget, WidgetInfo, WidgetType};

use tsurust_common::board::*;

use crate::app::Message;
use crate::messaging::send_ui_message;
use crate::rendering::{
    paint_tile_button_hoverlay, paint_tile_button_hoverlay_with_highlight,
    paint_tile_with_rotation, tile_to_screen_transform,
};

pub struct TileButton {
    tile: Tile,
    index: usize,
    sender: mpsc::Sender<Message>,
    // Animation state
    rotation_progress: f32,     // 0.0 = no animation, 1.0 = animation complete
    target_rotation_steps: i32, // How many 90° rotations to animate (positive = CW, negative = CCW)
    /// The engine would reject this rotation (forced-suicide rule): warn, but
    /// keep the button interactive — rotating it may make it legal.
    fatal: bool,
}

impl TileButton {
    pub fn new(tile: Tile, index: usize, sender: mpsc::Sender<Message>) -> Self {
        Self {
            tile,
            index,
            sender,
            rotation_progress: 1.0, // Start with no animation
            target_rotation_steps: 0,
            fatal: false,
        }
    }

    pub fn with_rotation_animation(mut self, clockwise: bool) -> Self {
        self.rotation_progress = 0.0;
        self.target_rotation_steps = if clockwise { 1 } else { -1 };
        self
    }

    pub fn with_fatal_warning(mut self) -> Self {
        self.fatal = true;
        self
    }
}

impl Widget for TileButton {
    fn ui(mut self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(vec2(140.0, 140.0), Sense::click().union(Sense::hover()));
        // Label the tile for screen readers (and UI tests)
        response.widget_info(|| {
            WidgetInfo::labeled(
                WidgetType::Button,
                true,
                format!("hand tile {}", self.index),
            )
        });

        // Handle animation
        let animation_duration = crate::app::animation::TILE_ROTATION_DURATION_SECS;
        let is_animating = self.rotation_progress < 1.0;

        if is_animating {
            // Animate using egui's animation system
            let animation_id = response.id.with("rotation");

            // Animate from current progress toward 1.0
            let progress = ui
                .ctx()
                .animate_value_with_time(animation_id, 1.0, animation_duration);
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

        // Paint directly on the allocated rect: wrapping this in a Frame would
        // allocate extra layout space and draw a stray empty box after the tile.
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
            0.0 // No rotation when animation is complete
        };

        // Draw tile with rotation
        paint_tile_with_rotation(
            &self.tile,
            Rect::from_center_size(rect.center(), vec2(139., 139.)),
            painter,
            rotation_angle,
        );

        // Forced-suicide warning: this rotation would be rejected by the
        // engine while a surviving option exists.
        if self.fatal {
            let warn_color = eframe::egui::Color32::from_rgb(220, 70, 70);
            painter.rect_stroke(
                rect.shrink(2.0),
                eframe::egui::CornerRadius::same(6),
                eframe::egui::Stroke::new(2.0, warn_color),
                eframe::egui::StrokeKind::Inside,
            );
            painter.text(
                rect.right_top() + vec2(-14.0, 14.0),
                eframe::egui::Align2::CENTER_CENTER,
                "☠",
                eframe::egui::FontId::proportional(16.0),
                warn_color,
            );
        }

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
        if self.fatal {
            return response.on_hover_text(
                "This rotation would eliminate you — rotate it or pick another tile.",
            );
        }
        response
    }
}
