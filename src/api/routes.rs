use crate::{api::handlers::hello_world, config::AppConfig};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use sqlx::PgPool;

use crate::api::handlers::{
    compute_poseidon_hash, create_withdrawal, get_pending_withdrawals, handle_deposit_post,
    handle_get_pending_deposits,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: AppConfig,
}

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(hello_world))
        .route(
            "/deposit",
            post(handle_deposit_post).get(handle_get_pending_deposits),
        )
        .route(
            "/withdrawals",
            post(create_withdrawal).get(get_pending_withdrawals),
        )
        .route("/poseidon/hash", post(compute_poseidon_hash))
        .layer(Extension(pool))
}
