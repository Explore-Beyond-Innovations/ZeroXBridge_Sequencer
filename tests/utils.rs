use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use zeroxbridge_sequencer::api::routes::AppState;
use zeroxbridge_sequencer::config::{
    AppConfig, ContractConfig, Contracts, DatabaseConfig, EthereumConfig, HerodotusConfig,
    LoggingConfig, MerkleConfig, OracleConfig, QueueConfig, RelayerConfig, ServerConfig,
    StarknetConfig, TreeBuilderConfig,
};

pub async fn create_test_app() -> Arc<AppState> {
    dotenv().ok();
    let configuration = create_test_config();
    let database_url = configuration.database.get_db_url();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Warning: Failed to connect to test database ({}): {}", database_url, e);
            eprintln!("Using fallback test configuration. Some tests may be skipped.");
            
            // Create a dummy pool for testing that won't actually be used
            // This allows tests to run even without a real database connection
            panic!("Test database connection required for integration tests. Please set up DATABASE_URL or use unit tests only.");
        });

    let state = Arc::new(AppState {
        db: pool.clone(),
        config: configuration.clone(),
    });

    state
}

/// Creates a test app with optional database connection
/// Returns None if database connection fails, allowing tests to handle gracefully
pub async fn try_create_test_app() -> Option<Arc<AppState>> {
    dotenv().ok();
    let configuration = create_test_config();
    let database_url = configuration.database.get_db_url();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect(&database_url)
        .await
        .ok()?;

    Some(Arc::new(AppState {
        db: pool,
        config: configuration,
    }))
}

// Helper function to create test config
pub fn create_test_config() -> AppConfig {
    AppConfig {
        contract: ContractConfig {
            name: String::new(),
        },
        contracts: Contracts {
            l1_contract_address: "0x123".to_string(),
            l2_contract_address: "0x456".to_string(),
        },
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            server_url: "http://localhost:8080".to_string(),
        },
        database: DatabaseConfig { max_connections: 5 },
        ethereum: EthereumConfig {
            chain_id: 11155111, // Sepolia testnet
            confirmations: 1,
        },
        starknet: StarknetConfig {
            chain_id: "0x534e5f4d41494e".to_string(),
            contract_address: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            account_address: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            private_key: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            max_retries: Some(5),
            retry_delay_ms: Some(5000),
            transaction_timeout_ms: Some(300000),
        },
        relayer: RelayerConfig {
            max_retries: 3,
            retry_delay_seconds: 60,
            gas_limit: 300000,
        },
        queue: QueueConfig {
            process_interval_sec: 60,
            wait_time_seconds: 30,
            max_retries: 3,
            initial_retry_delay_sec: 60,
            retry_delay_seconds: 60,
            merkle_update_confirmations: 1,
        },
        merkle: MerkleConfig {
            tree_depth: 32,
            cache_size: 1000,
        },
        logging: LoggingConfig {
            level: "info".to_string(),
            file: "logs/zeroxbridge.log".to_string(),
        },
        oracle: OracleConfig {
            tolerance_percent: Some(0.01), // 1% tolerance
            polling_interval_seconds: 60,
        },
        herodotus: HerodotusConfig {
            herodotus_endpoint: "https://herodotus.example.com/api".to_string(),
        },
        tree_builder: Some(TreeBuilderConfig {
            poll_interval_seconds: Some(10),
            batch_size: Some(100),
            max_retries: Some(3),
            retry_backoff_ms: Some(1000),
            max_checkpoints: Some(1000),
        }),
    }
}
