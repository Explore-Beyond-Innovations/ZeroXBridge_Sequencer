use crate::db::database::{
    get_last_processed_block, insert_deposit_hash_event, update_last_processed_block,
    DepositHashAppended,
};
use anyhow::Result;
use sqlx::PgPool;
use tracing::log::{debug, warn};

use std::str::FromStr;

use alloy::{
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};

// Name of the table for storing block tracker
pub const BLOCK_TRACKER_KEY: &str = "l1_deposit_events_last_block";
pub const DEPOSIT_HASH_BLOCK_TRACKER_KEY: &str = "l1_deposit_hash_events_last_block";

sol! {
    #[derive(Debug)]
    contract ZeroXBridge {
        enum AssetType {
            ETH,
            ERC20
        }

        event DepositEvent(
            address indexed token, AssetType assetType, uint256 amount, address indexed user, bytes32 commitmentHash
        );

        event DepositHashAppended(
            uint256 index,
            bytes32 commitmentHash,
            bytes32 rootHash,
            uint256 elementsCount
        );
    }
}

pub async fn fetch_l1_deposit_events(
    db_pool: &mut PgPool,
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
) -> Result<
    (
        Vec<Log<ZeroXBridge::DepositEvent>>,
        Vec<Log<ZeroXBridge::DepositHashAppended>>,
    ),
    Box<dyn std::error::Error>,
> {
    // Load last processed block for DepositEvent
    let from_block_deposit = match get_last_processed_block(db_pool, BLOCK_TRACKER_KEY).await {
        Ok(Some(last_block)) => last_block + 1,
        Ok(None) => from_block,
        Err(e) => {
            warn!("Failed to get last processed block for DepositEvent: {}", e);
            from_block
        }
    };

    // Load last processed block for DepositHashAppended
    let from_block_hash =
        match get_last_processed_block(db_pool, DEPOSIT_HASH_BLOCK_TRACKER_KEY).await {
            Ok(Some(last_block)) => last_block + 1,
            Ok(None) => from_block,
            Err(e) => {
                warn!(
                    "Failed to get last processed block for DepositHashAppended: {}",
                    e
                );
                from_block
            }
        };

    // Fetch DepositEvent logs
    let event_name = ZeroXBridge::DepositEvent::SIGNATURE;
    let deposit_logs =
        fetch_events_logs_at_address(rpc_url, from_block_deposit, contract_addr, event_name)
            .await?;

    // Fetch and store DepositHashAppended logs
    let hash_logs =
        fetch_deposit_hash_appended_events(db_pool, rpc_url, from_block_hash, contract_addr)
            .await?;
    // Update last processed block for DepositEvent
    if let Some(last_log) = deposit_logs.last() {
        let block_number = last_log.block_number.ok_or("Block number not found")?;
        if let Err(e) = update_last_processed_block(db_pool, BLOCK_TRACKER_KEY, block_number).await
        {
            warn!(
                "Failed to update last processed block for DepositEvent: {}",
                e
            );
        }
    }

    Ok((deposit_logs, hash_logs))
}

async fn fetch_events_logs_at_address<T>(
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
    event_name: &str,
) -> Result<Vec<Log<T>>, Box<dyn std::error::Error>>
where
    T: alloy::sol_types::SolEvent,
{
    let contract_addr = Address::from_str(contract_addr)?;

    let provider = ProviderBuilder::new().connect(rpc_url).await?;

    let filter = Filter::new()
        .address(contract_addr)
        .event(event_name)
        .from_block(from_block);

    let logs = provider.get_logs(&filter).await?;
    let decoded_logs = logs
        .into_iter()
        .map(|log| log.log_decode::<T>().map_err(|e| Box::new(e)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(decoded_logs)
}

pub async fn fetch_deposit_hash_appended_events(
    db_pool: &PgPool,
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
) -> Result<Vec<Log<ZeroXBridge::DepositHashAppended>>, Box<dyn std::error::Error>> {
    let event_name = ZeroXBridge::DepositHashAppended::SIGNATURE;
    let logs = fetch_events_logs_at_address(rpc_url, from_block, contract_addr, event_name).await?;

    for log in &logs {
        let event: &ZeroXBridge::DepositHashAppended = log.data();
        let deposit_hash_event = DepositHashAppended {
            id: 0, // Set by database
            index: event.index.to_string().parse::<u64>().unwrap() as i64,
            commitment_hash: event.commitmentHash.0.to_vec(),
            root_hash: event.rootHash.0.to_vec(),
            elements_count: event.elementsCount.to_string().parse::<u64>().unwrap() as i64,
            block_number: log.block_number.ok_or("Block number not found")? as i64,
            created_at: None, // Set by database
            updated_at: None, // Set by database
        };

        if let Err(e) = insert_deposit_hash_event(db_pool, &deposit_hash_event).await {
            warn!("Failed to insert DepositHashAppended event: {}", e);
        } else {
            debug!(
                "Inserted DepositHashAppended: index={}, commitment_hash={:x}, root_hash={:x}, elements_count={}",
                deposit_hash_event.index,
                B256::from_slice(&deposit_hash_event.commitment_hash),
                B256::from_slice(&deposit_hash_event.root_hash),
                deposit_hash_event.elements_count
            );
        }
    }

    // Update last processed block
    if let Some(last_log) = logs.last() {
        let block_number = last_log.block_number.ok_or("Block number not found")?;
        // let mut conn = db_pool.acquire().await?;
        if let Err(e) =
            update_last_processed_block(db_pool, DEPOSIT_HASH_BLOCK_TRACKER_KEY, block_number).await
        {
            warn!(
                "Failed to update last processed block for DepositHashAppended: {}",
                e
            );
        }
    }

    Ok(logs)
}
