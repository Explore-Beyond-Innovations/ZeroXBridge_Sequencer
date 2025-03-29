use axum::{Json, Extension, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::api::database::{insert_deposit, get_pending_deposits};
use crate::api::database::Deposit;

// pub async fn handle_deposits_post(
//     Extension(pool): Extension<PgPool>,
//     Json(payload): Json<DepositsRequest>,
// ) -> Result<Json<DepositsResponse>, (StatusCode, String)> {
//     // Validate inputs
//     if payload.amount <= 0 {
//         return Err((StatusCode::BAD_REQUEST, "Amount must be positive".to_string()));
//     }
    
//     if payload.stark_pub_key.trim().is_empty() {
//         return Err((StatusCode::BAD_REQUEST, "Stark public key cannot be empty".to_string()));
//     }

//     // Generate commitment hash
//     let nonce = Uuid::new_v4();
//     let mut hasher = Sha256::new();
//     hasher.update(format!(
//         "{}{}{}",
//         payload.stark_pub_key,
//         payload.amount,
//         nonce
//     ));
//     let commitment_hash = format!("{:x}", hasher.finalize());

//     // Insert into database
//     match insert_deposits(&pool, &payload.stark_pub_key, payload.amount, &commitment_hash).await {
//         Ok(_) => Ok(Json(DepositsResponse { commitment_hash })),
//         Err(e) => {
//             if e.to_string().contains("duplicate key") {
//                 Err((StatusCode::CONFLICT, "Duplicate commitment hash".to_string()))
//             } else {
//                 Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//             }
//         }
//     }
// }

pub async fn handle_get_pending_deposit(
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<Deposit>>, (StatusCode, String)> {
    let deposit = get_pending_deposits(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(deposit))
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
    hasher.update(format!("{}{}{}", payload.user_address, payload.amount, nonce));
    let commitment_hash = format!("{:x}", hasher.finalize());

    match insert_deposit(&pool, &payload.user_address, payload.amount, &commitment_hash).await {
            Ok(_) => Ok(Json(DepositResponse { commitment_hash })),
            Err(e) => {
                if e.to_string().contains("duplicate key") {
                    Err((StatusCode::CONFLICT, "Duplicate commitment hash".to_string()))
                } else {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
                }
            }
        }
}
