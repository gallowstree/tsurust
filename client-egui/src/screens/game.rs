use eframe::egui::{self, Context};
use std::sync::mpsc;
use crate::app::Message;
use crate::board_renderer::BoardRenderer;
use crate::hand_renderer::HandRenderer;
use crate::player_card::PlayerCard;
use tsurust_common::board::Player;
use tsurust_common::game::Game;

pub fn render_game_ui(ctx: &Context, game: &mut Game, sender: &mpsc::Sender<Message>) {
    egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(32.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if ui.button("🔄 Restart Game").clicked() {
                    if let Err(e) = sender.send(Message::RestartGame) {
                        eprintln!("Failed to send RestartGame message: {}", e);
                    }
                }
                if ui.button("⬅ Back to Menu").clicked() {
                    if let Err(e) = sender.send(Message::BackToMainMenu) {
                        eprintln!("Failed to send BackToMainMenu message: {}", e);
                    }
                }
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(20.);
            ui.add(BoardRenderer::new(&game.board.history, &game.players, &game.tile_trails, &game.player_trails));
        });
    });

    egui::SidePanel::right("right_panel").show(ctx, |ui| {
        ui.vertical(|ui| {
            // Sort players: alive first, then dead
            let mut sorted_players: Vec<&Player> = game.players.iter().collect();
            sorted_players.sort_by_key(|p| !p.alive);

            // Determine winner: if only one player is alive, they're the winner
            let alive_count = game.players.iter().filter(|p| p.alive).count();
            let winner_id = if alive_count == 1 {
                game.players.iter().find(|p| p.alive).map(|p| p.id)
            } else {
                None
            };

            for player in sorted_players {
                let hand_size = game.hands.get(&player.id).map(|h| h.len()).unwrap_or(0);
                let has_dragon = game.dragon == Some(player.id);
                let is_current = player.id == game.current_player_id;
                let is_winner = winner_id == Some(player.id);

                ui.horizontal(|ui| {
                    // Arrow indicator for current player
                    let (arrow_rect, _) = ui.allocate_exact_size(egui::Vec2::new(16.0, 60.0), egui::Sense::hover());

                    if is_current {
                        let triangle_center = arrow_rect.center();
                        let triangle_size = 12.0;

                        let points = [
                            triangle_center + egui::Vec2::new(-triangle_size/2.0, -triangle_size/2.0),
                            triangle_center + egui::Vec2::new(-triangle_size/2.0, triangle_size/2.0),
                            triangle_center + egui::Vec2::new(triangle_size/2.0, 0.0),
                        ];

                        ui.painter().add(egui::Shape::convex_polygon(
                            points.to_vec(),
                            egui::Color32::from_rgb(100, 150, 255),
                            egui::Stroke::NONE
                        ));
                    }

                    let mut card = PlayerCard::new(player, hand_size, has_dragon);
                    if is_current {
                        card = card.current_player();
                    }
                    if is_winner {
                        card = card.winner();
                    }
                    ui.add(card);
                });
            }

            ui.add_space(20.0);
            ui.separator();
        });

        // Hand section
        let hand = game.curr_player_hand().clone();
        ui.add(HandRenderer::new(hand, sender.clone()));
    });
}