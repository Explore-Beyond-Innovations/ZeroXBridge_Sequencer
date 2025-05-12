use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use log::{info, error};
use serde::Serialize;

use crate::proof_generator::scarb_builder::ScarbBuilder;

/// Structure to generate STARK proofs
pub struct StarkProver {
    cairo_project_dir: PathBuf,
}

impl StarkProver {
    /// Create a new StarkProver instance
    pub fn new(cairo_project_dir: PathBuf) -> Self {
        StarkProver {
            cairo_project_dir,
        }
    }

    /// Compiles Cairo code and prepares it for proof generation
    pub fn compile_cairo(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        info!("Compiling Cairo program at: {:?}", self.cairo_project_dir);

        // Create a ScarbBuilder for the Cairo project
        let builder = ScarbBuilder::new(&self.cairo_project_dir);

        // Run the build
        let output_path = builder.build()?;

        info!("Cairo compilation successful. Output file: {:?}", output_path);

        Ok(output_path)
    }

    /// Generates input files for Cairo 1 program
    pub fn generate_inputs(
        &self,
        commitment_hash: u64,
        proof_array: Vec<u64>,
        new_root: u64,
        output_dir: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Generating Cairo1 input files...");

        generate_cairo1_inputs(commitment_hash, proof_array, new_root, output_dir)?;

        info!("Cairo1 input generation completed: {:?}", output_dir);
        Ok(())
    }
}

/// Internal helper to generate Cairo1 input JSON and TXT
#[derive(Serialize)]
struct Cairo1Input {
    data: Vec<Vec<u64>>,
}

fn generate_cairo1_inputs(
    commitment_hash: u64,
    proof_array: Vec<u64>,
    new_root: u64,
    output_dir: &str,
) -> Result<(), std::io::Error> {
    // Combine inputs into a single array
    let mut input_data = vec![commitment_hash];
    input_data.extend(proof_array);
    input_data.push(new_root);

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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[ignore] // Requires Scarb and valid Cairo project path
    fn test_compile_cairo() {
        let cairo_project_dir = PathBuf::from("crates/cairo1-rust-vm");

        if !cairo_project_dir.exists() {
            eprintln!("Test skipped: Cairo project directory not found");
            return;
        }

        let prover = StarkProver::new(cairo_project_dir);
        let result = prover.compile_cairo();

        assert!(result.is_ok(), "Cairo compilation should succeed: {:?}", result.err());
        let output_path = result.unwrap();
        assert!(output_path.exists(), "Output file should exist");
    }

    #[test]
    fn test_generate_inputs() {
        let prover = StarkProver::new(PathBuf::from("."));

        let commitment_hash = 12345;
        let proof_array = vec![67890, 111213];
        let new_root = 141516;
        let output_dir = "test_input_output";

        fs::create_dir_all(output_dir).unwrap();

        let result = prover.generate_inputs(commitment_hash, proof_array.clone(), new_root, output_dir);
        assert!(result.is_ok(), "Input generation should succeed");

        let json_path = Path::new(output_dir).join("input.cairo1.json");
        let txt_path = Path::new(output_dir).join("input.cairo1.txt");

        assert!(json_path.exists());
        assert!(txt_path.exists());

        // Optional: check contents
        let json_content = fs::read_to_string(&json_path).unwrap();
        let expected_json = r#"{
  "data": [
    [12345, 67890, 111213, 141516]
  ]
}"#;
        assert_eq!(json_content.trim(), expected_json.trim());

        let txt_content = fs::read_to_string(&txt_path).unwrap();
        let expected_txt = "[12345 67890 111213 141516]";
        assert_eq!(txt_content.trim(), expected_txt);

        fs::remove_dir_all(output_dir).unwrap();
    }
}
