use alloy::rpc::types::Log;
use alloy_primitives::{Address, B256, U256};
use anyhow::Result;
use mockall::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use alloy::providers::Provider;
use zeroxbridge_sequencer::db::database::DepositHashAppended;
use zeroxbridge_sequencer::events::l1_event_watcher::{
    fetch_l1_deposit_events, BLOCK_TRACKER_KEY, DEPOSIT_HASH_BLOCK_TRACKER_KEY,
};

// —— Mock the alloy Provider trait ——
mock! {
    pub MyProvider { }
    impl Provider for MyProvider {
        fn get_logs(
            &self,
            filter: &alloy::rpc::types::Filter,
        ) -> futures::future::BoxFuture<
            Result<Vec<alloy::rpc::types::Log<()>>, Box<dyn std::error::Error>>
        >;

        fn root(&self) -> &dyn Provider {
            unimplemented!("MockMyProvider::root is not implemented")
        }
    }
}

// Spin up a throwaway Postgres pool
async fn setup_test_db() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5435/zeroxdb".into());
    PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("db connect")
}

// Build a DepositEvent log
fn make_deposit_log(
    block: u64,
    tx: &str,
    amt: u64,
    commitment: &str,
) -> Log<zeroxbridge_sequencer::events::l1_event_watcher::ZeroXBridge::DepositEvent> {
    let inner = alloy_primitives::Log {
        address: Address::zero(),
        data: vec![
            U256::from(0u64),
            U256::from(amt),
            U256::from_str_hex(commitment).unwrap(),
        ],
    };
    Log {
        block_hash: None,
        block_number: Some(block),
        transaction_hash: Some(B256::from_str_hex(tx).unwrap()),
        transaction_index: Some(0u64.into()),
        log_index: Some(0u64.into()),
        removed: false,
        block_timestamp: None,
        inner,
    }
}

// Build a DepositHashAppended log
fn make_hash_log(
    block: u64,
    tx: &str,
    idx: u64,
    comm: &str,
    root: &str,
    cnt: u64,
) -> Log<zeroxbridge_sequencer::events::l1_event_watcher::ZeroXBridge::DepositHashAppended> {
    let inner = alloy_primitives::Log {
        address: Address::zero(),
        data: vec![
            U256::from(idx),
            B256::from_str_hex(comm).unwrap().into(),
            B256::from_str_hex(root).unwrap().into(),
            U256::from(cnt),
        ],
    };
    Log {
        block_hash: None,
        block_number: Some(block),
        transaction_hash: Some(B256::from_str_hex(tx).unwrap()),
        transaction_index: Some(0u64.into()),
        log_index: Some(0u64.into()),
        removed: false,
        block_timestamp: None,
        inner,
    }
}

#[tokio::test]
async fn test_deposit_and_hash_fetch() -> Result<()> {
    let mut mock = MockMyProvider::new();

    // Prepare two deposit logs, two hash logs
    let deps = vec![
        make_deposit_log(100, "0x01", 1_000_000, "0xabc0"),
        make_deposit_log(101, "0x02", 2_000_000, "0xdef0"),
    ];
    let hashes = vec![
        make_hash_log(100, "0x11", 1, "0xaaa0", "0xbbb0", 10),
        make_hash_log(101, "0x12", 2, "0xccc0", "0xddd0", 11),
    ];

    // First get_logs → deposits; second call → hashes
    mock.expect_get_logs()
        .times(1)
        .returning(move |_| futures::future::ready(Ok(deps.clone())));
    mock.expect_get_logs()
        .times(1)
        .returning(move |_| futures::future::ready(Ok(hashes.clone())));

    let mut pool = setup_test_db().await;
    let (got_deps, got_hashes) =
        fetch_l1_deposit_events(&mut pool, &mock, 95, Address::zero()).await?;

    assert_eq!(got_deps.len(), 2);
    assert_eq!(got_hashes.len(), 2);

    // Trackers in DB should now reflect block 101
    let last_dep = sqlx::query!(
        "SELECT last_block FROM block_trackers WHERE key = $1",
        BLOCK_TRACKER_KEY
    )
    .fetch_one(&pool)
    .await?;
    let last_hash = sqlx::query!(
        "SELECT last_block FROM block_trackers WHERE key = $1",
        DEPOSIT_HASH_BLOCK_TRACKER_KEY
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(last_dep.last_block, 101);
    assert_eq!(last_hash.last_block, 101);

    Ok(())
}
