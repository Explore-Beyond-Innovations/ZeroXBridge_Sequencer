use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::db::database::{
    fetch_pending_deposits, fetch_pending_withdrawals, insert_deposit, insert_withdrawal, Deposit,
    Withdrawal,
};

use crate::utils::{compute_commitment_hash, hash_to_hex_string, parse_stark_pubkey};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct DepositResponse {
    pub deposit_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct WithrawalResponse {
    pub withdrawal_id: i32,
}

#[derive(Deserialize, Debug)]
pub struct HashRequest {
    pub stark_pubkey: String,
    pub usd_val: u128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Debug)]
pub struct HashResponse {
    pub commitment_hash: String,
    pub input_data: InputData,
}

#[derive(Serialize, Debug)]
pub struct InputData {
    pub stark_pubkey: String,
    pub usd_val: u128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

pub async fn handle_deposit_post(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, (StatusCode, String)> {
    if payload.amount <= 0 || payload.stark_pub_key.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Invalid input".to_string()));
    }
    let deposit_id = insert_deposit(
        &pool,
        &payload.stark_pub_key,
        payload.amount,
        &payload.commitment_hash,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DepositResponse { deposit_id }))
}

pub async fn handle_get_pending_deposits(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Deposit>>, (StatusCode, String)> {
    let deposit = fetch_pending_deposits(&pool, 5)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(deposit))
}

pub async fn create_withdrawal(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateWithdrawalRequest>,
) -> Result<Json<WithrawalResponse>, (StatusCode, String)> {
    let withdrawal_id = insert_withdrawal(
        &pool,
        &payload.stark_pub_key,
        payload.amount,
        &payload.commitment_hash,
    )
    .await
    .map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(WithrawalResponse { withdrawal_id }))
}

pub async fn get_pending_withdrawals(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Withdrawal>>, (StatusCode, String)> {
    let withdrawals = fetch_pending_withdrawals(&pool, 5).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB Error: {:?}", err),
        )
    })?;

    Ok(Json(withdrawals))
}

pub async fn hello_world(
    Extension(_): Extension<PgPool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "message": "hello world from zeroxbridge"
    })))
}

pub async fn compute_hash_handler(
    Json(payload): Json<HashRequest>,
) -> Result<Json<HashResponse>, impl IntoResponse> {
    // Validate and parse the Starknet public key
    let caller_bytes = match parse_stark_pubkey(&payload.stark_pubkey) {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_response = ErrorResponse {
                error: "Invalid stark_pubkey".to_string(),
                details: Some(e),
            };
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
    };

    // Compute the commitment hash
    let hash = compute_commitment_hash(
        caller_bytes,
        payload.usd_val,
        payload.nonce,
        payload.timestamp,
    );

    // Convert to hex string
    let hex_hash = hash_to_hex_string(hash);

    // Create response
    let response = HashResponse {
        commitment_hash: hex_hash,
        input_data: InputData {
            stark_pubkey: payload.stark_pubkey.clone(),
            usd_val: payload.usd_val,
            nonce: payload.nonce,
            timestamp: payload.timestamp,
        },
    };

    Ok(Json(response))
}
