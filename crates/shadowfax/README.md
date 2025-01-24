# Shadowfax 🐎

Shadowfax is a temporary experimental storage system for Ethereum block data in batches, named after the fastest horse in Middle-earth.

## Overview

Built with memory-mapped files and minimal SIMD optimizations, Shadowfax serves as a high-performance temporary storage for palatiri block data. 

## Usage

```rust
use shadowfax::ShadowfaxLSM;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let store = ShadowfaxLSM::new("block_cache");

    // Store a recent block
    let entries = vec![
        (block_number.to_be_bytes().to_vec(), block_data)
    ];
    store.batch_write(entries).await?;

    // Read block data
    let key = block_number.to_be_bytes().to_vec();
    let block = store.parallel_read(vec![key]).await;

    Ok(())
}