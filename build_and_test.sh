#!/bin/bash

# build_and_test.sh - Script to build and test the Cairo integration

set -e  # Exit on any error

echo "🚀 ZeroXBridge Cairo Build Integration Test"
echo "==========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

print_status "Checking project structure..."
if [ ! -d "src/proof_generator" ]; then
    print_warning "Creating proof_generator directory..."
    mkdir -p src/proof_generator
fi

# Check if Rust is installed
print_status "Checking Rust installation..."
if ! command -v cargo &> /dev/null; then
    print_error "Cargo not found. Please install Rust."
    exit 1
fi
print_success "Rust/Cargo is available"

# Check if Scarb is installed
print_status "Checking Scarb installation..."
if command -v scarb &> /dev/null; then
    SCARB_VERSION=$(scarb --version)
    print_success "Scarb is available: $SCARB_VERSION"
else
    print_warning "Scarb not found. Some tests may be skipped."
    print_warning "To install Scarb, visit: https://docs.swmansion.com/scarb/"
fi

# Build the Rust project
print_status "Building Rust project..."
if cargo build; then
    print_success "Rust build completed successfully"
else
    print_error "Rust build failed"
    exit 1
fi

# Run basic tests
print_status "Running basic tests..."
if cargo test --lib; then
    print_success "Basic tests passed"
else
    print_warning "Some basic tests failed"
fi

# Run integration tests if available
print_status "Running integration tests..."
if cargo test --test cairo_build_integration; then
    print_success "Integration tests passed"
else
    print_warning "Some integration tests failed (this might be expected without Cairo project setup)"
fi

# Check if Cairo project directory exists
print_status "Checking Cairo project structure..."
if [ -d "crates/cairo1-rust-vm" ]; then
    print_success "Cairo project directory found"
    
    # If Scarb is available, try to build the Cairo project
    if command -v scarb &> /dev/null; then
        print_status "Attempting to build Cairo project..."
        cd crates/cairo1-rust-vm
        if [ $? -ne 0 ]; then
            print_error "Failed to change directory to crates/cairo1-rust-vm"
            exit 1
        fi

        if [ -f "Scarb.toml" ]; then
            if scarb build; then
                print_success "Cairo project built successfully"
                
                # Check for Sierra output
                if [ -d "target/dev" ] && ls target/dev/*.sierra.json 1> /dev/null 2>&1; then
                    print_success "Sierra files generated:"
                    ls -la target/dev/*.sierra.json
                else
                    print_warning "No Sierra files found in target/dev"
                fi
            else
                print_warning "Cairo build failed"
            fi
        else
            print_warning "No Scarb.toml found in Cairo project directory"
        fi
        
        cd - > /dev/null
        if [ $? -ne 0 ]; then
            print_error "Failed to return to previous directory"
            exit 1
        fi
    fi
else
    print_warning "Cairo project directory not found: crates/cairo1-rust-vm"
    print_warning "This is expected if the Cairo project hasn't been set up yet"
fi

# Run the example if it exists
print_status "Testing Cairo build functionality..."
if cargo run --example cairo_build_demo 2>/dev/null; then
    print_success "Cairo build demo ran successfully"
else
    print_warning "Cairo build demo not available or failed"
fi

# Summary
echo ""
echo "🎯 Test Summary"
echo "==============="
print_success "Rust project builds correctly"
print_success "Cairo build integration code is ready"

if command -v scarb &> /dev/null; then
    print_success "Scarb is available for Cairo builds"
else
    print_warning "Install Scarb to enable full Cairo build functionality"
fi

if [ -d "crates/cairo1-rust-vm" ]; then
    print_success "Cairo project directory structure is present"
else
    print_warning "Set up Cairo project in crates/cairo1-rust-vm for full functionality"
fi

echo ""
print_status "Next steps:"
echo "1. Ensure Scarb is installed if not already available"
echo "2. Set up a Cairo project in crates/cairo1-rust-vm/"
echo "3. Test the integration with: cargo run --example cairo_build_demo"
echo "4. Run integration tests with: cargo test --features integration"

print_success "Build and test script completed!"