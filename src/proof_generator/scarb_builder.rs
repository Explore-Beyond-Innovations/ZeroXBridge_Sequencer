use std::process::Command;
use std::path::{Path, PathBuf};
use std::io::{Error, ErrorKind, Result};
use tracing::{info, error};

/// Structure to manage Scarb builds for Cairo 1.0 projects
pub struct ScarbBuilder {
    /// Path to the Cairo 1.0 project directory
    project_dir: PathBuf,
}

impl ScarbBuilder {
    /// Create a new ScarbBuilder instance
    /// 
    /// # Arguments
    /// * `project_dir` - The directory containing the Scarb.toml file
    pub fn new<P: AsRef<Path>>(project_dir: P) -> Self {
        ScarbBuilder {
            project_dir: project_dir.as_ref().to_path_buf(),
        }
    }

    /// Run `scarb build` on the Cairo 1.0 project
    /// 
    /// # Returns
    /// * `Result<PathBuf>` - The path to the generated Sierra JSON file on success
    pub fn build(&self) -> Result<PathBuf> {
        // Check if the directory exists
        if !self.project_dir.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Project directory does not exist: {:?}", self.project_dir),
            ));
        }

        // Check if Scarb.toml exists
        let scarb_toml = self.project_dir.join("Scarb.toml");
        if !scarb_toml.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Scarb.toml not found in: {:?}", self.project_dir),
            ));
        }

        info!("Running 'scarb build' in directory: {:?}", self.project_dir);

        // Execute scarb build command
        let status = Command::new("scarb")
            .arg("build")
            .current_dir(&self.project_dir)
            .status()
            .map_err(|e| {
                error!("Failed to execute scarb build: {}", e);
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to execute scarb build: {}", e),
                )
            })?;

        if !status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                format!("scarb build failed with exit code: {:?}", status.code()),
            ));
        }

        info!("Build succeeded");

        // Find the generated Sierra JSON file
        // This assumes the default output location for Scarb
        let target_dir = self.project_dir.join("target/dev");
        
        // Get all Sierra JSON files
        let entries = std::fs::read_dir(&target_dir).map_err(|e| {
            error!("Failed to read target directory: {}", e);
            Error::new(
                ErrorKind::NotFound,
                format!("Failed to read target directory: {}", e),
            )
        })?;

        // Find the first Sierra JSON file
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.to_string_lossy().ends_with(".sierra.json") {
                info!("Generated Sierra JSON file: {:?}", path);
                return Ok(path);
            }
        }

        Err(Error::new(
            ErrorKind::NotFound,
            "Sierra JSON file not found in target directory",
        ))
    }

    /// Get the expected output file path
    /// 
    /// # Returns
    /// * `PathBuf` - The expected path to the generated Sierra JSON file
    pub fn get_expected_output_path(&self) -> PathBuf {
        // This is a simplified approach; in reality, we would parse Scarb.toml
        // to determine the exact output file name
        self.project_dir.join("target/dev/cairo1.sierra.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    #[ignore] // Ignored by default since it requires scarb to be installed
    fn test_scarb_builder() {
        // This test assumes you have a valid Cairo project in the crates/cairo1-rust-vm directory
        let project_path = Path::new("crates/cairo1-rust-vm");
        
        if !project_path.exists() {
            eprintln!("Test skipped: Cairo project directory not found");
            return;
        }
        
        let builder = ScarbBuilder::new(project_path);
        let result = builder.build();
        
        assert!(result.is_ok(), "Build should succeed: {:?}", result.err());
        let output_path = result.unwrap();
        assert!(output_path.exists(), "Output file should exist");
    }
}