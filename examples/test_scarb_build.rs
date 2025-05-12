use std::path::PathBuf;
use zeroxbridge_sequencer::proof_generator::ScarbBuilder;

fn main() {
    // Set up tracing
    tracing_subscriber::fmt::init();
    
    // Path to Cairo project
    let cairo_project_dir = PathBuf::from("crates/cairo1-rust-vm");
    
    println!("Running scarb build for Cairo project at: {:?}", cairo_project_dir);
    
    // Create a ScarbBuilder
    let builder = ScarbBuilder::new(&cairo_project_dir);
    
    // Attempt to build the project
    match builder.build() {
        Ok(output_path) => {
            println!("✅ Build successful!");
            println!("Output Sierra JSON file: {:?}", output_path);
        },
        Err(err) => {
            eprintln!("❌ Build failed: {}", err);
            std::process::exit(1);
        }
    }
}