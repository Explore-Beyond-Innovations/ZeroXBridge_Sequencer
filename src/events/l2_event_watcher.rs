use crate::config::AppConfig;
use crate::db::database::{get_last_processed_block, update_last_processed_block};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use starknet::core::types::{BlockId, EventFilter, EventsPage, Felt};
use std::time::Duration;
use tokio::time::sleep;
use tracing::log::{info, warn};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;
const DEFAULT_PAGE_SIZE: u64 = 100;

// Name of the table for storing block tracker
const BLOCK_TRACKER_KEY: &str = "l2_burn_events_last_block";
// Event key for BurnEvent (calculated from event name "BurnEvent")
const BURN_EVENT_KEY: &str = "0x0099de3f38fed0a76764f614c6bc2b958814813685abc1af6deedab612df44f3";
// Event key for WithdrawalHashAppended
const WITHDRAWAL_HASH_APPENDED_EVENT_KEY: &str = "0x01e3ad31c1ae0cf5ec9a8eaf3c540d6cf961c8f4e3bfe1d55a5b92a09e1c9c1e";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentLog {
    pub commitment_hash: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub user: String,
    pub amount_low: String,
    pub amount_high: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawalCommitmentLog {
    pub index: String,
    pub commitment_hash: String,
    pub root_hash: String,
    pub elements_count: String,
    pub block_number: u64,
    pub transaction_hash: String,
}

pub trait TestProvider {
    fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<EventsPage, Box<dyn std::error::Error + Send + Sync>>;
}

/// This function queries the L2 burn contract for events,
/// filters them by the "BurnEvent" event selector, and parses them into CommitmentLog structs.
pub async fn fetch_l2_burn_events<P: TestProvider>(
    config: &AppConfig,
    db_pool: &PgPool,
    from_block: u64,
    provider: &P,
) -> Result<Vec<CommitmentLog>> {
    // Get a connection from the pool
    let mut conn = db_pool.acquire().await?;

    // Load last processed block if available
    let start_block = match get_last_processed_block(&mut conn, BLOCK_TRACKER_KEY).await {
        Ok(Some(last_block)) => last_block + 1,
        Ok(None) => from_block,
        Err(e) => {
            warn!("Failed to get last processed block: {}", e);
            from_block
        }
    };

    // Get current block for pagination purposes
    let latest_block = get_latest_block_with_retry(provider).await?;

    // Define the L2 burn contract address
    let burn_contract_address = Felt::from_hex(&config.contracts.l2_contract_address)
        .context("Invalid L2 burn contract address")?;

    // Create event selector for the "BurnEvent" event
    let burned_event_key = Felt::from_hex(BURN_EVENT_KEY).context("Failed to create event key")?;

    // Define the event filter
    let event_filter = EventFilter {
        from_block: Some(BlockId::Number(start_block)),
        to_block: Some(BlockId::Number(latest_block)),
        address: Some(burn_contract_address),
        keys: Some(vec![vec![burned_event_key]]),
    };

    let mut all_events = Vec::new();
    let mut continuation_token = None;

    // Fetch events with pagination
    loop {
        let events = fetch_events_with_retry(
            provider,
            &event_filter,
            continuation_token.clone(),
            DEFAULT_PAGE_SIZE,
        )
        .await?;

        if events.events.is_empty() {
            print!("No events found!!");
            break;
        }

        // Parse events into CommitmentLog structs
        for event in &events.events {
            if event.data.len() >= 4 {
                // BurnEvent data structure:
                // [user, amount_low, amount_high, commitment_hash]
                let user = event.data[0].to_hex_string();
                let amount_low = event.data[1].to_hex_string();
                let amount_high = event.data[2].to_hex_string();
                let commitment_hash = event.data[3].to_hex_string();

                all_events.push(CommitmentLog {
                    commitment_hash,
                    block_number: event.block_number.expect("Event must have a block number"),
                    transaction_hash: event.transaction_hash.to_hex_string(),
                    user,
                    amount_low,
                    amount_high,
                });
            } else {
                warn!(
                    "Invalid event data length: expected 4, got {}",
                    event.data.len()
                );
            }
        }

        // Update continuation token for next page
        continuation_token = events.continuation_token;

        // If no continuation token, we've fetched all events
        if continuation_token.is_none() {
            break;
        }
    }

    // Store the latest block number we've processed
    if !all_events.is_empty() {
        let max_block = all_events
            .iter()
            .map(|e| e.block_number)
            .max()
            .unwrap_or(start_block);
        if let Err(e) = update_last_processed_block(&mut conn, BLOCK_TRACKER_KEY, max_block).await {
            warn!("Failed to update last processed block: {}", e);
        }
    } else if latest_block > start_block {
        // Even if no events found, update the last processed block to avoid rescanning
        if let Err(e) =
            update_last_processed_block(&mut conn, BLOCK_TRACKER_KEY, latest_block).await
        {
            warn!("Failed to update last processed block: {}", e);
        }
    }

    info!(
        "Fetched {} L2 burn events from blocks {} to {}",
        all_events.len(),
        start_block,
        latest_block
    );

    Ok(all_events)
}

pub async fn fetch_l2_withdrawal_commitment_events<P: TestProvider>(
    config: &AppConfig,
    db_pool: &PgPool,
    from_block: u64,
    provider: &P,
) -> Result<Vec<WithdrawalCommitmentLog>> {
    let mut conn = db_pool.acquire().await?;
    let start_block = match get_last_processed_block(&mut conn, "l2_withdrawal_events_last_block").await
    {
        Ok(Some(last)) => last + 1,
        _ => from_block,
    };

    let latest_block = get_latest_block_with_retry(provider).await?;
    let contract_address = Felt::from_hex(&config.contracts.l2_contract_address)
        .context("Invalid L2 contract address")?;
    let withdrawal_event_key =
        Felt::from_hex(WITHDRAWAL_HASH_APPENDED_EVENT_KEY).context("Failed to create event key")?;

    let event_filter = EventFilter {
        from_block: Some(BlockId::Number(start_block)),
        to_block: Some(BlockId::Number(latest_block)),
        address: Some(contract_address),
        keys: Some(vec![vec![withdrawal_event_key]]),
    };

    let mut all_events = Vec::new();
    let mut continuation_token = None;

    loop {
        let events = fetch_events_with_retry(
            provider,
            &event_filter,
            continuation_token.clone(),
            DEFAULT_PAGE_SIZE,
        )
        .await?;

        for event in &events.events {
            if event.data.len() >= 4 {
                let index = event.data[0].to_hex_string();
                let commitment_hash = event.data[1].to_hex_string();
                let root_hash = event.data[2].to_hex_string();
                let elements_count = event.data[3].to_hex_string();

                all_events.push(WithdrawalCommitmentLog {
                    index,
                    root_hash,
                    elements_count,
                    commitment_hash,
                    block_number: event.block_number.unwrap_or_default(),
                    transaction_hash: event.transaction_hash.to_hex_string(),
                });
            } else {
                warn!(
                    "WithdrawalHashAppended event: invalid data length {}",
                    event.data.len()
                )
            }
        }

        continuation_token = events.continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }

    if !all_events.is_empty() {
        let max_block = all_events
            .iter()
            .map(|e| e.block_number)
            .max()
            .unwrap_or(start_block);
        update_last_processed_block(&mut conn, "l2_withdrawal_events_last_block", max_block)
            .await?;
    }

    Ok(all_events)
}

async fn get_latest_block_with_retry<P: TestProvider>(provider: &P) -> Result<u64> {
    for attempt in 1..=MAX_RETRIES {
        match provider.block_number() {
            Ok(block) => return Ok(block),
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow!("Failed to get latest block: {}", e));
                }
                warn!(
                    "Failed to get latest block (attempt {}/{}): {}",
                    attempt, MAX_RETRIES, e
                );
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }

    Err(anyhow!(
        "Failed to get latest block after {} attempts",
        MAX_RETRIES
    ))
}

async fn fetch_events_with_retry<P: TestProvider>(
    provider: &P,
    filter: &EventFilter,
    continuation_token: Option<String>,
    chunk_size: u64,
) -> Result<EventsPage> {
    for attempt in 1..=MAX_RETRIES {
        match provider.get_events(filter.clone(), continuation_token.clone(), chunk_size) {
            Ok(events) => return Ok(events),
            Err(e) => {
                if attempt == MAX_RETRIES {
                    return Err(anyhow!("Failed to fetch events: {}", e));
                }
                warn!(
                    "Failed to fetch events (attempt {}/{}): {}",
                    attempt, MAX_RETRIES, e
                );
                sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }

    Err(anyhow!(
        "Failed to fetch events after {} attempts",
        MAX_RETRIES
    ))
}
