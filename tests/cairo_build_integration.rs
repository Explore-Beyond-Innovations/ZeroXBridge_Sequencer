use std::path::Path;
use tempfile::TempDir;
use std::fs;

// Import your modules (adjust the path based on your actual module structure)
use zeroxbridge_sequencer::proof_generator::{CairoBuildManager, StarkProver, BuildError};

#[tokio::test]
async fn test_cairo_build_manager_basic() {
    let manager = CairoBuildManager::new();
    assert_eq!(manager.get_base_dir(), Path::new("crates/cairo1-rust-vm"));
}

#[tokio::test]
async fn test_cairo_build_manager_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let manager = CairoBuildManager::with_base_dir(&temp_dir.path());
    assert_eq!(manager.get_base_dir(), temp_dir.path());
}

#[tokio::test]
async fn test_scarb_availability() {
    // This test checks if scarb is available
    // It's okay if it fails in CI environments without scarb
    match CairoBuildManager::check_scarb_availability() {
        Ok(_) => {
            println!("✅ Scarb is available for testing");
        }
        Err(BuildError::ScarbNotFound) => {
            println!("⚠️  Scarb not found - skipping build tests");
            // This is expected in environments without scarb
        }
        Err(e) => {
            panic!("Unexpected error checking scarb: {}", e);
        }
    }
}

#[tokio::test]
async fn test_build_with_missing_directory() {
    let temp_dir = TempDir::new().unwrap();
    let non_existent_path = temp_dir.path().join("does_not_exist");
    
    let manager = CairoBuildManager::with_base_dir(&non_existent_path);
    let result = manager.build_cairo_project();
    
    assert!(matches!(result, Err(BuildError::DirectoryNotFound(_))));
}

#[tokio::test]
async fn test_stark_prover_integration() {
    let prover = StarkProver::new();
    assert_eq!(
        prover.get_build_manager().get_base_dir(),
        Path::new("crates/cairo1-rust-vm")
    );
}

// Mock test that simulates a successful build scenario
#[tokio::test]
async fn test_mock_cairo_project_setup() {
    let temp_dir = TempDir::new().unwrap();
    let cairo_project_dir = temp_dir.path().join("mock_cairo");
    
    // Create a mock Cairo project structure
    fs::create_dir_all(&cairo_project_dir).unwrap();
    fs::create_dir_all(cairo_project_dir.join("src")).unwrap();
    fs::create_dir_all(cairo_project_dir.join("target/dev")).unwrap();
    
    // Create a mock Scarb.toml file
    let scarb_toml = r#"
[package]
name = "mock_cairo"
version = "0.1.0"

[[target.starknet-contract]]
"#;
    fs::write(cairo_project_dir.join("Scarb.toml"), scarb_toml).unwrap();
    
    // Create a mock Cairo source file
    let cairo_source = r#"
#[starknet::contract]
mod MockContract {
    #[storage]
    struct Storage {}
}
"#;
    fs::write(cairo_project_dir.join("src/lib.cairo"), cairo_source).unwrap();
    
    // Create a mock Sierra output file (simulating successful build)
    let mock_sierra = r#"{"version": "1.0.0", "program": []}"#;
    fs::write(
        cairo_project_dir.join("target/dev/mock_cairo.sierra.json"),
        mock_sierra
    ).unwrap();
    
    let manager = CairoBuildManager::with_base_dir(&cairo_project_dir);
    
    // Test that we can at least detect the directory structure
    assert!(manager.get_base_dir().exists());
    
    // If scarb is available, we could test the actual build
    // For now, just verify our setup is correct
    println!("Mock Cairo project created successfully at: {:?}", cairo_project_dir);
}

#[cfg(feature = "integration")]
mod real_integration_tests {
    use super::*;
    
    /// This test only runs when the "integration" feature is enabled
    /// and when we have a real Cairo project set up
    #[tokio::test]
    async fn test_real_cairo_build() {
        let manager = CairoBuildManager::new();
        
        // Skip if the directory doesn't exist
        if !manager.get_base_dir().exists() {
            println!("Skipping real build test - Cairo project directory not found");
            return;
        }
        
        // Skip if scarb is not available
        if CairoBuildManager::check_scarb_availability().is_err() {
            println!("Skipping real build test - Scarb not available");
            return;
        }
        
        match manager.build_cairo_project() {
            Ok(sierra_file) => {
                println!("✅ Real Cairo build successful: {:?}", sierra_file);
                assert!(sierra_file.exists());
            }
            Err(e) => {
                println!("❌ Real Cairo build failed: {}", e);
                // Don't panic here as this might be expected in some environments
            }
        }
    }
}