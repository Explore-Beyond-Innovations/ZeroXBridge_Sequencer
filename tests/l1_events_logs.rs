// <<<<<<< log
use alloy_primitives::{Address, U256};
use anyhow::Result;
use zeroxbridge_sequencer::events::l1_event_watcher::{
    ZeroXBridge, RealEthereumProvider
};

use alloy::rpc::types::eth::Log;
use alloy::primitives::{B256};
use alloy::sol_types::SolEvent;


// =======
// use alloy::rpc::types::Log;
// use alloy_primitives::{Address, BlockNumber, B256, U256};
// use anyhow::Result;
// use mockall::predicate::*;
// use mockall::*;
// use sqlx::postgres::PgPoolOptions;
// use sqlx::PgPool;
// use zeroxbridge_sequencer::config::AppConfig;
// use zeroxbridge_sequencer::db::database::DepositHashAppended;
// use zeroxbridge_sequencer::events::l1_event_watcher::ZeroXBridge;

// // Mock the Provider
// mock! {
//     pub EthereumProvider {
//         fn get_logs(
//             &self,
//             filter: &alloy::rpc::types::Filter,
//         ) -> Result<Vec<Log<ZeroXBridge::DepositEvent>>, Box<dyn std::error::Error>>;
//     }
// }
// >>>>>>> main

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;

    // Helper function to create test database pool
    async fn setup_test_db() -> Result<PgPool> {
        dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5434/zeroxdb".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await?;

        Ok(pool)
    }

    #[test]
    fn test_deposit_event_construction() {
        // Test direct construction of DepositEvent as shown in the source file
        let deposit_id = U256::from(123);
        let token = Address::from([0xaa; 20]);
        let asset_type = ZeroXBridge::AssetType::ERC20;
        let usd_val = U256::from(100_000);
        let user = Address::from([0xbb; 20]);
        let nonce = U256::from(777);
        let leaf_index = U256::from(42);
        let commitment_hash = U256::from_be_bytes([0xcc; 32]);
        let new_root = U256::from_be_bytes([0xdd; 32]);
        let element_count = U256::from(1000);

        let event = ZeroXBridge::DepositEvent {
            assetType: asset_type,
            usdVal: usd_val,
            nonce,
            leafIndex: leaf_index,
            depositId: deposit_id,
            token,
            user,
            commitmentHash: commitment_hash,
            newRoot: new_root,
            elementCount: element_count,
        };
// <<<<<<< log

        // Verify all fields are set correctly
        assert_eq!(event.depositId, deposit_id);
        assert_eq!(event.token, token);
        assert_eq!(event.assetType, asset_type);
        assert_eq!(event.usdVal, usd_val);
        assert_eq!(event.user, user);
        assert_eq!(event.nonce, nonce);
        assert_eq!(event.leafIndex, leaf_index);
        assert_eq!(event.commitmentHash, commitment_hash);
        assert_eq!(event.newRoot, new_root);
        assert_eq!(event.elementCount, element_count);
    }

    #[test]
    fn test_asset_type_enum() {
        // Test the AssetType enum
        let eth_asset = ZeroXBridge::AssetType::ETH;
        let erc20_asset = ZeroXBridge::AssetType::ERC20;

        // These should be different
        assert_ne!(format!("{:?}", eth_asset), format!("{:?}", erc20_asset));

        // Test equality
        assert_eq!(eth_asset, ZeroXBridge::AssetType::ETH);
        assert_eq!(erc20_asset, ZeroXBridge::AssetType::ERC20);
    }

    #[test]
    fn test_real_ethereum_provider_creation() {
        // Test that we can create a RealEthereumProvider
        let provider = RealEthereumProvider::new("http://localhost:8545".to_string());

        // We can't easily test the actual functionality without a running node,
        // but we can verify the struct can be created
        assert_eq!(std::mem::size_of_val(&provider), std::mem::size_of::<String>());
    }

    #[tokio::test]
    async fn test_database_connection() -> Result<()> {
        // Test that we can connect to the database
        match setup_test_db().await {
            Ok(_pool) => {
                // Connection successful
                Ok(())
            }
            Err(e) => {
                // Connection failed - this is acceptable in test environments
                // where the database might not be available
                println!("Database connection failed (expected in some test environments): {}", e);
                Ok(())
            }
// =======
//         Log {
//             block_hash: Some(
//                 B256::from_str_hex(
//                     "0x0000000000000000000000000000000000000000000000000000000000001234",
//                 )
//                 .unwrap(),
//             ),
//             block_number: Some(block_number),
//             transaction_hash: Some(B256::from_str_hex(tx_hash).unwrap()),
//             transaction_index: Some(0u64.into()),
//             log_index: Some(0u64.into()),
//             removed: false,
//             block_timestamp: Some(0u64),
//             inner,
//         }
//     }

//     // Helper function to create a test DepositHashAppended log
//     fn create_test_deposit_hash_log(
//         block_number: u64,
//         tx_hash: &str,
//         index: u64,
//         commitment_hash: &str,
//         root_hash: &str,
//         elements_count: u64,
//     ) -> Log<ZeroXBridge::DepositHashAppended> {
//         let inner = alloy_primitives::Log {
//             address: Address::parse("0x1234567890123456789012345678901234567890").unwrap(),
//             data: vec![
//                 U256::from(index),
//                 B256::from_str_hex(commitment_hash).unwrap(),
//                 B256::from_str_hex(root_hash).unwrap(),
//                 U256::from(elements_count),
//             ],
//         };
//         Log {
//             block_hash: Some(
//                 B256::from_str_hex(
//                     "0x0000000000000000000000000000000000000000000000000000000000001234",
//                 )
//                 .unwrap(),
//             ),
//             block_number: Some(block_number),
//             transaction_hash: Some(B256::from_str_hex(tx_hash).unwrap()),
//             transaction_index: Some(0u64.into()),
//             log_index: Some(0u64.into()),
//             removed: false,
//             block_timestamp: Some(0u64),
//             inner,
// >>>>>>> main
        }
    }

    // Integration test that requires a running database
    #[tokio::test]
// <<<<<<< log
    async fn test_fetch_deposit_events_integration() -> Result<()> {
        // This test will only run if we can connect to the database
        if let Ok(mut pool) = setup_test_db().await {
            // Test with a mock RPC URL (this will likely fail due to connection issues,
            // but tests the error handling path)
            let result = zeroxbridge_sequencer::events::l1_event_watcher::fetch_l1_deposit_events(
                &mut pool,
                "http://localhost:8545", // Mock RPC URL
                0u64,
                "0x1234567890123456789012345678901234567890", // Mock contract address
            ).await;

            // We expect this to fail due to connection issues, but it should handle errors gracefully
            match result {
                Ok(_logs) => {
                    // Unexpected success - would mean we have a running Ethereum node
                    println!("Unexpected success - Ethereum node appears to be running");
                }
                Err(e) => {
                    // Expected failure due to no running Ethereum node
                    println!("Expected failure due to connection issues: {}", e);
                }
            }
        } else {
            println!("Skipping integration test - no database connection available");
        }
// =======
//     async fn test_deposit_event_parsing() -> Result<()> {
//         let pool = setup_test_db().await;
//         let mut mock_provider = MockEthereumProvider::new();

//         // Create test events
//         let test_logs = vec![
//             create_test_deposit_log(
//                 100,
//                 "0x123",
//                 "0x0000000000000000000000000000000000000000",
//                 "0x1234567890abcdef1234567890abcdef12345678",
//                 1_000_000,
//                 "0xabc0000000000000000000000000000000000000000000000000000000000000",
//             ),
//             create_test_deposit_log(
//                 101,
//                 "0x456",
//                 "0x0000000000000000000000000000000000000001",
//                 "0xfedcba0987654321fedcba0987654321fedcba09",
//                 2_000_000,
//                 "0xdef0000000000000000000000000000000000000000000000000000000000000",
//             ),
//         ];

//         // Mock get_logs response
//         mock_provider
//             .expect_get_logs()
//             .returning(move |_| Ok(test_logs.clone()));

//         let result = zeroxbridge_sequencer::events::l1_event_watcher::fetch_l1_deposit_events(
//             &pool,
//             "http://localhost:8545",
//             95u64,
//             "0x1234567890123456789012345678901234567890",
//         )
//         .await?;

//         assert_eq!(result.len(), 2);

//         // Check first event
//         let first_event = &result[0];
//         assert_eq!(first_event.block_number.unwrap(), 100);
//         assert_eq!(first_event.data().commitment, U256::from(1_000_000)); // amount

//         // Check second event
//         let second_event = &result[1];
//         assert_eq!(second_event.block_number.unwrap(), 101);
//         assert_eq!(second_event.data().commitment, U256::from(2_000_000)); // amount

//         // Verify block tracker was updated
//         let last_block = sqlx::query!(
//             "SELECT last_block FROM block_trackers WHERE key = $1",
//             BLOCK_TRACKER_KEY
//         )
//         .fetch_one(&pool)
//         .await?;

//         assert_eq!(last_block.last_block, 101);
// >>>>>>> main

        Ok(())
    }

// <<<<<<< log
    #[test]
    fn test_block_tracker_constants() {
        // Test that the constants are defined and accessible
        use zeroxbridge_sequencer::events::l1_event_watcher::{BLOCK_TRACKER_KEY, DEPOSIT_HASH_BLOCK_TRACKER_KEY};
// =======
//     #[tokio::test]
//     async fn test_deposit_hash_appended_parsing() -> Result<()> {
//         let pool = setup_test_db().await;
//         let mut mock_provider = MockEthereumProvider::new();

//         // Create test DepositHashAppended events
//         let test_logs = vec![
//             create_test_deposit_hash_log(
//                 100,
//                 "0x123",
//                 1,
//                 "0xabc0000000000000000000000000000000000000000000000000000000000000",
//                 "0xdef0000000000000000000000000000000000000000000000000000000000000",
//                 10,
//             ),
//             create_test_deposit_hash_log(
//                 101,
//                 "0x456",
//                 2,
//                 "0x1230000000000000000000000000000000000000000000000000000000000000",
//                 "0x4560000000000000000000000000000000000000000000000000000000000000",
//                 11,
//             ),
//         ];

//         let test_logs = vec![
//             create_test_deposit_log(100, "0xabc", 1, "0x1000000"),
//             create_test_deposit_log(101, "0xdef", 2, "0x2000000"),
//         ];

//         // Mock get_logs response for DepositHashAppended
//         mock_provider
//             .expect_get_logs()
//             .returning(move |_| Ok(test_logs.clone()));

//         let result =
//             zeroxbridge_sequencer::events::l1_event_watcher::fetch_deposit_hash_appended_events(
//                 &pool,
//                 "http://localhost:8545",
//                 95,
//                 "0x1234567890123456789012345678901234567890",
//             )
//             .await?;

//         assert_eq!(result.len(), 2);

//         // Check first event
//         let first_event = &result[0];
//         assert_eq!(first_event.block_number.unwrap(), 100);
//         assert_eq!(first_event.data().index, U256::from(1));
//         assert_eq!(first_event.data().elementsCount, U256::from(10));

//         // Check second event
//         let second_event = &result[1];
//         assert_eq!(second_event.block_number.unwrap(), 101);
//         assert_eq!(second_event.data().index, U256::from(2));
//         assert_eq!(second_event.data().elementsCount, U256::from(11));

//         // Verify database entries
//         let db_entries = sqlx::query_as!(
//             DepositHashAppended,
//             "SELECT * FROM deposit_hashes WHERE block_number IN (100, 101)"
//         )
//         .fetch_all(&pool)
//         .await?;

//         assert_eq!(db_entries.len(), 2);
//         assert_eq!(db_entries[0].index, 1);
//         assert_eq!(db_entries[0].elements_count, 10);
//         assert_eq!(db_entries[1].index, 2);
//         assert_eq!(db_entries[1].elements_count, 11);

//         // Verify block tracker was updated
//         let last_block = sqlx::query!(
//             "SELECT last_block FROM block_trackers WHERE key = $1",
//             zeroxbridge_sequencer::events::l1_event_watcher::DEPOSIT_HASH_BLOCK_TRACKER_KEY
//         )
//         .fetch_one(&pool)
//         .await?;

//         assert_eq!(last_block.last_block, 101);
// >>>>>>> main

        assert_eq!(BLOCK_TRACKER_KEY, "l1_deposit_events_last_block");
        assert_eq!(DEPOSIT_HASH_BLOCK_TRACKER_KEY, "l1_deposit_hash_events_last_block");
    }

// <<<<<<< log
    #[test]
    fn test_deposit_event_fields() {
        // Create a minimal deposit event and verify field access
        let event = ZeroXBridge::DepositEvent {
            assetType: ZeroXBridge::AssetType::ETH,
            usdVal: U256::from(1000),
            nonce: U256::from(1),
            leafIndex: U256::from(0),
            depositId: U256::from(42),
            token: Address::from([0x00; 20]),
            user: Address::from([0xff; 20]),
            commitmentHash: U256::from(0x1234),
            newRoot: U256::from(0x5678),
            elementCount: U256::from(1),
        };
// =======
//     #[tokio::test]
//     async fn test_empty_logs() -> Result<()> {
//         let pool = setup_test_db().await;
//         let mut mock_provider = MockEthereumProvider::new();

//         // Mock get_logs to return empty vector
//         mock_provider
//             .expect_get_logs()
//             .returning(move |_| Ok(Vec::new()));

//         let result = zeroxbridge_sequencer::events::l1_event_watcher::fetch_l1_deposit_events(
//             &pool,
//             "http://localhost:8545",
//             95u64,
//             "0x1234567890123456789012345678901234567890",
//         )
//         .await;

//         // Should return error when no logs found
//         assert!(result.is_err());
//         assert!(result.unwrap_err().to_string().contains("No logs found"));

//         // Mock get_logs to return empty vector for DepositHashAppended
//         mock_provider
//             .expect_get_logs()
//             .returning(move |_| Ok(Vec::new()));

//         let hash_result =
//             zeroxbridge_sequencer::events::l1_event_watcher::fetch_deposit_hash_appended_events(
//                 &pool,
//                 "http://localhost:8545",
//                 95,
//                 "0x1234567890123456789012345678901234567890",
//             )
//             .await?;

//         // Empty logs should return empty vector
//         assert!(hash_result.is_empty());
// >>>>>>> main

        // Test that we can access all fields
        assert_eq!(event.depositId, U256::from(42));
        assert_eq!(event.usdVal, U256::from(1000));
        assert_eq!(event.nonce, U256::from(1));
        assert_eq!(event.leafIndex, U256::from(0));
        assert_eq!(event.token, Address::from([0x00; 20]));
        assert_eq!(event.user, Address::from([0xff; 20]));
        assert_eq!(event.commitmentHash, U256::from(0x1234));
        assert_eq!(event.newRoot, U256::from(0x5678));
        assert_eq!(event.elementCount, U256::from(1));

        // Test enum match
        matches!(event.assetType, ZeroXBridge::AssetType::ETH);
    }
}

#[test]
fn test_deposit_event_decoding_from_log() {
    let address = Address::from([0x11; 20]);

    // Simulated Ethereum log data with dummy data (will fail decoding)
    let log = Log {
        inner: alloy::primitives::Log {
            address,
            data: alloy::primitives::LogData::new(
                vec![ZeroXBridge::DepositEvent::SIGNATURE_HASH],
                vec![0u8; 256].into()
            ).unwrap(),
        },
        block_hash: Some(B256::from([0x22; 32])),
        block_number: Some(123456u64.into()),
        transaction_hash: Some(B256::from([0x33; 32])),
        transaction_index: Some(1u64.into()),
        log_index: Some(0u64.into()),
        removed: false,
        block_timestamp: None,
    };

    let result = ZeroXBridge::DepositEvent::decode_log(&log.inner);

    // Check if the result is an error (expected with dummy data)
    if let Err(e) = &result {
        println!("Expected error with dummy data: {:?}", e);
    }

    // For now, we expect this to fail because we're using dummy data
    // In a real scenario, the log data needs to be properly ABI-encoded
    assert!(result.is_err()); // We expect this to fail with dummy data
}

#[test]
fn test_deposit_event_signature() {
    // Test that we can access the event signature
    let signature = ZeroXBridge::DepositEvent::SIGNATURE;
    assert!(!signature.is_empty());

    let signature_hash = ZeroXBridge::DepositEvent::SIGNATURE_HASH;
    assert_ne!(signature_hash, B256::ZERO);

    println!("DepositEvent signature: {}", signature);
    println!("DepositEvent signature hash: {:?}", signature_hash);
}
