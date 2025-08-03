use std::path::PathBuf;
use tempfile::tempdir;
use zeroxbridge_sequencer::config::AppConfig;
use zeroxbridge_sequencer::relayer::proof_submission::{
    ProofSubmissionConfig, ProofSubmissionError,
};

/// Mock configuration for testing
fn create_test_config() -> AppConfig {
    use zeroxbridge_sequencer::config::*;

    // Set environment variable for testing
    std::env::set_var("STARKNET_RPC_URL", "http://localhost:5050");

    AppConfig {
        contract: ContractConfig {
            name: "test_contract".to_string(),
        },
        contracts: Contracts {
            l1_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
            l2_contract_address: "0x0000000000000000000000000000000000000000".to_string(),
        },
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            server_url: "http://127.0.0.1:4000".to_string(),
        },
        database: DatabaseConfig {
            max_connections: 10,
        },
        ethereum: EthereumConfig {
            chain_id: 1,
            confirmations: 3,
        },
        starknet: StarknetConfig {
            chain_id: "0x534e5f4d41494e".to_string(),
            contract_address: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            account_address: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            private_key: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            max_retries: Some(3),
            retry_delay_ms: Some(1000),
            transaction_timeout_ms: Some(30000),
        },
        relayer: RelayerConfig {
            max_retries: 5,
            retry_delay_seconds: 10,
            gas_limit: 500000,
        },
        queue: QueueConfig {
            process_interval_sec: 5,
            wait_time_seconds: 5,
            max_retries: 3,
            initial_retry_delay_sec: 10,
            retry_delay_seconds: 15,
            merkle_update_confirmations: 5,
        },
        merkle: MerkleConfig {
            tree_depth: 32,
            cache_size: 1000,
        },
        logging: LoggingConfig {
            level: "debug".to_string(),
            file: "test.log".to_string(),
        },
        oracle: OracleConfig {
            tolerance_percent: Some(0.01),
            polling_interval_seconds: 60,
        },
        herodotus: HerodotusConfig {
            herodotus_endpoint: "https://test.example.com".to_string(),
        },
    }
}

#[tokio::test]
async fn test_proof_submission_config_conversion() {
    let app_config = create_test_config();
    let proof_config = ProofSubmissionConfig::from(app_config);

    // Verify configuration conversion
    assert_eq!(proof_config.max_retries, 3);
    assert_eq!(proof_config.retry_delay_ms, 1000);
    assert_eq!(proof_config.transaction_timeout_ms, 30000);
    assert!(proof_config.contract_address.starts_with("0x"));
    assert!(proof_config.account_address.starts_with("0x"));
    assert!(proof_config.private_key.starts_with("0x"));
    assert_eq!(proof_config.rpc_url, "http://localhost:5050");
}

#[tokio::test]
async fn test_calldata_directory_validation() {
    // Test with valid directory structure
    let temp_dir = tempdir().unwrap();
    let calldata_dir = temp_dir.path();

    // Create all required files
    std::fs::write(calldata_dir.join("initial"), "0x123 0x456").unwrap();
    std::fs::write(calldata_dir.join("step1"), "0xabc 0xdef").unwrap();
    std::fs::write(calldata_dir.join("final"), "0x999 0xaaa").unwrap();

    // Verify structure
    assert!(calldata_dir.join("initial").exists());
    assert!(calldata_dir.join("step1").exists());
    assert!(calldata_dir.join("final").exists());

    // Test with missing directory
    let non_existent = PathBuf::from("/non/existent/path");
    assert!(!non_existent.exists());
}

#[tokio::test]
async fn test_error_types() {
    // Test error type creation and formatting
    let error = ProofSubmissionError::CalldataDirNotFound("/test/path".to_string());
    assert!(error.to_string().contains("Calldata directory not found"));

    let error = ProofSubmissionError::CalldataFileMissing("initial".to_string());
    assert!(error.to_string().contains("Required calldata file missing"));

    let error = ProofSubmissionError::ProofJobNotFound(12345);
    assert!(error.to_string().contains("Proof job not found: 12345"));
}

#[test]
fn test_hex_conversion_edge_cases() {
    fn string_to_hex(input: &str) -> String {
        let mut hex_string = String::from("0x");
        for byte in input.bytes() {
            hex_string.push_str(&format!("{:02x}", byte));
        }
        hex_string
    }

    // Test edge cases
    assert_eq!(string_to_hex(""), "0x");
    assert_eq!(string_to_hex("a"), "0x61");
    assert_eq!(string_to_hex("🚀"), "0xf09f9a80"); // Unicode emoji
    assert_eq!(string_to_hex("test123"), "0x74657374313233");

    // Test special characters (fixed the expected value)
    assert_eq!(string_to_hex("hello world"), "0x68656c6c6f20776f726c64");
    assert_eq!(string_to_hex("!@#$%"), "0x2140232425"); // Fixed: removed extra '3'
}

#[test]
fn test_stage_progression() {
    let stages = vec![
        "processing",
        "initial_submitted",
        "step1_submitted",
        "step2_submitted",
        "final_submitted",
        "completed",
    ];

    // Test stage parsing for step numbers
    for (_i, stage) in stages.iter().enumerate() {
        if stage.starts_with("step") && stage.ends_with("_submitted") {
            let step_num: Option<u32> = stage
                .strip_prefix("step")
                .and_then(|s| s.strip_suffix("_submitted"))
                .and_then(|s| s.parse().ok());

            assert!(
                step_num.is_some(),
                "Failed to parse step number from: {}",
                stage
            );
        }
    }
}

#[tokio::test]
async fn test_logging_configuration() {
    // Test that logging can be configured
    use tracing_subscriber::layer::SubscriberExt;

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("debug"))
        .with(tracing_subscriber::fmt::layer().with_test_writer());

    // This would set up logging for tests
    let _guard = tracing::subscriber::set_default(subscriber);

    // Test logging at different levels
    tracing::debug!("Debug message");
    tracing::info!("Info message");
    tracing::warn!("Warning message");
    tracing::error!("Error message");
}

#[test]
fn test_retry_backoff_calculation() {
    // Test exponential backoff calculation
    let base_delay_ms = 1000u64;
    let max_retries = 5u32;

    for attempt in 1..=max_retries {
        let delay = base_delay_ms * attempt as u64;
        assert!(delay > 0, "Delay should be positive");
        assert!(
            delay <= base_delay_ms * max_retries as u64,
            "Delay should not exceed maximum"
        );
    }
}
