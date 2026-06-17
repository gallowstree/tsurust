/// End-to-end integration tests for the Tsurust WebSocket server.
///
/// Each test spins up a real server on an OS-assigned port, connects real
/// WebSocket clients, and exchanges actual protocol messages.
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use tsurust_common::board::{CellCoord, Move, PlayerID, PlayerPos};
use tsurust_common::game::Game;
use tsurust_common::protocol::{ClientMessage, RoomId, ServerMessage};

use crate::handler::handle_connection;
use crate::server::GameServer;

// ============================================================
// Test helpers
// ============================================================

type ClientWs = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// Start a server on a random port and return its address + the server handle.
async fn start_test_server() -> (SocketAddr, Arc<GameServer>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind test server");
    let addr = listener.local_addr().expect("Failed to get local addr");
    let server = Arc::new(GameServer::new());
    let server_clone = Arc::clone(&server);

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let server = Arc::clone(&server_clone);
            tokio::spawn(async move {
                match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => {
                        let conn_id = server.next_connection_id().await;
                        handle_connection(ws, conn_id, server).await;
                    }
                    Err(e) => eprintln!("[test server] accept error: {}", e),
                }
            });
        }
    });

    (addr, server)
}

/// Connect a WebSocket client to the test server.
async fn connect_client(addr: SocketAddr) -> ClientWs {
    let url = format!("ws://{}", addr);
    let (ws, _) = connect_async(url).await.expect("Failed to connect client");
    ws
}

/// Send a ClientMessage.
async fn send(ws: &mut ClientWs, msg: ClientMessage) {
    let json = serde_json::to_string(&msg).expect("Failed to serialize ClientMessage");
    ws.send(Message::Text(json.into()))
        .await
        .expect("Failed to send message");
}

/// Receive the next ServerMessage, transparently responding to pings.
async fn recv(ws: &mut ClientWs) -> ServerMessage {
    loop {
        match ws.next().await.expect("WS closed unexpectedly") {
            Ok(Message::Text(text)) => {
                return serde_json::from_str(&text).expect("Failed to deserialize ServerMessage");
            }
            Ok(Message::Ping(data)) => {
                ws.send(Message::Pong(data))
                    .await
                    .expect("Failed to send pong");
            }
            Ok(_) => continue,
            Err(e) => panic!("WebSocket error: {}", e),
        }
    }
}

/// Receive the next ServerMessage that matches a predicate, discarding others.
async fn recv_where(ws: &mut ClientWs, pred: impl Fn(&ServerMessage) -> bool) -> ServerMessage {
    loop {
        let msg = recv(ws).await;
        if pred(&msg) {
            return msg;
        }
    }
}

/// Drive two clients through the full create → join → place-pawns → start flow.
/// Returns the two connections, the room id, both player ids, and the started
/// game state (as broadcast in GameStarted). Both clients' GameStarted messages
/// are drained before returning.
async fn setup_two_player_game(
    addr: SocketAddr,
) -> (ClientWs, ClientWs, RoomId, PlayerID, PlayerID, Game) {
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated {
        room_id,
        player_id: alice_id,
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!("Expected RoomCreated");
    };

    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await
    else {
        panic!("Expected PlayerJoined");
    };

    send(
        &mut alice,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: alice_id,
            position: PlayerPos::new(0, 2, 5),
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PawnPlaced { .. })
    })
    .await;
    send(
        &mut bob,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: bob_id,
            position: PlayerPos::new(5, 3, 0),
        },
    )
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::PawnPlaced { .. })).await;

    send(
        &mut alice,
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;
    let ServerMessage::GameStarted { game, .. } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStarted { .. })
    })
    .await
    else {
        panic!("Expected GameStarted");
    };
    recv_where(&mut bob, |m| matches!(m, ServerMessage::GameStarted { .. })).await;

    (alice, bob, room_id, alice_id, bob_id, game)
}

// ============================================================
// Tests
// ============================================================

/// Creating a room produces RoomCreated with player_id=1 and a 4-char room_id.
#[tokio::test]
async fn test_create_room_over_websocket() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;

    // Expect LobbyStateUpdate then RoomCreated (order may vary)
    let msg = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await;

    let ServerMessage::RoomCreated { room_id, player_id } = msg else {
        panic!("Expected RoomCreated");
    };
    assert_eq!(player_id, 1, "Creator should be player 1");
    assert_eq!(room_id.len(), 4, "Room ID should be 4 characters");
}

/// A second client joining triggers PlayerJoined and LobbyStateUpdate on both sides.
#[tokio::test]
async fn test_join_room_notifies_both_clients() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;

    let ServerMessage::RoomCreated { room_id, .. } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!("Expected RoomCreated");
    };

    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;

    // Bob receives PlayerJoined confirmation
    let bob_msg = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id,
        player_name,
        ..
    } = bob_msg
    else {
        panic!("Expected PlayerJoined");
    };
    assert_eq!(bob_id, 2, "Second player should be ID 2");
    assert_eq!(player_name, "Bob");

    // Alice receives the PlayerJoined broadcast
    let alice_msg = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await;
    let ServerMessage::PlayerJoined { player_id, .. } = alice_msg else {
        panic!("Expected PlayerJoined broadcast on Alice's connection");
    };
    assert_eq!(player_id, 2);
}

/// Full lobby → game flow: create, join, place pawns, start game.
#[tokio::test]
async fn test_full_lobby_to_game_flow() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    // === Create room ===
    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Integration Test".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated {
        room_id,
        player_id: alice_id,
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!("Expected RoomCreated");
    };

    // === Join room ===
    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await
    else {
        panic!("Expected PlayerJoined");
    };

    // === Place pawns ===
    // Alice on top edge, Bob on bottom edge
    let alice_pos = PlayerPos::new(0, 2, 5); // top edge
    let bob_pos = PlayerPos::new(5, 3, 0); // bottom edge

    send(
        &mut alice,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: alice_id,
            position: alice_pos,
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PawnPlaced { .. })
    })
    .await;

    send(
        &mut bob,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: bob_id,
            position: bob_pos,
        },
    )
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::PawnPlaced { .. })).await;

    // === Start game ===
    send(
        &mut alice,
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;

    let alice_start = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStarted { .. })
    })
    .await;
    let bob_start = recv_where(&mut bob, |m| matches!(m, ServerMessage::GameStarted { .. })).await;

    let ServerMessage::GameStarted {
        game: alice_game, ..
    } = alice_start
    else {
        panic!("Expected GameStarted on Alice");
    };
    let ServerMessage::GameStarted { game: bob_game, .. } = bob_start else {
        panic!("Expected GameStarted on Bob");
    };

    assert_eq!(alice_game.players.len(), 2, "Game should have 2 players");
    assert_eq!(bob_game.players.len(), 2, "Game should have 2 players");
    assert_eq!(
        alice_game.current_player_id, alice_id,
        "Alice (player 1) should go first"
    );
}

/// After a game starts, the current player can place a tile and both clients
/// receive a GameStateUpdate with the next player's turn.
#[tokio::test]
async fn test_tile_placement_broadcasts_state_update() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    // Setup: create, join, place pawns, start
    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Tile Test".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated {
        room_id,
        player_id: alice_id,
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!();
    };

    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await
    else {
        panic!();
    };

    send(
        &mut alice,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: alice_id,
            position: PlayerPos::new(0, 2, 5),
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PawnPlaced { .. })
    })
    .await;
    send(
        &mut bob,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: bob_id,
            position: PlayerPos::new(5, 3, 0),
        },
    )
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::PawnPlaced { .. })).await;
    send(
        &mut alice,
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;

    let ServerMessage::GameStarted { game, .. } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStarted { .. })
    })
    .await
    else {
        panic!();
    };
    // Drain Bob's GameStarted too
    recv_where(&mut bob, |m| matches!(m, ServerMessage::GameStarted { .. })).await;

    // === Place a tile ===
    // Use whatever tile is actually in Alice's hand (first tile)
    let tile = game
        .hands
        .get(&alice_id)
        .and_then(|h| h.first())
        .copied()
        .expect("Alice should have tiles in hand");

    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov: Move {
                tile,
                cell: CellCoord { row: 0, col: 2 }, // Alice starts at (0,2)
                player_id: alice_id,
            },
        },
    )
    .await;

    // Both clients should receive a GameStateUpdate
    let alice_update = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await;
    let bob_update = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await;

    let ServerMessage::GameStateUpdate {
        state: alice_state, ..
    } = alice_update
    else {
        panic!()
    };
    let ServerMessage::GameStateUpdate {
        state: bob_state, ..
    } = bob_update
    else {
        panic!()
    };

    // After Alice's turn, it should be Bob's turn
    assert_eq!(
        alice_state.current_player_id, bob_id,
        "After Alice places, it should be Bob's turn"
    );
    assert_eq!(alice_state.current_player_id, bob_state.current_player_id);
}

/// Placing a tile out of turn returns an Error message (not a state update).
#[tokio::test]
async fn test_out_of_turn_placement_returns_error() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Turn Test".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated {
        room_id,
        player_id: alice_id,
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!()
    };

    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await
    else {
        panic!()
    };

    send(
        &mut alice,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: alice_id,
            position: PlayerPos::new(0, 2, 5),
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PawnPlaced { .. })
    })
    .await;
    send(
        &mut bob,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: bob_id,
            position: PlayerPos::new(5, 3, 0),
        },
    )
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::PawnPlaced { .. })).await;
    send(
        &mut alice,
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;

    let ServerMessage::GameStarted { game, .. } =
        recv_where(&mut bob, |m| matches!(m, ServerMessage::GameStarted { .. })).await
    else {
        panic!()
    };
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStarted { .. })
    })
    .await;

    // Bob tries to place a tile when it's Alice's turn
    let tile = game
        .hands
        .get(&bob_id)
        .and_then(|h| h.first())
        .copied()
        .expect("Bob should have tiles");

    send(
        &mut bob,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: bob_id,
            mov: Move {
                tile,
                cell: CellCoord { row: 5, col: 3 },
                player_id: bob_id,
            },
        },
    )
    .await;

    let msg = recv_where(&mut bob, |m| matches!(m, ServerMessage::Error { .. })).await;
    let ServerMessage::Error { message } = msg else {
        panic!()
    };
    assert!(
        message.contains("Not your turn"),
        "Error should say 'Not your turn', got: {}",
        message
    );
}

/// When a player disconnects mid-game, the server eliminates them and advances the turn.
#[tokio::test]
async fn test_disconnect_eliminates_player_and_advances_turn() {
    let (addr, server) = start_test_server().await;
    let mut alice = connect_client(addr).await;
    let mut bob = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Disconnect Test".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated {
        room_id,
        player_id: alice_id,
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!()
    };

    send(
        &mut bob,
        ClientMessage::JoinRoom {
            room_id: room_id.clone(),
            player_name: "Bob".to_string(),
        },
    )
    .await;
    let ServerMessage::PlayerJoined {
        player_id: bob_id, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::PlayerJoined { .. })
    })
    .await
    else {
        panic!()
    };

    send(
        &mut alice,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: alice_id,
            position: PlayerPos::new(0, 2, 5),
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::PawnPlaced { .. })
    })
    .await;
    send(
        &mut bob,
        ClientMessage::PlacePawn {
            room_id: room_id.clone(),
            player_id: bob_id,
            position: PlayerPos::new(5, 3, 0),
        },
    )
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::PawnPlaced { .. })).await;
    send(
        &mut alice,
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;
    recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStarted { .. })
    })
    .await;
    recv_where(&mut bob, |m| matches!(m, ServerMessage::GameStarted { .. })).await;

    // Alice (player 1) disconnects mid-game
    drop(alice);

    // Give server time to process the disconnect
    sleep(Duration::from_millis(200)).await;

    // Verify server state: Alice should be eliminated
    let rooms = server.rooms.read().await;
    let room = rooms
        .get(&room_id)
        .expect("Room should still exist (Bob is alive)");
    let alice_player = room
        .game
        .players
        .iter()
        .find(|p| p.id == alice_id)
        .expect("Alice should still be in the player list");

    assert!(
        !alice_player.alive,
        "Alice should be marked as not alive after disconnect"
    );
    assert_eq!(
        room.game.current_player_id, bob_id,
        "Turn should have advanced to Bob after Alice disconnected"
    );
}

/// When the last player in a room disconnects, the room is removed.
#[tokio::test]
async fn test_last_player_disconnect_removes_room() {
    let (addr, server) = start_test_server().await;
    let mut alice = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Cleanup Test".to_string(),
            creator_name: "Alice".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated { room_id, .. } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!()
    };

    // Alice disconnects
    drop(alice);
    sleep(Duration::from_millis(200)).await;

    let rooms = server.rooms.read().await;
    assert!(
        !rooms.contains_key(&room_id),
        "Room should be removed after last player disconnects"
    );
}

/// Each player is dealt their own hand. Because tiles come from one shared deck
/// drawn without replacement, the two hands must be full (3 tiles) and disjoint.
/// Regression guard for the "both players see the same tiles" bug.
#[tokio::test]
async fn test_players_receive_distinct_disjoint_hands() {
    let (_alice, _bob, _room_id, alice_id, bob_id, game) =
        setup_two_player_game(start_test_server().await.0).await;

    let alice_hand = game.hands.get(&alice_id).expect("Alice should have a hand");
    let bob_hand = game.hands.get(&bob_id).expect("Bob should have a hand");

    assert_eq!(alice_hand.len(), 3, "Alice should hold 3 tiles");
    assert_eq!(bob_hand.len(), 3, "Bob should hold 3 tiles");

    for tile in alice_hand {
        assert!(
            !bob_hand.contains(tile),
            "Players must not share tiles: {:?} is in both hands",
            tile
        );
    }
}

/// After a tile is placed, both clients receive a GameStateUpdate carrying the
/// same authoritative state. Regression guard for client desync.
#[tokio::test]
async fn test_both_clients_observe_identical_state_after_move() {
    let (mut alice, mut bob, room_id, alice_id, _bob_id, game) =
        setup_two_player_game(start_test_server().await.0).await;

    let tile = game
        .hands
        .get(&alice_id)
        .and_then(|h| h.first())
        .copied()
        .expect("Alice should have a tile");

    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov: Move {
                tile,
                cell: CellCoord { row: 0, col: 2 },
                player_id: alice_id,
            },
        },
    )
    .await;

    let ServerMessage::GameStateUpdate {
        state: alice_state, ..
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await
    else {
        panic!()
    };
    let ServerMessage::GameStateUpdate {
        state: bob_state, ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await
    else {
        panic!()
    };

    assert_eq!(
        alice_state.board.history.len(),
        1,
        "exactly one tile placed"
    );
    assert_eq!(
        alice_state.current_player_id, bob_state.current_player_id,
        "both clients agree on whose turn it is"
    );
    assert_eq!(
        alice_state.board.history, bob_state.board.history,
        "both clients agree on the board"
    );
    assert_eq!(
        alice_state.hands, bob_state.hands,
        "both clients agree on all hands"
    );
    let positions = |g: &Game| {
        g.players
            .iter()
            .map(|p| (p.id, p.pos, p.alive))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        positions(&alice_state),
        positions(&bob_state),
        "both clients agree on all player positions"
    );
}

/// Two players take consecutive turns; the board history grows and both clients
/// stay in sync across turns. Regression guard for "failing to play a tile".
#[tokio::test]
async fn test_consecutive_turns_keep_clients_in_sync() {
    let (mut alice, mut bob, room_id, alice_id, bob_id, game) =
        setup_two_player_game(start_test_server().await.0).await;

    // === Alice's turn ===
    let alice_tile = game
        .hands
        .get(&alice_id)
        .and_then(|h| h.first())
        .copied()
        .expect("Alice should have a tile");
    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov: Move {
                tile: alice_tile,
                cell: CellCoord { row: 0, col: 2 },
                player_id: alice_id,
            },
        },
    )
    .await;
    let ServerMessage::GameStateUpdate {
        state: after_alice, ..
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await
    else {
        panic!()
    };
    recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await;
    assert_eq!(
        after_alice.current_player_id, bob_id,
        "turn should pass to Bob"
    );

    // === Bob's turn === (place at Bob's current cell, whatever it is now)
    let bob_player = after_alice
        .players
        .iter()
        .find(|p| p.id == bob_id)
        .expect("Bob should be in the state");
    assert!(bob_player.alive, "Bob should be alive for his turn");
    let bob_cell = bob_player.pos.cell;
    let bob_tile = after_alice
        .hands
        .get(&bob_id)
        .and_then(|h| h.first())
        .copied()
        .expect("Bob should have a tile");
    send(
        &mut bob,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: bob_id,
            mov: Move {
                tile: bob_tile,
                cell: bob_cell,
                player_id: bob_id,
            },
        },
    )
    .await;
    let ServerMessage::GameStateUpdate {
        state: after_bob_on_alice,
        ..
    } = recv_where(&mut alice, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await
    else {
        panic!()
    };
    let ServerMessage::GameStateUpdate {
        state: after_bob_on_bob,
        ..
    } = recv_where(&mut bob, |m| {
        matches!(m, ServerMessage::GameStateUpdate { .. })
    })
    .await
    else {
        panic!()
    };

    assert_eq!(
        after_bob_on_alice.board.history.len(),
        2,
        "two tiles placed across the two turns"
    );
    assert_eq!(
        after_bob_on_alice.board.history, after_bob_on_bob.board.history,
        "both clients agree on the board after two turns"
    );
    assert_eq!(
        after_bob_on_alice.hands, after_bob_on_bob.hands,
        "both clients agree on all hands after two turns"
    );
}
