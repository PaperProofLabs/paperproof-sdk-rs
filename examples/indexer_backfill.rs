// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    EventQueryInput, JsonlEventSink, PaginationInput, PaperProofEventSink, PaperProofIndexerClient,
    PaperProofQueryClient,
};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    init_tracing();
    let limit = env_u64("PAPERPROOF_RS_BACKFILL_LIMIT").unwrap_or(100);
    let pages = env_u64("PAPERPROOF_RS_BACKFILL_PAGES").unwrap_or(1);
    let output_dir = std::env::var("PAPERPROOF_RS_INDEXER_OUT")
        .unwrap_or_else(|_| "examples/artifacts/indexer".to_string());

    let query = PaperProofQueryClient::mainnet();
    let indexer = PaperProofIndexerClient::new(query);
    let sink = JsonlEventSink::new(
        format!("{output_dir}/accepted.jsonl"),
        format!("{output_dir}/rejected.jsonl"),
    );

    for module in PaperProofIndexerClient::canonical_module_filters(&indexer.query.deployment) {
        let mut cursor = None;
        for page_index in 0..pages {
            let batch = indexer
                .scan_once(paperproof_sdk_rs::IndexerScanOptions {
                    filter: EventQueryInput {
                        package_id: Some(module.package_id.clone()),
                        module: Some(module.module.clone()),
                        pagination: PaginationInput {
                            cursor: cursor.clone(),
                            limit: Some(limit),
                            descending_order: Some(false),
                        },
                        ..Default::default()
                    },
                    canonical_only: true,
                })
                .await?;
            cursor = batch.progress.cursor.clone();
            let summary = sink.write_batch(&batch).await?;
            println!(
                "module={} page={} accepted={} rejected={} wrote={}/{}",
                module.module,
                page_index,
                batch.progress.accepted_events,
                batch.progress.rejected_events,
                summary.accepted_written,
                summary.rejected_written
            );
            if !batch.progress.has_next_page {
                break;
            }
        }
    }
    Ok(())
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "paperproof_sdk_rs=info".to_string()),
        )
        .try_init();
}
