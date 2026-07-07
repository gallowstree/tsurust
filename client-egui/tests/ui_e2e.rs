//! End-to-end UI tests: drive the real `TemplateApp` headlessly through
//! `egui_kittest`, talking to the real WebSocket server embedded in-process.
//!
//! These cover what unit tests can't: the full client → socket → server →
//! broadcast → client loop, driven through actual widget interactions.
#![cfg(not(target_arch = "wasm32"))]

use std::time::{Duration, Instant};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use client_egui::TemplateApp;

/// Generous ceiling for localhost round-trips; each wait loop bails out with
/// a descriptive panic if the condition never becomes true.
const NET_TIMEOUT: Duration = Duration::from_secs(10);

fn new_app() -> Harness<'static, TemplateApp> {
    Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        .build_eframe(|cc| TemplateApp::new(cc))
}

/// Bind the real server to an ephemeral port inside this process. The
/// returned runtime must stay alive for the duration of the test.
fn start_server() -> (tokio::runtime::Runtime, String) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime should build");
    let listener = rt
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .expect("an ephemeral port should be bindable");
    let addr = listener
        .local_addr()
        .expect("bound listener should have an address");
    rt.spawn(tsurust_server::serve(listener, Duration::from_secs(300)));
    (rt, format!("ws://{}", addr))
}

/// Advance frames until `read` yields a value. Uses `run_steps` rather than
/// `run()` because the board's border-glow animation requests a repaint every
/// frame, so `run()` would never settle on a game screen.
fn wait_for<T>(
    harness: &mut Harness<'_, TemplateApp>,
    what: &str,
    read: impl Fn(&TemplateApp) -> Option<T>,
) -> T {
    let deadline = Instant::now() + NET_TIMEOUT;
    loop {
        harness.run_steps(2);
        if let Some(value) = read(harness.state()) {
            return value;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Advance both clients until `pred` holds across them (e.g. a broadcast has
/// reached everyone).
fn wait_for_both(
    a: &mut Harness<'_, TemplateApp>,
    b: &mut Harness<'_, TemplateApp>,
    what: &str,
    pred: impl Fn(&TemplateApp, &TemplateApp) -> bool,
) {
    let deadline = Instant::now() + NET_TIMEOUT;
    loop {
        a.run_steps(2);
        b.run_steps(2);
        if pred(a.state(), b.state()) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Click at an absolute position. Needed for widgets where the click location
/// matters within the widget (a TileButton rotates on its left/right thirds).
fn click_at(harness: &mut Harness<'_, TemplateApp>, pos: egui::Pos2) {
    harness.event(egui::Event::PointerMoved(pos));
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run_steps(4);
}

#[test]
fn main_menu_boots_and_opens_a_local_lobby() {
    let mut harness = new_app();
    harness.run();

    harness.get_by_label_contains("Local Game").click();
    harness.run();

    harness.get_by_label_contains("Select your starting position");
    assert!(
        harness.state().visible_lobby().is_some(),
        "clicking Local Game should open a lobby"
    );
    // The lobby board exposes labeled spawn spots.
    harness.get_by_label("spawn r0c0e5");
}

#[test]
fn two_clients_create_join_and_play_a_turn_over_a_real_server() {
    let (_rt, ws_url) = start_server();
    std::env::set_var("WS_SERVER_URL", &ws_url);

    // --- Client A creates a room through the UI ---
    let mut a = new_app();
    a.run();
    a.get_by_label_contains("Create Online Lobby").click();
    a.run();
    // The form is prefilled ("Test Lobby" / "Player 1"); just submit it.
    a.get_by_label_contains("Create & Join").click();

    let room_id = wait_for(&mut a, "the server to confirm room creation", |app| {
        app.current_room_id().map(str::to_string)
    });

    // --- Client B joins that room through the UI ---
    let mut b = new_app();
    b.run();
    b.get_by_label_contains("Join Online Lobby").click();
    b.run();
    // Focus the (empty) lobby-id field, then type into it: text events go to
    // the focused widget, and kittest's type_text doesn't focus by itself.
    b.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|node| node.value().unwrap_or_default().is_empty())
        .expect("the join form should show an empty lobby-id field")
        .focus();
    b.run();
    b.event(egui::Event::Text(room_id.clone()));
    b.run();
    b.get_by_label("Join").click();

    wait_for(&mut b, "the server to confirm the join", |app| {
        app.current_room_id().map(|_| ())
    });
    assert_eq!(a.state().client_player_id(), 1);
    assert_eq!(b.state().client_player_id(), 2);

    // A must also see B arrive (broadcast round-trip).
    wait_for_both(
        &mut a,
        &mut b,
        "both lobbies to show two players",
        |a, b| {
            a.visible_lobby().is_some_and(|l| l.players.len() == 2)
                && b.visible_lobby().is_some_and(|l| l.players.len() == 2)
        },
    );

    // --- Both players place pawns by clicking spawn spots on the board ---
    a.get_by_label("spawn r0c2e4").click();
    wait_for(&mut a, "A's pawn placement to be confirmed", |app| {
        app.visible_lobby()
            .and_then(|l| l.players.get(&1))
            .and_then(|p| p.spawn_position)
            .map(|_| ())
    });

    b.get_by_label("spawn r5c3e0").click();
    wait_for_both(
        &mut a,
        &mut b,
        "both pawns to be visible on both clients",
        |a, b| {
            let placed = |app: &TemplateApp| {
                app.visible_lobby().is_some_and(|l| {
                    l.players
                        .values()
                        .filter(|p| p.spawn_position.is_some())
                        .count()
                        == 2
                })
            };
            placed(a) && placed(b)
        },
    );

    // --- A starts the game ---
    a.get_by_label_contains("Start Game").click();
    wait_for_both(&mut a, &mut b, "both clients to enter the game", |a, b| {
        a.visible_game().is_some() && b.visible_game().is_some()
    });
    assert_eq!(
        a.state()
            .visible_game()
            .expect("A should be in the game")
            .current_player_id,
        1,
        "the creator should have the first turn"
    );

    // --- B rotates a hand tile locally (presentation-only planning) ---
    // Pick a rotation-asymmetric tile so the survival assertion below can't
    // pass by accident.
    let b_hand = b
        .state()
        .visible_game()
        .expect("B should be in the game")
        .hands[&2]
        .clone();
    let (slot, tile_before) = b_hand
        .iter()
        .copied()
        .enumerate()
        .find(|(_, t)| t.rotated(true) != *t)
        .expect("a hand of three distinct Tsuro tiles should contain an asymmetric one");

    let tile_rect = b.get_by_label(&format!("hand tile {slot}")).rect();
    // Right third of the tile button = rotate clockwise.
    click_at(
        &mut b,
        egui::pos2(
            tile_rect.right() - tile_rect.width() * 0.15,
            tile_rect.center().y,
        ),
    );

    let rotated = tile_before.rotated(true);
    assert_eq!(
        b.state().visible_game().expect("B stays in game").hands[&2][slot],
        rotated,
        "clicking the right third of a hand tile should rotate it clockwise"
    );

    // --- A places a tile; the move must reach both clients ---
    a.get_by_label("hand tile 0").click();
    wait_for_both(&mut a, &mut b, "the move to reach both clients", |a, b| {
        let placed = |app: &TemplateApp| {
            app.visible_game()
                .is_some_and(|g| g.board.history.len() == 1)
        };
        placed(a) && placed(b)
    });

    // A's random opening tile has two legitimate outcomes: A survives and the
    // turn passes to B, or A's own tile carries it off the board edge — and
    // with two players that elimination ends the game with B the winner
    // (complete_turn returns PlayerWins without advancing the turn pointer).
    let game = b.state().visible_game().expect("B in game");
    let a_alive = game
        .players
        .iter()
        .find(|p| p.id == 1)
        .expect("player 1 should be in the game")
        .alive;
    if a_alive {
        assert_eq!(
            game.current_player_id, 2,
            "A survived its move, so it should be B's turn"
        );
    } else {
        assert!(
            game.is_game_over(),
            "with two players, A eliminating itself should end the game"
        );
    }

    // The key regression guard: B's local rotation survived the authoritative
    // GameStateUpdate that A's move broadcast to everyone.
    assert_eq!(
        b.state().visible_game().expect("B in game").hands[&2][slot],
        rotated,
        "B's local tile rotation should survive the server's game state update"
    );
}
