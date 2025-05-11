use alloy::rpc::types::{BlockNumber, Log};
use anyhow::Result;
use mockall::predicate::*;
use mockall::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use zeroxbridge_sequencer::config::AppConfig;
use zeroxbridge_sequencer::event_logs::ZeroXBridge;

// Mock the Provider
mock! {
    pub EthereumProvider {
        fn get_logs(
            &self,
            filter: &alloy::rpc::types::Filter,
        ) -> Result<Vec<Log<ZeroXBridge::DepositEvent>>, Box<dyn std::error::Error>>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, B256, U256};

    // Helper function to create test database pool
    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5435/zeroxdb".to_string());

        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    // Helper function to create a test deposit event log
    // Create a complete Log struct with all required fields
    fn create_test_deposit_log(
        block_number: u64,
        tx_hash: &str,
        token: &str,
        user: &str,
        amount: u64,
        commitment: &str,
    ) -> Log<ZeroXBridge::DepositEvent> {
        let inner = alloy_primitives::Log {
            address: Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
            data: vec![
                U256::from(0u64), // AssetType::ETH
                U256::from(amount),
                U256::from_str(commitment).unwrap(),
            ],
        };
        Log {
            block_hash: Some(B256::from_str("0x1234").unwrap()),
            block_number: Some(BlockNumber::Number(block_number.into())),
            transaction_hash: Some(B256::from_str(tx_hash).unwrap()),
            transaction_index: Some(0u64.into()),
            log_index: Some(0u64.into()),
            removed: false,
            block_timestamp: Some(U256::from(0u64)),
            inner,
        }
    }

    #[tokio::test]
    async fn test_deposit_event_parsing() -> Result<()> {
        let pool = setup_test_db().await;
        let mut mock_provider = MockEthereumProvider::new();

        // Create test events
        let test_logs = vec![
            create_test_deposit_log(
                100,
                "0x123",
                "0x0000000000000000000000000000000000000000",
                "0x1234567890abcdef1234567890abcdef12345678",
                1_000_000,
                "0xabc0000000000000000000000000000000000000000000000000000000000000",
            ),
            create_test_deposit_log(
                101,
                "0x456",
                "0x0000000000000000000000000000000000000001",
                "0xfedcba0987654321fedcba0987654321fedcba09",
                2_000_000,
                "0xdef0000000000000000000000000000000000000000000000000000000000000",
            ),
        ];

        // Mock get_logs response
        mock_provider
            .expect_get_logs()
            .returning(move |_| Ok(test_logs.clone()));

        let result = zeroxbridge_sequencer::event_logs::fetch_l1_deposit_events(
            &mut pool.acquire().await?,
            "http://localhost:8545",
            Some(95),
            "0x1234567890123456789012345678901234567890",
        )
        .await?;

        assert_eq!(result.len(), 2);

        // Check first event
        let first_event = &result[0];
        assert_eq!(first_event.block_number.unwrap().to::<u64>(), 100);
        assert_eq!(first_event.data[1], U256::from(1_000_000)); // amount

        // Check second event
        let second_event = &result[1];
        assert_eq!(second_event.block_number.unwrap().to::<u64>(), 101);
        assert_eq!(second_event.data[1], U256::from(2_000_000)); // amount

        // Verify block tracker was updated
        let last_block =
            sqlx::query!("SELECT last_block FROM event_log_block_tracker WHERE id = true")
                .fetch_one(&pool)
                .await?;

        assert_eq!(last_block.last_block, 101);

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_logs() -> Result<()> {
        let pool = setup_test_db().await;
        let mut mock_provider = MockEthereumProvider::new();

        // Mock get_logs to return empty vector
        mock_provider
            .expect_get_logs()
            .returning(move |_| Ok(Vec::new()));

        let result = zeroxbridge_sequencer::event_logs::fetch_l1_deposit_events(
            &mut pool.acquire().await?,
            "http://localhost:8545",
            Some(95),
            "0x1234567890123456789012345678901234567890",
        )
        .await;

        // Should return error when no logs found
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No logs found"));

        Ok(())
    }
}