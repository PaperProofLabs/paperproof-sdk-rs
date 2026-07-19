// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    CliExecutionOutput,
    deployment::mainnet_deployment,
    events::{
        events_from_value, extract_comment_result, extract_proposal_result, extract_publish_result,
    },
};
use serde_json::json;

#[test]
fn extracts_publish_result_from_cli_output() {
    let deployment = mainnet_deployment();
    let output = CliExecutionOutput {
        status_success: true,
        digest: Some("digest".to_string()),
        raw_stdout: String::new(),
        raw_stderr: String::new(),
        json: Some(json!({
            "events": [{
                "packageId": deployment.packages.publishing,
                "transactionModule": "publishing",
                "sender": "0x1",
                "type": format!("{}::publishing::ArtifactPublishedEvent", deployment.packages.publishing),
                "parsedJson": {
                    "series_id": "0x1111",
                    "version_id": "0x2222",
                    "comments_tree_id": "0x3333",
                    "likes_book_id": "0x4444",
                    "artifact_code": "PaperProof-preprint-test",
                    "artifact_type": 1
                }
            }]
        })),
    };
    let result = output.publish_result(&deployment).unwrap();
    assert_eq!(result.series_id, "0x1111");
    assert_eq!(result.comments_tree_id, "0x3333");
    assert_eq!(result.artifact_type, 1);
}

#[test]
fn extracts_comment_result_with_string_numbers() {
    let deployment = mainnet_deployment();
    let value = json!({
        "events": [{
            "packageId": deployment.packages.comments,
            "transactionModule": "comments",
            "sender": "0x1",
            "type": format!("{}::comments::CommentAddedEvent", deployment.packages.comments),
            "parsedJson": {
                "tree_id": "0x3333",
                "comment_id": "7",
                "parent_comment_id": "0",
                "content_mode": 1.0
            }
        }]
    });
    let events = events_from_value(&value).unwrap();
    let result = extract_comment_result(&events, Some(&deployment)).unwrap();
    assert_eq!(result.comment_id, 7);
    assert_eq!(result.content_mode, 1);
}

#[test]
fn extracts_proposal_result() {
    let deployment = mainnet_deployment();
    let value = json!({
        "events": [{
            "packageId": deployment.packages.governance,
            "transactionModule": "governance_voting",
            "sender": "0x1",
            "type": format!("{}::governance_voting::ProposalCreatedEvent", deployment.packages.governance),
            "parsedJson": {
                "proposal_id": "42",
                "proposal_object_id": "0xabcd",
                "action_type": 101,
                "proposal_type": 2
            }
        }]
    });
    let events = events_from_value(&value).unwrap();
    let result = extract_proposal_result(&events, Some(&deployment)).unwrap();
    assert_eq!(result.proposal_id, 42);
    assert_eq!(result.proposal_object_id, "0xabcd");
}

#[test]
fn rejects_wrong_package_when_extracting() {
    let deployment = mainnet_deployment();
    let value = json!({
        "events": [{
            "packageId": "0x1234",
            "transactionModule": "publishing",
            "sender": "0x1",
            "type": "0x1234::publishing::ArtifactPublishedEvent",
            "parsedJson": {
                "series_id": "0x1111",
                "version_id": "0x2222",
                "comments_tree_id": "0x3333",
                "likes_book_id": "0x4444",
                "artifact_code": "fake",
                "artifact_type": 1
            }
        }]
    });
    let events = events_from_value(&value).unwrap();
    assert!(extract_publish_result(&events, Some(&deployment)).is_err());
}
