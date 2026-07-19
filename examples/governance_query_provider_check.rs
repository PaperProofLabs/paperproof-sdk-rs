// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{EventQueryInput, PaginationInput, PaperProofQueryClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let query = PaperProofQueryClient::mainnet();
    let page = query
        .query_governance_proposal_created_events(Some(EventQueryInput {
            pagination: PaginationInput {
                limit: Some(20),
                descending_order: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }))
        .await?;
    println!(
        "provider=graphql proposal_created_events={} sample={}",
        page.data.len(),
        page.data
            .first()
            .map(|event| event.parsed_json.to_string())
            .unwrap_or_else(|| "null".to_string())
    );
    Ok(())
}
