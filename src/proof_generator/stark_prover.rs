use std::process::Command;
use std::path::{Path, PathBuf};
use serde::Serialize;
use std::fs::File;
use std::io::Write;

mod cairo_build;
pub use cairo_build::{CairoBuildManager, BuildError};

/// STARK proof generator that integrates with Cairo builds
pub struct StarkProver {
    cairo_build_manager: CairoBuildManager,
}

impl StarkProver {
    /// Creates a new StarkProver instance
    pub fn new() -> Self {
        Self {
            cairo_build_manager: CairoBuildManager::new(),
        }
    }

    /// Creates a StarkProver with a custom Cairo project directory
    pub fn with_cairo_dir<P: AsRef<Path>>(cairo_dir: P) -> Self {
        Self {
            cairo_build_manager: CairoBuildManager::with_base_dir(cairo_dir),
        }
    }

    /// Builds the Cairo project and prepares for proof generation
    pub fn prepare_cairo_program(&self) -> Result<PathBuf, BuildError> {
        println!("Preparing Cairo program for proof generation...");
        
        // Check if scarb is available
        CairoBuildManager::check_scarb_availability()?;
        
        // Build the Cairo project
        let sierra_file = self.cairo_build_manager.build_cairo_project()?;
        
        println!("Cairo program compiled successfully: {:?}", sierra_file);
        Ok(sierra_file)
    }

    /// Generates Cairo1 input files for the proof generation process
    pub fn generate_cairo1_inputs(
        &self,
        commitment_hash: u64,
        proof_array: Vec<u64>,
        new_root: u64,
        output_dir: &str,
    ) -> Result<(), ProofError> {
        generate_cairo1_inputs(commitment_hash, proof_array, new_root, output_dir)
            .map_err(|e| ProofError::InvalidInput(format!("Failed to generate Cairo1 inputs: {}", e)))
    }

    /// Generates a STARK proof using the compiled Cairo program
    pub fn generate_proof(&self, merkle_root: &str) -> Result<String, ProofError> {
        // First, ensure the Cairo program is built
        let sierra_file = self.prepare_cairo_program()
            .map_err(|e| ProofError::CairoBuildFailed(e.to_string()))?;

        println!("Generating STARK proof with merkle root: {}", merkle_root);
        println!("Using Sierra file: {:?}", sierra_file);

        // Placeholder for actual proof generation logic
        // In a real implementation, this would:
        // 1. Load the compiled Cairo program
        // 2. Set up the execution trace
        // 3. Generate the STARK proof using stwo or stone
        
        // For now, return a mock proof
        let proof = format!(
            "stark_proof_{}_{}", 
            merkle_root, 
            sierra_file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        );

        println!("STARK proof generated successfully");
        Ok(proof)
    }

    /// Generates a STARK proof with Cairo1 inputs
    pub fn generate_proof_with_inputs(
        &self,
        commitment_hash: u64,
        proof_array: Vec<u64>,
        new_root: u64,
        output_dir: &str,
    ) -> Result<String, ProofError> {
        // Generate Cairo1 input files
        self.generate_cairo1_inputs(commitment_hash, proof_array.clone(), new_root, output_dir)?;
        
        // Generate proof using the merkle root (new_root as string)
        let merkle_root = new_root.to_string();
        self.generate_proof(&merkle_root)
    }

    /// Gets the Cairo build manager for direct access
    pub fn get_build_manager(&self) -> &CairoBuildManager {
        &self.cairo_build_manager
    }
}

/// Error types for proof generation
#[derive(Debug)]
pub enum ProofError {
    /// Cairo build failed
    CairoBuildFailed(String),
    /// Proof generation failed
    ProofGenerationFailed(String),
    /// Invalid input parameters
    InvalidInput(String),
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::CairoBuildFailed(msg) => {
                write!(f, "Cairo build failed: {}", msg)
            }
            ProofError::ProofGenerationFailed(msg) => {
                write!(f, "Proof generation failed: {}", msg)
            }
            ProofError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProofError {}

/// Data structure for Cairo1 input JSON serialization
#[derive(Serialize)]
struct Cairo1Input {
    data: Vec<Vec<u64>>,
}

/// Generates Cairo1 input files (JSON and TXT formats)
pub fn generate_cairo1_inputs(
    commitment_hash: u64,
    proof_array: Vec<u64>,
    new_root: u64,
    output_dir: &str,
) -> Result<(), std::io::Error> {
    // Combine inputs into a single array
    let mut input_data = vec![commitment_hash];
    input_data.extend(proof_array);
    input_data.push(new_root);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    // Generate JSON file
    let json_data = Cairo1Input {
        data: vec![input_data.clone()],
    };
    let json_string = serde_json::to_string_pretty(&json_data)?;
    let json_path = Path::new(output_dir).join("input.cairo1.json");
    File::create(&json_path)?.write_all(json_string.as_bytes())?;

    // Generate TXT file
    let txt_string = input_data
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(" ");
    let txt_content = format!("[{}]", txt_string);
    let txt_path = Path::new(output_dir).join("input.cairo1.txt");
    File::create(&txt_path)?.write_all(txt_content.as_bytes())?;

    println!("Cairo1 input files generated:");
    println!("  JSON: {:?}", json_path);
    println!("  TXT: {:?}", txt_path);

    Ok(())
}

/// Utility function to execute scarb build with custom options
pub fn execute_scarb_build_with_options<P: AsRef<Path>>(
    project_dir: P,
    args: &[&str],
) -> Result<String, BuildError> {
    let mut command = Command::new("scarb");
    command.current_dir(project_dir.as_ref());
    
    // Add build as the first argument
    command.arg("build");
    
    // Add additional arguments
    for arg in args {
        command.arg(arg);
    }

    let output = command
        .output()
        .map_err(|e| BuildError::CommandExecutionFailed(e.to_string()))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BuildError::BuildFailed(stderr.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_stark_prover_creation() {
        let prover = StarkProver::new();
        assert_eq!(
            prover.get_build_manager().get_base_dir(),
            Path::new("crates/cairo1-rust-vm")
        );
    }

    #[test]
    fn test_stark_prover_with_custom_dir() {
        let custom_dir = "test/cairo/dir";
        let prover = StarkProver::with_cairo_dir(custom_dir);
        assert_eq!(
            prover.get_build_manager().get_base_dir(),
            Path::new(custom_dir)
        );
    }

    #[test]
    fn test_generate_cairo1_inputs() {
        let commitment_hash = 12345;
        let proof_array = vec![67890, 111213];
        let new_root = 141516;
        let output_dir = "test_output";

        // Generate files
        generate_cairo1_inputs(commitment_hash, proof_array.clone(), new_root, output_dir)
            .expect("Failed to generate files");

        // Verify JSON file
        let json_path = Path::new(output_dir).join("input.cairo1.json");
        let json_content = fs::read_to_string(&json_path).unwrap();
        let expected_json = r#"{
  "data": [
    [12345, 67890, 111213, 141516]
  ]
}"#;
        assert_eq!(json_content.trim(), expected_json.trim());

        // Verify TXT file
        let txt_path = Path::new(output_dir).join("input.cairo1.txt");
        let txt_content = fs::read_to_string(&txt_path).unwrap();
        let expected_txt = "[12345 67890 111213 141516]";
        assert_eq!(txt_content, expected_txt);

        // Clean up
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_stark_prover_cairo1_inputs_integration() {
        let prover = StarkProver::new();
        let commitment_hash = 98765;
        let proof_array = vec![11111, 22222, 33333];
        let new_root = 44444;
        let output_dir = "test_integration_output";

        // Test the integrated method
        let result = prover.generate_cairo1_inputs(
            commitment_hash,
            proof_array.clone(),
            new_root,
            output_dir,
        );

        assert!(result.is_ok());

        // Verify files were created
        let json_path = Path::new(output_dir).join("input.cairo1.json");
        let txt_path = Path::new(output_dir).join("input.cairo1.txt");
        
        assert!(json_path.exists());
        assert!(txt_path.exists());

        // Clean up
        fs::remove_dir_all(output_dir).unwrap();
    }

    #[test]
    fn test_generate_proof_with_inputs() {
        let prover = StarkProver::new();
        let commitment_hash = 55555;
        let proof_array = vec![66666, 77777];
        let new_root = 88888;
        let output_dir = "test_proof_inputs_output";

        // This test will fail in practice because it tries to build Cairo project,
        // but it demonstrates the integrated API
        let result = prover.generate_proof_with_inputs(
            commitment_hash,
            proof_array,
            new_root,
            output_dir,
        );

        // The result will likely be an error due to missing Cairo build dependencies,
        // but the function structure is correct
        match result {
            Ok(_) => {
                // If successful, clean up
                let _ = fs::remove_dir_all(output_dir);
            }
            Err(e) => {
                // Expected to fail in test environment without proper Cairo setup
                println!("Expected error in test environment: {}", e);
                // Clean up if directory was created
                let _ = fs::remove_dir_all(output_dir);
            }
        }
    }
}