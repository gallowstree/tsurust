//! End-to-end UI tests: drive the real `TemplateApp` headlessly through
//! `egui_kittest`, talking to the real WebSocket server embedded in-process.
//!
//! These cover what unit tests can't: the full client → socket → server →
//! broadcast → client loop, driven through actual widget interactions.
#![cfg(not(target_arch = "wasm32"))]

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;

use client_egui::TemplateApp;

/// Generous ceiling for localhost round-trips; each wait loop bails out with
/// a descriptive panic if the condition never becomes true.
const NET_TIMEOUT: Duration = Duration::from_secs(10);

fn new_app() -> Harness<'static, TemplateApp> {
    new_app_with_ctx().0
}

/// Also hands back the app's `egui::Context`, for tests that need to observe
/// repaint requests without driving frames.
fn new_app_with_ctx() -> (Harness<'static, TemplateApp>, egui::Context) {
    let ctx_slot = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = std::rc::Rc::clone(&ctx_slot);
    let harness = Harness::builder()
        .with_size(egui::Vec2::new(1400.0, 900.0))
        // Grids (forms, the lobby browser) can need several frames to settle
        // their column widths when content arrives mid-run; the default
        // max_steps of 4 makes run() flaky when that coincides with a
        // server response.
        .with_max_steps(64)
        .build_eframe(move |cc| {
            *slot.borrow_mut() = Some(cc.egui_ctx.clone());
            TemplateApp::new(cc)
        });
    let ctx = ctx_slot
        .borrow_mut()
        .take()
        .expect("the eframe creation closure should have run");
    (harness, ctx)
}

/// Start one shared in-process server for the whole test binary and point
/// WS_SERVER_URL at it. Rooms are independent, so tests can share a server;
/// per-test servers would race on the process-wide env var when tests run
/// in parallel.
fn ensure_server() {
    static SERVER_URL: OnceLock<String> = OnceLock::new();
    SERVER_URL.get_or_init(|| {
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
        // Keep the server alive for the rest of the test run.
        std::mem::forget(rt);

        let url = format!("ws://{}", addr);
        std::env::set_var("WS_SERVER_URL", &url);
        url
    });
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

/// Drive a fresh client through the create-room form; returns the room id
/// the server assigned.
fn create_room(a: &mut Harness<'_, TemplateApp>) -> String {
    a.run();
    a.get_by_label_contains("Create Online Lobby").click();
    a.run();
    // The form is prefilled ("Test Lobby" / "Player 1"); just submit it.
    a.get_by_label_contains("Create & Join").click();

    wait_for(a, "the server to confirm room creation", |app| {
        app.current_room_id().map(str::to_string)
    })
}

/// On the create form: replace the prefilled lobby name with a unique one.
/// All e2e tests share one in-process server, so browser-facing tests need
/// names that can't collide with rooms from concurrently running tests.
fn set_lobby_name(h: &mut Harness<'_, TemplateApp>, name: &str) {
    h.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.value().unwrap_or_default() == "Test Lobby")
        .expect("the create form should show the prefilled lobby name")
        .focus();
    h.run();
    h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::A]);
    h.event(egui::Event::Text(name.to_string()));
    h.run();
}

/// Drive a fresh client through the join form into the given room.
fn join_room(b: &mut Harness<'_, TemplateApp>, room_id: &str) {
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
    b.event(egui::Event::Text(room_id.to_string()));
    b.run();
    b.get_by_label("Join by code").click();

    wait_for(b, "the server to confirm the join", |app| {
        app.current_room_id().map(|_| ())
    });
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

/// The main menu exposes an editable Server field seeded with the resolved
/// WebSocket URL — the entry point for pasting a host's wss:// address (or
/// arriving via a `?server=` invite link). This is the only text input on the
/// menu, and it must be bound to the app's `server_url`.
#[test]
fn main_menu_shows_a_server_field_bound_to_the_resolved_url() {
    let mut harness = new_app();
    harness.run();

    let value = harness
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .find_map(|n| n.value())
        .expect("the main menu should have a Server text field");
    assert!(
        value.starts_with("ws"),
        "server field should be seeded with a ws(s):// URL, got {value:?}"
    );
}

#[test]
fn two_clients_create_join_and_play_a_turn_over_a_real_server() {
    ensure_server();

    // --- Client A creates a room, client B joins it, all through the UI ---
    let mut a = new_app();
    let room_id = create_room(&mut a);
    let mut b = new_app();
    join_room(&mut b, &room_id);

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

    // The online lobby shows the connection status chip.
    a.get_by_label_contains("Connected");
    b.get_by_label_contains("Connected");

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

    // --- Redaction: neither client can read the other's tiles off the wire ---
    let a_game = a.state().visible_game().expect("A should be in the game");
    assert!(
        !a_game.hands[&1].is_empty() && a_game.hands[&2].is_empty(),
        "A sees only her own hand; B's tiles are hidden"
    );
    let b_game = b.state().visible_game().expect("B should be in the game");
    assert!(
        !b_game.hands[&2].is_empty() && b_game.hands[&1].is_empty(),
        "B sees only his own hand; A's tiles are hidden"
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

/// A rejected join must not strand the user: the error surfaces in the toast
/// and the app stays on the join form, re-enabled for another attempt.
#[test]
fn joining_a_missing_room_shows_the_error_and_keeps_the_form() {
    ensure_server();

    let mut b = new_app();
    b.run();
    b.get_by_label_contains("Join Online Lobby").click();
    b.run();
    b.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|node| node.value().unwrap_or_default().is_empty())
        .expect("the join form should show an empty lobby-id field")
        .focus();
    b.run();
    b.event(egui::Event::Text("ZZZZ".to_string()));
    b.run();
    b.get_by_label("Join by code").click();

    // The server's rejection lands in the error toast.
    let deadline = Instant::now() + NET_TIMEOUT;
    while b.query_by_label_contains("not found").is_none() {
        b.run_steps(2);
        assert!(
            Instant::now() < deadline,
            "the server's rejection should surface in the error toast"
        );
        std::thread::sleep(Duration::from_millis(10));
    }

    // Still on the join form — no phantom lobby, no room membership — and the
    // form is present for a retry.
    assert!(
        b.state().visible_lobby().is_none(),
        "a failed join must not open a lobby"
    );
    assert!(b.state().current_room_id().is_none());
    b.get_by_label("Join by code");
}

/// The lobby browser: a public room shows up in the join screen's directory
/// and can be joined with a click — no room code involved — while a private
/// room created on the same server never appears there.
#[test]
fn public_lobbies_are_browsable_and_private_ones_are_not() {
    ensure_server();

    // One public room…
    let mut a = new_app();
    a.run();
    a.get_by_label_contains("Create Online Lobby").click();
    a.run();
    set_lobby_name(&mut a, "Beacon Table");
    // The create form defaults to public; just submit.
    a.get_by_label_contains("Create & Join").click();
    let beacon_id = wait_for(&mut a, "public room creation", |app| {
        app.current_room_id().map(str::to_string)
    });

    // …and one private room on the same server.
    let mut p = new_app();
    p.run();
    p.get_by_label_contains("Create Online Lobby").click();
    p.run();
    set_lobby_name(&mut p, "Hidden Table");
    p.get_by_label_contains("Public lobby").click(); // untick: make it private
    p.get_by_label_contains("Create & Join").click();
    wait_for(&mut p, "private room creation", |app| {
        app.current_room_id().map(str::to_string)
    });

    // A fresh client browses the directory.
    let mut b = new_app();
    b.run();
    b.get_by_label_contains("Join Online Lobby").click();

    let deadline = Instant::now() + NET_TIMEOUT;
    while b.query_by_label("Beacon Table").is_none() {
        b.run_steps(2);
        assert!(
            Instant::now() < deadline,
            "the public room should appear in the lobby browser"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        b.query_by_label("Hidden Table").is_none(),
        "a private room must never appear in the browser"
    );

    // Join the public room with a click — no code was ever typed.
    b.get_by_label("Join Beacon Table").click();
    wait_for(&mut b, "the click-join to be confirmed", |app| {
        (app.current_room_id() == Some(beacon_id.as_str())).then_some(())
    });
    wait_for(&mut b, "the joined lobby to arrive", |app| {
        app.visible_lobby()
            .is_some_and(|l| l.players.len() == 2)
            .then_some(())
    });
}

/// Spectating from the browser: an in-progress public game can be watched
/// live. The spectator has no hand, no identity, and sees moves as they land.
#[test]
fn a_spectator_can_watch_a_public_game_from_the_browser() {
    ensure_server();

    // Two players start a public game.
    let mut a = new_app();
    a.run();
    a.get_by_label_contains("Create Online Lobby").click();
    a.run();
    set_lobby_name(&mut a, "Arena Match");
    a.get_by_label_contains("Create & Join").click();
    let room_id = wait_for(&mut a, "room creation", |app| {
        app.current_room_id().map(str::to_string)
    });
    let mut b = new_app();
    join_room(&mut b, &room_id);
    wait_for_both(&mut a, &mut b, "both players in the lobby", |a, b| {
        a.visible_lobby().is_some_and(|l| l.players.len() == 2)
            && b.visible_lobby().is_some_and(|l| l.players.len() == 2)
    });
    a.get_by_label("spawn r0c2e4").click();
    b.get_by_label("spawn r5c3e0").click();
    wait_for_both(&mut a, &mut b, "both pawns placed", |a, b| {
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
    });
    a.get_by_label_contains("Start Game").click();
    wait_for_both(&mut a, &mut b, "the game to start", |a, b| {
        a.visible_game().is_some() && b.visible_game().is_some()
    });

    // A third client finds the game in the browser and spectates it.
    let mut c = new_app();
    c.run();
    c.get_by_label_contains("Join Online Lobby").click();
    let deadline = Instant::now() + NET_TIMEOUT;
    while c.query_by_label("Arena Match").is_none() {
        c.run_steps(2);
        assert!(
            Instant::now() < deadline,
            "the in-progress game should appear in the browser"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    c.get_by_label("Spectate Arena Match").click();
    wait_for(&mut c, "the spectator to receive the game", |app| {
        app.visible_game().map(|_| ())
    });

    // Spectators are nobody: no hand is shown and no player is "(You)".
    assert_eq!(c.state().client_player_id(), 0);
    assert!(
        c.query_by_label("hand tile 0").is_none(),
        "spectators must not see a hand"
    );
    c.get_by_label("👁 Spectating");

    // A move by the current player reaches the spectator live.
    a.get_by_label("hand tile 0").click();
    let deadline = Instant::now() + NET_TIMEOUT;
    loop {
        a.run_steps(2);
        c.run_steps(2);
        if c.state()
            .visible_game()
            .is_some_and(|g| g.board.history.len() == 1)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the spectator should see the move arrive"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The event-driven repaint contract: a client that is not rendering any
/// frames must still be woken (via `Context::request_repaint` from the
/// WebSocket thread) when the server broadcasts something. This is what the
/// `connect_with_wakeup` change replaced per-frame polling with — if it
/// breaks, an idle client's UI silently stops reflecting other players'
/// actions until the user wiggles the mouse.
#[test]
fn a_server_broadcast_wakes_an_idle_client() {
    ensure_server();

    let mut a = new_app();
    let room_id = create_room(&mut a);
    let (mut b, b_ctx) = new_app_with_ctx();
    join_room(&mut b, &room_id);

    wait_for_both(
        &mut a,
        &mut b,
        "both lobbies to show two players",
        |a, b| {
            a.visible_lobby().is_some_and(|l| l.players.len() == 2)
                && b.visible_lobby().is_some_and(|l| l.players.len() == 2)
        },
    );

    // Let B settle into a truly idle lobby: no frames driven from here on,
    // and no repaint request pending. (The lobby screen has no animations,
    // so it does settle — unlike the game screen's border glow.)
    let deadline = Instant::now() + NET_TIMEOUT;
    while b_ctx.has_requested_repaint() {
        b.run_steps(1);
        assert!(
            Instant::now() < deadline,
            "B should settle into an idle lobby with no repaint pending"
        );
    }

    // A places a pawn. The resulting broadcast must wake idle B: its socket
    // thread requests a repaint even though nobody is driving B's frames.
    a.get_by_label("spawn r0c2e4").click();
    let deadline = Instant::now() + NET_TIMEOUT;
    while !b_ctx.has_requested_repaint() {
        a.run_steps(1);
        assert!(
            Instant::now() < deadline,
            "the pawn broadcast should request a repaint on idle B"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // And the frame that the wakeup triggers actually drains the message.
    b.run_steps(2);
    assert!(
        b.state()
            .visible_lobby()
            .and_then(|l| l.players.get(&1))
            .is_some_and(|p| p.spawn_position.is_some()),
        "the woken frame should show A's pawn on B's lobby board"
    );
}
