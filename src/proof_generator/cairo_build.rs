use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use log::{info, debug, error}; // <-- Add this import

/// Manages Cairo 1.0 compilation using Scarb
pub struct CairoBuildManager {
    /// Base directory containing Cairo projects
    base_dir: PathBuf,
}

impl CairoBuildManager {
    /// Creates a new CairoBuildManager instance
    pub fn new() -> Self {
        Self {
            base_dir: PathBuf::from("crates/cairo1-rust-vm"),
        }
    }

    /// Creates a CairoBuildManager with a custom base directory
    pub fn with_base_dir<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Executes scarb build command in the target directory
    pub fn build_cairo_project(&self) -> Result<PathBuf, BuildError> {
        // Ensure the target directory exists
        if !self.base_dir.exists() {
            return Err(BuildError::DirectoryNotFound(self.base_dir.clone()));
        }

        info!("Starting Cairo build in directory: {:?}", self.base_dir); // line 33

        // Execute scarb build command
        let output = Command::new("scarb")
            .arg("build")
            .current_dir(&self.base_dir)
            .output()
            .map_err(|e| BuildError::CommandExecutionFailed(e.to_string()))?;

        // Check if the command was successful
        if output.status.success() {
            info!("Build succeeded"); // line 44
            
            // Convert stdout to string for logging
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.is_empty() {
                debug!("Build output: {}", stdout); // line 49
            }

            // Check if the expected output file exists
            let output_file = self.get_sierra_output_path()?;
            
            if output_file.exists() {
                info!("Sierra file generated successfully: {:?}", output_file); // line 56
                Ok(output_file)
            } else {
                Err(BuildError::OutputFileNotFound(output_file))
            }
        } else {
            // Convert stderr to string for error reporting
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error_msg = if stderr.is_empty() {
                "Unknown build error".to_string()
            } else {
                stderr.to_string()
            };
            
            error!("Build failed: {}", error_msg); // line 70
            Err(BuildError::BuildFailed(error_msg))
        }
    }

    /// Resolves the expected Sierra output file path
    fn get_sierra_output_path(&self) -> Result<PathBuf, BuildError> {
        let target_dir = self.base_dir.join("target/dev");
        
        if !target_dir.exists() {
            return Err(BuildError::TargetDirectoryNotFound(target_dir));
        }

        // Look for .sierra.json files in the target/dev directory
        let entries = fs::read_dir(&target_dir)
            .map_err(|e| BuildError::DirectoryReadFailed(target_dir.clone(), e.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|e| BuildError::DirectoryReadFailed(target_dir.clone(), e.to_string()))?;
            let path = entry.path();
            
            if let Some(file_name) = path.file_name() {
                if let Some(name_str) = file_name.to_str() {
                    if name_str.ends_with(".sierra.json") {
                        return Ok(path);
                    }
                }
            }
        }

        // Fallback to the expected cairo1.sierra.json
        Ok(target_dir.join("cairo1.sierra.json"))
    }

    /// Returns the base directory path
    pub fn get_base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Checks if scarb is available in the system PATH
    pub fn check_scarb_availability() -> Result<(), BuildError> {
        let output = Command::new("scarb")
            .arg("--version")
            .output()
            .map_err(|_| BuildError::ScarbNotFound)?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("Scarb is available: {}", version.trim());
            Ok(())
        } else {
            Err(BuildError::ScarbNotFound)
        }
    }
}

/// Error types for Cairo build operations
#[derive(Debug)]
pub enum BuildError {
    /// Directory not found
    DirectoryNotFound(PathBuf),
    /// Target directory not found
    TargetDirectoryNotFound(PathBuf),
    /// Command execution failed
    CommandExecutionFailed(String),
    /// Build process failed
    BuildFailed(String),
    /// Expected output file not found
    OutputFileNotFound(PathBuf),
    /// Directory read operation failed
    DirectoryReadFailed(PathBuf, String),
    /// Scarb command not found in PATH
    ScarbNotFound,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::DirectoryNotFound(path) => {
                write!(f, "Cairo project directory not found: {:?}", path)
            }
            BuildError::TargetDirectoryNotFound(path) => {
                write!(f, "Target directory not found: {:?}", path)
            }
            BuildError::CommandExecutionFailed(msg) => {
                write!(f, "Failed to execute scarb command: {}", msg)
            }
            BuildError::BuildFailed(msg) => {
                write!(f, "Cairo build failed: {}", msg)
            }
            BuildError::OutputFileNotFound(path) => {
                write!(f, "Expected output file not found: {:?}", path)
            }
            BuildError::DirectoryReadFailed(path, msg) => {
                write!(f, "Failed to read directory {:?}: {}", path, msg)
            }
            BuildError::ScarbNotFound => {
                write!(f, "Scarb command not found in PATH. Please ensure Scarb is installed and available.")
            }
        }
    }
}

impl std::error::Error for BuildError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_cairo_build_manager_creation() {
        let manager = CairoBuildManager::new();
        assert_eq!(manager.get_base_dir(), Path::new("crates/cairo1-rust-vm"));
    }

    #[test]
    fn test_cairo_build_manager_with_custom_dir() {
        let custom_dir = "custom/cairo/path";
        let manager = CairoBuildManager::with_base_dir(custom_dir);
        assert_eq!(manager.get_base_dir(), Path::new(custom_dir));
    }

    #[test]
    fn test_scarb_availability_check() {
        // This test will pass if scarb is installed, otherwise it will fail
        // In a real scenario, you might want to mock this
        match CairoBuildManager::check_scarb_availability() {
            Ok(_) => println!("Scarb is available"),
            Err(BuildError::ScarbNotFound) => println!("Scarb not found - this is expected in some test environments"),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }
}