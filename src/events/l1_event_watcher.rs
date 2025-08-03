use crate::db::database::{get_last_processed_block, update_last_processed_block, upsert_deposit};
use anyhow::Result;
use sqlx::PgPool;
use tracing::log::{debug, warn};

use std::str::FromStr;

use alloy::{
    primitives::{Address},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};

// Name of the table for storing block tracker
pub const BLOCK_TRACKER_KEY: &str = "l1_deposit_events_last_block";
pub const DEPOSIT_HASH_BLOCK_TRACKER_KEY: &str = "l1_deposit_hash_events_last_block";

sol! {
    #[derive(Debug, PartialEq)]
    contract ZeroXBridge {
        enum AssetType {
            ETH,
            ERC20
        }

        event DepositEvent(
            AssetType assetType,
            uint256 usdVal,
            uint256 nonce,
            uint256 leafIndex,
            uint256 indexed depositId,
            address indexed token,
            address indexed user,
            uint256 commitmentHash,
            uint256 newRoot,
            uint256 elementCount
        );
    }
}

pub async fn fetch_l1_deposit_events(
    db_pool: &mut PgPool,
    rpc_url: &str,
    from_block: u64,
    contract_addr: &str,
) -> Result<Vec<Log<ZeroXBridge::DepositEvent>>, Box<dyn std::error::Error>> {
    // Load last processed block for DepositEvent
    let from_block_deposit = match get_last_processed_block(db_pool, BLOCK_TRACKER_KEY).await {
        Ok(Some(last_block)) => last_block + 1,
        Ok(None) => from_block,
        Err(e) => {
            warn!("Failed to get last processed block for DepositEvent: {}", e);
            from_block
        }
    };

    // Fetch DepositEvent logs
    let event_name = ZeroXBridge::DepositEvent::SIGNATURE;
    let deposit_logs =
        fetch_events_logs_at_address(rpc_url, from_block_deposit, contract_addr, event_name)
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

    for log in &deposit_logs {
        let event: &ZeroXBridge::DepositEvent = log.data();

        debug!(
            "Recieved DepositEvent: depositId={}, token={:?}, assetType={:?}, usdVal={}, user={:?}, nonce={}, leafIndex={}, commitmentHash={:x}, newRoot={:x}, elementCount={}",
            event.depositId,
            event.token,
            event.assetType,
            event.usdVal,
            event.user,
            event.nonce,
            event.leafIndex,
            event.commitmentHash,
            event.newRoot,
            event.elementCount
        );

        if let Err(e) = upsert_deposit(
            db_pool,
            &event.user.to_string(),
            event.usdVal.to_string().parse::<i64>().unwrap_or(0),
            &format!("{:x}", event.commitmentHash),
            "PENDING_TREE_INCLUSION",
        )
        .await
        {
            warn!("Failed to upsert deposit: {}", e)
        }
    }

    Ok(deposit_logs)
}

use std::time::Duration;
use tokio::time::sleep;

const MAX_RETRIES: usize = 5; // we can update this. i'm not sure if 10 (retries) would be too much
const INITIAL_BACKOFF_MS: u64 = 500;

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

    let mut retries = 0;
    let mut backoff = INITIAL_BACKOFF_MS;

    loop {
        match provider.get_logs(&filter).await {
            Ok(logs) => {
                let decoded_logs = logs
                    .into_iter()
                    .map(|log| log.log_decode::<T>().map_err(|e| Box::new(e) as Box<dyn std::error::Error>))
                    .collect::<Result<Vec<_>, _>>()?;

                return Ok(decoded_logs);
            }

            Err(e) => {
                retries += 1;
                if retries > MAX_RETRIES {
                    warn!("Max retries reached while fetching logs: {}", e);
                    return Err(Box::new(e));
                }

                warn!(
                    "Failed to fetch logs (attempt {}/{}): {}. Retrying in {} ms...",
                    retries,
                    MAX_RETRIES,
                    e,
                    backoff
                );

                sleep(Duration::from_millis(backoff)).await;
                backoff *= 2;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{primitives::{Address, U256}};

    #[test]
    fn construct_deposit_event_directly() {
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
}
