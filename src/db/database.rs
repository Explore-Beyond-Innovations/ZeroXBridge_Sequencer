use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, PgPool};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Withdrawal {
    pub id: i32,
    pub stark_pub_key: String,
    pub amount: i64,
    pub l1_token: String,
    pub l2_tx_id: Option<i32>,
    pub commitment_hash: String,
    pub status: String,
    pub retry_count: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Deposit {
    pub id: i32,
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub status: String, // "pending", "processed", etc.
    pub retry_count: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub async fn insert_withdrawal(
    conn: &PgPool,
    stark_pub_key: &str,
    amount: i64,
    commitment_hash: &str,
) -> Result<i32, sqlx::Error> {
    let row_id = sqlx::query_scalar!(
        r#"
        INSERT INTO withdrawals (stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
        stark_pub_key,
        amount,
        commitment_hash
    )
    .fetch_one(conn)
    .await?;

    Ok(row_id)
}

pub async fn insert_deposit(
    conn: &PgPool,
    stark_pub_key: &str,
    amount: i64,
    commitment_hash: &str,
) -> Result<i32, sqlx::Error> {
    let row_id = sqlx::query_scalar!(
        r#"
        INSERT INTO deposits (stark_pub_key, amount, commitment_hash, status)
        VALUES ($1, $2, $3, 'pending')
        RETURNING id
        "#,
        stark_pub_key,
        amount,
        commitment_hash
    )
    .fetch_one(conn)
    .await?;

    Ok(row_id)
}

pub async fn fetch_pending_withdrawals(
    conn: &PgPool,
    max_retries: u32,
) -> Result<Vec<Withdrawal>, sqlx::Error> {
    let withdrawals = sqlx::query_as!(
        Withdrawal,
        r#"
        SELECT * FROM withdrawals
        WHERE status = 'pending'
        AND retry_count < $1
        ORDER BY created_at ASC
        LIMIT 10
        "#,
        max_retries as i32
    )
    .fetch_all(conn)
    .await?;

    Ok(withdrawals)
}

pub async fn fetch_pending_deposits(
    conn: &PgPool,
    max_retries: u32,
) -> Result<Vec<Deposit>, sqlx::Error> {
    let deposits = sqlx::query_as!(
        Deposit,
        r#"
        SELECT *
        FROM deposits
        WHERE status = 'pending' AND retry_count < $1
        ORDER BY created_at ASC
        LIMIT 10
        "#,
        max_retries as i32
    )
    .fetch_all(conn)
    .await?;

    Ok(deposits)
}

pub async fn update_deposit_status(
    conn: &mut PgConnection,
    id: i32,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE deposits
        SET status = $2, updated_at = NOW()
        WHERE id = $1
        "#,
        id,
        status
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn process_deposit_retry(conn: &mut PgConnection, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE deposits
        SET retry_count = retry_count + 1, updated_at = NOW()
        WHERE id = $1
        "#,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn process_withdrawal_retry(conn: &mut PgConnection, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE withdrawals
        SET retry_count = retry_count + 1,
        updated_at = NOW()
        WHERE id = $1
        "#,
        id
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_withdrawal_status(
    conn: &mut PgConnection,
    id: i32,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE withdrawals
        SET status = $2,
        updated_at = NOW()
        WHERE id = $1
        "#,
        id,
        status
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_last_processed_block(
    conn: &mut PgConnection,
    key: &str,
    block_number: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO block_trackers (key, last_block)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE
        SET last_block = $2, updated_at = NOW()
        "#,
        key,
        block_number as i64
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn get_last_processed_block(
    conn: &mut PgConnection,
    key: &str,
) -> Result<Option<u64>, sqlx::Error> {
    let record = sqlx::query!(
        r#"
        SELECT last_block FROM block_trackers
        WHERE key = $1
        "#,
        key
    )
    .fetch_optional(conn)
    .await?;

    Ok(record.map(|r| r.last_block as u64))
}
