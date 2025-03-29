use axum::routing::{get, post};
use axum::{Router, extract::State, Json};
use crate::api::models::{Withdrawal, CreateWithdrawalRequest};
use crate::api::handlers::{get_pending_withdrawals, create_withdrawal};

pub fn withdrawal_routes() -> Router {
    Router::new()
        .route("/withdrawals", get(get_pending_withdrawals))
        .route("/withdrawals", post(create_withdrawal))
}