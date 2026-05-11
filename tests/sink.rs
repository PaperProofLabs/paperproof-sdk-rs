// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "sqlite")]
use paperproof_sdk_rs::indexer::{IndexerCursorStore, StoredIndexerCursor, StreamId};
use paperproof_sdk_rs::{
    EventId, IndexedPaperProofEvent, POSTGRES_SCHEMA_SQL, PaperProofEventSink, SQLITE_SCHEMA_SQL,
    accepted_event_to_sql_params,
    deployment::mainnet_deployment,
    events::SuiEventEnvelope,
    events_trust::{EventTrustLevel, EventTrustResult, verification_report_from_canonical_check},
    indexer::IndexerEventBatch,
    sink::JsonlEventSink,
};
use serde_json::json;

#[test]
fn sql_schema_exports_core_tables() {
    assert!(POSTGRES_SCHEMA_SQL.contains("paperproof_events"));
    assert!(POSTGRES_SCHEMA_SQL.contains("paperproof_rejected_events"));
    assert!(SQLITE_SCHEMA_SQL.contains("paperproof_indexer_cursors"));
    assert!(SQLITE_SCHEMA_SQL.contains("paperproof_processed_events"));
}

#[test]
fn accepted_event_sql_params_include_idempotency_key() {
    let event = mock_event();
    let params = accepted_event_to_sql_params(&event);
    assert_eq!(params["event_key"], event.id.key());
    assert_eq!(params["kind"], "ArtifactPublished");
}

#[tokio::test]
async fn jsonl_sink_writes_accepted_and_rejected_files() {
    let dir = std::env::temp_dir().join(format!("paperproof-rs-sink-{}", std::process::id()));
    let accepted = dir.join("accepted.jsonl");
    let rejected = dir.join("rejected.jsonl");
    let sink = JsonlEventSink::new(
        accepted.to_string_lossy().to_string(),
        rejected.to_string_lossy().to_string(),
    );
    let batch = IndexerEventBatch {
        accepted: vec![mock_event()],
        rejected: vec![],
        progress: Default::default(),
        raw: json!({}),
    };
    let summary = sink.write_batch(&batch).await.unwrap();
    assert_eq!(summary.accepted_written, 1);
    assert!(accepted.exists());
    let content = std::fs::read_to_string(accepted).unwrap();
    assert!(content.contains("ArtifactPublished"));
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_cursor_store_persists_cursor_and_processed_ids() {
    let path = temp_db_path("cursor");
    let store =
        paperproof_sdk_rs::SqliteCursorStore::new(path.to_string_lossy().to_string()).unwrap();
    let stream = StreamId("checkpoint".to_string());
    let cursor = StoredIndexerCursor {
        event_cursor: Some(json!({ "txDigest": "digest", "eventSeq": "1" })),
        checkpoint_cursor: Some(paperproof_sdk_rs::CheckpointCursor::new(42)),
    };
    store.save_cursor(&stream, cursor.clone()).await.unwrap();
    assert_eq!(store.load_cursor(&stream).await.unwrap(), Some(cursor));

    let id = mock_event().id;
    assert!(store.mark_processed(&id).await.unwrap());
    assert!(!store.mark_processed(&id).await.unwrap());
}

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn sqlite_event_sink_batch_upserts_and_skips_duplicates() {
    let path = temp_db_path("events");
    let sink = paperproof_sdk_rs::SqliteEventSink::new(path.to_string_lossy().to_string()).unwrap();
    let batch = IndexerEventBatch {
        accepted: vec![mock_event()],
        rejected: vec![],
        progress: Default::default(),
        raw: json!({}),
    };
    let first = sink.write_batch(&batch).await.unwrap();
    assert_eq!(first.accepted_written, 1);
    assert_eq!(first.duplicate_skipped, 0);

    let second = sink.write_batch(&batch).await.unwrap();
    assert_eq!(second.accepted_written, 0);
    assert_eq!(second.duplicate_skipped, 1);
}

fn mock_event() -> IndexedPaperProofEvent {
    let deployment = mainnet_deployment();
    let event = SuiEventEnvelope {
        id: Some(json!({ "txDigest": "digest", "eventSeq": "0", "checkpoint": 1 })),
        package_id: deployment.packages.publishing.clone(),
        transaction_module: "publishing".to_string(),
        sender: "0x1".to_string(),
        event_type: format!(
            "{}::publishing::ArtifactPublishedEvent",
            deployment.packages.publishing
        ),
        parsed_json: json!({ "root_id": deployment.objects.root }),
        bcs: None,
        timestamp_ms: Some("1700000000000".to_string()),
    };
    IndexedPaperProofEvent {
        id: EventId {
            checkpoint: Some(1),
            transaction_digest: Some("digest".to_string()),
            event_seq: Some(0),
            package_id: deployment.packages.publishing.clone(),
            module: "publishing".to_string(),
            event_type: format!(
                "{}::publishing::ArtifactPublishedEvent",
                deployment.packages.publishing
            ),
        },
        verification: verification_report_from_canonical_check(
            &event,
            &deployment,
            EventTrustLevel::Canonical,
        ),
        event,
        kind: paperproof_sdk_rs::events::PaperProofEventKind::ArtifactPublished,
        trust: EventTrustResult::trusted(),
    }
}

#[cfg(feature = "sqlite")]
fn temp_db_path(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "paperproof-rs-{label}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    path
}
