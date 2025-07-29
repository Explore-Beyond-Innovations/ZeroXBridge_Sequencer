use clap::{Arg, Command};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{error, info};
use zeroxbridge_sequencer::config::load_config;
use zeroxbridge_sequencer::db::database::get_db_pool;
use zeroxbridge_sequencer::relayer::client::ProofSubmissionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("Proof Submitter")
        .version("1.0")
        .about("Submit proofs from calldata directory to Starknet")
        .arg(
            Arg::new("calldata_dir")
                .long("calldata_dir")
                .value_name("PATH")
                .help("Path to the calldata directory")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("job_id")
                .long("job_id")
                .value_name("ID")
                .help("Proof job ID")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("layout")
                .long("layout")
                .value_name("LAYOUT")
                .help("Layout parameter")
                .default_value("recursive_with_poseidon")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("hasher")
                .long("hasher")
                .value_name("HASHER")
                .help("Hasher parameter")
                .default_value("keccak_160_lsb")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("stone_version")
                .long("stone_version")
                .value_name("VERSION")
                .help("Stone version parameter")
                .default_value("stone6")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("memory_verification")
                .long("memory_verification")
                .value_name("VERIFICATION")
                .help("Memory verification parameter")
                .default_value("true")
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .value_name("CONFIG_FILE")
                .help("Path to configuration file")
                .default_value("config.toml")
                .value_parser(clap::value_parser!(String)),
        )
        .get_matches();

    // Parse arguments
    let calldata_dir = PathBuf::from(matches.get_one::<String>("calldata_dir").unwrap());
    let job_id = u64::from_str(matches.get_one::<String>("job_id").unwrap())
        .map_err(|e| format!("Invalid job_id: {}", e))?;
    let layout = matches.get_one::<String>("layout").unwrap().clone();
    let hasher = matches.get_one::<String>("hasher").unwrap().clone();
    let stone_version = matches.get_one::<String>("stone_version").unwrap().clone();
    let memory_verification = matches
        .get_one::<String>("memory_verification")
        .unwrap()
        .clone();
    let config_path = PathBuf::from(matches.get_one::<String>("config").unwrap());

    info!("Starting proof submission with parameters:");
    info!("  Calldata directory: {:?}", calldata_dir);
    info!("  Job ID: {}", job_id);
    info!("  Layout: {}", layout);
    info!("  Hasher: {}", hasher);
    info!("  Stone version: {}", stone_version);
    info!("  Memory verification: {}", memory_verification);

    // Load configuration
    let config = load_config(Some(&config_path))?;
    info!("Configuration loaded successfully");

    // Initialize database connection
    let db_pool = get_db_pool(&config.database.get_db_url()).await?;
    info!("Database connection established");

    // Create proof submission client
    let client = ProofSubmissionClient::new(db_pool, config)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    info!("Proof submission client initialized");

    // Submit the proof
    match client
        .submit_proof(
            calldata_dir,
            job_id,
            layout,
            hasher,
            stone_version,
            memory_verification,
        )
        .await
    {
        Ok(_) => {
            info!("Proof submission completed successfully!");
            Ok(())
        }
        Err(e) => {
            error!("Proof submission failed: {:?}", e);
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}
