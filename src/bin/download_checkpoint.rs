//! Download Julia-1 checkpoint to a local directory.

use decide_rs::checkpoint::{FILE_HASHES, FILES, download};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let dest = Path::new("julia-checkpoint");
    println!("Downloading Julia-1 checkpoint to: {}", dest.display());
    println!("Files to download: {:?}", FILES);
    for (file, hash) in FILES.iter().zip(FILE_HASHES.iter()) {
        println!("  {file} -> {hash}");
    }

    download(dest)?;

    println!("Checkpoint downloaded and verified successfully!");
    Ok(())
}
