// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(feature = "postgres", feature = "sqlite"))]
use paperproof_sdk_rs::indexer::IndexerEventBatch;
use paperproof_sdk_rs::{
    POSTGRES_SCHEMA_SQL, SQLITE_SCHEMA_SQL, accepted_event_to_sql_params,
    deployment::mainnet_deployment,
    events::SuiEventEnvelope,
    events_trust::{EventTrustLevel, EventTrustResult, verification_report_from_canonical_check},
    indexer::{EventId, IndexedPaperProofEvent},
};
use serde_json::json;

#[tokio::main]
async fn main() -> paperproof_sdk_rs::Result<()> {
    println!("Postgres schema:\n{POSTGRES_SCHEMA_SQL}");
    println!("SQLite schema:\n{SQLITE_SCHEMA_SQL}");

    let deployment = mainnet_deployment();
    let envelope = SuiEventEnvelope {
        id: Some(json!({ "txDigest": "example-digest", "eventSeq": "0", "checkpoint": 1 })),
        package_id: deployment.packages.publishing.clone(),
        transaction_module: "publishing".to_string(),
        sender: "0x1".to_string(),
        event_type: format!(
            "{}::publishing::ArtifactPublishedEvent",
            deployment.packages.publishing
        ),
        parsed_json: json!({
            "root_id": deployment.objects.root,
            "series_id": "0xseries",
            "version_id": "0xversion"
        }),
        bcs: None,
        timestamp_ms: Some("1700000000000".to_string()),
    };
    let event = IndexedPaperProofEvent {
        id: EventId {
            checkpoint: Some(1),
            transaction_digest: Some("example-digest".to_string()),
            event_seq: Some(0),
            package_id: deployment.packages.publishing.clone(),
            module: "publishing".to_string(),
            event_type: format!(
                "{}::publishing::ArtifactPublishedEvent",
                deployment.packages.publishing
            ),
        },
        verification: verification_report_from_canonical_check(
            &envelope,
            &deployment,
            EventTrustLevel::Canonical,
        ),
        event: envelope,
        kind: paperproof_sdk_rs::events::PaperProofEventKind::ArtifactPublished,
        trust: EventTrustResult::trusted(),
    };
    let params = accepted_event_to_sql_params(&event);
    println!(
        "Example upsert params:\n{}",
        serde_json::to_string_pretty(&params)?
    );

    #[cfg(feature = "sqlite")]
    {
        use paperproof_sdk_rs::PaperProofEventSink;

        let path = std::env::temp_dir().join("paperproof-indexer-example.sqlite");
        let sink = paperproof_sdk_rs::SqliteEventSink::new(path.to_string_lossy().to_string())?;
        let summary = sink
            .write_batch(&IndexerEventBatch {
                accepted: vec![event.clone()],
                rejected: vec![],
                progress: Default::default(),
                raw: json!({ "example": true }),
            })
            .await?;
        println!("SQLite sink wrote {summary:?} into {}", path.display());
    }

    #[cfg(feature = "postgres")]
    if let Ok(connection_string) = std::env::var("PAPERPROOF_RS_POSTGRES_URL") {
        use paperproof_sdk_rs::PaperProofEventSink;

        let sink = paperproof_sdk_rs::PostgresEventSink::connect(&connection_string).await?;
        let summary = sink
            .write_batch(&IndexerEventBatch {
                accepted: vec![event.clone()],
                rejected: vec![],
                progress: Default::default(),
                raw: json!({ "example": true }),
            })
            .await?;
        println!("Postgres sink wrote {summary:?}");
    }
    Ok(())
}
