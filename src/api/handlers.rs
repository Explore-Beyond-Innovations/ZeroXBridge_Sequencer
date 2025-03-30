use axum::{http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::database::{
    create_withdrawal as db_create_withdrawal, get_pending_deposits,
    get_pending_withdrawals as db_get_pending_withdrawals, insert_deposit, Deposit, Withdrawal,
};

#[derive(Debug, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
}

#[derive(Deserialize)]
pub struct DepositRequest {
    pub user_address: String,
    pub amount: i64,
}

#[derive(Serialize)]
pub struct DepositResponse {
    pub commitment_hash: String,
}

pub async fn handle_deposit_post(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, (StatusCode, String)> {
    if payload.amount <= 0 || payload.user_address.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Invalid input".to_string()));
    }

    // Generate a salted commitment hash using UUID
    let nonce = Uuid::new_v4();
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{}{}{}",
        payload.user_address, payload.amount, nonce
    ));
    let commitment_hash = format!("{:x}", hasher.finalize());

    insert_deposit(
        &pool,
        &payload.user_address,
        payload.amount,
        &commitment_hash,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DepositResponse { commitment_hash }))
}

pub async fn handle_get_pending_deposits(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Deposit>>, (StatusCode, String)> {
    let deposit = get_pending_deposits(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(deposit))
}

pub async fn create_withdrawal(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<Withdrawal>, (StatusCode, String)> {
    let withdrawal = db_create_withdrawal(
        &pool,
        payload.stark_pub_key,
        payload.amount,
        payload.commitment_hash,
    )
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(withdrawal))
}

pub async fn get_pending_withdrawals(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Withdrawal>>, (StatusCode, String)> {
    let withdrawals = db_get_pending_withdrawals(&pool).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(withdrawals))
}
