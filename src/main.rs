use crate::oracle_service::oracle_service::sync_tvl;
use axum::{routing::get, Extension, Router};
use ethers::contract::Contract;
use ethers::providers::{Http, Provider};
use std::sync::Arc;
use std::{net::SocketAddr, path::Path, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;

use crate::api::routes::create_router;
use config::load_config;
use db::client::DBClient;

#[tokio::main]
async fn main() {
    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let l1_provider =
        Provider::<Http>::try_from(config.ethereum.rpc_url.clone()).expect("Invalid L1 RPC URL");
    let l2_provider =
        Provider::<Http>::try_from(config.starknet.rpc_url.clone()).expect("Invalid L2 RPC URL");

    let l1_contract = Contract::new(
        config.contracts.l1_contract_address.parse().unwrap(),
        l1_abi(),
        Arc::new(l1_provider),
    );
    let l2_contract = Contract::new(
        config.contracts.l2_contract_address.parse().unwrap(),
        l2_abi(),
        Arc::new(l2_provider),
    );

    tokio::spawn(async move {
        sync_tvl(l1_contract, l2_contract, &config)
            .await
            .expect("TVL sync failed");
    });

    let pool = get_db_pool().await.expect("Failed to connect to database");

    let db = DBClient::new(&config)
        .await
        .expect("Failed to connect to DB");

    db.run_migrations().await.expect("Failed to run migrations");

    let shared_db = Arc::new(db);
    let app = Router::new()
        .route("/", get(handler))
        .merge(create_router(shared_db.pool.clone()));

    let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    println!("ZeroXBridge Sequencer listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn handler() -> &'static str {
    "Welcome to ZeroXBridge Sequencer"
}
