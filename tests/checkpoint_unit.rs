//! Network-free unit tests for the pinned checkpoint downloader.
//!
//! None of these tests touch the network: they check URL pinning, the file
//! list, and sha256 verification against a locally written fixture.
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use decide_rs::checkpoint::{self, FILES, HF_REPO, REVISION, WEIGHTS_SHA256};

#[test]
fn download_url_is_revision_pinned_and_never_floating() {
    let url = checkpoint::download_url("model.safetensors");
    assert!(url.starts_with("https://huggingface.co/SupersonicLabs/Julia-1/resolve/"));
    assert!(url.contains(&format!("/resolve/{REVISION}/")));
    assert!(url.ends_with("/model.safetensors"));
    assert!(!url.contains("main"), "floating reference in {url}");
    assert!(!url.contains("latest"), "floating reference in {url}");
    assert_eq!(HF_REPO, "SupersonicLabs/Julia-1");
    assert_eq!(REVISION.len(), 40, "revision must be a full commit sha");
    assert!(REVISION.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn files_are_exactly_the_five_expected_in_order() {
    assert_eq!(
        FILES,
        [
            "julia_config.json",
            "encoder/config.json",
            "tokenizer/tokenizer.json",
            "tokenizer/tokenizer_config.json",
            "model.safetensors",
        ]
    );
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "decide-rs-checkpoint-unit-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn verify_weights_hash_accepts_correct_and_rejects_wrong_digest() {
    use sha2::{Digest, Sha256};

    let dir = unique_temp_dir();
    let path = dir.join("fixture.bin");
    let contents = b"decide-rs checkpoint hash fixture";
    fs::File::create(&path)
        .expect("create fixture")
        .write_all(contents)
        .expect("write fixture");

    let correct = format!("{:x}", Sha256::digest(contents));
    checkpoint::verify_weights_hash(&correct, &path).expect("correct digest must verify");

    let err = checkpoint::verify_weights_hash(WEIGHTS_SHA256, &path)
        .expect_err("wrong digest must fail")
        .to_string();
    assert!(err.contains("mismatch"), "unhelpful error: {err}");
    assert!(
        err.contains(&correct),
        "error must include actual digest: {err}"
    );
    assert!(
        err.contains(WEIGHTS_SHA256),
        "error must include expected digest: {err}"
    );

    let missing = dir.join("absent.bin");
    assert!(
        checkpoint::verify_weights_hash(&correct, &missing).is_err(),
        "missing file must fail"
    );
    fs::remove_dir_all(&dir).ok();
}
