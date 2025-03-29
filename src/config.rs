
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Loads configuration from a given config file or environment variables.
pub fn load_config(config_file_path: Option<&Path>) -> anyhow::Result<AppConfig> {
    let mut settings = Config::builder();

    if let Some(path) = config_file_path {
        settings = settings.add_source(File::from(path).required(true));
    }

    let settings = settings
        .add_source(Environment::with_prefix("ZEROOXBRIDGE").separator("__"))
        .build()?;

    Ok(settings.try_deserialize::<AppConfig>()?)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub contract: ContractConfig,
    pub contracts: Contracts,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub ethereum: EthereumConfig,
    pub starknet: StarknetConfig,
    pub prover: ProverConfig,
    pub relayer: RelayerConfig,
    pub queue: QueueConfig,
    pub merkle: MerkleConfig,
    pub logging: LoggingConfig,
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
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EthereumConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub confirmations: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarknetConfig {
    pub rpc_url: String,
    pub chain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProverConfig {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayerConfig {
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
    pub gas_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueConfig {
    pub wait_time_seconds: u32,
    pub max_retries: u32,
    pub retry_delay_seconds: u32,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleConfig {
    pub tolerance_percent: Option<f64>, // e.g., 0.01 for 1%
    pub polling_interval_seconds: u64, // e.g., 60 seconds
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    // ...existing fields...
    pub oracle: OracleConfig,
}
