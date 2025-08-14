# L1 Tree Builder Client Usage Guide

## Overview

The L1 Tree Builder Client is a long-running service that manages Merkle tree updates for deposits in the ZeroXBridge Sequencer. It processes deposits with `PENDING_TREE_INCLUSION` status, generates Merkle proofs, and updates the database with the results.

## Prerequisites

### 1. Database Setup

Ensure PostgreSQL is running and accessible. The service requires database migrations to be applied.

**Start Database (Docker):**
```bash
# Copy environment file
cp .env.example .env

# Start PostgreSQL container
make docker-build
make docker-run
```

**Apply Migrations:**
```bash
# Set your DATABASE_URL in .env file
export DATABASE_URL=postgres://postgres:postgres@localhost:5434/zeroxdb

# Run migrations
sqlx migrate run
# or
make migrate-run

# Prepare SQLx offline mode (for production deployments)
cargo sqlx prepare
```

**Note:** The migrations are now embedded in the binary for production deployments, so you don't need to worry about migration paths at runtime.

### 2. Environment Configuration

Create and configure your `.env` file:

```bash
# Database
DATABASE_URL=postgres://postgres:postgres@localhost:5434/zeroxdb

# Logging
RUST_LOG=info

# Optional: Tree Builder specific settings
TREE_BUILDER_POLL_INTERVAL_SECONDS=10
TREE_BUILDER_BATCH_SIZE=100
```

### 3. Configuration File

Update `config.toml` with your tree builder settings:

```toml
[tree_builder]
poll_interval_seconds = 10
batch_size = 100
enable_startup_rebuild = true

[logging]
level = "info"
```

## Running the Tree Builder Client

### Option 1: As Part of the Full Sequencer

Run the complete sequencer with tree builder enabled:

```bash
# Build the project
cargo build --release

# Run with tree builder enabled (default)
cargo run --bin sequencer -- --enable-tree-builder=true

# Or specify custom config
cargo run --bin sequencer -- --config=custom_config.toml
```

### Option 2: Standalone Mode

To run only the tree builder service without other components:

```bash
# Disable other services, run only tree builder
cargo run --bin sequencer -- --enable-tree-builder=true --enable-api=false
```

### Option 3: Integration in Custom Application

```rust
use zeroxbridge_sequencer::tree_builder::client::TreeBuilderClient;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setup database pool
    let database_url = std::env::var("DATABASE_URL")?;
    let db_pool = PgPool::connect(&database_url).await?;
    
    // Create and start tree builder client
    let mut tree_builder = TreeBuilderClient::new(db_pool, 10); // 10 second polling
    tree_builder.start().await?;
    
    Ok(())
}
```

## How It Works

### 1. Startup Process

1. **Tree Reconstruction**: On startup, the client rebuilds the in-memory Merkle tree from all deposits that have `included = true`
2. **Service Initialization**: Starts the periodic polling loop

### 2. Processing Loop

Every `poll_interval_seconds` (default 10), the service:

1. **Query Database**: Fetches deposits with `status = 'PENDING_TREE_INCLUSION'`
2. **Process Deposits**: For each deposit:
   - Decodes the `commitment_hash`
   - Appends to the Merkle tree
   - Generates a proof
   - Updates the database with:
     - `proof`: Merkle proof data (JSONB)
     - `leaf_index`: Position in the tree
     - `merkle_root`: Current tree root
     - `included`: Set to `true`
     - `status`: Changed to `'PENDING_PROOF_GENERATION'`

### 3. Database Schema

The service requires these fields in the `deposits` table:

```sql
-- Added by migration 20250812000000_add_tree_fields_to_deposits.sql
ALTER TABLE deposits 
ADD COLUMN IF NOT EXISTS leaf_index BIGINT,
ADD COLUMN IF NOT EXISTS proof JSONB,
ADD COLUMN IF NOT EXISTS included BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS merkle_root TEXT;
```

## Testing

### Run Unit Tests

```bash
# Test tree builder client specifically
cargo test tree_builder

# Test the tree-builder crate
cargo test -p tree-builder

# Run all tests
cargo test
```

### Integration Testing

```bash
# Ensure database is running
make docker-run

# Set test database URL
export DATABASE_URL=postgres://postgres:postgres@localhost:5434/zeroxdb

# Run integration tests
cargo test --test '*'
```

### Manual Testing

1. **Setup Test Data:**
   ```sql
   INSERT INTO deposits (stark_pub_key, amount, commitment_hash, status) 
   VALUES ('0x123...', 1000000, '0xabc123...', 'PENDING_TREE_INCLUSION');
   ```

2. **Start Tree Builder:**
   ```bash
   RUST_LOG=debug cargo run --bin sequencer -- --enable-api=false
   ```

3. **Monitor Logs:**
   - Watch for "Processing X pending deposits" messages
   - Verify deposits transition to `PENDING_PROOF_GENERATION` status
   - Check proof data is stored in database

## Monitoring and Troubleshooting

### Logging

The service provides detailed logging at different levels:

```bash
# Debug level - verbose
RUST_LOG=debug cargo run --bin sequencer

# Info level - standard
RUST_LOG=info cargo run --bin sequencer

# Error level - errors only  
RUST_LOG=error cargo run --bin sequencer
```

### Common Issues

**1. Database Connection Errors:**
```
Error: Failed to create database pool
```
- Verify `DATABASE_URL` is correct
- Ensure PostgreSQL is running
- Check network connectivity

**2. Migration Errors:**
```
Error: Failed to run database migrations
```
- Run migrations manually: `sqlx migrate run`
- Check migration files exist in `migrations/` directory

**3. Tree Building Errors:**
```
Error processing pending deposits: Failed to append to tree
```
- Check commitment_hash format (should be hex with 0x prefix)
- Verify tree-builder crate dependencies

**4. No Deposits Processing:**
```
No pending deposits to process
```
- Check deposits table has records with `status = 'PENDING_TREE_INCLUSION'`
- Verify `included = false` for pending deposits

### Performance Monitoring

Monitor these metrics:

- **Processing Rate**: Deposits processed per minute
- **Memory Usage**: Tree size vs available memory  
- **Database Query Time**: Time to fetch/update deposits
- **Error Rate**: Failed deposit processing attempts

### Configuration Tuning

Adjust these settings based on your workload:

```toml
[tree_builder]
# Increase for higher throughput
poll_interval_seconds = 5

# Process more deposits per batch
batch_size = 200

# Logging level
[logging]
level = "info"  # debug for troubleshooting
```

## API Integration

The tree builder integrates with the sequencer's deposit flow:

1. **API receives deposit** → `status = 'pending'`
2. **Migration updates** → `status = 'PENDING_TREE_INCLUSION'`
3. **Tree builder processes** → `status = 'PENDING_PROOF_GENERATION'`
4. **Proof service handles** → `status = 'READY_TO_CLAIM'`

## Shutdown

The service handles graceful shutdown on `CTRL+C`:

```
INFO  [zeroxbridge_sequencer] Received shutdown signal
INFO  [zeroxbridge_sequencer] Shutting down services...
INFO  [zeroxbridge_sequencer] Sequencer shutdown complete
```

This ensures:
- Current processing completes
- Database connections are closed properly
- No data corruption occurs