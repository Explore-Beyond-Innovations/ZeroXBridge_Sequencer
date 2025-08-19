use axum::extract::Query;
use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use crate::utils::{compute_poseidon_commitment_hash, BurnData, HashMethod};
use chrono::Utc;

use crate::db::database::{
    fetch_pending_deposits, fetch_pending_withdrawals, get_or_create_nonce,
    insert_deposit_with_l2_hash, insert_withdrawal, Deposit, Withdrawal,
};

use starknet::core::types::Felt;

// UPDATED: Added l1_token field
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWithdrawalRequest {
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub l1_token: String, // ADDED: New required field
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
    /// Optional hash method to use: "batch" or "sequential" (default: "BatchHash")
    #[serde(default)]
    pub hash_method: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PoseidonHashResponse {
    pub commitment_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct HashRequest {
    pub stark_pubkey: String,
    pub usd_val: u64,
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
    pub usd_val: u64,
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

    // Parse recipient address as Felt (felt252)
    let recipient_felt = match Felt::from_hex(&payload.stark_pub_key) {
        Ok(felt) => felt,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid Starknet address format. Must be a valid hex format (0x...).".to_string(),
            ))
        }
    };

    let mut tx: Transaction<'_, Postgres> = pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let nonce = get_or_create_nonce(&pool, &payload.stark_pub_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let timestamp = Utc::now().timestamp() as u64;

    let l2_hash = compute_poseidon_commitment_hash(
        recipient_felt,
        payload.amount as u128,
        nonce as u64,
        timestamp,
        HashMethod::BatchHash,
    );
    let l2_hash_hex = format!("0x{:x}", l2_hash);

    let deposit_id = insert_deposit_with_l2_hash(
        &mut tx,
        &payload.stark_pub_key,
        payload.amount,
        &payload.commitment_hash,
        &l2_hash_hex,
        nonce,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
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
    use crate::db::database::{get_and_increment_withdrawal_nonce, insert_withdrawal_v2};
    use crate::utils::BurnData;

    // Validation logic
    if payload.amount <= 0
        || payload.stark_pub_key.trim().is_empty()
        || payload.l1_token.trim().is_empty()
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid input".to_string()));
    }

    // Fetch and increment the nonce for the user
    let nonce = get_and_increment_withdrawal_nonce(&pool, &payload.stark_pub_key)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get/increment nonce: {:?}", err),
            )
        })?;

    // Use current timestamp if not provided (for compatibility, but ideally should be provided)
    let timestamp = chrono::Utc::now().timestamp() as u64;

    // Construct BurnData and compute the hash
    let burn_data = BurnData {
        caller: payload.stark_pub_key.clone(),
        amount: payload.amount as u64, // assuming amount is always positive
        nonce: nonce as u64,
        time_stamp: timestamp,
    };
    if BurnData::hex_to_bytes32(&burn_data.caller).is_err() {
        return Err((StatusCode::BAD_REQUEST, "Invalid stark_pubkey format".to_string()));
    }
    let l1_hash = burn_data.hash_to_hex_string();

    // Insert withdrawal with l1_hash and nonce
    let withdrawal_id = insert_withdrawal_v2(
        &pool,
        &payload.stark_pub_key,
        payload.amount,
        &payload.l1_token,
        &l1_hash,
        &l1_hash, // commitment_hash and l1_hash are the same for now
        nonce,
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

    // Determine which hash method to use (default to BatchHash for efficiency)
    let method = match payload.hash_method.as_deref() {
        Some("BatchHash") | Some("batch") | None => HashMethod::BatchHash,
        Some("SequentialPairwise") | Some("sequential") => HashMethod::SequentialPairwise,
        Some(method) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                "Invalid hash method: '{}'. Valid options are 'BatchHash' or 'SequentialPairwise'",
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

pub async fn compute_hash_handler(
    Json(payload): Json<HashRequest>,
) -> Result<Json<HashResponse>, impl IntoResponse> {
    // Validate the Starknet public key format before hashing
    let burn_data = BurnData {
        caller: payload.stark_pubkey.clone(),
        amount: payload.usd_val,
        nonce: payload.nonce,
        time_stamp: payload.timestamp,
    };
    if BurnData::hex_to_bytes32(&burn_data.caller).is_err() {
        let error_response = ErrorResponse {
            error: "Invalid stark_pubkey".to_string(),
            details: Some("Invalid hex string for caller address".to_string()),
        };
        return Err((StatusCode::BAD_REQUEST, Json(error_response)));
    }
    // Compute the commitment hash
    let hex_hash = burn_data.hash_to_hex_string();
    // Create response
    let response = HashResponse {
        commitment_hash: hex_hash,
        input_data: InputData {
            stark_pubkey: burn_data.caller,
            usd_val: burn_data.amount,
            nonce: burn_data.nonce,
            timestamp: burn_data.time_stamp,
        },
    };
    Ok(Json(response))
}

#[derive(Serialize, Deserialize)]
pub struct FetchDepositQuery {
    pub stark_pub_key: Option<String>,
    pub user_address: Option<String>,
}

// pub struct

pub async fn fetch_user_latest_deposit_handler(
    Extension(pool): Extension<PgPool>,
    Query(payload): Query<FetchDepositQuery>,
) -> Result<Json<Deposit>, (StatusCode, String)> {
    let key = extract_user_key(&payload)?;

    let deposit = get_user_latest_deposit(&pool, &key).await;

    match deposit {
        Ok(Some(dp)) => Ok(Json(dp)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            "No deposits found for the given user".to_string(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

pub async fn fetch_user_deposits_handler(
    Extension(pool): Extension<PgPool>,
    Query(payload): Query<FetchDepositQuery>,
) -> Result<Json<Vec<Deposit>>, (StatusCode, String)> {
    let key = extract_user_key(&payload)?;

    let deposit = get_user_deposits(&pool, &key, 2)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(deposit))
}

fn extract_user_key(payload: &FetchDepositQuery) -> Result<String, (StatusCode, String)> {
    match (
        payload.stark_pub_key.as_ref(),
        payload.user_address.as_ref(),
    ) {
        (Some(stark), _) => Ok(stark.trim().to_string()),
        (None, Some(user)) => Ok(user.trim().to_string()),
        (None, None) => Err((
            StatusCode::BAD_REQUEST,
            "Either stark_pub_key or user_address must be provided".into(),
        )),
    }
}
