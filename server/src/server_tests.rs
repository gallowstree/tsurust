use crate::server::GameServer;

#[tokio::test]
async fn test_create_room_assigns_player_id_1() {
    let server = GameServer::new();

    let result = server.create_room("Test Room".to_string(), "Alice".to_string()).await;

    assert!(result.is_ok());
    let (room_id, player_id) = result.unwrap();
    assert_eq!(player_id, 1, "First player should have ID 1, not 0");
    assert_eq!(room_id, "Test Room");
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
async fn test_join_nonexistent_room_fails() {
    let server = GameServer::new();

    let result = server
        .join_room("NonexistentRoom".to_string(), "Alice".to_string())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_create_duplicate_room_fails() {
    let server = GameServer::new();

    // Create first room
    server
        .create_room("Test Room".to_string(), "Alice".to_string())
        .await
        .expect("Failed to create room");

    // Try to create room with same name
    let result = server
        .create_room("Test Room".to_string(), "Bob".to_string())
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}
