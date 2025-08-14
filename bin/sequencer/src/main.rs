use std::path::Path;

use anyhow::Result;
use clap::Parser;
use tokio::signal;
use tracing::{error, info};

use zeroxbridge_sequencer::{
    config::load_config,
    db::database::get_db_pool,
    tree_builder::l1_client::TreeBuilderClient,
    api::routes::create_router,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Enable tree builder service
    #[arg(long)]
    enable_tree_builder: bool,
    
    /// Enable API server
    #[arg(long)]
    enable_api: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt::init();

    let config_path = args.config.as_ref().map(|p| Path::new(p));
    let app_config = load_config(config_path)?;

    let database_url = app_config.database.get_db_url();
    let db_pool = get_db_pool(&database_url).await?;
    sqlx::migrate!().run(&db_pool).await?;

    // Tree Builder Service
    if args.enable_tree_builder {
        let mut tree_builder = TreeBuilderClient::new(db_pool.clone(), 10);
        tree_builder.start().await?;
        info!("Tree builder service started");
    }

    // API Service
    if args.enable_api {
        let router = create_router(db_pool.clone());
        let listener = tokio::net::TcpListener::bind(&app_config.server.host).await?;
        
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, router).await {
                error!("Server error: {}", e);
            }
        });

        info!("API service started on {}", app_config.server.host);
    }

    signal::ctrl_c().await.ok();

    info!("Shutting down...");
    Ok(())
}

