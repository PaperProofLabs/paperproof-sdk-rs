// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{PaperProofReadClient, format_deployment_verification, verify_deployment};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    let read = PaperProofReadClient::mainnet();
    let verification = verify_deployment(&read).await?;
    println!("{}", format_deployment_verification(&verification));
    if !verification.ok {
        std::process::exit(1);
    }
    Ok(())
}
