#[path = "utils.rs"]
mod utils;

use std::usize;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;
use utils::create_test_app;
use zeroxbridge_sequencer::api::routes::create_router;

#[tokio::test]
async fn test_post_valid_withdrawal() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/withdrawals")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "0xabc123",
                "amount": 5000,
                "commitment_hash": "0xcommitment123",
                "l1_token": "0xtoken123"  // ADDED: New required field
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(parsed.get("withdrawal_id").is_some());
}

#[tokio::test]
async fn test_post_invalid_withdrawal() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/withdrawals")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "",
                "amount": -10,
                "commitment_hash": "0xtest123",
                "l1_token": "0xtoken123"  // ADDED: New required field
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // UPDATED: Now that validation is implemented, we expect BAD_REQUEST
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    // ADDED: Verify the error message content
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(
        body_str.contains("Invalid input"),
        "Expected invalid input error, got: {}",
        body_str
    );
}

#[tokio::test]
async fn test_get_pending_withdrawals() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    // ADDED: First create a test withdrawal (following deposit_api.rs pattern)
    let post_request = Request::builder()
        .method("POST")
        .uri("/withdrawals")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "0xtest123",
                "amount": 500,
                "commitment_hash": "0xcommitment456",
                "l1_token": "0xtoken789"  // ADDED: New required field
            })
            .to_string(),
        ))
        .unwrap();

    let _ = router.clone().oneshot(post_request).await.unwrap();

    // Then test the GET endpoint
    let request: Request<Body> = Request::builder()
        .method("GET")
        .uri("/withdrawals")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap(); // UPDATED: Changed to Vec to match deposit pattern
    
    // ADDED: Verify we have withdrawals and check status (following deposit_api.rs pattern)
    assert!(!parsed.is_empty());
    assert_eq!(parsed[0]["status"], "pending");
}
