use axum::{extract::State, routing::get, Extension, Router};
use sqlx::PgPool;
use std::sync::Arc;
use dotenvy::dotenv;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use hyper::Server;
use std::{net::SocketAddr, path::Path};
use ethers::providers::{Http, Provider};
use ethers::contract::Contract;
use crate::oracle_service::oracle_service::sync_tvl;

mod config;
mod api;

use api::routes::withdrawal_routes;
use config::get_db_pool;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

    let l1_provider = Provider::<Http>::try_from(config.ethereum.rpc_url.clone()).expect("Invalid L1 RPC URL");
    let l2_provider = Provider::<Http>::try_from(config.starknet.rpc_url.clone()).expect("Invalid L2 RPC URL");

    let l1_contract = Contract::new(config.contracts.l1_contract_address.parse().unwrap(), l1_abi(), Arc::new(l1_provider));
    let l2_contract = Contract::new(config.contracts.l2_contract_address.parse().unwrap(), l2_abi(), Arc::new(l2_provider));

    tokio::spawn(async move {
        sync_tvl(l1_contract, l2_contract, &config).await.expect("TVL sync failed");
    });

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = get_db_pool()
        .await
        .expect("Failed to connect to database");

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    let state = Arc::new(pool);

    let app = create_router(state.clone());

    let addr = "0.0.0.0:3000".parse().unwrap();

    println!("🚀 Listening on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_router(pool: Arc<PgPool>) -> Router {
    Router::new()
        .merge(withdrawal_routes())
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
}