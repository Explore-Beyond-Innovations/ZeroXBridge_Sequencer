use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Withdrawal {
    pub id: i32,
    pub stark_pub_key: String,
    pub amount: i64,
    pub commitment_hash: String,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
}

