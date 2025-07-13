pub mod pipeline;
use crate::pipeline::{run_full_stone_pipeline, CalldataArtifacts, ProofError, ProofInputArgs};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "proof-generator", about = "STARK proof generation pipeline")]
struct Cli {
    #[structopt(long)]
    sierra_path: PathBuf,
    
    #[structopt(long)]
    inputs_path: PathBuf,
    
    #[structopt(long, default_value = "prover_params.json")]
    prover_params: PathBuf,
    
    #[structopt(long, default_value = "prover_config.json")]
    prover_config: PathBuf,
    
    #[structopt(long, default_value = "recursive_with_poseidon")]
    layout: String,
    
    #[structopt(long, default_value = "keccak_160_lsb")]
    hasher: String,
    
    #[structopt(long, default_value = "stone6")]
    stone_version: String,
    
    #[structopt(long)]
    verify: bool,
    
    #[structopt(long)]
    keep_temp_files: bool,
}

fn main() -> Result<(), ProofError> {
    env_logger::init();
    log::info!("Starting STARK proof generation pipeline");

    let args = Cli::from_args();
    
    let inputs = std::fs::read_to_string(&args.inputs_path)?;
    let program_inputs = serde_json::from_str(&inputs)?;

    let proof_args = ProofInputArgs {
        sierra_path: args.sierra_path,
        program_inputs,
        prover_parameters: args.prover_params,
        prover_config: args.prover_config,
        layout: args.layout,
        hasher: args.hasher,
        stone_version: args.stone_version,
        run_verifier: args.verify,
        keep_temp_files: args.keep_temp_files,
    };

    let artifacts = run_full_stone_pipeline(proof_args)?;

    println!("\nProof generation successful!");
    println!("Calldata directory: {:?}", artifacts.calldata_dir);
    if let Some(fact_hash) = artifacts.fact_hash {
        println!("Fact hash: {}", fact_hash);
    }
    println!("Proof path: {:?}", artifacts.proof_path);

    Ok(())
}