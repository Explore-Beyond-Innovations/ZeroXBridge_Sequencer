use axum::{extract::State, Json};
use sqlx::PgPool;
use crate::api::models::{Withdrawal, CreateWithdrawalRequest};
use crate::db::database::{get_pending_withdrawals as db_get_pending_withdrawals, create_withdrawal as db_create_withdrawal};
use uuid::Uuid;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: String,
    pub commitment_hash: String,
}

pub async fn create_withdrawal(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<Withdrawal>, (axum::http::StatusCode, String)> {
    let withdrawal = db_create_withdrawal(
        &pool, 
        payload.stark_pub_key, 
        payload.amount, 
        payload.commitment_hash
    )
    .await
    .map_err(|err| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", err))
    })?;

    Ok(Json(withdrawal))
}

pub async fn get_pending_withdrawals(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Withdrawal>>, (axum::http::StatusCode, String)> {
    let withdrawals = db_get_pending_withdrawals(&pool)
        .await
        .map_err(|err| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", err))
        })?;

    Ok(Json(withdrawals))
}