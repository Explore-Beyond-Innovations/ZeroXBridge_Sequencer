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
async fn test_hello_world() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    assert_eq!(status, StatusCode::OK);

    let parsed: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let message = parsed.get("message").unwrap().as_str().unwrap();
    assert_eq!(message, "hello world from zeroxbridge");
}

#[tokio::test]
async fn test_post_valid_deposit() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "0xuser123",
                "amount": 1000,
                "commitment_hash": "0xcommitment123"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    println!("Status: {:?}", response.status());
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(parsed.get("deposit_id").is_some());
}

#[tokio::test]
async fn test_deposit_missing_commitment_hash() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "",
                "amount": 1000,
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        body_str.contains("missing field `commitment_hash`"),
        "Expected missing field error, got: {}",
        body_str
    );
}

#[tokio::test]
async fn test_semantic_invalid_deposit() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "",
                "amount": -100,
                "commitment_hash": "0xdeadbeef"
            })
            .to_string(),
        ))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);

    println!("Status: {}", status);
    println!("Body: {}", body_str);

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body_str.contains("Invalid input"),
        "Expected invalid input error, got: {}",
        body_str
    );
}

#[tokio::test]
async fn test_get_pending_deposits() {
    let app = create_test_app().await;
    let router = create_router(app.db.clone());

    // First create a test deposit
    let post_request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "0xtest123",
                "amount": 500
            })
            .to_string(),
        ))
        .unwrap();

    let _ = router.clone().oneshot(post_request).await.unwrap();

    // Then test the GET endpoint
    let get_request = Request::builder()
        .method("GET")
        .uri("/deposit")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!parsed.is_empty());
    assert_eq!(parsed[0]["status"], "pending");
}
