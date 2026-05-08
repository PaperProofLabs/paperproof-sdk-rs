// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{PaperProofReadClient, verify_deployment};

#[tokio::test]
async fn optional_mainnet_read_verifies_deployment() {
    if std::env::var("PAPERPROOF_RS_MAINNET_READ").ok().as_deref() != Some("1") {
        eprintln!("skipping mainnet read test; set PAPERPROOF_RS_MAINNET_READ=1");
        return;
    }
    let read = PaperProofReadClient::mainnet();
    let verification = verify_deployment(&read).await.unwrap();
    assert!(
        verification.ok,
        "{}",
        paperproof_sdk_rs::format_deployment_verification(&verification)
    );
}
