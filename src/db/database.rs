use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use sqlx::PgPool;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Deposit {
    pub id: i32,
    pub user_address: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub status: String, // "pending", "processed", etc.
    pub created_at: chrono::NaiveDateTime,
}

pub async fn insert_deposit(
    pool: &PgPool,
    user_address: &str,
    amount: i64,
    commitment_hash: &str,
) -> Result<i32, sqlx::Error> {
    let row = sqlx::query_scalar!(
        r#"
        INSERT INTO deposits (user_address, amount, commitment_hash, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
        user_address,
        amount,
        commitment_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_pending_deposits(pool: &PgPool) -> Result<Vec<Deposit>, sqlx::Error> {
    let deposits = sqlx::query_as!(
        Deposit,
        r#"SELECT * FROM deposits WHERE status = 'pending' ORDER BY created_at DESC"#
    )
    .fetch_all(pool)
    .await?;

    Ok(deposits)
}