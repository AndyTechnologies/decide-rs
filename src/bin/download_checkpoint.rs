//! Download Julia-1 checkpoint to a local directory.

use decide_rs::checkpoint::{download, FILES, WEIGHTS_SHA256};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let dest = Path::new("julia-checkpoint");
    println!("Downloading Julia-1 checkpoint to: {}", dest.display());
    println!("Files to download: {:?}", FILES);
    println!("Expected weights SHA256: {}", WEIGHTS_SHA256);
    
    download(dest)?;
    
    println!("Checkpoint downloaded and verified successfully!");
    Ok(())
}