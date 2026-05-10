// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::{fs::OpenOptions, io::Write, path::Path};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "postgres", feature = "sqlite"))]
use serde_json::Value;

#[cfg(any(feature = "postgres", feature = "sqlite"))]
use crate::{
    error::PaperProofError,
    indexer::{EventId, IndexerCursorStore, StoredIndexerCursor, StreamId},
};
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

#[async_trait]
impl<T> PaperProofEventSink for Box<T>
where
    T: PaperProofEventSink + ?Sized,
{
    async fn write_batch(&self, batch: &IndexerEventBatch) -> Result<SinkWriteSummary> {
        (**self).write_batch(batch).await
    }
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

#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub struct SqliteCursorStore {
    path: String,
}

#[cfg(feature = "sqlite")]
impl SqliteCursorStore {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let store = Self { path: path.into() };
        store.initialize()?;
        Ok(store)
    }

    fn initialize(&self) -> Result<()> {
        let connection = self.connection()?;
        connection
            .execute_batch(SQLITE_SCHEMA_SQL)
            .map_err(sqlite_error("sqlite initialize cursor store"))?;
        Ok(())
    }

    fn connection(&self) -> Result<rusqlite::Connection> {
        rusqlite::Connection::open(&self.path).map_err(sqlite_error(&self.path))
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl IndexerCursorStore for SqliteCursorStore {
    async fn load_cursor(&self, stream: &StreamId) -> Result<Option<StoredIndexerCursor>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "select event_cursor, checkpoint_cursor from paperproof_indexer_cursors where stream_id = ?1",
            )
            .map_err(sqlite_error("sqlite prepare load_cursor"))?;
        let mut rows = statement
            .query([stream.0.as_str()])
            .map_err(sqlite_error("sqlite query load_cursor"))?;
        let Some(row) = rows
            .next()
            .map_err(sqlite_error("sqlite next load_cursor"))?
        else {
            return Ok(None);
        };
        let event_cursor_text: Option<String> = row
            .get(0)
            .map_err(sqlite_error("sqlite read event_cursor"))?;
        let checkpoint_cursor: Option<u64> = row
            .get::<_, Option<i64>>(1)
            .map_err(sqlite_error("sqlite read checkpoint_cursor"))?
            .and_then(|value| u64::try_from(value).ok());
        Ok(Some(StoredIndexerCursor {
            event_cursor: parse_optional_json(event_cursor_text)?,
            checkpoint_cursor: checkpoint_cursor.map(crate::indexer::CheckpointCursor::new),
        }))
    }

    async fn save_cursor(&self, stream: &StreamId, cursor: StoredIndexerCursor) -> Result<()> {
        let event_cursor = match cursor.event_cursor {
            Some(value) => Some(serde_json::to_string(&value)?),
            None => None,
        };
        let checkpoint_cursor = cursor
            .checkpoint_cursor
            .map(|cursor| i64::try_from(cursor.next_checkpoint))
            .transpose()
            .map_err(|_| {
                PaperProofError::invalid_input("checkpoint_cursor", "checkpoint cursor exceeds i64")
            })?;
        let connection = self.connection()?;
        connection
            .execute(
                "insert into paperproof_indexer_cursors (stream_id, event_cursor, checkpoint_cursor, updated_at)
                 values (?1, ?2, ?3, current_timestamp)
                 on conflict(stream_id) do update set
                    event_cursor = excluded.event_cursor,
                    checkpoint_cursor = excluded.checkpoint_cursor,
                    updated_at = current_timestamp",
                rusqlite::params![stream.0, event_cursor, checkpoint_cursor],
            )
            .map_err(sqlite_error("sqlite save_cursor"))?;
        Ok(())
    }

    async fn mark_processed(&self, event_id: &EventId) -> Result<bool> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "insert or ignore into paperproof_processed_events (
                    event_key, checkpoint, transaction_digest, event_seq, package_id,
                    module, event_type
                 ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    event_id.key(),
                    optional_u64_to_i64(event_id.checkpoint)?,
                    event_id.transaction_digest,
                    optional_u64_to_i64(event_id.event_seq)?,
                    event_id.package_id,
                    event_id.module,
                    event_id.event_type,
                ],
            )
            .map_err(sqlite_error("sqlite mark_processed"))?;
        Ok(changed > 0)
    }
}

#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub struct SqliteEventSink {
    path: String,
}

#[cfg(feature = "sqlite")]
impl SqliteEventSink {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let sink = Self { path: path.into() };
        sink.initialize()?;
        Ok(sink)
    }

    fn initialize(&self) -> Result<()> {
        let connection = self.connection()?;
        connection
            .execute_batch(SQLITE_SCHEMA_SQL)
            .map_err(sqlite_error("sqlite initialize event sink"))?;
        Ok(())
    }

    fn connection(&self) -> Result<rusqlite::Connection> {
        rusqlite::Connection::open(&self.path).map_err(sqlite_error(&self.path))
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl PaperProofEventSink for SqliteEventSink {
    async fn write_batch(&self, batch: &IndexerEventBatch) -> Result<SinkWriteSummary> {
        let mut connection = self.connection()?;
        let tx = connection
            .transaction()
            .map_err(sqlite_error("sqlite begin event sink transaction"))?;
        let mut accepted_written = 0;
        let mut rejected_written = 0;
        let mut duplicate_skipped = 0;

        for event in &batch.accepted {
            let changed = tx
                .execute(
                    "insert or ignore into paperproof_events (
                        event_key, checkpoint, transaction_digest, event_seq, package_id,
                        module, event_type, kind, sender, timestamp_ms, parsed_json
                     ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        event.id.key(),
                        optional_u64_to_i64(event.id.checkpoint)?,
                        event.id.transaction_digest,
                        optional_u64_to_i64(event.id.event_seq)?,
                        event.id.package_id,
                        event.id.module,
                        event.id.event_type,
                        format!("{:?}", event.kind),
                        event.event.sender,
                        event
                            .event
                            .timestamp_ms
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok()),
                        serde_json::to_string(&event.event.parsed_json)?,
                    ],
                )
                .map_err(sqlite_error("sqlite insert accepted event"))?;
            if changed > 0 {
                accepted_written += 1;
            } else {
                duplicate_skipped += 1;
            }
        }

        for event in &batch.rejected {
            let id = crate::indexer::event_id(&event.event);
            let changed = tx
                .execute(
                    "insert or ignore into paperproof_rejected_events (
                        event_key, checkpoint, transaction_digest, event_seq, package_id,
                        module, event_type, sender, timestamp_ms, reason, parsed_json
                     ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    rusqlite::params![
                        id.key(),
                        optional_u64_to_i64(id.checkpoint)?,
                        id.transaction_digest,
                        optional_u64_to_i64(id.event_seq)?,
                        id.package_id,
                        id.module,
                        id.event_type,
                        event.event.sender,
                        event
                            .event
                            .timestamp_ms
                            .as_deref()
                            .and_then(|value| value.parse::<i64>().ok()),
                        event.reason,
                        serde_json::to_string(&event.event.parsed_json)?,
                    ],
                )
                .map_err(sqlite_error("sqlite insert rejected event"))?;
            if changed > 0 {
                rejected_written += 1;
            } else {
                duplicate_skipped += 1;
            }
        }

        tx.commit()
            .map_err(sqlite_error("sqlite commit event sink transaction"))?;
        Ok(SinkWriteSummary {
            accepted_written,
            rejected_written,
            duplicate_skipped,
        })
    }
}

#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresCursorStore {
    client: std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>,
}

#[cfg(feature = "postgres")]
impl PostgresCursorStore {
    pub async fn connect(connection_string: &str) -> Result<Self> {
        let (client, connection) =
            tokio_postgres::connect(connection_string, tokio_postgres::NoTls)
                .await
                .map_err(postgres_error("postgres connect cursor store"))?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("paperproof postgres cursor connection closed: {error}");
            }
        });
        Self::from_client(client).await
    }

    pub async fn from_client(client: tokio_postgres::Client) -> Result<Self> {
        let store = Self {
            client: std::sync::Arc::new(tokio::sync::Mutex::new(client)),
        };
        store.initialize().await?;
        Ok(store)
    }

    async fn initialize(&self) -> Result<()> {
        self.client
            .lock()
            .await
            .batch_execute(POSTGRES_SCHEMA_SQL)
            .await
            .map_err(postgres_error("postgres initialize cursor store"))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl IndexerCursorStore for PostgresCursorStore {
    async fn load_cursor(&self, stream: &StreamId) -> Result<Option<StoredIndexerCursor>> {
        let row = self
            .client
            .lock()
            .await
            .query_opt(
                "select event_cursor, checkpoint_cursor from paperproof_indexer_cursors where stream_id = $1",
                &[&stream.0],
            )
            .await
            .map_err(postgres_error("postgres load_cursor"))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let event_cursor: Option<Value> = row.get(0);
        let checkpoint_cursor = row
            .get::<_, Option<i64>>(1)
            .and_then(|value| u64::try_from(value).ok())
            .map(crate::indexer::CheckpointCursor::new);
        Ok(Some(StoredIndexerCursor {
            event_cursor,
            checkpoint_cursor,
        }))
    }

    async fn save_cursor(&self, stream: &StreamId, cursor: StoredIndexerCursor) -> Result<()> {
        let checkpoint_cursor = cursor
            .checkpoint_cursor
            .map(|cursor| i64::try_from(cursor.next_checkpoint))
            .transpose()
            .map_err(|_| {
                PaperProofError::invalid_input("checkpoint_cursor", "checkpoint cursor exceeds i64")
            })?;
        self.client
            .lock()
            .await
            .execute(
                "insert into paperproof_indexer_cursors (stream_id, event_cursor, checkpoint_cursor, updated_at)
                 values ($1, $2, $3, now())
                 on conflict(stream_id) do update set
                    event_cursor = excluded.event_cursor,
                    checkpoint_cursor = excluded.checkpoint_cursor,
                    updated_at = now()",
                &[&stream.0, &cursor.event_cursor, &checkpoint_cursor],
            )
            .await
            .map_err(postgres_error("postgres save_cursor"))?;
        Ok(())
    }

    async fn mark_processed(&self, event_id: &EventId) -> Result<bool> {
        let checkpoint = optional_u64_to_i64(event_id.checkpoint)?;
        let event_seq = optional_u64_to_i64(event_id.event_seq)?;
        let changed = self
            .client
            .lock()
            .await
            .execute(
                "insert into paperproof_processed_events (
                    event_key, checkpoint, transaction_digest, event_seq, package_id,
                    module, event_type
                 ) values ($1, $2, $3, $4, $5, $6, $7)
                 on conflict(event_key) do nothing",
                &[
                    &event_id.key(),
                    &checkpoint,
                    &event_id.transaction_digest,
                    &event_seq,
                    &event_id.package_id,
                    &event_id.module,
                    &event_id.event_type,
                ],
            )
            .await
            .map_err(postgres_error("postgres mark_processed"))?;
        Ok(changed > 0)
    }
}

#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgresEventSink {
    client: std::sync::Arc<tokio::sync::Mutex<tokio_postgres::Client>>,
}

#[cfg(feature = "postgres")]
impl PostgresEventSink {
    pub async fn connect(connection_string: &str) -> Result<Self> {
        let (client, connection) =
            tokio_postgres::connect(connection_string, tokio_postgres::NoTls)
                .await
                .map_err(postgres_error("postgres connect event sink"))?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("paperproof postgres sink connection closed: {error}");
            }
        });
        Self::from_client(client).await
    }

    pub async fn from_client(client: tokio_postgres::Client) -> Result<Self> {
        let sink = Self {
            client: std::sync::Arc::new(tokio::sync::Mutex::new(client)),
        };
        sink.initialize().await?;
        Ok(sink)
    }

    async fn initialize(&self) -> Result<()> {
        self.client
            .lock()
            .await
            .batch_execute(POSTGRES_SCHEMA_SQL)
            .await
            .map_err(postgres_error("postgres initialize event sink"))
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl PaperProofEventSink for PostgresEventSink {
    async fn write_batch(&self, batch: &IndexerEventBatch) -> Result<SinkWriteSummary> {
        let mut client = self.client.lock().await;
        let tx = client
            .transaction()
            .await
            .map_err(postgres_error("postgres begin event sink transaction"))?;
        let mut accepted_written = 0;
        let mut rejected_written = 0;
        let mut duplicate_skipped = 0;

        for event in &batch.accepted {
            let checkpoint = optional_u64_to_i64(event.id.checkpoint)?;
            let event_seq = optional_u64_to_i64(event.id.event_seq)?;
            let timestamp_ms = event
                .event
                .timestamp_ms
                .as_deref()
                .and_then(|value| value.parse::<i64>().ok());
            let changed = tx
                .execute(
                    "insert into paperproof_events (
                        event_key, checkpoint, transaction_digest, event_seq, package_id,
                        module, event_type, kind, sender, timestamp_ms, parsed_json
                     ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                     on conflict(event_key) do nothing",
                    &[
                        &event.id.key(),
                        &checkpoint,
                        &event.id.transaction_digest,
                        &event_seq,
                        &event.id.package_id,
                        &event.id.module,
                        &event.id.event_type,
                        &format!("{:?}", event.kind),
                        &event.event.sender,
                        &timestamp_ms,
                        &event.event.parsed_json,
                    ],
                )
                .await
                .map_err(postgres_error("postgres insert accepted event"))?;
            if changed > 0 {
                accepted_written += 1;
            } else {
                duplicate_skipped += 1;
            }
        }

        for event in &batch.rejected {
            let id = crate::indexer::event_id(&event.event);
            let checkpoint = optional_u64_to_i64(id.checkpoint)?;
            let event_seq = optional_u64_to_i64(id.event_seq)?;
            let timestamp_ms = event
                .event
                .timestamp_ms
                .as_deref()
                .and_then(|value| value.parse::<i64>().ok());
            let changed = tx
                .execute(
                    "insert into paperproof_rejected_events (
                        event_key, checkpoint, transaction_digest, event_seq, package_id,
                        module, event_type, sender, timestamp_ms, reason, parsed_json
                     ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                     on conflict(event_key) do nothing",
                    &[
                        &id.key(),
                        &checkpoint,
                        &id.transaction_digest,
                        &event_seq,
                        &id.package_id,
                        &id.module,
                        &id.event_type,
                        &event.event.sender,
                        &timestamp_ms,
                        &event.reason,
                        &event.event.parsed_json,
                    ],
                )
                .await
                .map_err(postgres_error("postgres insert rejected event"))?;
            if changed > 0 {
                rejected_written += 1;
            } else {
                duplicate_skipped += 1;
            }
        }

        tx.commit()
            .await
            .map_err(postgres_error("postgres commit event sink transaction"))?;
        Ok(SinkWriteSummary {
            accepted_written,
            rejected_written,
            duplicate_skipped,
        })
    }
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

#[cfg(feature = "sqlite")]
fn parse_optional_json(text: Option<String>) -> Result<Option<Value>> {
    text.map(|text| serde_json::from_str(&text))
        .transpose()
        .map_err(Into::into)
}

#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn optional_u64_to_i64(value: Option<u64>) -> Result<Option<i64>> {
    value
        .map(i64::try_from)
        .transpose()
        .map_err(|_| PaperProofError::invalid_input("u64", "value exceeds i64"))
}

#[cfg(feature = "sqlite")]
fn sqlite_error(context: impl Into<String>) -> impl Fn(rusqlite::Error) -> PaperProofError {
    let context = context.into();
    move |error| PaperProofError::network(context.clone(), error.to_string())
}

#[cfg(feature = "postgres")]
fn postgres_error(context: impl Into<String>) -> impl Fn(tokio_postgres::Error) -> PaperProofError {
    let context = context.into();
    move |error| PaperProofError::network(context.clone(), error.to_string())
}
