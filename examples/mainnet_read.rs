// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    EventQueryInput, PaginationInput, PaperProofQueryClient, PaperProofReadClient,
    format_deployment_verification, verify_deployment,
};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    let read = PaperProofReadClient::mainnet();
    let verification = verify_deployment(&read).await?;
    println!("{}", format_deployment_verification(&verification));

    let root = read.get_root_view().await?;
    println!("root.paused={:?}", root.paused);

    let query = PaperProofQueryClient::mainnet();
    let events = query
        .query_canonical_events(EventQueryInput {
            package_id: Some(query.deployment.packages.publishing.clone()),
            module: Some("publishing".to_string()),
            pagination: PaginationInput {
                limit: Some(5),
                descending_order: Some(true),
                ..Default::default()
            },
            ..Default::default()
        })
        .await?;
    println!(
        "latest canonical publishing events: count={} has_next_page={}",
        events.data.len(),
        events.has_next_page
    );
    for event in events.data {
        println!(
            "{} sender={} timestamp={:?}",
            event.event_type, event.sender, event.timestamp_ms
        );
    }
    Ok(())
}
