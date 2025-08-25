#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let matches = clap::Command::new("Proof Submitter")
        .about("Submit proofs from calldata directory to Starknet")
        .arg(clap::Arg::new("calldata_dir").long("calldata_dir").required(true))
        .arg(clap::Arg::new("job_id").long("job_id").required(true))
        .arg(clap::Arg::new("layout").long("layout").default_value("recursive_with_poseidon"))
        .arg(clap::Arg::new("hasher").long("hasher").default_value("keccak_160_lsb"))
        .arg(clap::Arg::new("stone_version").long("stone_version").default_value("stone6"))
        .arg(clap::Arg::new("memory_verification").long("memory_verification").default_value("true"))
        .arg(clap::Arg::new("config").long("config").default_value("config.toml"))
        .get_matches();

    // TODO: Map matches to your library API and delegate:
    // sequencer_lib::proof_submitter::run(args...).await?;
    Ok(())
}