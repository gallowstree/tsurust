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

use tsurust_common::board::{seg, CellCoord, Move, PlayerID, PlayerPos, Tile};
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

// Distinct, valid edge spawn positions — one per player (up to four).
const TOP: PlayerPos = PlayerPos {
    cell: CellCoord { row: 0, col: 2 },
    endpoint: 5,
};
const BOTTOM: PlayerPos = PlayerPos {
    cell: CellCoord { row: 5, col: 3 },
    endpoint: 0,
};
const LEFT: PlayerPos = PlayerPos {
    cell: CellCoord { row: 2, col: 0 },
    endpoint: 6,
};
const RIGHT: PlayerPos = PlayerPos {
    cell: CellCoord { row: 3, col: 5 },
    endpoint: 2,
};

/// Drive N clients through create → join → place-pawns → start. `positions`
/// supplies one distinct edge spawn per player, and its length is the player
/// count. Returns the connections (index `i` is `player_ids[i]`), the room id,
/// the player ids in join order, and the started game state. Each player's own
/// PawnPlaced is awaited so placement is committed before StartGame, and every
/// client is drained through GameStarted before returning.
async fn setup_n_player_game(
    addr: SocketAddr,
    positions: &[PlayerPos],
) -> (Vec<ClientWs>, RoomId, Vec<PlayerID>, Game) {
    assert!(positions.len() >= 2, "need at least two players");

    let mut clients: Vec<ClientWs> = Vec::with_capacity(positions.len());
    let mut player_ids: Vec<PlayerID> = Vec::with_capacity(positions.len());

    // Creator opens the room.
    let mut creator = connect_client(addr).await;
    send(
        &mut creator,
        ClientMessage::CreateRoom {
            room_name: "N-Player Room".to_string(),
            creator_name: "Player1".to_string(),
        },
    )
    .await;
    let ServerMessage::RoomCreated { room_id, player_id } = recv_where(&mut creator, |m| {
        matches!(m, ServerMessage::RoomCreated { .. })
    })
    .await
    else {
        panic!("Expected RoomCreated");
    };
    player_ids.push(player_id);
    clients.push(creator);

    // Remaining players join.
    for i in 1..positions.len() {
        let mut client = connect_client(addr).await;
        send(
            &mut client,
            ClientMessage::JoinRoom {
                room_id: room_id.clone(),
                player_name: format!("Player{}", i + 1),
            },
        )
        .await;
        let ServerMessage::PlayerJoined { player_id, .. } = recv_where(&mut client, |m| {
            matches!(m, ServerMessage::PlayerJoined { .. })
        })
        .await
        else {
            panic!("Expected PlayerJoined");
        };
        player_ids.push(player_id);
        clients.push(client);
    }

    // Every player places their pawn. Wait for each player's *own* PawnPlaced so
    // the placement is committed server-side before we try to start the game.
    for i in 0..positions.len() {
        let pid = player_ids[i];
        send(
            &mut clients[i],
            ClientMessage::PlacePawn {
                room_id: room_id.clone(),
                player_id: pid,
                position: positions[i],
            },
        )
        .await;
        recv_where(
            &mut clients[i],
            |m| matches!(m, ServerMessage::PawnPlaced { player_id, .. } if *player_id == pid),
        )
        .await;
    }

    // Creator starts the game; drain GameStarted (and all prior lobby traffic) on
    // every client so no stale broadcasts remain buffered.
    send(
        &mut clients[0],
        ClientMessage::StartGame {
            room_id: room_id.clone(),
        },
    )
    .await;
    let mut game = None;
    for client in clients.iter_mut() {
        let ServerMessage::GameStarted { game: g, .. } =
            recv_where(client, |m| matches!(m, ServerMessage::GameStarted { .. })).await
        else {
            panic!("Expected GameStarted");
        };
        game = Some(g);
    }

    (
        clients,
        room_id,
        player_ids,
        game.expect("at least one GameStarted"),
    )
}

/// The board cell a player currently occupies in the given game state.
fn player_cell(game: &Game, id: PlayerID) -> CellCoord {
    game.players
        .iter()
        .find(|p| p.id == id)
        .expect("player present in game")
        .pos
        .cell
}

/// The first tile in a player's hand.
fn first_hand_tile(game: &Game, id: PlayerID) -> Tile {
    game.hands
        .get(&id)
        .and_then(|hand| hand.first())
        .copied()
        .expect("player should have at least one tile")
}

/// Find a move for `player_id`, placed at their current cell, that keeps them on
/// the board (does not run them off an edge), trying every hand tile in all four
/// rotations. This makes "the placer survives their turn" deterministic despite
/// the randomly dealt hand — a raw first tile can send a player straight off the
/// edge. Panics if no surviving move exists (not expected from a fresh spawn).
fn find_surviving_move(game: &Game, player_id: PlayerID) -> Move {
    let start = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .expect("player present in game")
        .pos;
    let cell = start.cell;
    for tile in game.hands.get(&player_id).expect("player has a hand") {
        let mut candidate = *tile;
        for _ in 0..4 {
            let mut board = game.board.clone();
            board.place_tile(Move {
                tile: candidate,
                cell,
                player_id,
            });
            if !board.traverse_from(start).end_pos.on_edge() {
                return Move {
                    tile: candidate,
                    cell,
                    player_id,
                };
            }
            candidate = candidate.rotated(true);
        }
    }
    panic!("no surviving move found for player {player_id}");
}

/// Receive, from every client in order, the next GameStateUpdate whose state
/// satisfies `accept`. The predicate lets callers skip stale updates (e.g. an
/// earlier disconnect broadcast) and wait for the one they care about.
async fn collect_states(
    clients: &mut [ClientWs],
    accept: impl Fn(&Game) -> bool + Copy,
) -> Vec<Game> {
    let mut states = Vec::with_capacity(clients.len());
    for client in clients.iter_mut() {
        let state = loop {
            let ServerMessage::GameStateUpdate { state, .. } = recv_where(client, |m| {
                matches!(m, ServerMessage::GameStateUpdate { .. })
            })
            .await
            else {
                unreachable!("recv_where matched GameStateUpdate")
            };
            if accept(&state) {
                break state;
            }
        };
        states.push(state);
    }
    states
}

/// Assert every client's snapshot agrees on the authoritative game fields.
fn assert_states_agree(states: &[Game]) {
    let positions = |g: &Game| {
        g.players
            .iter()
            .map(|p| (p.id, p.pos, p.alive))
            .collect::<Vec<_>>()
    };
    let first = &states[0];
    for other in &states[1..] {
        assert_eq!(
            other.current_player_id, first.current_player_id,
            "clients disagree on current player"
        );
        assert_eq!(
            other.board.history, first.board.history,
            "clients disagree on board history"
        );
        assert_eq!(other.hands, first.hands, "clients disagree on hands");
        assert_eq!(
            positions(other),
            positions(first),
            "clients disagree on player positions"
        );
    }
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
    // Choose a tile that keeps Alice on the board, so the turn deterministically
    // advances to Bob (a random hand tile can run her off the top edge).
    let mov = find_surviving_move(&game, alice_id);

    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov,
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
    let game = room.game().expect("room should be in the playing phase");
    let alice_player = game
        .players
        .iter()
        .find(|p| p.id == alice_id)
        .expect("Alice should still be in the player list");

    assert!(
        !alice_player.alive,
        "Alice should be marked as not alive after disconnect"
    );
    assert_eq!(
        game.current_player_id, bob_id,
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

    // === Alice's turn === (pick a surviving tile so the turn passes to Bob)
    let alice_mov = find_surviving_move(&game, alice_id);
    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov: alice_mov,
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

/// Three players each receive a full hand of three tiles and no tile appears in
/// more than one hand. Extends the two-player disjoint-hands guard to three.
#[tokio::test]
async fn test_three_players_receive_distinct_disjoint_hands() {
    let (addr, _server) = start_test_server().await;
    let (_clients, _room_id, ids, game) = setup_n_player_game(addr, &[TOP, BOTTOM, LEFT]).await;

    for id in &ids {
        assert_eq!(
            game.hands.get(id).expect("player has a hand").len(),
            3,
            "player {id} should hold 3 tiles"
        );
    }
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = &game.hands[&ids[i]];
            let b = &game.hands[&ids[j]];
            for tile in a {
                assert!(
                    !b.contains(tile),
                    "players {} and {} share tile {:?}",
                    ids[i],
                    ids[j],
                    tile
                );
            }
        }
    }
}

/// In a three-player game, after the first player moves every client observes
/// the same authoritative state and the turn passes to the second player.
#[tokio::test]
async fn test_three_players_agree_on_state_after_move() {
    let (addr, _server) = start_test_server().await;
    let (mut clients, room_id, ids, game) = setup_n_player_game(addr, &[TOP, BOTTOM, LEFT]).await;

    let current = ids[0];
    let mov = Move {
        tile: first_hand_tile(&game, current),
        cell: player_cell(&game, current),
        player_id: current,
    };
    send(
        &mut clients[0],
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: current,
            mov,
        },
    )
    .await;

    let states = collect_states(&mut clients, |_| true).await;
    assert_states_agree(&states);
    assert_eq!(states[0].board.history.len(), 1, "exactly one tile placed");
    assert_eq!(
        states[0].current_player_id, ids[1],
        "turn should pass to the second player"
    );
}

/// Four players take one turn each in join order; the current player advances
/// 1 → 2 → 3 → 4 and all clients stay in sync at every step.
#[tokio::test]
async fn test_four_player_turn_order_cycles_through_players() {
    let (addr, _server) = start_test_server().await;
    let (mut clients, room_id, ids, game) =
        setup_n_player_game(addr, &[TOP, BOTTOM, LEFT, RIGHT]).await;

    let mut state = game;
    for turn in 0..ids.len() {
        let current = ids[turn];
        assert_eq!(
            state.current_player_id, current,
            "expected player {current} to be current on turn {turn}"
        );

        let mov = Move {
            tile: first_hand_tile(&state, current),
            cell: player_cell(&state, current),
            player_id: current,
        };
        send(
            &mut clients[turn],
            ClientMessage::PlaceTile {
                room_id: room_id.clone(),
                player_id: current,
                mov,
            },
        )
        .await;

        let states = collect_states(&mut clients, |_| true).await;
        assert_states_agree(&states);
        assert_eq!(
            states[0].board.history.len(),
            turn + 1,
            "one tile placed per turn"
        );
        state = states.into_iter().next().expect("a state per client");
    }

    assert_eq!(
        state.board.history.len(),
        4,
        "four tiles placed across four turns"
    );
}

/// When a player disconnects mid-game, the turn order skips the eliminated
/// player: with player 2 gone, play advances from player 1 straight to player 3.
#[tokio::test]
async fn test_disconnected_player_is_skipped_in_turn_order() {
    let (addr, server) = start_test_server().await;
    let (mut clients, room_id, ids, game) =
        setup_n_player_game(addr, &[TOP, BOTTOM, LEFT, RIGHT]).await;

    // Player 2 (not the current player) drops. `clients` is now [p1, p3, p4].
    let player_two = clients.remove(1);
    drop(player_two);
    sleep(Duration::from_millis(200)).await;

    {
        let rooms = server.rooms.read().await;
        let room = rooms.get(&room_id).expect("room should still exist");
        let p2 = room
            .game()
            .expect("room should be in the playing phase")
            .players
            .iter()
            .find(|p| p.id == ids[1])
            .expect("player 2 still listed");
        assert!(!p2.alive, "disconnected player 2 should be eliminated");
    }

    // Player 1 (still the current player) takes a turn.
    let current = ids[0];
    let mov = Move {
        tile: first_hand_tile(&game, current),
        cell: player_cell(&game, current),
        player_id: current,
    };
    send(
        &mut clients[0],
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: current,
            mov,
        },
    )
    .await;

    // Skip the disconnect's empty-history update; take the post-move one.
    let states = collect_states(&mut clients, |g| !g.board.history.is_empty()).await;
    assert_states_agree(&states);
    assert_eq!(
        states[0].current_player_id, ids[2],
        "turn should skip eliminated player 2 and land on player 3"
    );
    let p2 = states[0]
        .players
        .iter()
        .find(|p| p.id == ids[1])
        .expect("player 2 still listed");
    assert!(
        !p2.alive,
        "player 2 remains eliminated in the broadcast state"
    );
}

/// When the *current* player disconnects, the turn passes forward in rotation
/// order (player 2 out → player 3 up), not back to the first alive player, and
/// the disconnected player's hand is returned to the deck like any elimination.
#[tokio::test]
async fn test_current_player_disconnect_passes_turn_in_rotation_order() {
    let (addr, server) = start_test_server().await;
    let (mut clients, room_id, ids, game) = setup_n_player_game(addr, &[TOP, BOTTOM, LEFT]).await;

    // Player 1 takes a surviving turn so player 2 becomes the current player.
    let mov = find_surviving_move(&game, ids[0]);
    send(
        &mut clients[0],
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: ids[0],
            mov,
        },
    )
    .await;
    let states = collect_states(&mut clients, |g| !g.board.history.is_empty()).await;
    assert_eq!(
        states[0].current_player_id, ids[1],
        "player 2 should be current before the disconnect"
    );

    // Player 2 — the current player — disconnects.
    let player_two = clients.remove(1);
    drop(player_two);
    sleep(Duration::from_millis(200)).await;

    let rooms = server.rooms.read().await;
    let room = rooms.get(&room_id).expect("room should still exist");
    let game = room.game().expect("room should be in the playing phase");
    assert_eq!(
        game.current_player_id, ids[2],
        "turn should pass forward to player 3, not back to player 1"
    );
    let p2_hand = game
        .hands
        .get(&ids[1])
        .expect("player 2 still has a hand entry");
    assert!(
        p2_hand.is_empty(),
        "player 2's hand should return to the deck on disconnect"
    );
    let p2_stats = game.stats.get(&ids[1]).expect("player 2 has stats");
    assert_eq!(
        p2_stats.elimination_turn,
        Some(1),
        "the disconnect elimination is recorded in stats"
    );
}

/// A connection cannot act as another player: Bob claims Alice's player_id on
/// a move that would be legal had Alice sent it herself, and gets an Error.
#[tokio::test]
async fn test_impersonating_another_player_returns_error() {
    let (_alice, mut bob, room_id, alice_id, _bob_id, game) =
        setup_two_player_game(start_test_server().await.0).await;

    // It's Alice's turn; the move itself is perfectly valid for Alice.
    let mov = find_surviving_move(&game, alice_id);
    send(
        &mut bob,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov,
        },
    )
    .await;

    let ServerMessage::Error { message } =
        recv_where(&mut bob, |m| matches!(m, ServerMessage::Error { .. })).await
    else {
        unreachable!("recv_where matched Error");
    };
    assert!(
        message.contains("cannot act as player"),
        "error should reject the impersonation, got: {message}"
    );
}

/// Joining a room whose game has already started is rejected instead of adding
/// a ghost player that would wedge the turn rotation.
#[tokio::test]
async fn test_joining_a_started_game_returns_error() {
    let (addr, _server) = start_test_server().await;
    let (_alice, _bob, room_id, _alice_id, _bob_id, _game) = setup_two_player_game(addr).await;

    let mut charlie = connect_client(addr).await;
    send(
        &mut charlie,
        ClientMessage::JoinRoom {
            room_id,
            player_name: "Charlie".to_string(),
        },
    )
    .await;

    let ServerMessage::Error { message } =
        recv_where(&mut charlie, |m| matches!(m, ServerMessage::Error { .. })).await
    else {
        unreachable!("recv_where matched Error");
    };
    assert!(
        message.contains("already started"),
        "error should say the game already started, got: {message}"
    );
}

/// Placing a tile while the room is still in the lobby phase is rejected.
#[tokio::test]
async fn test_placing_a_tile_before_game_starts_returns_error() {
    let (addr, _server) = start_test_server().await;
    let mut alice = connect_client(addr).await;

    send(
        &mut alice,
        ClientMessage::CreateRoom {
            room_name: "Lobby Phase Test".to_string(),
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

    let tile = Tile::new([seg(0, 1), seg(2, 3), seg(4, 5), seg(6, 7)]);
    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id,
            player_id: alice_id,
            mov: Move {
                tile,
                cell: CellCoord { row: 0, col: 0 },
                player_id: alice_id,
            },
        },
    )
    .await;

    let ServerMessage::Error { message } =
        recv_where(&mut alice, |m| matches!(m, ServerMessage::Error { .. })).await
    else {
        unreachable!("recv_where matched Error");
    };
    assert!(
        message.contains("not started"),
        "error should say the game has not started, got: {message}"
    );
}

/// The engine's placement rule is enforced over the wire: a tile placed on an
/// empty cell away from the player's pawn is rejected.
#[tokio::test]
async fn test_tile_placed_away_from_pawn_returns_error() {
    let (mut alice, _bob, room_id, alice_id, _bob_id, game) =
        setup_two_player_game(start_test_server().await.0).await;

    // Alice's pawn is at (0,2); (3,3) is empty but not her cell.
    let tile = first_hand_tile(&game, alice_id);
    send(
        &mut alice,
        ClientMessage::PlaceTile {
            room_id: room_id.clone(),
            player_id: alice_id,
            mov: Move {
                tile,
                cell: CellCoord { row: 3, col: 3 },
                player_id: alice_id,
            },
        },
    )
    .await;

    let ServerMessage::Error { message } =
        recv_where(&mut alice, |m| matches!(m, ServerMessage::Error { .. })).await
    else {
        unreachable!("recv_where matched Error");
    };
    assert!(
        message.contains("current cell"),
        "error should explain the placement rule, got: {message}"
    );
}
