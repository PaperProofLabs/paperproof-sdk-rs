// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::{fs::OpenOptions, io::Write, path::Path};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    indexer::{IndexedPaperProofEvent, IndexerEventBatch, RejectedPaperProofEvent},
};

pub const POSTGRES_SCHEMA_SQL: &str = include_str!("../sql/postgres_indexer_schema.sql");
pub const SQLITE_SCHEMA_SQL: &str = include_str!("../sql/sqlite_indexer_schema.sql");

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct SinkWriteSummary {
    pub accepted_written: usize,
    pub rejected_written: usize,
    pub duplicate_skipped: usize,
}

#[async_trait]
pub trait PaperProofEventSink: Send + Sync {
    async fn write_batch(&self, batch: &IndexerEventBatch) -> Result<SinkWriteSummary>;
}

#[derive(Clone, Debug)]
pub struct JsonlEventSink {
    pub accepted_path: String,
    pub rejected_path: String,
}

impl JsonlEventSink {
    pub fn new(accepted_path: impl Into<String>, rejected_path: impl Into<String>) -> Self {
        Self {
            accepted_path: accepted_path.into(),
            rejected_path: rejected_path.into(),
        }
    }
}

#[async_trait]
impl PaperProofEventSink for JsonlEventSink {
    async fn write_batch(&self, batch: &IndexerEventBatch) -> Result<SinkWriteSummary> {
        append_jsonl(&self.accepted_path, &batch.accepted)?;
        append_jsonl(&self.rejected_path, &batch.rejected)?;
        Ok(SinkWriteSummary {
            accepted_written: batch.accepted.len(),
            rejected_written: batch.rejected.len(),
            duplicate_skipped: 0,
        })
    }
}

pub fn accepted_event_to_sql_params(event: &IndexedPaperProofEvent) -> serde_json::Value {
    serde_json::json!({
        "event_key": event.id.key(),
        "checkpoint": event.id.checkpoint,
        "transaction_digest": event.id.transaction_digest,
        "event_seq": event.id.event_seq,
        "package_id": event.id.package_id,
        "module": event.id.module,
        "event_type": event.id.event_type,
        "kind": format!("{:?}", event.kind),
        "sender": event.event.sender,
        "timestamp_ms": event.event.timestamp_ms,
        "parsed_json": event.event.parsed_json,
    })
}

pub fn rejected_event_to_sql_params(event: &RejectedPaperProofEvent) -> serde_json::Value {
    let id = crate::indexer::event_id(&event.event);
    serde_json::json!({
        "event_key": id.key(),
        "checkpoint": id.checkpoint,
        "transaction_digest": id.transaction_digest,
        "event_seq": id.event_seq,
        "package_id": id.package_id,
        "module": id.module,
        "event_type": id.event_type,
        "sender": event.event.sender,
        "timestamp_ms": event.event.timestamp_ms,
        "reason": event.reason,
        "parsed_json": event.event.parsed_json,
    })
}

fn append_jsonl<T>(path: &str, items: &[T]) -> Result<()>
where
    T: Serialize,
{
    if items.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| crate::error::PaperProofError::Network {
            endpoint: path.to_string(),
            message: err.to_string(),
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| crate::error::PaperProofError::Network {
            endpoint: path.to_string(),
            message: err.to_string(),
        })?;
    for item in items {
        let line = serde_json::to_string(item)?;
        writeln!(file, "{line}").map_err(|err| crate::error::PaperProofError::Network {
            endpoint: path.to_string(),
            message: err.to_string(),
        })?;
    }
    Ok(())
}
