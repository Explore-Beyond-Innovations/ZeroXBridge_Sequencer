use axum::{routing::get, Extension, Router};
use std::{net::SocketAddr, path::Path, sync::Arc};
use ethers::providers::{Http, Provider};
use ethers::contract::Contract;
use std::sync::Arc;
use crate::oracle_service::oracle_service::sync_tvl;


mod config;
mod db;

use config::load_config;
use db::client::DBClient;
use crate::api::routes::create_router;

#[tokio::main]
async fn main() {

    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

    let l1_provider = Provider::<Http>::try_from(config.ethereum.rpc_url.clone()).expect("Invalid L1 RPC URL");
    let l2_provider = Provider::<Http>::try_from(config.starknet.rpc_url.clone()).expect("Invalid L2 RPC URL");

    let l1_contract = Contract::new(config.contracts.l1_contract_address.parse().unwrap(), l1_abi(), Arc::new(l1_provider));
    let l2_contract = Contract::new(config.contracts.l2_contract_address.parse().unwrap(), l2_abi(), Arc::new(l2_provider));

    tokio::spawn(async move {
        sync_tvl(l1_contract, l2_contract, &config).await.expect("TVL sync failed");
    });


    let config = load_config(Some(Path::new("config.toml"))).expect("Failed to load config");

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
