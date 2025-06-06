use std::path::Path;

// Import the proof generator modules
// Note: Adjust the import path based on your actual crate structure
use zeroxbridge_sequencer::proof_generator::{
    CairoBuildManager, StarkProver, BuildError, ProofError
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ZeroXBridge Cairo Build Integration Demo");
    println!("==========================================\n");

    // Initialize logging
    setup_logging();

    // Demo 1: Basic Cairo Build Manager
    demo_cairo_build_manager().await?;
    
    println!("\n{}", "=".repeat(60));
    
    // Demo 2: STARK Prover Integration
    demo_stark_prover().await?;
    
    println!("\n{}", "=".repeat(60));
    
    // Demo 3: Advanced Usage
    demo_advanced_usage().await?;

    println!("\n✨ Demo completed successfully!");
    Ok(())
}

async fn demo_cairo_build_manager() -> Result<(), BuildError> {
    println!("🔧 Demo 1: Cairo Build Manager");
    println!("------------------------------");

    // Check system requirements
    println!("📋 Checking system requirements...");
    
    match CairoBuildManager::check_scarb_availability() {
        Ok(_) => println!("✅ Scarb is available"),
        Err(e) => {
            println!("❌ Scarb check failed: {}", e);
            println!("💡 Please install Scarb from: https://docs.swmansion.com/scarb/");
            return Ok(()); // Don't fail the demo, just skip
        }
    }

    // Create build manager
    let build_manager = CairoBuildManager::new();
    println!("📁 Target directory: {:?}", build_manager.get_base_dir());

    // Check if project exists
    if !build_manager.get_base_dir().exists() {
        println!("⚠️  Cairo project not found. Creating sample project...");
        create_sample_cairo_project().await?;
    }

    // Attempt build
    println!("🔨 Building Cairo project...");
    match build_manager.build_cairo_project() {
        Ok(sierra_file) => {
            println!("✅ Build successful!");
            println!("📄 Generated Sierra file: {:?}", sierra_file);
            
            if let Ok(metadata) = std::fs::metadata(&sierra_file) {
                println!("📊 File size: {} bytes", metadata.len());
            }
        }
        Err(e) => {
            println!("❌ Build failed: {}", e);
            println!("💡 This might be expected if the Cairo project isn't set up");
        }
    }

    Ok(())
}

async fn demo_stark_prover() -> Result<(), ProofError> {
    println!("🔐 Demo 2: STARK Prover Integration");
    println!("----------------------------------");

    // Create STARK prover
    let prover = StarkProver::new();
    println!("🎯 Initialized STARK prover");
    println!("📁 Cairo project: {:?}", prover.get_build_manager().get_base_dir());

    // Simulate proof generation scenarios
    let test_cases = vec![
        ("0x1234567890abcdef1234567890abcdef12345678", "Deposit proof"),
        ("0xfedcba0987654321fedcba0987654321fedcba09", "Withdrawal proof"),
        ("0xabcdef1234567890abcdef1234567890abcdef12", "Balance update proof"),
    ];

    for (merkle_root, description) in test_cases {
        println!("\n🧮 Generating {} for root: {}", description, merkle_root);
        
        match prover.generate_proof(merkle_root) {
            Ok(proof) => {
                println!("✅ Proof generated: {}", proof);
                println!("🔗 Ready for L1/L2 relay");
            }
            Err(e) => {
                println!("❌ Proof generation failed: {}", e);
            }
        }
    }

    Ok(())
}

async fn demo_advanced_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Demo 3: Advanced Usage");
    println!("------------------------");

    // Demo custom build scenarios
    println!("🔧 Testing custom build options...");
    
    let project_dir = Path::new("crates/cairo1-rust-vm");
    if project_dir.exists() {
        use zeroxbridge_sequencer::proof_generator::execute_scarb_build_with_options;
        
        // Test different build configurations
        let build_configs = vec![
            (vec![], "Standard build"),
            (vec!["--release"], "Release build"),
            (vec!["--json"], "JSON output build"),
        ];

        for (args, description) in build_configs {
            println!("\n🔨 Running: {}", description);
            match execute_scarb_build_with_options(project_dir, &args) {
                Ok(output) => {
                    println!("✅ {} completed", description);
                    if !output.trim().is_empty() && args.contains(&"--json") {
                        println!("📄 Build output: {}", output.chars().take(200).collect::<String>());
                    }
                }
                Err(e) => {
                    println!("❌ {} failed: {}", description, e);
                }
            }
        }
    }

    // Demo error handling
    println!("\n🚨 Testing error handling...");
    let invalid_prover = StarkProver::with_cairo_dir("/non/existent/path");
    match invalid_prover.generate_proof("0x123") {
        Ok(_) => println!("❌ Expected this to fail!"),
        Err(e) => println!("✅ Correctly handled error: {}", e),
    }

    Ok(())
}

async fn create_sample_cairo_project() -> Result<(), BuildError> {
    use std::fs;
    
    let cairo_dir = Path::new("crates/cairo1-rust-vm");
    
    println!("🏗️  Creating sample Cairo project...");
    
    // Create directories
    fs::create_dir_all(cairo_dir.join("src"))
        .map_err(|e| BuildError::DirectoryNotFound(cairo_dir.to_path_buf()))?;

    // Create Scarb.toml
    let scarb_toml = r#"[package]
name = "cairo1"
version = "0.1.0"
edition = "2023_11"

[dependencies]
starknet = ">=2.3.0"

[[target.starknet-contract]]
"#;
    fs::write(cairo_dir.join("Scarb.toml"), scarb_toml)
        .map_err(|e| BuildError::CommandExecutionFailed(e.to_string()))?;

    // Create simple Cairo contract
    let cairo_source = r#"#[starknet::contract]
mod SimpleContract {
    #[storage]
    struct Storage {
        value: u256,
    }

    #[abi(embed_v0)]
    impl SimpleContractImpl of super::ISimpleContract<ContractState> {
        fn get_value(self: @ContractState) -> u256 {
            self.value.read()
        }
        
        fn set_value(ref self: ContractState, new_value: u256) {
            self.value.write(new_value);
        }
    }
}

#[starknet::interface]
trait ISimpleContract<TContractState> {
    fn get_value(self: @TContractState) -> u256;
    fn set_value(ref self: TContractState, new_value: u256);
}
"#;
    fs::write(cairo_dir.join("src/lib.cairo"), cairo_source)
        .map_err(|e| BuildError::CommandExecutionFailed(e.to_string()))?;

    println!("✅ Sample Cairo project created");
    Ok(())
}

fn setup_logging() {
    // Simple logging setup for the demo
    println!("📝 Demo logging initialized");
}

// Helper function to demonstrate the integration in different contexts
pub async fn integration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Integration Example: How to use in your sequencer");
    println!("---------------------------------------------------");

    // This shows how you might integrate this into the actual sequencer
    let prover = StarkProver::new();
    
    // Simulate a deposit scenario
    let deposit_merkle_root = "0xdeposit123456789abcdef";
    println!("💰 Processing deposit with merkle root: {}", deposit_merkle_root);
    
    match prover.generate_proof(deposit_merkle_root) {
        Ok(proof) => {
            println!("✅ Deposit proof ready for L2 relay");
            println!("🔐 Proof: {}", proof);
            
            // Here you would normally send the proof to L2
            println!("📡 Would relay to Starknet L2...");
        }
        Err(e) => {
            println!("❌ Deposit proof failed: {}", e);
        }
    }

    // Simulate a withdrawal scenario
    let withdrawal_merkle_root = "0xwithdrawal987654321fedcba";
    println!("\n💸 Processing withdrawal with merkle root: {}", withdrawal_merkle_root);
    
    match prover.generate_proof(withdrawal_merkle_root) {
        Ok(proof) => {
            println!("✅ Withdrawal proof ready for L1 relay");
            println!("🔐 Proof: {}", proof);
            
            // Here you would normally send the proof to L1
            println!("📡 Would relay to Ethereum L1...");
        }
        Err(e) => {
            println!("❌ Withdrawal proof failed: {}", e);
        }
    }

    Ok(())
}