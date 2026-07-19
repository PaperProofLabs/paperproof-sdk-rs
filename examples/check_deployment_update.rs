// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{check_deployment_update_from_url, format_deployment_update_check};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    let result = check_deployment_update_from_url(None, None).await;
    println!("{}", format_deployment_update_check(&result));
    if !result.differences.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
