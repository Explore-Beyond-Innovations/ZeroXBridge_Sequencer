mod api;
mod config;
mod db;
mod proof_generator;
mod queue;
mod relayer;
mod merkle_tree;
mod oracle_service;

use crate::proof_generator::StarkProver;
use crate::relayer::starknet_relayer::{StarknetRelayer, StarknetRelayerConfig};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::spawn;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env (if using dotenv crate)
    dotenv::dotenv().ok();

    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting ZeroXBridge Sequencer");

    // Compile Cairo project using StarkProver
    let cairo_project_dir = env::var("CAIRO_PROJECT_DIR").unwrap_or_else(|_| "crates/cairo1-rust-vm".to_string());
    let cairo_path = PathBuf::from(&cairo_project_dir);

    if cairo_path.exists() {
        info!("🧱 Found Cairo project at: {:?}", cairo_path);

        let prover = StarkProver::new(cairo_path);
        match prover.compile_cairo() {
            Ok(output_path) => {
                info!("✅ Cairo compilation successful!");
                info!("Output Sierra JSON file: {:?}", output_path);
            }
            Err(err) => {
                error!("❌ Cairo compilation failed: {}", err);
                // Optional: Exit if compilation fails
                // return Err(Box::new(err));
            }
        }
    } else {
        error!("❌ Cairo project directory not found at: {:?}", cairo_path);
    }

    // === Database Setup ===
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    info!("🔄 Running database migrations...");
    sqlx::migrate!("./migrations").run(&db_pool).await?;

    let db_pool_arc = Arc::new(db_pool);

    // === Start Starknet Relayer ===
    spawn_starknet_relayer(db_pool_arc.clone()).await?;

    // === Add more services here ===
    // spawn_api();
    // spawn_queue_handler();
    // ...

    info!("✅ All services started successfully");

    // Keep alive until Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("🛑 Shutting down ZeroXBridge Sequencer");

    Ok(())
}

async fn spawn_starknet_relayer(db_pool: Arc<Pool<Postgres>>) -> Result<(), Box<dyn Error>> {
    let config = StarknetRelayerConfig {
        bridge_contract_address: env::var("STARKNET_BRIDGE_CONTRACT")
            .expect("STARKNET_BRIDGE_CONTRACT must be set"),
        rpc_url: env::var("STARKNET_RPC_URL").expect("STARKNET_RPC_URL must be set"),
        private_key: env::var("STARKNET_PRIVATE_KEY").expect("STARKNET_PRIVATE_KEY must be set"),
        max_retries: env::var("STARKNET_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .expect("STARKNET_MAX_RETRIES must be a valid number"),
        retry_delay_ms: env::var("STARKNET_RETRY_DELAY_MS")
            .unwrap_or_else(|_| "5000".to_string())
            .parse()
            .expect("STARKNET_RETRY_DELAY_MS must be a valid number"),
        transaction_timeout_ms: env::var("STARKNET_TX_TIMEOUT_MS")
            .unwrap_or_else(|_| "60000".to_string())
            .parse()
            .expect("STARKNET_TX_TIMEOUT_MS must be a valid number"),
        account_address: env::var("STARKNET_ACCOUNT_ADDRESS")
            .expect("STARKNET_ACCOUNT_ADDRESS must be set"),
    };

    let relayer = StarknetRelayer::new(db_pool.as_ref().clone(), config).await.map_err(|e| {
        error!("❌ Failed to initialize Starknet relayer: {:?}", e);
        Box::new(e) as Box<dyn Error>
    })?;

    spawn(async move {
        info!("🔁 Starting Starknet relayer service");
        if let Err(e) = relayer.start().await {
            error!("⚠️ Starknet relayer service stopped with error: {:?}", e);
        }
    });

    info!("✅ Starknet relayer service spawned");

    Ok(())
}
