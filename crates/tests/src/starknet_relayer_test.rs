#[cfg(test)]
mod tests {
    use mockall::mock;
    use mockall::predicate::*;
    use sqlx::{Pool, Postgres};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use zeroxbridge_sequencer::queue::l2_queue::L2Transaction;
    use zeroxbridge_sequencer::relayer::starknet_relayer::StarknetRelayer;
    use zeroxbridge_sequencer::relayer::starknet_relayer::StarknetRelayerConfig;

    // Mock the Starknet provider
    mock! {
        pub StarknetProvider {
            fn execute_transaction(&self, tx_hash: String) -> Result<String, String>;
            fn get_transaction_receipt(&self, tx_hash: String) -> Result<bool, String>;
        }
    }

    // Helper function to create a test database pool
    async fn create_test_db_pool() -> Pool<Postgres> {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL environment variable must be set for tests");

        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to database")
    }

    // Helper function to create sample L2Transaction
    fn create_sample_l2_transaction() -> L2Transaction {
        L2Transaction {
            id: 1,
            stark_pub_key: "0x1234567890".to_string(),
            amount: 1000000000000000000,
            token_address: "0xabcdef1234567890".to_string(),
            status: "ready_for_relay".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            retry_count: 0,
            tx_hash: None,
            error: None,
            proof_data: Some(
                r#"{
                "proof_array": ["0x1", "0x2", "0x3"],
                "merkle_root": "0xabcdef123456789"
            }"#
                .to_string(),
            ),
        }
    }

    fn create_sample_config() -> StarknetRelayerConfig {
        StarknetRelayerConfig {
            bridge_contract_address: "0x1234567890abcdef".to_string(),
            rpc_url: "http://localhost:8545".to_string(),
            private_key: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            max_retries: 3,
            retry_delay_ms: 1000,
            transaction_timeout_ms: 30000,
            account_address: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        }
    }

    #[tokio::test]
    async fn test_fetch_ready_transactions() {
        let config = create_sample_config();
        let pool = create_test_db_pool().await;
        // Create relayer config

        let mock_provider = MockStarknetProvider::new();
        // configure mock_provider expectations...

        // Insert a test transaction
        let test_tx = create_sample_l2_transaction();
        sqlx::query!(
            r#"
            INSERT INTO l2_transactions (
            id, stark_pub_key, amount, token_address, status,
            created_at, updated_at, retry_count, tx_hash, error, proof_data
            ) VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10, $11
            )
            "#,
            test_tx.id,
            test_tx.stark_pub_key,
            test_tx.amount,
            test_tx.token_address,
            test_tx.status,
            test_tx.created_at,
            test_tx.updated_at,
            test_tx.retry_count,
            test_tx.tx_hash,
            test_tx.error,
            test_tx.proof_data
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test transaction");

        let relayer = StarknetRelayer::new(pool.clone(), config)
            .await
            .expect("Failed to create relayer");

        let ready_txs = relayer
            .fetch_ready_transactions()
            .await
            .expect("Failed to fetch");

        assert!(
            ready_txs.iter().any(|tx| tx.id == test_tx.id),
            "Expected transaction ID not found"
        );

        sqlx::query!("DELETE FROM l2_transactions WHERE id = $1", test_tx.id)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_process_transaction_success() {
        let config = create_sample_config();
        let pool = create_test_db_pool().await;

        let relayer = StarknetRelayer::new(pool.clone(), config)
            .await
            .expect("Failed to create relayer");
        // Create a mock provider
        let mut mock_provider = MockStarknetProvider::new();

        mock_provider
            .expect_execute_transaction()
            .returning(|_| Ok("0xsuccesstxhash".to_string()));
        mock_provider
            .expect_get_transaction_receipt()
            .returning(|_| Ok(true));

        // Create test transaction
        let test_tx = create_sample_l2_transaction();

        // Create a mock relayer with customized methods
        // Execute the test
        let result = relayer.process_transaction(&mut test_tx.clone()).await;

        // Verify results
        assert!(result.is_ok());

        // Verify the transaction was marked as completed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");

        assert_eq!(updated_tx.status, "completed");
        assert_eq!(updated_tx.tx_hash, Some("0xsuccesstxhash".to_string()));
    }

    #[tokio::test]
    async fn test_process_transaction_with_retries() {
        let config = create_sample_config();
        let pool = create_test_db_pool().await;
        // Create relayer config

        let mut mock_provider = MockStarknetProvider::new();
        // configure mock_provider expectations...

        let mock_relayer = StarknetRelayer::new(pool.clone(), config)
            .await
            .expect("Failed to create relayer");

        let call_counter = Arc::new(AtomicUsize::new(0));
        let call_counter_clone = Arc::clone(&call_counter);

        mock_provider
            .expect_execute_transaction()
            .returning(move |_| {
                let count = call_counter_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err("Temporary failure".to_string())
                } else {
                    Ok("0xsuccesstxhash".to_string())
                }
            });

        mock_provider
            .expect_get_transaction_receipt()
            .returning(|_| Ok(true));

        // Create test transaction
        let test_tx = create_sample_l2_transaction();

        // Execute the test
        let result = mock_relayer.process_transaction(&mut test_tx.clone()).await;

        // Verify results
        assert!(result.is_ok());

        // Verify the transaction was marked as completed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");

        assert_eq!(updated_tx.status, "completed");
    }

    #[tokio::test]
    async fn test_process_transaction_failure() {
        let config = create_sample_config();
        let pool = create_test_db_pool().await;

        // Create relayer config
        let mock_relayer = StarknetRelayer::new(pool.clone(), config)
            .await
            .expect("Failed to create relayer");

        // Create a mock provider that always fails
        let mut mock_provider = MockStarknetProvider::new();
        mock_provider
            .expect_execute_transaction()
            .returning(|_| Err("Critical failure".to_string()));

        // Create test transaction
        let test_tx = create_sample_l2_transaction();

        // Execute the test
        let result = mock_relayer.process_transaction(&mut test_tx.clone()).await;

        // Verify results
        assert!(result.is_err());

        // Verify the transaction was marked as failed
        let updated_tx = sqlx::query_as!(
            L2Transaction,
            "SELECT * FROM l2_transactions WHERE id = $1",
            test_tx.id
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch updated transaction");

        assert_eq!(updated_tx.status, "failed");
        assert!(updated_tx.error.is_some());
    }
}
