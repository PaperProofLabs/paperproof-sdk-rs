// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use paperproof_sdk_rs::{
    JsonlEventSink, PaperProofEventSink, PaperProofIndexerClient, PaperProofQueryClient,
};

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    init_tracing();
    if std::env::var("PAPERPROOF_RS_TAIL").ok().as_deref() != Some("1") {
        println!("Set PAPERPROOF_RS_TAIL=1 to run the polling tail example.");
        return Ok(());
    }

    let interval_ms = env_u64("PAPERPROOF_RS_TAIL_INTERVAL_MS").unwrap_or(10_000);
    let limit = env_u64("PAPERPROOF_RS_TAIL_LIMIT").unwrap_or(25);
    let output_dir = std::env::var("PAPERPROOF_RS_INDEXER_OUT")
        .unwrap_or_else(|_| "examples/artifacts/indexer".to_string());

    let query = PaperProofQueryClient::mainnet();
    let indexer = PaperProofIndexerClient::new(query);
    let sink = JsonlEventSink::new(
        format!("{output_dir}/tail-accepted.jsonl"),
        format!("{output_dir}/tail-rejected.jsonl"),
    );
    let modules = PaperProofIndexerClient::canonical_module_filters(&indexer.query.deployment);
    let mut progress = vec![None; modules.len()];

    loop {
        for (index, module) in modules.iter().cloned().enumerate() {
            let batch = indexer
                .scan_module_once(module.clone(), progress[index].clone(), Some(limit))
                .await?;
            progress[index] = Some(batch.progress.clone());
            let summary = sink.write_batch(&batch).await?;
            println!(
                "tail module={} accepted={} rejected={} wrote={}/{}",
                module.module,
                batch.progress.accepted_events,
                batch.progress.rejected_events,
                summary.accepted_written,
                summary.rejected_written
            );
        }
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
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
