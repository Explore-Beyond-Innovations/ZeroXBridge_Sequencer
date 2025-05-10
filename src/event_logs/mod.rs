use std::str::FromStr;

use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
};
use sqlx::PgConnection;

use crate::db::database::{get_last_processed_event_block, update_last_processed_event_block};

sol! {
    contract ZeroXBridge {
        enum AssetType {
            ETH,
            ERC20
        }

        event DepositEvent(
            address indexed token, AssetType assetType, uint256 amount, address indexed user, bytes32 commitmentHash
        );
    }
}

pub async fn fetch_l1_deposit_events(
    db_conn: &mut PgConnection,
    rpc_url: &str,
    from_block: Option<u64>,
    contract_addr: &str,
) -> Result<Vec<Log<ZeroXBridge::DepositEvent>>, Box<dyn std::error::Error>> {
    let from_block = match from_block {
        Some(block) => block,
        None => {
            let last_processed_block = get_last_processed_event_block(db_conn).await?;
            last_processed_block.ok_or("Last processed block not found")?
        }
    };

    let event_name = ZeroXBridge::DepositEvent::SIGNATURE;
    let logs = fetch_events_logs_at_address(rpc_url, from_block, contract_addr, event_name).await?;

    let last_log = logs.last().ok_or("No logs found")?;
    let block_number = last_log.block_number.ok_or("Block number not found")?;

    // Update the last processed block in the database
    update_last_processed_event_block(db_conn, block_number).await?;

    Ok(logs)
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
    let rpc_url = rpc_url.parse()?;
    let contract_addr = Address::from_str(contract_addr)?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

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