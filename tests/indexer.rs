// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use paperproof_sdk_rs::{
    deployment::mainnet_deployment,
    error::Result,
    events::{PaperProofEventKind, SuiEventEnvelope},
    indexer::{
        CheckpointData, CheckpointDataProvider, CheckpointScanOptions, EventId, IndexerCursorStore,
        MemoryIndexerCursorStore, PaperProofIndexerClient, PaperProofIndexerState,
        StoredIndexerCursor, StreamId, event_id, event_kind_counts, indexer_batch_from_page,
    },
    query::{EventPage, PaperProofQueryClient},
};
use serde_json::json;
#[cfg(feature = "async")]
use std::sync::{Arc, Mutex};

#[test]
fn indexer_batch_splits_canonical_and_rejected_events() {
    let deployment = mainnet_deployment();
    let page = EventPage {
        data: vec![
            event(
                &deployment.packages.publishing,
                "publishing",
                "ArtifactPublishedEvent",
                json!({
                    "root_id": deployment.objects.root,
                    "series_id": "0xseries",
                    "version_id": "0xversion",
                    "comments_tree_id": "0xtree",
                    "likes_book_id": "0xlikes"
                }),
            ),
            event(
                "0xfake",
                "publishing",
                "ArtifactPublishedEvent",
                json!({
                    "root_id": deployment.objects.root,
                    "series_id": "0xseries",
                    "version_id": "0xversion",
                    "comments_tree_id": "0xtree",
                    "likes_book_id": "0xlikes"
                }),
            ),
        ],
        next_cursor: Some(json!({ "txDigest": "abc", "eventSeq": "1" })),
        has_next_page: true,
        raw: json!({ "data": [] }),
    };

    let batch = indexer_batch_from_page(page, &deployment, true);
    assert_eq!(batch.accepted.len(), 1);
    assert_eq!(batch.rejected.len(), 1);
    assert_eq!(batch.progress.scanned_events, 2);
    assert_eq!(batch.progress.accepted_events, 1);
    assert_eq!(batch.progress.rejected_events, 1);
    assert!(batch.progress.has_next_page);
    assert!(
        batch.rejected[0]
            .reason
            .contains("configured PaperProof package")
    );
}

#[test]
fn indexer_state_reduces_core_activity_counts() {
    let deployment = mainnet_deployment();
    let batch = indexer_batch_from_page(
        EventPage {
            data: vec![
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "ArtifactPublishedEvent",
                    json!({
                        "root_id": deployment.objects.root,
                        "series_id": "0xseries",
                        "version_id": "0xv1",
                        "comments_tree_id": "0xtree",
                        "likes_book_id": "0xlikes"
                    }),
                ),
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "ArtifactVersionAddedEvent",
                    json!({
                        "root_id": deployment.objects.root,
                        "series_id": "0xseries",
                        "new_version_id": "0xv2"
                    }),
                ),
                event(
                    &deployment.packages.comments,
                    "comments",
                    "CommentAddedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "tree_id": "0xtree",
                        "comment_id": "42"
                    }),
                ),
                event(
                    &deployment.packages.comments,
                    "comments",
                    "PaperLikedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "likes_book_id": "0xlikes",
                        "target_series_id": "0xseries",
                        "like_count": "7"
                    }),
                ),
            ],
            next_cursor: None,
            has_next_page: false,
            raw: json!({}),
        },
        &deployment,
        true,
    );

    let counts = event_kind_counts(&batch.accepted);
    assert_eq!(
        counts.get(&PaperProofEventKind::ArtifactPublished),
        Some(&1)
    );
    assert_eq!(counts.get(&PaperProofEventKind::PaperLiked), Some(&1));

    let mut state = PaperProofIndexerState::default();
    state.apply_batch(&batch);
    assert_eq!(state.total_events, 4);
    assert_eq!(state.published_series, 1);
    assert_eq!(state.versions_added, 1);
    assert_eq!(state.comments_added, 1);
    assert_eq!(state.likes, 1);
    assert_eq!(
        state
            .latest_series_versions
            .get("0xseries")
            .map(String::as_str),
        Some("0xv2")
    );
    assert_eq!(state.latest_comment_by_tree.get("0xtree"), Some(&42));
    assert_eq!(state.latest_like_count_by_book.get("0xlikes"), Some(&7));
}

#[test]
fn canonical_module_filters_cover_core_event_packages() {
    let deployment = mainnet_deployment();
    let filters = PaperProofIndexerClient::canonical_module_filters(&deployment);
    assert_eq!(filters.len(), 3);
    assert!(filters.iter().any(|filter| filter.module == "publishing"));
    assert!(filters.iter().any(|filter| filter.module == "comments"));
    assert!(
        filters
            .iter()
            .any(|filter| filter.module == "governance_voting")
    );
}

#[test]
fn governance_original_events_remain_trusted_for_upgrade_history() {
    let deployment = mainnet_deployment();
    let batch = indexer_batch_from_page(
        EventPage {
            data: vec![event(
                &deployment.packages.governance_original,
                "governance_voting",
                "ProposalCreatedEvent",
                json!({
                    "registry_id": deployment.objects.root,
                    "proposal_id": "1",
                    "proposal_object_id": "0xproposal"
                }),
            )],
            next_cursor: None,
            has_next_page: false,
            raw: json!({}),
        },
        &deployment,
        true,
    );

    assert_eq!(batch.accepted.len(), 1);
    assert_eq!(batch.accepted[0].kind, PaperProofEventKind::ProposalCreated);
}

#[test]
fn event_id_is_stable_and_idempotency_friendly() {
    let deployment = mainnet_deployment();
    let event = event(
        &deployment.packages.comments,
        "comments",
        "CommentAddedEvent",
        json!({
            "registry_id": deployment.objects.root,
            "tree_id": "0xtree",
            "comment_id": "7"
        }),
    );
    let id = event_id(&event);
    assert_eq!(id.transaction_digest.as_deref(), Some("digest"));
    assert_eq!(id.event_seq, Some(0));
    assert_eq!(id.package_id, deployment.packages.comments);
    assert!(id.key().contains("digest:0"));
}

#[tokio::test]
async fn memory_cursor_store_tracks_cursor_and_processed_events() {
    let store = MemoryIndexerCursorStore::default();
    let stream = StreamId("publishing".to_string());
    let cursor = StoredIndexerCursor {
        event_cursor: Some(json!({ "txDigest": "digest", "eventSeq": "0" })),
        checkpoint_cursor: None,
    };
    store.save_cursor(&stream, cursor.clone()).await.unwrap();
    assert_eq!(store.load_cursor(&stream).await.unwrap(), Some(cursor));

    let event_id = EventId {
        checkpoint: Some(10),
        transaction_digest: Some("digest".to_string()),
        event_seq: Some(0),
        package_id: "0x1".to_string(),
        module: "m".to_string(),
        event_type: "0x1::m::E".to_string(),
    };
    assert!(store.mark_processed(&event_id).await.unwrap());
    assert!(!store.mark_processed(&event_id).await.unwrap());
}

#[tokio::test]
async fn checkpoint_scan_defaults_to_canonical_filtering() {
    let deployment = mainnet_deployment();
    let query = PaperProofQueryClient::mainnet();
    let indexer = PaperProofIndexerClient::new(query);
    let provider = MockCheckpointProvider {
        deployment: deployment.clone(),
    };
    let batch = indexer
        .scan_checkpoint_range_once(
            &provider,
            CheckpointScanOptions {
                start_checkpoint: 5,
                limit: 1,
                canonical_only: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(batch.accepted.len(), 1);
    assert_eq!(batch.rejected.len(), 1);
    assert_eq!(batch.progress.scanned_events, 2);
    assert_eq!(batch.accepted[0].id.checkpoint, Some(5));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn checkpoint_ingestion_runs_workers_persists_resume_and_metrics() {
    let query = PaperProofQueryClient::mainnet();
    let indexer = PaperProofIndexerClient::new(query);
    let provider = MockCheckpointProvider {
        deployment: mainnet_deployment(),
    };
    let sink = CollectingSink::default();
    let store = MemoryIndexerCursorStore::default();
    let report = indexer
        .ingest_checkpoint_range_once(
            Arc::new(provider),
            Arc::new(sink.clone()),
            Arc::new(store.clone()),
            paperproof_sdk_rs::CheckpointIngestionOptions {
                start_checkpoint: Some(5),
                checkpoint_count: 4,
                batch_size: 2,
                worker_count: 2,
                max_checkpoints_per_second: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(report.start_checkpoint, 5);
    assert_eq!(report.next_checkpoint, 9);
    assert_eq!(report.metrics.processed_events, 4);
    assert_eq!(report.metrics.rejected_events, 4);
    assert_eq!(report.metrics.checkpoints_scanned, 4);
    assert_eq!(sink.accepted(), 4);
    let stored = store
        .load_cursor(&StreamId::checkpoint())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.checkpoint_cursor.unwrap().next_checkpoint, 9);
}

struct MockCheckpointProvider {
    deployment: paperproof_sdk_rs::Deployment,
}

#[cfg(feature = "async")]
#[derive(Clone, Default)]
struct CollectingSink {
    accepted: Arc<Mutex<usize>>,
}

#[cfg(feature = "async")]
impl CollectingSink {
    fn accepted(&self) -> usize {
        *self.accepted.lock().unwrap()
    }
}

#[cfg(feature = "async")]
#[async_trait]
impl paperproof_sdk_rs::PaperProofEventSink for CollectingSink {
    async fn write_batch(
        &self,
        batch: &paperproof_sdk_rs::IndexerEventBatch,
    ) -> Result<paperproof_sdk_rs::SinkWriteSummary> {
        *self.accepted.lock().unwrap() += batch.accepted.len();
        Ok(paperproof_sdk_rs::SinkWriteSummary {
            accepted_written: batch.accepted.len(),
            rejected_written: batch.rejected.len(),
            duplicate_skipped: 0,
        })
    }
}

#[async_trait]
impl CheckpointDataProvider for MockCheckpointProvider {
    async fn get_checkpoint_data(&self, sequence_number: u64) -> Result<CheckpointData> {
        let canonical = event_with_id(
            &self.deployment.packages.publishing,
            "publishing",
            "ArtifactPublishedEvent",
            json!({
                "root_id": self.deployment.objects.root,
                "series_id": "0xseries",
                "version_id": "0xversion",
                "comments_tree_id": "0xtree",
                "likes_book_id": "0xlikes",
                "checkpoint": sequence_number
            }),
            sequence_number,
            0,
        );
        let fake = event_with_id(
            "0xfake",
            "publishing",
            "ArtifactPublishedEvent",
            json!({
                "root_id": self.deployment.objects.root,
                "series_id": "0xseries",
                "version_id": "0xversion",
                "comments_tree_id": "0xtree",
                "likes_book_id": "0xlikes",
                "checkpoint": sequence_number
            }),
            sequence_number,
            1,
        );
        Ok(CheckpointData {
            sequence_number,
            digest: Some(format!("checkpoint-{sequence_number}")),
            events: vec![canonical, fake],
            raw: json!({ "checkpoint": sequence_number }),
        })
    }
}

fn event(
    package: &str,
    module: &str,
    struct_name: &str,
    parsed_json: serde_json::Value,
) -> SuiEventEnvelope {
    SuiEventEnvelope {
        id: Some(json!({ "txDigest": "digest", "eventSeq": "0" })),
        package_id: package.to_string(),
        transaction_module: module.to_string(),
        sender: "0x1".to_string(),
        event_type: format!("{package}::{module}::{struct_name}"),
        parsed_json,
        bcs: None,
        timestamp_ms: Some("1700000000000".to_string()),
    }
}

fn event_with_id(
    package: &str,
    module: &str,
    struct_name: &str,
    parsed_json: serde_json::Value,
    checkpoint: u64,
    event_seq: u64,
) -> SuiEventEnvelope {
    let mut event = event(package, module, struct_name, parsed_json);
    event.id = Some(json!({
        "txDigest": format!("digest-{checkpoint}"),
        "eventSeq": event_seq.to_string(),
        "checkpoint": checkpoint
    }));
    event
}
