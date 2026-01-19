# Proposal 001: Integration Testing for Multiplayer

**Status:** Draft
**Author:** Claude
**Date:** 2026-01-18
**Estimated Effort:** 1 week

---

## Summary

Implement comprehensive integration tests for the multiplayer WebSocket functionality. Tests will spawn real server instances and simulate multiple clients to verify game state synchronization, lobby flows, and error handling.

---

## Motivation

Currently, there are **no integration tests** for the online multiplayer functionality. This is a critical gap because:

1. **State synchronization bugs** can cause players to see different game states
2. **Protocol changes** can break client-server communication silently
3. **Race conditions** in concurrent game actions are hard to catch manually
4. **Regression prevention** - changes to server or protocol could break existing functionality

The existing 74 tests cover game logic but not network interactions.

---

## Technical Approach

### Test Infrastructure

```
tests/
├── integration/
│   ├── mod.rs
│   ├── helpers/
│   │   ├── mod.rs
│   │   ├── test_server.rs    # Server spawning utilities
│   │   ├── test_client.rs    # WebSocket client for tests
│   │   └── assertions.rs     # Custom test assertions
│   ├── lobby_tests.rs        # Lobby flow tests
│   ├── game_tests.rs         # Game play tests
│   ├── sync_tests.rs         # State synchronization tests
│   ├── error_tests.rs        # Error handling tests
│   └── stress_tests.rs       # Load/concurrency tests
```

### Test Server Helper

```rust
// tests/integration/helpers/test_server.rs

use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

pub struct TestServer {
    process: Child,
    pub port: u16,
    pub url: String,
}

impl TestServer {
    /// Spawn a new server on a random available port
    pub async fn spawn() -> Self {
        let port = get_available_port();
        let process = Command::new("cargo")
            .args(["run", "--bin", "server", "--"])
            .env("PORT", port.to_string())
            .env("HOST", "127.0.0.1")
            .spawn()
            .expect("Failed to spawn server");

        // Wait for server to be ready
        wait_for_server(port, Duration::from_secs(10)).await;

        TestServer {
            process,
            port,
            url: format!("ws://127.0.0.1:{}", port),
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
```

### Test Client Helper

```rust
// tests/integration/helpers/test_client.rs

use tsurust_common::protocol::{ClientMessage, ServerMessage};
use tokio_tungstenite::{connect_async, WebSocketStream};
use futures::{SinkExt, StreamExt};

pub struct TestClient {
    ws: WebSocketStream<...>,
    pub player_id: Option<u32>,
    pub room_id: Option<String>,
}

impl TestClient {
    pub async fn connect(url: &str) -> Self { ... }

    pub async fn send(&mut self, msg: ClientMessage) { ... }

    pub async fn receive(&mut self) -> ServerMessage { ... }

    pub async fn receive_timeout(&mut self, timeout: Duration) -> Option<ServerMessage> { ... }

    pub async fn create_room(&mut self, name: &str) -> String { ... }

    pub async fn join_room(&mut self, room_id: &str, player_name: &str) { ... }

    pub async fn place_pawn(&mut self, position: PlayerPos) { ... }

    pub async fn place_tile(&mut self, tile_idx: usize, cell: CellCoord) { ... }
}
```

---

## Test Scenarios

### 1. Lobby Flow Tests (`lobby_tests.rs`)

| Test | Description | Priority |
|------|-------------|----------|
| `test_create_room` | Client creates room, receives RoomCreated with valid ID | High |
| `test_join_room` | Second client joins existing room | High |
| `test_join_nonexistent_room` | Joining invalid room ID returns error | High |
| `test_room_full` | Joining full room (8 players) returns error | Medium |
| `test_player_name_broadcast` | All players see each other's names | High |
| `test_pawn_placement_broadcast` | Pawn placement syncs to all clients | High |
| `test_all_pawns_placed_starts_game` | Game starts when all pawns placed | High |
| `test_creator_can_start_early` | Room creator can start with 2+ players | Medium |
| `test_duplicate_room_join` | Same client joining twice is idempotent | Medium |

### 2. Game Play Tests (`game_tests.rs`)

| Test | Description | Priority |
|------|-------------|----------|
| `test_tile_placement_broadcast` | Tile placement syncs to all clients | High |
| `test_turn_order_enforced` | Only current player can place tiles | High |
| `test_invalid_tile_rejected` | Invalid tile placement returns error | High |
| `test_player_movement_broadcast` | Player position updates sync correctly | High |
| `test_player_elimination_broadcast` | Elimination event sent to all clients | High |
| `test_game_over_broadcast` | Winner announcement sent to all | High |
| `test_hand_refill_after_placement` | Player receives new tile after placing | Medium |
| `test_dragon_tile_transfer` | Dragon tile logic works correctly | Medium |

### 3. State Synchronization Tests (`sync_tests.rs`)

| Test | Description | Priority |
|------|-------------|----------|
| `test_late_joiner_gets_full_state` | Client joining mid-game gets correct state | High |
| `test_state_consistency_after_10_moves` | All clients have identical state after moves | High |
| `test_concurrent_actions_handled` | Rapid actions from multiple clients | High |
| `test_message_ordering_preserved` | Messages arrive in correct order | Medium |
| `test_reconnect_restores_state` | Reconnecting client gets current state | High |

### 4. Error Handling Tests (`error_tests.rs`)

| Test | Description | Priority |
|------|-------------|----------|
| `test_malformed_message_rejected` | Invalid JSON returns error | High |
| `test_invalid_room_id_format` | Non-alphanumeric room ID rejected | Medium |
| `test_player_name_validation` | Empty/too-long names rejected | Medium |
| `test_action_without_room` | Actions before joining room fail | Medium |
| `test_action_wrong_room` | Actions with wrong room ID fail | Medium |

### 5. Stress Tests (`stress_tests.rs`)

| Test | Description | Priority |
|------|-------------|----------|
| `test_10_concurrent_games` | 10 games running simultaneously | Medium |
| `test_rapid_reconnections` | Client connecting/disconnecting rapidly | Low |
| `test_large_message_handling` | Very large (but valid) messages | Low |
| `test_many_spectators` | Many clients watching same game | Low |

---

## Example Test Implementation

```rust
// tests/integration/lobby_tests.rs

use crate::helpers::{TestServer, TestClient};

#[tokio::test]
async fn test_create_and_join_room() {
    // Spawn server
    let server = TestServer::spawn().await;

    // Client 1 creates room
    let mut client1 = TestClient::connect(&server.url).await;
    let room_id = client1.create_room("Test Game").await;
    assert_eq!(room_id.len(), 4);
    assert!(room_id.chars().all(|c| c.is_ascii_uppercase()));

    // Client 1 joins as Alice
    client1.join_room(&room_id, "Alice").await;
    assert_eq!(client1.player_id, Some(1));

    // Client 2 joins same room
    let mut client2 = TestClient::connect(&server.url).await;
    client2.join_room(&room_id, "Bob").await;
    assert_eq!(client2.player_id, Some(2));

    // Client 1 should receive PlayerJoined for Bob
    let msg = client1.receive().await;
    assert!(matches!(msg, ServerMessage::PlayerJoined { player_id: 2, .. }));
}

#[tokio::test]
async fn test_tile_placement_syncs_to_all_clients() {
    let server = TestServer::spawn().await;

    // Set up 2-player game
    let (mut client1, mut client2) = setup_two_player_game(&server).await;

    // Client 1 places tile
    let cell = CellCoord { row: 0, col: 0 };
    client1.place_tile(0, cell).await;

    // Both clients should receive TilePlaced
    let msg1 = client1.receive().await;
    let msg2 = client2.receive().await;

    assert!(matches!(msg1, ServerMessage::TilePlaced { .. }));
    assert!(matches!(msg2, ServerMessage::TilePlaced { .. }));

    // Verify same tile data
    if let (ServerMessage::TilePlaced { tile: t1, .. },
            ServerMessage::TilePlaced { tile: t2, .. }) = (msg1, msg2) {
        assert_eq!(t1, t2);
    }
}

#[tokio::test]
async fn test_only_current_player_can_move() {
    let server = TestServer::spawn().await;
    let (mut client1, mut client2) = setup_two_player_game(&server).await;

    // Client 2 tries to place tile (not their turn)
    let result = client2.try_place_tile(0, CellCoord { row: 0, col: 0 }).await;

    assert!(matches!(result, Err(ServerMessage::Error { .. })));
}
```

---

## Protocol Serialization Tests

In addition to integration tests, add unit tests for all protocol messages:

```rust
// common/src/protocol.rs

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_all_client_messages_roundtrip() {
        let messages = vec![
            ClientMessage::CreateRoom { room_name: "Test".into() },
            ClientMessage::JoinRoom { room_id: "ABCD".into(), player_name: "Alice".into() },
            ClientMessage::LeaveRoom { room_id: "ABCD".into() },
            ClientMessage::PlacePawn { room_id: "ABCD".into(), player_id: 1, position: PlayerPos::default() },
            ClientMessage::PlaceTile { room_id: "ABCD".into(), player_id: 1, tile_index: 0, cell: CellCoord { row: 0, col: 0 } },
            ClientMessage::StartGame { room_id: "ABCD".into() },
            // ... all variants
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).expect("serialize");
            let decoded: ClientMessage = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(msg, decoded);
        }
    }

    #[test]
    fn test_all_server_messages_roundtrip() {
        // Similar for ServerMessage variants
    }
}
```

---

## Success Criteria

- [ ] All 30+ integration test scenarios pass
- [ ] All protocol messages have serialization tests
- [ ] Tests complete in < 2 minutes total
- [ ] Tests are parallelizable (no shared state between tests)
- [ ] Tests work on CI (Linux, macOS, Windows)
- [ ] No flaky tests (run 10x without failure)

---

## Implementation Plan

### Day 1-2: Test Infrastructure
- [ ] Create test helpers (TestServer, TestClient)
- [ ] Set up test module structure
- [ ] Implement basic spawn/connect/send/receive

### Day 3-4: Lobby Tests
- [ ] Implement all lobby flow tests
- [ ] Add protocol serialization tests
- [ ] Verify tests pass locally

### Day 5-6: Game Play Tests
- [ ] Implement tile placement tests
- [ ] Implement turn order and validation tests
- [ ] Implement game completion tests

### Day 7: Sync & Stress Tests
- [ ] Implement state synchronization tests
- [ ] Add basic stress tests
- [ ] Final review and documentation

---

## Dependencies

- `tokio-tungstenite` - WebSocket client for tests
- `tokio` with `rt-multi-thread` and `macros` features
- `portpicker` - Find available ports for test servers

Add to `Cargo.toml`:
```toml
[dev-dependencies]
tokio-tungstenite = "0.21"
portpicker = "0.1"
```

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Flaky tests due to timing | Use explicit waits, generous timeouts |
| Port conflicts | Use random ports with portpicker |
| Server startup time | Cache compiled binary, warm start |
| CI environment differences | Test on all target platforms |

---

## Approval

- [ ] Technical approach approved
- [ ] Test scenarios reviewed
- [ ] Effort estimate accepted
- [ ] Ready to implement
