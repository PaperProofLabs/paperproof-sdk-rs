// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "sui-native")]
use std::sync::Arc;

use paperproof_sdk_rs::{
    EventQueryInput, IndexerCursorStore, JsonlEventSink, MemoryIndexerCursorStore, PaginationInput,
    PaperProofError, PaperProofEventSink, PaperProofIndexerClient, PaperProofQueryClient,
    StoredIndexerCursor, StreamId,
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
    let sink = build_sink(&output_dir, "backfill").await?;
    let cursor_store = build_cursor_store(&output_dir).await?;

    if checkpoint_ingestion_enabled() {
        run_checkpoint_backfill(indexer, sink, cursor_store).await?;
        return Ok(());
    }

    for module in PaperProofIndexerClient::canonical_module_filters(&indexer.query.deployment) {
        let stream = StreamId::from(&module);
        let mut cursor = cursor_store
            .load_cursor(&stream)
            .await?
            .and_then(|stored| stored.event_cursor);
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
                    trust_policy: paperproof_sdk_rs::IndexerTrustPolicy::Canonical,
                })
                .await?;
            cursor = batch.progress.cursor.clone();
            let summary = sink.write_batch(&batch).await?;
            cursor_store
                .save_cursor(
                    &stream,
                    StoredIndexerCursor {
                        event_cursor: cursor.clone(),
                        checkpoint_cursor: None,
                    },
                )
                .await?;
            println!(
                "module={} page={} accepted={} rejected={} wrote={}/{} duplicate_skipped={}",
                module.module,
                page_index,
                batch.progress.accepted_events,
                batch.progress.rejected_events,
                summary.accepted_written,
                summary.rejected_written,
                summary.duplicate_skipped
            );
            if !batch.progress.has_next_page {
                break;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "sui-native")]
async fn run_checkpoint_backfill(
    indexer: PaperProofIndexerClient,
    sink: Box<dyn PaperProofEventSink>,
    cursor_store: Box<dyn IndexerCursorStore>,
) -> paperproof_sdk_rs::Result<()> {
    let provider =
        paperproof_sdk_rs::SuiNativeProvider::new(indexer.query.deployment.rpc_url.clone())?;
    let report = indexer
        .ingest_checkpoint_range_once(
            Arc::new(provider),
            Arc::new(sink),
            Arc::new(cursor_store),
            paperproof_sdk_rs::CheckpointIngestionOptions {
                start_checkpoint: env_u64("PAPERPROOF_RS_CHECKPOINT_START"),
                checkpoint_count: env_u64("PAPERPROOF_RS_CHECKPOINT_COUNT").unwrap_or(100),
                batch_size: env_u64("PAPERPROOF_RS_CHECKPOINT_BATCH").unwrap_or(10),
                worker_count: env_u64("PAPERPROOF_RS_CHECKPOINT_WORKERS")
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or(4),
                max_checkpoints_per_second: env_u64("PAPERPROOF_RS_CHECKPOINTS_PER_SECOND"),
                ..Default::default()
            },
        )
        .await?;
    println!(
        "checkpoint_backfill start={} next={} processed={} rejected={} lag={:?} db_write_latency_ms={} retries={}",
        report.start_checkpoint,
        report.next_checkpoint,
        report.metrics.processed_events,
        report.metrics.rejected_events,
        report.metrics.checkpoint_lag,
        report.metrics.db_write_latency_ms,
        report.metrics.retry_count
    );
    Ok(())
}

#[cfg(not(feature = "sui-native"))]
async fn run_checkpoint_backfill(
    _indexer: PaperProofIndexerClient,
    _sink: Box<dyn PaperProofEventSink>,
    _cursor_store: Box<dyn IndexerCursorStore>,
) -> paperproof_sdk_rs::Result<()> {
    Err(PaperProofError::invalid_input(
        "PAPERPROOF_RS_CHECKPOINT",
        "checkpoint ingestion requires `--features sui-native`",
    ))
}

async fn build_sink(
    output_dir: &str,
    prefix: &str,
) -> paperproof_sdk_rs::Result<Box<dyn PaperProofEventSink>> {
    match std::env::var("PAPERPROOF_RS_SINK")
        .unwrap_or_else(|_| "jsonl".to_string())
        .as_str()
    {
        "jsonl" => Ok(Box::new(JsonlEventSink::new(
            format!("{output_dir}/{prefix}-accepted.jsonl"),
            format!("{output_dir}/{prefix}-rejected.jsonl"),
        ))),
        "sqlite" => sqlite_sink(output_dir),
        "postgres" => postgres_sink().await,
        other => Err(PaperProofError::invalid_input(
            "PAPERPROOF_RS_SINK",
            format!("unsupported sink `{other}`; expected jsonl, sqlite, or postgres"),
        )),
    }
}

async fn build_cursor_store(
    output_dir: &str,
) -> paperproof_sdk_rs::Result<Box<dyn IndexerCursorStore>> {
    match std::env::var("PAPERPROOF_RS_SINK")
        .unwrap_or_else(|_| "jsonl".to_string())
        .as_str()
    {
        "jsonl" => Ok(Box::<MemoryIndexerCursorStore>::default()),
        "sqlite" => sqlite_cursor_store(output_dir),
        "postgres" => postgres_cursor_store().await,
        other => Err(PaperProofError::invalid_input(
            "PAPERPROOF_RS_SINK",
            format!("unsupported sink `{other}`; expected jsonl, sqlite, or postgres"),
        )),
    }
}

#[cfg(feature = "sqlite")]
fn sqlite_sink(output_dir: &str) -> paperproof_sdk_rs::Result<Box<dyn PaperProofEventSink>> {
    Ok(Box::new(paperproof_sdk_rs::SqliteEventSink::new(
        sqlite_path(output_dir)?,
    )?))
}

#[cfg(not(feature = "sqlite"))]
fn sqlite_sink(_output_dir: &str) -> paperproof_sdk_rs::Result<Box<dyn PaperProofEventSink>> {
    Err(PaperProofError::invalid_input(
        "PAPERPROOF_RS_SINK",
        "sqlite sink requires `--features sqlite`",
    ))
}

#[cfg(feature = "sqlite")]
fn sqlite_cursor_store(output_dir: &str) -> paperproof_sdk_rs::Result<Box<dyn IndexerCursorStore>> {
    Ok(Box::new(paperproof_sdk_rs::SqliteCursorStore::new(
        sqlite_path(output_dir)?,
    )?))
}

#[cfg(not(feature = "sqlite"))]
fn sqlite_cursor_store(
    _output_dir: &str,
) -> paperproof_sdk_rs::Result<Box<dyn IndexerCursorStore>> {
    Err(PaperProofError::invalid_input(
        "PAPERPROOF_RS_SINK",
        "sqlite cursor store requires `--features sqlite`",
    ))
}

#[cfg(feature = "sqlite")]
fn sqlite_path(output_dir: &str) -> paperproof_sdk_rs::Result<String> {
    Ok(std::env::var("PAPERPROOF_RS_SQLITE_PATH")
        .unwrap_or_else(|_| format!("{output_dir}/paperproof-indexer.sqlite")))
}

#[cfg(feature = "postgres")]
async fn postgres_sink() -> paperproof_sdk_rs::Result<Box<dyn PaperProofEventSink>> {
    Ok(Box::new(
        paperproof_sdk_rs::PostgresEventSink::connect(&postgres_url()?).await?,
    ))
}

#[cfg(not(feature = "postgres"))]
async fn postgres_sink() -> paperproof_sdk_rs::Result<Box<dyn PaperProofEventSink>> {
    Err(PaperProofError::invalid_input(
        "PAPERPROOF_RS_SINK",
        "postgres sink requires `--features postgres`",
    ))
}

#[cfg(feature = "postgres")]
async fn postgres_cursor_store() -> paperproof_sdk_rs::Result<Box<dyn IndexerCursorStore>> {
    Ok(Box::new(
        paperproof_sdk_rs::PostgresCursorStore::connect(&postgres_url()?).await?,
    ))
}

#[cfg(not(feature = "postgres"))]
async fn postgres_cursor_store() -> paperproof_sdk_rs::Result<Box<dyn IndexerCursorStore>> {
    Err(PaperProofError::invalid_input(
        "PAPERPROOF_RS_SINK",
        "postgres cursor store requires `--features postgres`",
    ))
}

#[cfg(feature = "postgres")]
fn postgres_url() -> paperproof_sdk_rs::Result<String> {
    std::env::var("PAPERPROOF_RS_POSTGRES_URL").map_err(|_| {
        PaperProofError::invalid_input(
            "PAPERPROOF_RS_POSTGRES_URL",
            "set PAPERPROOF_RS_POSTGRES_URL when PAPERPROOF_RS_SINK=postgres",
        )
    })
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

fn checkpoint_ingestion_enabled() -> bool {
    std::env::var("PAPERPROOF_RS_CHECKPOINT").ok().as_deref() == Some("1")
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "paperproof_sdk_rs=info".to_string()),
        )
        .try_init();
}
