use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use zeroxbridge_sequencer::{
    config::{load_config, AppConfig},
    db::database::get_db_pool,
    tree_builder::{L1TreeBuilderClient, TreeBuilderConfig},
    api::routes::create_router,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Enable tree builder service
    #[arg(long, default_value = "true")]
    enable_tree_builder: bool,
    
    /// Enable API server
    #[arg(long, default_value = "true")]
    enable_api: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "zeroxbridge_sequencer=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting ZeroXBridge Sequencer");

    // Load configuration
    let config_path = args.config.as_ref().map(|p| Path::new(p));
    let app_config = load_config(config_path)
        .context("Failed to load configuration")?;

    info!("Configuration loaded successfully");

    // Setup database connection pool
    let database_url = app_config.database.get_db_url();
    let db_pool = get_db_pool(&database_url).await
        .context("Failed to create database pool")?;

    info!("Database connection established");

    // Run database migrations
    sqlx::migrate!("../../../migrations").run(&db_pool).await
        .context("Failed to run database migrations")?;

    info!("Database migrations completed");

    // Initialize services
    let mut services = Vec::new();

    // Tree Builder Service
    if args.enable_tree_builder {
        let tree_config = TreeBuilderConfig::from(app_config.tree_builder.clone());

        let mut tree_builder = L1TreeBuilderClient::new(db_pool.clone(), tree_config);
        tree_builder.start().await
            .context("Failed to start tree builder service")?;

        services.push(Box::new(tree_builder) as Box<dyn ServiceManager>);
        info!("Tree builder service started");
    }

    // API Service
    if args.enable_api {
        let api_service = ApiService::new(app_config.clone(), db_pool.clone());
        let api_handle = api_service.start().await
            .context("Failed to start API service")?;

        services.push(Box::new(api_service) as Box<dyn ServiceManager>);
        info!("API service started on {}", app_config.server.host);
    }

    info!("All services started successfully");

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down services...");
    for mut service in services {
        if let Err(e) = service.stop().await {
            error!("Error stopping service: {}", e);
        }
    }

    info!("Sequencer shutdown complete");
    Ok(())
}

// Service management trait
trait ServiceManager: Send {
    async fn stop(&mut self) -> Result<()>;
}

impl ServiceManager for L1TreeBuilderClient {
    async fn stop(&mut self) -> Result<()> {
        L1TreeBuilderClient::stop(self).await
    }
}

// API Service wrapper
struct ApiService {
    config: AppConfig,
    db_pool: sqlx::PgPool,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<Result<()>>>,
}

impl ApiService {
    fn new(config: AppConfig, db_pool: sqlx::PgPool) -> Self {
        Self {
            config,
            db_pool,
            shutdown_tx: None,
            task_handle: None,
        }
    }

    async fn start(&mut self) -> Result<tokio::task::JoinHandle<()>> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let router = create_router(self.db_pool.clone());
        let listener = tokio::net::TcpListener::bind(&self.config.server.host).await
            .context("Failed to bind to address")?;

        let handle = tokio::spawn(async move {
            let server = axum::serve(listener, router);
            
            tokio::select! {
                result = server => {
                    if let Err(e) = result {
                        error!("Server error: {}", e);
                    }
                }
                _ = shutdown_rx => {
                    info!("API server shutdown requested");
                }
            }
        });

        Ok(handle)
    }
}

impl ServiceManager for ApiService {
    async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(task_handle) = self.task_handle.take() {
            match task_handle.await {
                Ok(_) => info!("API service stopped successfully"),
                Err(e) => error!("Error stopping API service: {}", e),
            }
        }

        Ok(())
    }
}
