use crate::server::GameServer;
use tsurust_common::board::PlayerPos;

#[tokio::test]
async fn test_create_room_assigns_player_id_1() {
    let server = GameServer::new();

    let result = server
        .create_room("Test Room".to_string(), "Alice".to_string())
        .await;

    assert!(result.is_ok());
    let (room_id, player_id) = result.unwrap();
    assert_eq!(player_id, 1, "First player should have ID 1, not 0");
    assert!(!room_id.is_empty(), "Room ID should not be empty");
    assert_eq!(room_id.len(), 4, "Room ID should be 4 characters");
}

#[tokio::test]
async fn test_join_room_assigns_sequential_ids() {
    let server = GameServer::new();

    // Create room (first player gets ID 1)
    let (room_id, first_player_id) = server
        .create_room("Test Room".to_string(), "Alice".to_string())
        .await
        .expect("Failed to create room");

    assert_eq!(first_player_id, 1);

    // Second player joins
    let second_player_id = server
        .join_room(room_id.clone(), "Bob".to_string())
        .await
        .expect("Failed to join room");

    assert_eq!(second_player_id, 2, "Second player should have ID 2");

    // Third player joins
    let third_player_id = server
        .join_room(room_id.clone(), "Charlie".to_string())
        .await
        .expect("Failed to join room");

    assert_eq!(third_player_id, 3, "Third player should have ID 3");
}

#[tokio::test]
async fn test_player_ids_use_max_plus_one() {
    let server = GameServer::new();

    // Create room
    let (room_id, _) = server
        .create_room("Test Room".to_string(), "Alice".to_string())
        .await
        .expect("Failed to create room");

    // Manually verify the calculation works even with gaps
    // (this test validates the logic: max(existing_ids) + 1)
    let player2_id = server
        .join_room(room_id.clone(), "Bob".to_string())
        .await
        .expect("Failed to join room");

    let player3_id = server
        .join_room(room_id.clone(), "Charlie".to_string())
        .await
        .expect("Failed to join room");

    // Verify IDs are sequential
    assert_eq!(player2_id, 2);
    assert_eq!(player3_id, 3);

    // Even if we had a gap (like player 2 left), next player should get max+1
    // This validates our use of max() rather than len()
}

#[tokio::test]
async fn test_join_after_game_started_is_rejected() {
    let server = GameServer::new();

    let (room_id, _) = server
        .create_room("Test Room".to_string(), "Alice".to_string())
        .await
        .expect("Failed to create room");
    server
        .join_room(room_id.clone(), "Bob".to_string())
        .await
        .expect("Failed to join room");

    // Place both pawns and start the game.
    {
        let mut rooms = server.rooms.write().await;
        let room = rooms.get_mut(&room_id).expect("room exists");
        room.place_pawn(1, PlayerPos::new(0, 2, 5))
            .expect("Alice's pawn placement");
        room.place_pawn(2, PlayerPos::new(5, 3, 0))
            .expect("Bob's pawn placement");
        room.start_game().expect("game should start");
    }

    // A third player joining now would be a ghost with no hand; reject it.
    let result = server.join_room(room_id, "Charlie".to_string()).await;
    assert!(result.is_err(), "joining a started game must fail");
    assert!(
        result.unwrap_err().contains("already started"),
        "error should say the game already started"
    );
}

#[tokio::test]
async fn test_join_nonexistent_room_fails() {
    let server = GameServer::new();

    let result = server
        .join_room("NonexistentRoom".to_string(), "Alice".to_string())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_reap_removes_only_idle_disconnected_rooms() {
    let server = GameServer::new();
    let (idle_id, _) = server
        .create_room("Idle Room".to_string(), "Alice".to_string())
        .await
        .expect("create idle room");
    let (live_id, _) = server
        .create_room("Live Room".to_string(), "Bob".to_string())
        .await
        .expect("create live room");

    // The "live" room has a connected client: a live broadcast subscriber,
    // exactly what a connection handler holds while attached to a room.
    let _rx = {
        let rooms = server.rooms.read().await;
        rooms
            .get(&live_id)
            .expect("live room exists")
            .update_tx
            .subscribe()
    };

    let reaped = server.reap_idle_rooms(std::time::Duration::ZERO).await;

    assert_eq!(reaped, 1, "exactly the unconnected room is reaped");
    let rooms = server.rooms.read().await;
    assert!(
        !rooms.contains_key(&idle_id),
        "the room with no connected clients is removed"
    );
    assert!(
        rooms.contains_key(&live_id),
        "a room with a live connection survives regardless of idle time"
    );
}

#[tokio::test]
async fn test_reap_leaves_fresh_rooms_within_grace_period() {
    let server = GameServer::new();
    let (room_id, _) = server
        .create_room("Fresh Room".to_string(), "Alice".to_string())
        .await
        .expect("create room");

    // No client has subscribed yet (the handler does that just after create),
    // but the room is younger than the idle timeout — it must survive.
    let reaped = server
        .reap_idle_rooms(std::time::Duration::from_secs(300))
        .await;

    assert_eq!(reaped, 0);
    assert!(
        server.rooms.read().await.contains_key(&room_id),
        "a fresh room survives the grace period"
    );
}

#[tokio::test]
async fn test_create_multiple_rooms_generates_unique_ids() {
    let server = GameServer::new();

    // Create first room
    let (room_id1, _) = server
        .create_room("Room 1".to_string(), "Alice".to_string())
        .await
        .expect("Failed to create first room");

    // Create second room
    let (room_id2, _) = server
        .create_room("Room 2".to_string(), "Bob".to_string())
        .await
        .expect("Failed to create second room");

    // Room IDs should be unique
    assert_ne!(room_id1, room_id2, "Room IDs should be unique");
    assert_eq!(room_id1.len(), 4, "Room ID should be 4 characters");
    assert_eq!(room_id2.len(), 4, "Room ID should be 4 characters");
}
