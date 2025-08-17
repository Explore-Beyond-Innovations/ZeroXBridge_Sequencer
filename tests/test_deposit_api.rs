#[path = "utils.rs"]
mod utils;

use zeroxbridge_sequencer::db::database::get_db_pool;
use zeroxbridge_sequencer::api::routes::create_router;


#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use sqlx::{PgPool,};
    use tower::ServiceExt;

    async fn setup_test_app() -> (Router, PgPool) {
        // Use the existing database with data
        let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://blurbeast:Oladele326@localhost:5432/zero_bridge".to_string());
        let pool = get_db_pool(&database_url)
            .await
            .expect("Failed to create test database pool");

        // Create the router with the pool
        let router = create_router(pool.clone());

        (router, pool)
    }

#[tokio::test]
async fn test_fetch_user_latest_deposit_handler_success() {
    let (app, pool) = setup_test_app().await;
    let stark_pub_key = "0x1111111111111111111111111111111111111111";
    // let router = create_router(app.db.clone());

    let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?stark_pub_key={}", stark_pub_key))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_fetch_user_latest_deposit_with_user_address_handler() {
    let (app, pool) = setup_test_app().await;
    let user_address = "0x1111111111111111111111111111111111111111";

    let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_user_address_not_found_handler() {
    let (app, pool) = setup_test_app().await;
    let user_address = "0x1111111111111111111111111111111111111qqqq";

    let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits/latest?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);   
}

#[tokio::test]
async fn test_user_address_for_all_deposits() {
    let (app, pool) = setup_test_app().await;
    let user_address = "0x1111111111111111111111111111111111111111";

    let request = Request::builder()
            .method("GET")
            .uri(format!("/deposits?user_address={}", user_address))
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap();
    
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
}


}