# Proof Generator Module

The Proof Generator module is responsible for compiling Cairo programs and generating STARK proofs for verification on L1 and L2.

## Components

- `ScarbBuilder`: A utility for compiling Cairo 1.0 projects using the Scarb build system
- `StarkProver`: Manages the proof generation process, including Cairo compilation and STARK proof generation

## Usage

### Compiling Cairo Programs

```rust
use std::path::PathBuf;
use zeroxbridge_sequencer::proof_generator::ScarbBuilder;

// Initialize the builder with the path to the Cairo project
let cairo_project_dir = PathBuf::from("crates/cairo1-rust-vm");
let builder = ScarbBuilder::new(cairo_project_dir);

// Compile the project
match builder.build() {
    Ok(output_path) => {
        println!("Compilation successful!");
        println!("Output file: {:?}", output_path);
    },
    Err(err) => {
        eprintln!("Compilation failed: {}", err);
    }
}
```

### Prerequisites

- The `scarb` command-line tool must be installed and available in your PATH.
- The Cairo project must have a valid `Scarb.toml` file.

### Configuration

No additional configuration is required beyond having a properly structured Cairo project.

## Error Handling

The `ScarbBuilder` will return informative errors in the following cases:

- If the project directory does not exist
- If the `Scarb.toml` file is not found
- If the `scarb build` command fails
- If the output Sierra JSON file cannot be found

## Testing

To run the tests for this module:

```bash
cargo test -p zeroxbridge_sequencer --test proof_generator
```

Some tests are marked with `#[ignore]` as they require `scarb` to be installed. To run these tests:

```bash
cargo test -p zeroxbridge_sequencer --test proof_generator -- --ignored
```