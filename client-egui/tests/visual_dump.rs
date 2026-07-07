//! Renders each screen of the real app to PNG files for manual visual
//! inspection (e.g. after an egui upgrade). Ignored by default because it
//! needs a GPU; run explicitly with:
//!
//!   cargo test -p client-egui --test visual_dump -- --ignored
//!
//! Images land in `target/ui-renders/` (override with TSURUST_RENDER_DIR).
#![cfg(not(target_arch = "wasm32"))]

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use client_egui::TemplateApp;

#[test]
#[ignore = "renders PNGs for manual inspection; needs a GPU"]
fn dump_screen_renders() {
    let dir =
        std::env::var("TSURUST_RENDER_DIR").unwrap_or_else(|_| "target/ui-renders".to_string());
    std::fs::create_dir_all(&dir).expect("render output directory should be creatable");

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_eframe(|cc| TemplateApp::new(cc));

    let save = |harness: &mut Harness<'_, TemplateApp>, name: &str| {
        let image = harness
            .render()
            .expect("headless wgpu rendering should succeed");
        let path = format!("{dir}/{name}.png");
        image.save(&path).expect("PNG should be writable");
        println!("wrote {path}");
    };

    // Main menu
    harness.run();
    save(&mut harness, "01-main-menu");

    // Create-lobby form
    harness.get_by_label_contains("Create Online Lobby").click();
    harness.run();
    save(&mut harness, "02-create-lobby-form");
    harness.get_by_label("Back").click();
    harness.run();

    // Join form
    harness.get_by_label_contains("Join Online Lobby").click();
    harness.run();
    save(&mut harness, "03-join-lobby-form");
    harness.get_by_label("Back").click();
    harness.run();

    // Local lobby, then place a pawn and add a second (debug) player
    harness.get_by_label_contains("Local Game").click();
    harness.run();
    save(&mut harness, "04-local-lobby");

    harness.get_by_label("spawn r0c2e4").click();
    harness.run();
    harness.get_by_label_contains("Add Test Player").click();
    harness.run();
    save(&mut harness, "05-lobby-placing-second-player");

    harness.get_by_label("spawn r5c3e0").click();
    harness.run();

    // Start the game. From here on the board's border glow animates every
    // frame, so use run_steps instead of run().
    harness.get_by_label_contains("Start Game").click();
    harness.run_steps(5);
    save(&mut harness, "06-game-empty-board");

    // Rotate a hand tile (shows the hover overlay + rotation), then place it.
    let tile_rect = harness.get_by_label("hand tile 0").rect();
    harness.hover_at(egui::pos2(
        tile_rect.right() - tile_rect.width() * 0.15,
        tile_rect.center().y,
    ));
    harness.run_steps(2);
    save(&mut harness, "07-hand-tile-hover");

    harness.get_by_label("hand tile 0").click();
    // Let placement/movement animations play out (~1s of frames).
    harness.run_steps(90);
    save(&mut harness, "08-game-after-first-move");

    // A second move for a fuller board.
    harness.get_by_label("hand tile 0").click();
    harness.run_steps(90);
    save(&mut harness, "09-game-after-second-move");
}
