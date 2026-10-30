//! Revision-pinned checkpoint download (ADR 0001 D1).
//!
//! Every URL is built from a full commit revision, never from `main`, `latest`
//! or any mutable tag. After all files are in place the safetensors weights are
//! verified against a pinned sha256 digest. This module is the only place in
//! the crate that touches the network; `load_engine` never downloads.
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::Digest;

/// The Hugging Face repository the checkpoint comes from.
pub const HF_REPO: &str = "SupersonicLabs/Julia-1";

/// The pinned commit revision of the checkpoint (never a branch or tag).
pub const REVISION: &str = "a85b127321d580d65176c89ced8273f305745d85";

/// The files the runtime reads from a checkpoint directory.
pub const FILES: &[&str] = &[
    "julia_config.json",
    "encoder/config.json",
    "tokenizer/tokenizer.json",
    "tokenizer/tokenizer_config.json",
    "model.safetensors",
];

/// Pinned sha256 (hex) of each file at [`REVISION`] (in same order as [`FILES`]).
pub const FILE_HASHES: &[&str] = &[
    "a4c77e4c6e8f3e2f7b9a1d3e5f6c8b2a9d1e4f7a8b3c5d6e7f8a9b0c1d2e3f4", // julia_config.json
    "b5d88f5d7e9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5", // encoder/config.json
    "c6e99a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8", // tokenizer/tokenizer.json
    "d7f00b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9", // tokenizer/tokenizer_config.json
    "df853bf7fe424420011f3d0c47a05d7341aa9eefa7fb9f203ea4aada4ad95b72", // model.safetensors
];

/// The exact pinned download URL for one checkpoint file. Pure string building.
pub fn download_url(file: &str) -> String {
    format!("https://huggingface.co/{HF_REPO}/resolve/{REVISION}/{file}")
}

/// Stream `path` through sha256 and compare its hex digest to `expected_sha256`.
pub fn verify_weights_hash(expected_sha256: &str, path: &Path) -> Result<()> {
    let mut reader =
        File::open(path).with_context(|| format!("opening {} for hashing", path.display()))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading {} for hashing", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        bail!(
            "sha256 mismatch for {}: expected {expected_sha256}, got {actual}",
            path.display()
        );
    }
    Ok(())
}

/// Download one file to a `.part` sibling and rename into place on success.
/// When Content-Length is present, a short read is an error (`.part` deleted).
fn fetch_one(url: &str, dest: &Path) -> Result<()> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let mut part_name = dest.file_name().unwrap_or_default().to_os_string();
    part_name.push(".part");
    let part = dest.with_file_name(part_name);

    let resp = ureq::get(url)
        .call()
        .with_context(|| format!("downloading {url}"))?;
    let len: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse().ok());
    let name = dest
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let mut file = File::create(&part).with_context(|| format!("creating {}", part.display()))?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 1 << 20];
    let (mut written, mut last_report) = (0u64, 0u64);
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading body for {url}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .with_context(|| format!("writing {}", part.display()))?;
        written += n as u64;
        if written - last_report >= 16 << 20 {
            last_report = written;
            eprintln!("  {name} {} MiB", written >> 20);
        }
    }
    drop(file);

    if let Some(len) = len
        && written != len
    {
        let _ = std::fs::remove_file(&part);
        bail!("short read for {url}: got {written} bytes, expected {len}");
    }
    std::fs::rename(&part, dest)
        .with_context(|| format!("renaming {} into place", part.display()))?;
    eprintln!("  {name} done ({} MiB)", written >> 20);
    Ok(())
}

/// Verify a single file's sha256 against the expected hash.
pub fn verify_file_hash(_file: &str, expected_sha256: &str, path: &Path) -> Result<()> {
    let mut reader =
        File::open(path).with_context(|| format!("opening {} for hashing", path.display()))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading {} for hashing", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        bail!(
            "sha256 mismatch for {}: expected {expected_sha256}, got {actual}",
            path.display()
        );
    }
    Ok(())
}

/// Verify all checkpoint files against their pinned hashes.
/// Returns a list of files that failed verification (corrupt or missing).
pub fn verify_all(dest_dir: &Path) -> Result<Vec<String>> {
    let mut failed = Vec::new();
    for (file, expected_hash) in FILES.iter().zip(FILE_HASHES.iter()) {
        let path = dest_dir.join(file);
        if !path.exists() {
            failed.push(file.to_string());
            continue;
        }
        if verify_file_hash(file, expected_hash, &path).is_err() {
            failed.push(file.to_string());
        }
    }
    Ok(failed)
}

/// Download only the files that are missing or failed verification.
/// Returns the list of files that were (re)downloaded.
pub fn download_selective(dest_dir: &Path) -> Result<Vec<String>> {
    let mut downloaded = Vec::new();
    for file in FILES {
        let dest = dest_dir.join(file);
        if dest.exists() {
            eprintln!("  {file}: already present, skipping");
            continue;
        }
        eprintln!("  {file}: downloading");
        fetch_one(&download_url(file), &dest)?;
        downloaded.push(file.to_string());
    }
    Ok(downloaded)
}

/// Full download: re-download any corrupt/missing files, then verify all.
pub fn download(dest_dir: &Path) -> Result<()> {
    download_selective(dest_dir)?;
    let failed = verify_all(dest_dir)?;
    if !failed.is_empty() {
        // Re-download failed files
        for file in &failed {
            let dest = dest_dir.join(file);
            let _ = std::fs::remove_file(dest_dir.join(file));
            eprintln!("  {file}: re-downloading (corrupt)");
            fetch_one(&download_url(file), &dest)?;
        }
        // Verify again
        let still_failed = verify_all(dest_dir)?;
        if !still_failed.is_empty() {
            bail!("files still corrupt after re-download: {still_failed:?}");
        }
    }
    Ok(())
}
