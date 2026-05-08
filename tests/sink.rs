// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    EventId, IndexedPaperProofEvent, POSTGRES_SCHEMA_SQL, PaperProofEventSink, SQLITE_SCHEMA_SQL,
    accepted_event_to_sql_params, deployment::mainnet_deployment, events::SuiEventEnvelope,
    events_trust::EventTrustResult, indexer::IndexerEventBatch, sink::JsonlEventSink,
};
use serde_json::json;

#[test]
fn sql_schema_exports_core_tables() {
    assert!(POSTGRES_SCHEMA_SQL.contains("paperproof_events"));
    assert!(POSTGRES_SCHEMA_SQL.contains("paperproof_rejected_events"));
    assert!(SQLITE_SCHEMA_SQL.contains("paperproof_indexer_cursors"));
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

fn mock_event() -> IndexedPaperProofEvent {
    let deployment = mainnet_deployment();
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
        event: SuiEventEnvelope {
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
        },
        kind: paperproof_sdk_rs::events::PaperProofEventKind::ArtifactPublished,
        trust: EventTrustResult {
            trusted: true,
            reason: None,
        },
    }
}
