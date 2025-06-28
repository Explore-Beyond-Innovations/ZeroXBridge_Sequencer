use crate::db::database::{
    fetch_pending_deposits, fetch_pending_withdrawals, insert_deposit, insert_withdrawal, Deposit,
    Withdrawal,
};
use crate::utils::hash::{compute_poseidon_commitment_hash, HashMethod};
use axum::{http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use starknet::core::types::Felt;

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

#[derive(Serialize, Deserialize)]
pub struct PoseidonHashRequest {
    /// Starknet address of the recipient
    pub recipient: String,
    /// USD amount to mint
    pub amount: u128,
    /// Transaction nonce
    pub nonce: u64,
    /// Block timestamp
    pub timestamp: u64,
    /// Optional hash method to use: "batch" or "sequential" (default: "sequential")
    #[serde(default)]
    pub hash_method: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PoseidonHashResponse {
    pub commitment_hash: String,
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
    match fetch_pending_withdrawals(&pool, 3).await {
        Ok(withdrawals) => Ok(Json(withdrawals)),
        Err(err) => Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string())),
    }
}

pub async fn hello_world(
    Extension(_): Extension<PgPool>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "message": "hello world from zeroxbridge"
    })))
}

/// Computes a Poseidon commitment hash for deposit transactions
///
/// This endpoint allows users to generate the same hash that the L2 contract
/// will compute and verify using Cairo-native Poseidon logic. Users should call
/// this endpoint before depositing to L1.
///
/// The hash is computed using the following fields:
/// - recipient: Starknet address of the receiver
/// - amount: USD amount to mint
/// - nonce: Transaction nonce
/// - timestamp: Block timestamp
///
/// Returns the commitment hash that should be used when making the deposit.
pub async fn compute_poseidon_hash(
    Json(payload): Json<PoseidonHashRequest>,
) -> Result<Json<PoseidonHashResponse>, (StatusCode, String)> {
    // Parse recipient address as Felt (felt252)
    let recipient_felt = match Felt::from_hex(&payload.recipient) {
        Ok(felt) => felt,
        Err(_) => return Err((StatusCode::BAD_REQUEST,
            "Invalid recipient address format. Must be a valid Starknet address in hex format (0x...).".
            to_string())
        ),
    };

    // Determine which hash method to use (default to sequential pairwise which is more common in Cairo contracts)
    let method = match payload.hash_method.as_deref() {
        Some("batch") => HashMethod::BatchHash,
        Some("sequential") | None => HashMethod::SequentialPairwise,
        Some(method) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid hash method: '{}'. Valid options are 'batch' or 'sequential'",
                    method
                ),
            ))
        }
    };

    // Compute the Poseidon hash using the utility function
    let hash = compute_poseidon_commitment_hash(
        recipient_felt,
        payload.amount,
        payload.nonce,
        payload.timestamp,
        method,
    );

    // Convert hash to hex string format
    let hash_hex = format!("0x{:x}", hash);

    Ok(Json(PoseidonHashResponse {
        commitment_hash: hash_hex,
    }))
}
