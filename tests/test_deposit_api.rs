#[path = "utils.rs"]
mod utils;

use serde_json::json;
use zeroxbridge_sequencer::api::routes::create_router;
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use utils::create_test_app;

    #[tokio::test]
    async fn test_fetch_user_latest_deposit_handler_success() {
        let app = create_test_app().await;
        let router = create_router(app.db.clone());

        let stark_pub_key = "0x1111111111111111111111111111111111111111";

        // First create a test deposit
        let post_request = Request::builder()
            .method("POST")
            .uri("/deposit")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "stark_pub_key": stark_pub_key.to_string(),
                    "amount": 500,
                    "commitment_hash": stark_pub_key.to_string()
                })
                .to_string(),
            ))
            .unwrap();

        let _ = router.clone().oneshot(post_request).await.unwrap();

        let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?stark_pub_key={}", stark_pub_key))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_fetch_user_latest_deposit_with_user_address_handler() {
        let app = create_test_app().await;
        let router = create_router(app.db.clone());

        let stark_pub_key = "0x1111111111111111111111111111111111111111";
        let user_address = "0x1111111111111111111111111111111111111111";

        // First create a test deposit
        let post_request = Request::builder()
            .method("POST")
            .uri("/deposit")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "stark_pub_key": stark_pub_key.to_string(),
                    "amount": 500,
                    "commitment_hash": user_address.to_string()
                })
                .to_string(),
            ))
            .unwrap();

        let _ = router.clone().oneshot(post_request).await.unwrap();

        let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_user_address_not_found_handler() {
        let app = create_test_app().await;
        let router = create_router(app.db.clone());
        let user_address = "0x1111111111111111111111111111111111111qqqq";

        let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_user_address_for_all_deposits() {
        let app = create_test_app().await;
        let router = create_router(app.db.clone());
        let user_address = "0x1111111111111111111111111111111111111111";

        let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
