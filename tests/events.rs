// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    deployment::mainnet_deployment,
    event_verifier::{PaperProofEventVerifier, VerifyEventOptions},
    events::{
        PaperProofEventKind, SuiEventEnvelope, extract_owner_transferred_result,
        extract_proposal_expired_result, extract_status_changed_result,
        extract_vote_claimed_result, parse_event,
    },
    events_trust::{
        EventTrustLevel, EventVerificationStatus, validate_event_trust,
        verification_report_from_canonical_check,
    },
    read::PaperProofReadClient,
    JsonRpcClient,
};
use serde_json::json;

#[test]
fn parses_known_event_kind() {
    let deployment = mainnet_deployment();
    let event = SuiEventEnvelope {
        id: None,
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
            "version_id": "0xversion",
            "comments_tree_id": "0xtree",
            "likes_book_id": "0xlikes"
        }),
        bcs: None,
        timestamp_ms: None,
    };
    let parsed = parse_event(&event);
    assert_eq!(parsed.kind, PaperProofEventKind::ArtifactPublished);
    assert!(validate_event_trust(&event, &deployment).trusted);
}

#[test]
fn canonical_trust_reports_explain_rejections() {
    let deployment = mainnet_deployment();
    let good = event(
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
    );
    let report =
        verification_report_from_canonical_check(&good, &deployment, EventTrustLevel::Canonical);
    assert_eq!(report.status, EventVerificationStatus::Canonical);
    assert!(report.trusted);

    let fake = event(
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
    );
    let report =
        verification_report_from_canonical_check(&fake, &deployment, EventTrustLevel::Canonical);
    assert_eq!(report.status, EventVerificationStatus::Rejected);
    assert_eq!(report.issues[0].code, "PACKAGE_NOT_CONFIGURED");
}

#[test]
fn rejects_fake_package_event() {
    let deployment = mainnet_deployment();
    let event = SuiEventEnvelope {
        id: None,
        package_id: "0x1234".to_string(),
        transaction_module: "comments".to_string(),
        sender: "0x1".to_string(),
        event_type: "0x1234::comments::CommentAddedEvent".to_string(),
        parsed_json: json!({ "root_id": deployment.objects.root }),
        bcs: None,
        timestamp_ms: None,
    };
    let result = validate_event_trust(&event, &deployment);
    assert!(!result.trusted);
    assert!(
        result
            .reason
            .unwrap()
            .contains("configured PaperProof package")
    );
}

#[test]
fn rejects_wrong_root_event() {
    let deployment = mainnet_deployment();
    let event = SuiEventEnvelope {
        id: None,
        package_id: deployment.packages.comments.clone(),
        transaction_module: "comments".to_string(),
        sender: "0x1".to_string(),
        event_type: format!(
            "{}::comments::CommentAddedEvent",
            deployment.packages.comments
        ),
        parsed_json: json!({ "root_id": "0x1234" }),
        bcs: None,
        timestamp_ms: None,
    };
    assert!(!validate_event_trust(&event, &deployment).trusted);
}

#[test]
fn extracts_additional_governance_and_status_events() {
    let deployment = mainnet_deployment();
    let events = vec![
        event(
            &deployment.packages.governance,
            "governance_voting",
            "ProposalExpiredEvent",
            json!({
                "registry_id": deployment.objects.root,
                "proposal_id": "7",
                "expired_at_epoch": "99"
            }),
        ),
        event(
            &deployment.packages.governance,
            "governance_voting",
            "VoteClaimedEvent",
            json!({
                "registry_id": deployment.objects.root,
                "proposal_id": 7,
                "voter": "0xabc",
                "side": 1,
                "voting_power": "100"
            }),
        ),
        event(
            &deployment.packages.publishing,
            "publishing",
            "ProtocolPausedChangedEvent",
            json!({
                "root_id": deployment.objects.root,
                "changed_by": "0xabc",
                "old_paused": false,
                "new_paused": true
            }),
        ),
    ];

    assert_eq!(
        extract_proposal_expired_result(&events, Some(&deployment))
            .unwrap()
            .unwrap()
            .proposal_id,
        7
    );
    assert_eq!(
        extract_vote_claimed_result(&events, Some(&deployment))
            .unwrap()
            .unwrap()
            .voting_power,
        100
    );
    assert_eq!(
        extract_status_changed_result(&events, "ProtocolPausedChangedEvent", Some(&deployment))
            .unwrap()
            .unwrap()
            .new_paused,
        Some(true)
    );
}

#[test]
fn extracts_owner_transfer_event() {
    let deployment = mainnet_deployment();
    let events = vec![event(
        &deployment.packages.comments,
        "comments",
        "TreeOwnerTransferredEvent",
        json!({
            "registry_id": deployment.objects.root,
            "tree_id": "0xtree",
            "changed_by": "0xold",
            "old_owner": "0xold",
            "new_owner": "0xnew"
        }),
    )];
    let result = extract_owner_transferred_result(&events, Some(&deployment))
        .unwrap()
        .unwrap();
    assert_eq!(result.tree_id.as_deref(), Some("0xtree"));
    assert_eq!(result.new_owner, "0xnew");
}

#[tokio::test]
async fn verified_unknown_event_type_is_incomplete() {
    let deployment = mainnet_deployment();
    let read = PaperProofReadClient::new(JsonRpcClient::new("http://127.0.0.1:9"), deployment.clone());
    let verifier = PaperProofEventVerifier::new(read);
    let event = event(
        &deployment.packages.publishing,
        "publishing",
        "FutureEvent",
        json!({ "root_id": deployment.objects.root }),
    );
    let report = verifier
        .verify_event(
            &event,
            VerifyEventOptions {
                trust: EventTrustLevel::Verified,
                verify_walrus: false,
                provider: None,
            },
        )
        .await
        .expect("verify future event");
    assert_eq!(report.status, EventVerificationStatus::Incomplete);
    assert!(!report.trusted);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "VERIFIED_RULE_NOT_REGISTERED")
    );
}

fn event(
    package: &str,
    module: &str,
    struct_name: &str,
    parsed_json: serde_json::Value,
) -> SuiEventEnvelope {
    SuiEventEnvelope {
        id: None,
        package_id: package.to_string(),
        transaction_module: module.to_string(),
        sender: "0x1".to_string(),
        event_type: format!("{package}::{module}::{struct_name}"),
        parsed_json,
        bcs: None,
        timestamp_ms: None,
    }
}
