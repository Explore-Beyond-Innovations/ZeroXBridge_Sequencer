use config::{Config, Environment, File};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Loads configuration from a given config file or environment variables.
pub fn load_config(config_file_path: Option<&Path>) -> anyhow::Result<AppConfig> {
    // Load .env file if it exists, ignore if not present
    dotenv().ok();

    let mut settings = Config::builder();

    if let Some(path) = config_file_path {
        settings = settings.add_source(File::from(path).required(true));
    }

    // Add environment variables with prefix ZEROOXBRIDGE
    settings = settings.add_source(Environment::with_prefix("ZEROOXBRIDGE").separator("__"));
    settings = settings.add_source(Environment::with_prefix("HERODOTUS").separator("__"));

    let app_config = settings.build()?.try_deserialize::<AppConfig>()?;

    Ok(app_config)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub contract: ContractConfig,
    pub contracts: Contracts,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub starknet: StarknetConfig,
    pub relayer: RelayerConfig,
    pub queue: QueueConfig,
    pub merkle: MerkleConfig,
    pub logging: LoggingConfig,
    pub oracle: OracleConfig,
    pub herodotus: HerodotusConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HerodotusConfig {
    pub herodotus_endpoint: String,
}

impl HerodotusConfig {
    pub fn get_api_key(&self) -> String {
        std::env::var("HERODOTUS_API_KEY")
            .unwrap_or_else(|_| panic!("HERODOTUS_API_KEY is not set in environment or .env file"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contracts {
    pub l1_contract_address: String,
    pub l2_contract_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub server_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn get_db_url(&self) -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| panic!("DATABASE_URL is not set in environment or .env file"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub chain_id: u64,
    pub confirmations: u32,
}

impl EthereumConfig {
    pub fn get_rpc_url(&self) -> String {
        std::env::var("ETHEREUM_RPC_URL")
            .unwrap_or_else(|_| panic!("ETHEREUM_RPC_URL is not set in environment or .env file"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarknetConfig {
    pub chain_id: String,
    /// Contract address for proof verification
    pub contract_address: String,
    /// Account address for submitting transactions
    pub account_address: String,
    /// Private key for the account (should be set via environment variable in production)
    pub private_key: String,
    /// Maximum number of retry attempts for failed transactions
    pub max_retries: Option<u32>,
    /// Delay between retry attempts in milliseconds
    pub retry_delay_ms: Option<u64>,
    /// Timeout for transaction confirmation in milliseconds
    pub transaction_timeout_ms: Option<u64>,
}

impl StarknetConfig {
    pub fn get_rpc_url(&self) -> String {
        std::env::var("STARKNET_RPC_URL")
            .unwrap_or_else(|_| panic!("STARKNET_RPC_URL is not set in environment or .env file"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConfig {
    pub process_interval_sec: u64,
    pub wait_time_seconds: u32,
    pub max_retries: u32,
    pub initial_retry_delay_sec: u64,
    pub retry_delay_seconds: u32,
    pub merkle_update_confirmations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleConfig {
    pub tree_depth: u32,
    pub cache_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String, // "debug" | "info" | "warn" | "error"
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleConfig {
    pub tolerance_percent: Option<f64>, // e.g., 0.01 for 1%
    pub polling_interval_seconds: u64,  // e.g., 60 seconds
}
