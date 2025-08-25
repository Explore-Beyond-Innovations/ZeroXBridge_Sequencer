use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;

use crate::config::AppConfig;

pub type DbPool = PgPool;

/// Database client wrapper
#[derive(Clone)]
pub struct DBClient {
    pub pool: Arc<DbPool>,
}

impl DBClient {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .connect(&config.database.get_db_url())
            .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&*self.pool).await?;
        Ok(())
    }
}
