use anyhow::Result;
use mockall::predicate::*;
use mockall::*;
use starknet::core::types::{EmittedEvent, EventFilter, EventsPage, Felt};

use zeroxbridge_sequencer::events::fetch_l2_events;
use zeroxbridge_sequencer::events::l2_event_watcher::TestProvider;

#[path = "utils.rs"]
mod utils;

// Mock the Starknet Provider
mock! {
    pub StarknetProvider {
        fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
        fn get_events(
            &self,
            filter: EventFilter,
            continuation_token: Option<String>,
            chunk_size: u64,
        ) -> Result<EventsPage, Box<dyn std::error::Error + Send + Sync>>;
    }
    impl Clone for StarknetProvider {
        fn clone(&self) -> Self;
    }
}

impl TestProvider for MockStarknetProvider {
    fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        self.block_number()
    }

    fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<EventsPage, Box<dyn std::error::Error + Send + Sync>> {
        self.get_events(filter, continuation_token, chunk_size)
    }
}

// Test module to group all L2 event watcher tests
#[cfg(test)]
mod tests {
    use super::*;
    use utils::create_test_app;

    // Helper function to create a test event
    fn create_test_burn_event(
        block_number: u64,
        tx_hash: &str,
        user: &str,
        amount_low: &str,
        amount_high: &str,
        commitment: &str,
    ) -> EmittedEvent {
        EmittedEvent {
            from_address: Felt::from_hex("0x456").unwrap(),
            keys: vec![Felt::from_hex(
                "0x0099de3f38fed0a76764f614c6bc2b958814813685abc1af6deedab612df44f3",
            )
            .unwrap()],
            data: vec![
                Felt::from_hex(user).unwrap(),
                Felt::from_hex(amount_low).unwrap(),
                Felt::from_hex(amount_high).unwrap(),
                Felt::from_hex(commitment).unwrap(),
            ],
            block_number: Some(block_number),
            block_hash: None,
            transaction_hash: Felt::from_hex(tx_hash).unwrap(),
        }
    }

    #[tokio::test]
    async fn test_commitment_hash_decoding() -> Result<()> {
        let app = create_test_app().await;
        let mut mock_provider = MockStarknetProvider::new();

        // Mock block number response
        mock_provider.expect_block_number().returning(|| Ok(100));

        // Create test events
        let test_events = vec![
            create_test_burn_event(95, "0x123", "0x1234567890abcdef", "0x1000", "0x0", "0xabc"),
            create_test_burn_event(96, "0x456", "0xfedcba0987654321", "0x2000", "0x0", "0xdef"),
        ];

        // Mock get_events response
        mock_provider.expect_get_events().returning(move |_, _, _| {
            Ok(EventsPage {
                events: test_events.clone(),
                continuation_token: None,
            })
        });

        let result = fetch_l2_events(&app.config, &app.db, 90, &mock_provider).await?;

        assert_eq!(result.burn_events.len(), 2);
        assert_eq!(result.burn_events[0].commitment_hash, "0xabc");
        assert_eq!(result.burn_events[0].block_number, 95);
        assert_eq!(result.burn_events[0].transaction_hash, "0x123");
        assert_eq!(result.burn_events[1].commitment_hash, "0xdef");
        assert_eq!(result.burn_events[1].block_number, 96);
        assert_eq!(result.burn_events[1].transaction_hash, "0x456");

        Ok(())
    }

    #[tokio::test]
    async fn test_block_index_tracking() -> Result<()> {
        let app = create_test_app().await;
        let mut mock_provider = MockStarknetProvider::new();

        mock_provider.expect_block_number().returning(|| Ok(100));

        let test_events = vec![create_test_burn_event(
            95,
            "0x123",
            "0x1234567890abcdef",
            "0x1000",
            "0x0",
            "0xabc",
        )];

        mock_provider
            .expect_get_events()
            .times(1)
            .returning(move |_, _, _| {
                Ok(EventsPage {
                    events: test_events.clone(),
                    continuation_token: None,
                })
            });

        // First call: should process blocks 90-95
        let result = fetch_l2_events(&app.config, &app.db, 90, &mock_provider).await?;
        assert_eq!(result.burn_events.len(), 1);

        // Verify block tracker was updated
        let last_block = sqlx::query!(
            "SELECT last_block FROM block_trackers WHERE key = 'l2_events_last_block'"
        )
        .fetch_one(&app.db)
        .await?;

        assert_eq!(result.burn_events[0].block_number, 95);
        assert_eq!(last_block.last_block, 95);

        Ok(())
    }

    #[tokio::test]
    async fn test_large_amount_handling() -> Result<()> {
        let app = create_test_app().await;
        let mut mock_provider = MockStarknetProvider::new();

        mock_provider.expect_block_number().returning(|| Ok(100));

        let test_events = vec![create_test_burn_event(
            95,
            "0x123",
            "0x1234567890abcdef",
            "0xffffffffffffffff",
            "0x1",
            "0xabc",
        )];

        mock_provider.expect_get_events().returning(move |_, _, _| {
            Ok(EventsPage {
                events: test_events.clone(),
                continuation_token: None,
            })
        });

        let result = fetch_l2_events(&app.config, &app.db, 92, &mock_provider).await?;

        assert_eq!(result.burn_events.len(), 1);
        assert_eq!(result.burn_events[0].commitment_hash, "0xabc");
        assert_eq!(result.burn_events[0].block_number, 95);
        assert_eq!(result.burn_events[0].transaction_hash, "0x123");
        assert_eq!(result.burn_events[0].amount_low, "0xffffffffffffffff");
        assert_eq!(result.burn_events[0].amount_high, "0x1");

        Ok(())
    }
}
