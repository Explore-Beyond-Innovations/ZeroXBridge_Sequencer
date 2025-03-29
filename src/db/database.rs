use serde::{Serialize, Deserialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Withdrawal {
    pub id: Uuid,
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub status: String, // "pending", "processed", etc.
    pub created_at: chrono::NaiveDateTime,
}

pub async fn insert_withdrawal(
    pool: &PgPool,
    stark_pub_key: &str,
    amount: i64,
    commitment_hash: &str,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO withdrawals (stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
        stark_pub_key,
        amount,
        commitment_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

pub async fn get_pending_withdrawals(pool: &PgPool) -> Result<Vec<Withdrawal>, sqlx::Error> {
    sqlx::query_as!(
        Withdrawal,
        r#"SELECT * FROM withdrawals WHERE status = 'pending' ORDER BY created_at DESC"#
    )
    .fetch_all(pool)
    .await
}

pub async fn create_withdrawal(
    pool: &PgPool,
    stark_pub_key: String,
    amount: i64,
    commitment_hash: String,
) -> Result<Withdrawal, sqlx::Error> {
    sqlx::query_as!(
        Withdrawal,
        r#"
        INSERT INTO withdrawals (id, stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, $4, 'pending')
        RETURNING *
        "#,
        Uuid::new_v4(), 
        stark_pub_key, 
        amount, 
        commitment_hash
    )
    .fetch_one(pool)
    .await
}