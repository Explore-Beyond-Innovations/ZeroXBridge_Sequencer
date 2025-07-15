use alloy::{
    primitives::Address,
    providers::Provider,
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use sqlx::PgPool;
use tracing::log::{debug, warn};

use crate::db::database::{
    get_last_processed_block, insert_deposit_hash_event, update_last_processed_block,
    DepositHashAppended,
};

pub const BLOCK_TRACKER_KEY: &str = "l1_deposit_events_last_block";
pub const DEPOSIT_HASH_BLOCK_TRACKER_KEY: &str = "l1_deposit_hash_events_last_block";

sol! {
    #[derive(Debug)]
    contract ZeroXBridge {
        enum AssetType { ETH, ERC20 }

        event DepositEvent(
            address indexed token,
            AssetType assetType,
            uint256 amount,
            address indexed user,
            bytes32 commitmentHash
        );

        event DepositHashAppended(
            uint256 index,
            bytes32 commitmentHash,
            bytes32 rootHash,
            uint256 elementsCount
        );
    }
}

/// Fetch both kinds of logs in one shot, reading & updating DB trackers.
pub async fn fetch_l1_deposit_events<P: Provider>(
    db_pool: &mut PgPool,
    provider: &P,
    from_block: u64,
    contract_addr: Address,
) -> Result<
    (
        Vec<Log<ZeroXBridge::DepositEvent>>,
        Vec<Log<ZeroXBridge::DepositHashAppended>>,
    ),
    Box<dyn std::error::Error>,
> {
    // figure starting points
    let from_deposit = get_last_processed_block(db_pool, BLOCK_TRACKER_KEY)
        .await?
        .map(|b| (b + 1) as u64)
        .unwrap_or(from_block);

    let from_hash = get_last_processed_block(db_pool, DEPOSIT_HASH_BLOCK_TRACKER_KEY)
        .await?
        .map(|b| (b + 1) as u64)
        .unwrap_or(from_block);

    // pull raw logs
    let deposits = fetch_events_logs_at_address::<_, P>(
        provider,
        from_deposit,
        contract_addr,
        ZeroXBridge::DepositEvent::SIGNATURE,
    )
    .await?;

    let hashes = fetch_events_logs_at_address::<_, P>(
        provider,
        from_hash,
        contract_addr,
        ZeroXBridge::DepositHashAppended::SIGNATURE,
    )
    .await?;

    for log in &hashes {
        let e: &ZeroXBridge::DepositHashAppended = log.data();
        let record = DepositHashAppended {
            id: 0,
            index: e.index.to::<u64>() as i64,
            commitment_hash: e.commitmentHash.0.to_vec(),
            root_hash: e.rootHash.0.to_vec(),
            elements_count: e.elementsCount.to::<u64>() as i64,
            block_number: log.block_number.unwrap() as i64,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        };
        if let Err(err) = insert_deposit_hash_event(db_pool, &record).await {
            warn!("Failed to insert hash event: {}", err);
        } else {
            debug!(
                "Inserted hash event idx={} count={}",
                record.index, record.elements_count
            );
        }
    }

    // update trackers
    if let Some(last) = deposits.last() {
        let bn = last.block_number.unwrap() as i64;
        if let Err(err) = update_last_processed_block(db_pool, BLOCK_TRACKER_KEY, bn as u64).await {
            warn!("Failed to update deposit tracker: {}", err);
        }
    }
    if let Some(last) = hashes.last() {
        let bn = last.block_number.unwrap() as i64;
        if let Err(err) =
            update_last_processed_block(db_pool, DEPOSIT_HASH_BLOCK_TRACKER_KEY, bn as u64).await
        {
            warn!("Failed to update hash tracker: {}", err);
        }
    }

    Ok((deposits, hashes))
}

/// Generic helper: pull & decode logs of any `SolEvent` type.
async fn fetch_events_logs_at_address<T, P>(
    provider: &P,
    from_block: u64,
    contract: Address,
    event_sig: &str,
) -> Result<Vec<Log<T>>, Box<dyn std::error::Error>>
where
    T: SolEvent,
    P: Provider,
{
    let filter = Filter::new()
        .address(contract)
        .event(event_sig)
        .from_block(from_block);

    let raw = provider.get_logs(&filter).await?;
    let decoded = raw
        .into_iter()
        .map(|l| l.log_decode::<T>().map_err(|e| Box::new(e) as _))
        .collect::<Result<Vec<Log<T>>, Box<dyn std::error::Error>>>()?;
    Ok(decoded)
}
