// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    EventQueryInput, EventTrustLevel, PaginationInput, PaperProofQueryClient,
    TrustedEventQueryInput, assert_no_incomplete,
};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    let query = PaperProofQueryClient::mainnet();
    let page = query
        .query_trusted_events(TrustedEventQueryInput {
            query: EventQueryInput {
                move_event_type: Some(format!(
                    "{}::publishing::ArtifactPublishedEvent",
                    query.deployment.packages.publishing
                )),
                pagination: PaginationInput {
                    limit: Some(5),
                    descending_order: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            },
            trust: EventTrustLevel::Verified,
            include_rejected: true,
            verify_walrus: false,
        })
        .await?;

    println!(
        "provider=graphql trust={:?} data={} verification={} rejected={} incomplete={}",
        page.trust,
        page.data.len(),
        page.verification.len(),
        page.rejected.len(),
        page.incomplete.len()
    );
    for report in &page.verification {
        let issues = report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>()
            .join(",");
        println!("{:?} {}", report.status, issues);
    }
    assert_no_incomplete(&page)?;
    Ok(())
}
