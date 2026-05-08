// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    PaperProofDomainChange, PaperProofIndexerState,
    deployment::mainnet_deployment,
    events::{PaperProofEventKind, SuiEventEnvelope, classify_event_type},
    indexer::indexer_batch_from_page,
    query::EventPage,
};
use serde_json::json;

#[test]
fn domain_reducer_emits_and_applies_core_changes() {
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
                    &deployment.packages.comments,
                    "comments",
                    "CommentAddedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "tree_id": "0xtree",
                        "comment_id": "9",
                        "parent_comment_id": "0"
                    }),
                ),
                event(
                    &deployment.packages.governance,
                    "governance_voting",
                    "ProposalFinalizedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "proposal_id": "3",
                        "status": "4"
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

    let changes = PaperProofIndexerState::domain_changes(&batch);
    assert!(matches!(
        changes[0],
        PaperProofDomainChange::SeriesCreated { .. }
    ));
    assert!(matches!(
        changes[1],
        PaperProofDomainChange::CommentAdded { .. }
    ));
    assert!(matches!(
        changes[2],
        PaperProofDomainChange::ProposalResolved { .. }
    ));

    let mut state = PaperProofIndexerState::default();
    state.apply_batch(&batch);
    assert_eq!(state.published_series, 1);
    assert_eq!(state.comments_added, 1);
    assert_eq!(state.proposals_resolved, 1);
    assert_eq!(state.latest_proposal_status.get(&3), Some(&4));
}

#[test]
fn domain_reducer_covers_lifecycle_governance_and_upgrade_events() {
    let deployment = mainnet_deployment();
    let batch = indexer_batch_from_page(
        EventPage {
            data: vec![
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "PaperProofRootCreatedEvent",
                    json!({
                        "root_id": deployment.objects.root,
                        "governance_vault_id": "0xvault",
                        "fee_manager_id": "0xfee",
                        "type_registry_id": "0xregistry"
                    }),
                ),
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "TypeIndexCreatedEvent",
                    json!({
                        "root_id": deployment.objects.root,
                        "artifact_type": "7",
                        "type_index_id": "0xtypeindex"
                    }),
                ),
                event(
                    &deployment.packages.comments,
                    "comments",
                    "TreeCreatedEvent",
                    json!({
                        "tree_id": "0xtree",
                        "registry_id": deployment.objects.root,
                        "target_series_id": "0xseries",
                        "likes_book_id": "0xlikes"
                    }),
                ),
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "ArtifactTypeStatusChangedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "artifact_type": "7",
                        "enabled": false
                    }),
                ),
                event(
                    &deployment.packages.governance,
                    "governance_voting",
                    "ProposalDurationChangedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "old_duration_epochs": "7",
                        "new_duration_epochs": "9"
                    }),
                ),
                event(
                    &deployment.packages.governance,
                    "governance",
                    "FeeCollectedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "payer": "0xpayer",
                        "recipient": "0xrecipient",
                        "amount": "123"
                    }),
                ),
                event(
                    &deployment.packages.governance,
                    "governance",
                    "ManagedUpgradeAuthorizedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "package_id": deployment.packages.publishing
                    }),
                ),
                event(
                    &deployment.packages.comments,
                    "comments",
                    "CommentsTreeMigratedEvent",
                    json!({
                        "registry_id": deployment.objects.root,
                        "tree_id": "0xtree",
                        "new_version": "2"
                    }),
                ),
                event(
                    &deployment.packages.publishing,
                    "publishing",
                    "ProtocolPausedChangedEvent",
                    json!({
                        "root_id": deployment.objects.root,
                        "new_paused": true
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

    let changes = PaperProofIndexerState::domain_changes(&batch);
    assert!(matches!(
        changes[0],
        PaperProofDomainChange::RootCreated { .. }
    ));
    assert!(matches!(
        changes[3],
        PaperProofDomainChange::ArtifactTypeStatusChanged { .. }
    ));
    assert!(matches!(
        changes[4],
        PaperProofDomainChange::GovernanceParameterChanged { .. }
    ));
    assert!(matches!(
        changes[5],
        PaperProofDomainChange::FeeCollected { .. }
    ));
    assert!(matches!(
        changes[6],
        PaperProofDomainChange::ManagedUpgradeChanged { .. }
    ));
    assert!(matches!(
        changes[7],
        PaperProofDomainChange::ObjectMigrated { .. }
    ));

    let mut state = PaperProofIndexerState::default();
    for change in &changes {
        state.apply_change(change);
    }
    assert_eq!(state.total_events, 9);
    assert_eq!(state.type_status_changes, 1);
    assert_eq!(state.governance_parameter_changes, 1);
    assert_eq!(state.fees_collected, 1);
    assert_eq!(state.managed_upgrade_events, 1);
    assert_eq!(state.migrations, 1);
    assert_eq!(
        state.tree_to_series.get("0xtree"),
        Some(&"0xseries".to_string())
    );
    assert_eq!(state.artifact_type_enabled.get(&7), Some(&false));
    assert_eq!(
        state.protocol_paused_by_root.get(&deployment.objects.root),
        Some(&true)
    );
    assert_eq!(
        state.governance_object_by_kind.get("fee_manager"),
        Some(&"0xfee".to_string())
    );
}

#[test]
fn classifier_knows_indexer_relevant_contract_events() {
    for (event_type, kind) in [
        (
            "0x1::publishing::PaperProofRootCreatedEvent",
            PaperProofEventKind::RootCreated,
        ),
        (
            "0x1::comments::TreeCreatedEvent",
            PaperProofEventKind::TreeCreated,
        ),
        (
            "0x1::publishing::ArtifactTypeStatusChangedEvent",
            PaperProofEventKind::ArtifactTypeStatusChanged,
        ),
        (
            "0x1::governance_voting::ProposalCreationPausedChangedEvent",
            PaperProofEventKind::ProposalCreationPausedChanged,
        ),
        (
            "0x1::governance::ManagedUpgradeCommittedEvent",
            PaperProofEventKind::ManagedUpgradeCommitted,
        ),
    ] {
        assert_eq!(classify_event_type(event_type), kind);
    }
}

fn event(
    package: &str,
    module: &str,
    struct_name: &str,
    parsed_json: serde_json::Value,
) -> SuiEventEnvelope {
    SuiEventEnvelope {
        id: Some(json!({ "txDigest": format!("{module}-{struct_name}"), "eventSeq": "0" })),
        package_id: package.to_string(),
        transaction_module: module.to_string(),
        sender: "0x1".to_string(),
        event_type: format!("{package}::{module}::{struct_name}"),
        parsed_json,
        bcs: None,
        timestamp_ms: Some("1700000000000".to_string()),
    }
}
