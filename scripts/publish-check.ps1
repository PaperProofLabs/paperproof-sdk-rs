# Copyright (c) 2026 PaperProof Labs
# SPDX-License-Identifier: Apache-2.0

$ErrorActionPreference = "Stop"

Write-Host "Running PaperProof Rust SDK publish checks..."

cargo fmt --check
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps --all-features
cargo package --allow-dirty

Write-Host "Publish checks completed."
