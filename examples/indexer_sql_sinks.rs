// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    POSTGRES_SCHEMA_SQL, SQLITE_SCHEMA_SQL, accepted_event_to_sql_params,
    deployment::mainnet_deployment,
    events::SuiEventEnvelope,
    indexer::{EventId, IndexedPaperProofEvent},
};
use serde_json::json;

fn main() -> paperproof_sdk_rs::Result<()> {
    println!("Postgres schema:\n{POSTGRES_SCHEMA_SQL}");
    println!("SQLite schema:\n{SQLITE_SCHEMA_SQL}");

    let deployment = mainnet_deployment();
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
        event: SuiEventEnvelope {
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
        },
        kind: paperproof_sdk_rs::events::PaperProofEventKind::ArtifactPublished,
        trust: paperproof_sdk_rs::events_trust::EventTrustResult {
            trusted: true,
            reason: None,
        },
    };
    let params = accepted_event_to_sql_params(&event);
    println!(
        "Example upsert params:\n{}",
        serde_json::to_string_pretty(&params)?
    );
    Ok(())
}
