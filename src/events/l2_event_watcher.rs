use crate::config::AppConfig;
use crate::db::database::{get_last_processed_block, update_last_processed_block};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use starknet::core::types::{BlockId, EventFilter, EventsPage, Felt};
use std::time::Duration;
use tokio::time::sleep;
use tracing::log::warn;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;
const DEFAULT_PAGE_SIZE: u64 = 100;

// Event key for BurnEvent (calculated from event name "BurnEvent")
const BURN_EVENT_KEY: &str = "0x0099de3f38fed0a76764f614c6bc2b958814813685abc1af6deedab612df44f3";
// Event key for WithdrawalHashAppended
const WITHDRAWAL_HASH_APPENDED_EVENT_KEY: &str =
    "0x01e3ad31c1ae0cf5ec9a8eaf3c540d6cf961c8f4e3bfe1d55a5b92a09e1c9c1e";

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

pub struct L2EventResults {
    pub burn_events: Vec<CommitmentLog>,
    pub withdrawal_events: Vec<WithdrawalCommitmentLog>,
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

/// This function queries the L2 contract for events and parses both:
/// - Burn events into `CommitmentLog`
/// - WithdrawalHashAppended events into `WithdrawalCommitmentLog`
///
/// Events are returned together in a unified `L2EventResults` struct.
/// Pagination and block tracking are handled to ensure no events are missed.
pub async fn fetch_l2_events<P: TestProvider>(
    config: &AppConfig,
    db_pool: &PgPool,
    from_block: u64,
    provider: &P,
) -> Result<L2EventResults> {
    let start_block = match get_last_processed_block(&db_pool, "l2_events_last_block").await {
        Ok(Some(last)) => last + 1,
        _ => from_block,
    };

    let latest_block = get_latest_block_with_retry(provider).await?;
    let contract_address = Felt::from_hex(&config.contracts.l2_contract_address)?;

    let burn_event_key = Felt::from_hex(BURN_EVENT_KEY)?;
    let withdrawal_event_key = Felt::from_hex(WITHDRAWAL_HASH_APPENDED_EVENT_KEY)?;

    let event_filter = EventFilter {
        from_block: Some(BlockId::Number(start_block)),
        to_block: Some(BlockId::Number(latest_block)),
        address: Some(contract_address),
        keys: Some(vec![vec![burn_event_key, withdrawal_event_key]]),
    };

    let mut burn_events = Vec::new();
    let mut withdrawal_events = Vec::new();
    let mut continuation_token = None;

    loop {
        let page = fetch_events_with_retry(
            provider,
            &event_filter,
            continuation_token.clone(),
            DEFAULT_PAGE_SIZE,
        )
        .await?;

        for event in &page.events {
            let block_number = event.block_number.unwrap_or_else(|| {
                warn!("Missing block number for event: {:?}", event);
                0
            });

            if event.keys.contains(&burn_event_key) && event.data.len() >= 4 {
                burn_events.push(CommitmentLog {
                    block_number,
                    user: event.data[0].to_hex_string(),
                    amount_low: event.data[1].to_hex_string(),
                    amount_high: event.data[2].to_hex_string(),
                    commitment_hash: event.data[3].to_hex_string(),
                    transaction_hash: event.transaction_hash.to_hex_string(),
                });
            } else if event.keys.contains(&withdrawal_event_key) && event.data.len() >= 4 {
                withdrawal_events.push(WithdrawalCommitmentLog {
                    block_number,
                    index: event.data[0].to_hex_string(),
                    commitment_hash: event.data[1].to_hex_string(),
                    root_hash: event.data[2].to_hex_string(),
                    elements_count: event.data[3].to_hex_string(),
                    transaction_hash: event.transaction_hash.to_hex_string(),
                });
            } else {
                warn!("Unknown or malformed event: {:?}", event);
            }
        }

        continuation_token = page.continuation_token;
        if continuation_token.is_none() {
            break;
        }
    }

    let max_block = std::cmp::max(
        burn_events
            .iter()
            .map(|e| e.block_number)
            .max()
            .unwrap_or(start_block),
        withdrawal_events
            .iter()
            .map(|e| e.block_number)
            .max()
            .unwrap_or(start_block),
    );

    update_last_processed_block(&db_pool, "l2_events_last_block", max_block).await?;

    Ok(L2EventResults {
        burn_events,
        withdrawal_events,
    })
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
