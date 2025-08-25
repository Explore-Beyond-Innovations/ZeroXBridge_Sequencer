use crate::config::AppConfig;
use ethers::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

// Constants
const DEFAULT_TOLERANCE_PERCENT: f64 = 0.01; // 1%

pub async fn initializer() {
    // let l1_provider =
    //     Provider::<Http>::try_from(config.ethereum.rpc_url.clone()).expect("Invalid L1 RPC URL");
    // let l2_provider =
    //     Provider::<Http>::try_from(config.starknet.rpc_url.clone()).expect("Invalid L2 RPC URL");

    // let l1_contract = Contract::new(
    //     config.contracts.l1_contract_address.parse().unwrap(),
    //     l1_abi(),
    //     Arc::new(l1_provider),
    // );
    // let l2_contract = Contract::new(
    //     config.contracts.l2_contract_address.parse().unwrap(),
    //     l2_abi(),
    //     Arc::new(l2_provider),
    // );

    // tokio::spawn(async move {
    //     sync_tvl(l1_contract, l2_contract, &config)
    //         .await
    //         .expect("TVL sync failed");
    // });
}

/// Fetch TVL from the L1 contract
async fn fetch_l1_tvl(
    l1_contract: &Contract<Provider<Http>>,
) -> Result<U256, ContractError<Provider<Http>>> {
    l1_contract
        .method::<_, U256>("get_total_tvl", ())?
        .call()
        .await
}

/// Fetch TVL from the L2 Oracle contract
async fn fetch_l2_tvl(
    l2_contract: &Contract<Provider<Http>>,
) -> Result<U256, ContractError<Provider<Http>>> {
    l2_contract
        .method::<_, U256>("get_total_tvl", ())?
        .call()
        .await
}

/// Update TVL on the L2 Oracle contract
async fn update_l2_tvl(
    l2_contract: &Contract<Provider<Http>>,
    new_tvl: U256,
) -> Result<(), ContractError<Provider<Http>>> {
    l2_contract
        .method::<_, ()>("update_tvl", new_tvl)?
        .send()
        .await?;
    Ok(())
}

/// Sync TVL between L1 and L2
pub async fn sync_tvl(
    l1_contract: Contract<Provider<Http>>,
    l2_contract: Contract<Provider<Http>>,
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let tolerance_percent = config
        .oracle
        .tolerance_percent
        .unwrap_or(DEFAULT_TOLERANCE_PERCENT);
    let polling_interval = Duration::from_secs(config.oracle.polling_interval_seconds);

    loop {
        // Fetch TVL values
        let l1_tvl = fetch_l1_tvl(&l1_contract).await?;
        let l2_tvl = fetch_l2_tvl(&l2_contract).await?;

        // Calculate percentage difference
        let l1_tvl_f64 = l1_tvl.as_u128() as f64;
        let l2_tvl_f64 = l2_tvl.as_u128() as f64;
        let diff = ((l1_tvl_f64 - l2_tvl_f64).abs() / l1_tvl_f64).max(0.0);

        // Check if update is needed
        if diff > tolerance_percent {
            println!(
                "Significant TVL difference detected: L1 = {}, L2 = {}, updating L2...",
                l1_tvl, l2_tvl
            );
            update_l2_tvl(&l2_contract, l1_tvl).await?;
        } else {
            println!(
                "No significant TVL difference detected: L1 = {}, L2 = {}",
                l1_tvl, l2_tvl
            );
        }

        // Wait for the next polling interval
        sleep(polling_interval).await;
    }
}
