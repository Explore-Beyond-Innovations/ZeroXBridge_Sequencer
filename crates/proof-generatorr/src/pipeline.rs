use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{tempdir, TempDir};

#[derive(Debug)]
pub enum ProofError {
    Io(io::Error),
    Serialization(serde_json::Error),
    CommandExecution {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    VerificationFailed,
}

#[derive(Debug)]
pub struct CalldataArtifacts {
    pub calldata_dir: PathBuf,
    pub fact_hash: Option<String>,
    pub proof_path: PathBuf,
    _temp_dir: Option<TempDir>,
}

pub struct ProofInputArgs {
    pub sierra_path: PathBuf,
    pub program_inputs: serde_json::Value,
    pub prover_parameters: PathBuf,
    pub prover_config: PathBuf,
    pub layout: String,
    pub hasher: String,
    pub stone_version: String,
    pub run_verifier: bool,
    pub keep_temp_files: bool,
}

fn execute_command(
    command: &str,
    args: &[&str],
    description: &str,
) -> Result<(), ProofError> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|e| ProofError::Io(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(ProofError::CommandExecution {
            command: format!("{command} {}", args.join(" ")),
            exit_code: output.status.code(),
            stderr,
        });
    }

    log::info!("✅ {description} completed successfully");
    Ok(())
}

pub fn run_full_stone_pipeline(
    args: ProofInputArgs,
) -> Result<CalldataArtifacts, ProofError> {
    let temp_dir = tempdir().map_err(ProofError::Io)?;
    let temp_path = temp_dir.path();
    let target_dir = temp_path.join("target");
    std::fs::create_dir(&target_dir)?;

    // 1. Prepare input file
    let input_file = temp_path.join("input.json");
    std::fs::write(&input_file, serde_json::to_vec(&args.program_inputs)?)?;

    // 2. Execute cairo1-run
    let public_input = target_dir.join("public_input.json");
    let private_input = target_dir.join("private_input.json");
    let trace_file = target_dir.join("trace");
    let memory_file = target_dir.join("memory");

    execute_command(
        "cairo1-run",
        &[
            args.sierra_path.to_str().unwrap(),
            "--layout",
            &args.layout,
            "--arguments-file",
            input_file.to_str().unwrap(),
            "--proof_mode",
            "--air_public_input",
            public_input.to_str().unwrap(),
            "--air_private_input",
            private_input.to_str().unwrap(),
            "--trace_file",
            trace_file.to_str().unwrap(),
            "--memory_file",
            memory_file.to_str().unwrap(),
        ],
        "Cairo execution (cairo1-run)",
    )?;

    // 3. Generate proof with cpu_air_prover
    let proof_path = target_dir.join("proof.json");
    execute_command(
        "cpu_air_prover",
        &[
            "--parameter_file",
            args.prover_parameters.to_str().unwrap(),
            "--prover_config_file",
            args.prover_config.to_str().unwrap(),
            "--private_input_file",
            private_input.to_str().unwrap(),
            "--public_input_file",
            public_input.to_str().unwrap(),
            "--out_file",
            proof_path.to_str().unwrap(),
            "--generate_annotations",
            "true",
        ],
        "Proof generation (cpu_air_prover)",
    )?;

    // 4. Optionally verify proof
    if args.run_verifier {
        execute_command(
            "cpu_air_verifier",
            &["--in_file", proof_path.to_str().unwrap()],
            "Proof verification (cpu_air_verifier)",
        )?;
    }

    // 5. Prepare calldata with swiftness
    let calldata_dir = temp_path.join("calldata");
    execute_command(
        "swiftness",
        &[
            "--proof",
            proof_path.to_str().unwrap(),
            "--layout",
            &args.layout,
            "--hasher",
            &args.hasher,
            "--stone-version",
            &args.stone_version,
            "--out",
            calldata_dir.to_str().unwrap(),
        ],
        "Calldata preparation (swiftness)",
    )?;

    // Handle temp directory persistence
    let (calldata_dir, proof_path, _temp_dir) = if args.keep_temp_files {
        let persistent_path = temp_dir.into_path();
        (
            persistent_path.join("calldata"),
            persistent_path.join("target/proof.json"),
            None,
        )
    } else {
        (
            calldata_dir,
            proof_path.clone(),
            Some(temp_dir),
        )
    };

    Ok(CalldataArtifacts {
        calldata_dir: calldata_dir.clone(),
        fact_hash: extract_fact_hash(&calldata_dir)?,
        proof_path,
        _temp_dir,
    })
}

/// Extract fact hash from swiftness output
fn extract_fact_hash(calldata_dir: &Path) -> Result<Option<String>, ProofError> {
    let fact_file = calldata_dir.join("fact.txt");
    if !fact_file.exists() {
        return Ok(None);
    }

    let fact_hash = std::fs::read_to_string(&fact_file)
        .map_err(ProofError::Io)?
        .trim()
        .to_owned();

    Ok(Some(fact_hash))
}

// Implement error conversions
impl From<io::Error> for ProofError {
    fn from(e: io::Error) -> Self {
        ProofError::Io(e)
    }
}

impl From<serde_json::Error> for ProofError {
    fn from(e: serde_json::Error) -> Self {
        ProofError::Serialization(e)
    }
}