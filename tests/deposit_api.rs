use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::json;
use tower::ServiceExt;
use zero_x_bridge_sequencer::api::routes::create_test_app;

#[tokio::test]
async fn test_post_valid_deposit() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "user_address": "0xuser123",
                "amount": 1000
            }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(parsed.get("commitment_hash").is_some());
}

#[tokio::test]
async fn test_post_invalid_deposit() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "user_address": "",
                "amount": -100
            }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_pending_deposits() {
    let app = create_test_app().await;

    // First create a test deposit
    let post_request = Request::builder()
        .method("POST")
        .uri("/deposit")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "user_address": "0xtest123",
                "amount": 500
            }).to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(post_request).await.unwrap();

    // Then test the GET endpoint
    let get_request = Request::builder()
        .method("GET")
        .uri("/deposit")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!parsed.is_empty());
    assert_eq!(parsed[0]["status"], "pending");
}