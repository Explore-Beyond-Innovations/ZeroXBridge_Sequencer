use axum::{body::Body, http::{Request, StatusCode}};
use axum::body::Bytes;
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

use zeroxbridge_sequencer::api::routes::create_router;
use zeroxbridge_sequencer::utils::BurnData;

async fn body_to_bytes(body: axum::body::Body) -> Bytes {
    use futures_util::stream::StreamExt;
    let mut data = Vec::new();
    let mut stream = body.into_data_stream();
    while let Some(chunk) = stream.next().await {
        data.extend_from_slice(&chunk.unwrap());
    }
    Bytes::from(data)
}

#[tokio::test]
async fn test_compute_hash_api_success() {
    // Setup router (no DB needed for this endpoint)
    let pool = sqlx::PgPool::connect_lazy("postgres://postgres:postgres@localhost/test").unwrap();
    let app = create_router(pool);

    // Example input matching the Rust hash test
    let req_body = json!({
        "stark_pubkey": "0x0101010101010101010101010101010101010101010101010101010101010101",
        "usd_val": 1000u64,
        "nonce": 42u64,
        "timestamp": 1640995200u64
    });

    let request = Request::post("/compute-hash")
        .header("content-type", "application/json")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = body_to_bytes(response.into_body()).await;
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let hash = json["commitment_hash"].as_str().unwrap();
    assert!(hash.starts_with("0x"));
    assert_eq!(hash.len(), 66); // 0x + 64 hex chars
}

#[tokio::test]
async fn test_compute_hash_api_invalid_pubkey() {
    let pool = sqlx::PgPool::connect_lazy("postgres://postgres:postgres@localhost/test").unwrap();
    let app = create_router(pool);

    let req_body = json!({
        "stark_pubkey": "not_a_hex_pubkey",
        "usd_val": 1000u64,
        "nonce": 42u64,
        "timestamp": 1640995200u64
    });

    let request = Request::post("/compute-hash")
        .header("content-type", "application/json")
        .body(Body::from(req_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_utils_reference_solidity_compatibility() {
    let data = BurnData {
        caller: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7".to_string(),
        amount: 50000u64,
        nonce: 123u64,
        time_stamp: 1672531200u64,
    };
    let hex_hash = data.hash_to_hex_string();
    let expected = "0x2b6876060a11edcc5dde925cda8fad185f34564e35802fa40ee8ead2f9acb06f";
    assert_eq!(hex_hash, expected);
}
