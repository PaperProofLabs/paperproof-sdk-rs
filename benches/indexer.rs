// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use criterion::{Criterion, criterion_group, criterion_main};
use paperproof_sdk_rs::{
    PaperProofIndexerState,
    deployment::mainnet_deployment,
    events::SuiEventEnvelope,
    indexer::{domain_change_from_event, indexer_batch_from_page},
    query::EventPage,
};
use serde_json::json;

fn bench_indexer_batch(c: &mut Criterion) {
    let deployment = mainnet_deployment();
    let page = EventPage {
        data: (0..1_000)
            .map(|index| SuiEventEnvelope {
                id: Some(json!({ "txDigest": format!("digest-{index}"), "eventSeq": "0", "checkpoint": index })),
                package_id: deployment.packages.publishing.clone(),
                transaction_module: "publishing".to_string(),
                sender: "0x1".to_string(),
                event_type: format!(
                    "{}::publishing::ArtifactPublishedEvent",
                    deployment.packages.publishing
                ),
                parsed_json: json!({
                    "root_id": deployment.objects.root,
                    "series_id": format!("0xseries{index}"),
                    "version_id": format!("0xversion{index}"),
                    "comments_tree_id": format!("0xtree{index}"),
                    "likes_book_id": format!("0xlikes{index}")
                }),
                bcs: None,
                timestamp_ms: Some("1700000000000".to_string()),
            })
            .collect(),
        next_cursor: None,
        has_next_page: false,
        raw: json!({}),
    };

    c.bench_function("indexer_filter_and_reduce_1000_events", |b| {
        b.iter(|| {
            let batch = indexer_batch_from_page(page.clone(), &deployment, true);
            let mut state = PaperProofIndexerState::default();
            state.apply_batch(&batch);
            state
        })
    });

    let batch = indexer_batch_from_page(page, &deployment, true);
    c.bench_function("domain_change_1000_events", |b| {
        b.iter(|| {
            batch
                .accepted
                .iter()
                .map(domain_change_from_event)
                .collect::<Vec<_>>()
        })
    });
}

criterion_group!(benches, bench_indexer_batch);
criterion_main!(benches);
