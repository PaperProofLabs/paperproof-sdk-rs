// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

use paperproof_sdk_rs::{
    EventTrustLevel, JsonRpcClient, PaperProofQueryClient, PaperProofWatchClient, WatchOptions,
    deployment::{DeploymentPackageFamily, deployment_package_ids, mainnet_deployment},
};
use serde_json::{Value, json};

fn mock_query_client(calls: Arc<Mutex<Vec<String>>>, responses: usize) -> PaperProofQueryClient {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rpc");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        for _ in 0..responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0_u8; 8192];
            let size = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..size]);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("{}");
            let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            let event_type = value
                .pointer("/params/0/MoveEventType")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    Box::leak(
                        format!(
                            "{}::publishing::ArtifactPublishedEvent",
                            mainnet_deployment().packages.publishing
                        )
                        .into_boxed_str(),
                    )
                })
                .to_string();
            calls.lock().expect("calls").push(event_type.clone());
            let package_id = event_type.split("::").next().unwrap_or_default();
            let module = event_type.split("::").nth(1).unwrap_or_default();
            let response = json!({
                "jsonrpc": "2.0",
                "id": value.get("id").cloned().unwrap_or(json!(1)),
                "result": {
                    "data": [{
                        "id": { "txDigest": "digest", "eventSeq": "1" },
                        "packageId": package_id,
                        "transactionModule": module,
                        "sender": "0x1111111111111111111111111111111111111111111111111111111111111111",
                        "type": event_type,
                        "parsedJson": {
                            "root_id": mainnet_deployment().objects.root,
                            "registry_id": mainnet_deployment().objects.root,
                            "series_id": "0x1111",
                            "version_id": "0x2222",
                            "comments_tree_id": "0x3333",
                            "likes_book_id": "0x4444",
                            "target_series_id": "0x1111",
                            "comment_id": "1"
                        }
                    }],
                    "nextCursor": null,
                    "hasNextPage": false
                }
            });
            let text = response.to_string();
            let http = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                text.len(),
                text
            );
            stream.write_all(http.as_bytes()).expect("write response");
        }
    });
    let deployment = mainnet_deployment();
    PaperProofQueryClient::new_jsonrpc(JsonRpcClient::new(format!("http://{addr}")), deployment)
}

#[tokio::test]
async fn watch_verified_events_returns_trust_reports() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let query = mock_query_client(calls, 2);
    let watch = PaperProofWatchClient::new(query);
    let page = watch
        .watch_verified_events(WatchOptions {
            limit: Some(1),
            ..Default::default()
        })
        .next()
        .await
        .expect("verified watch");
    assert_eq!(page.trust, EventTrustLevel::Verified);
    assert_eq!(page.incomplete.len(), 1);
    assert_eq!(page.data.len(), 0);
}

#[tokio::test]
async fn verified_typed_watch_helper_applies_move_event_filter() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let deployment = mainnet_deployment();
    let expected = deployment_package_ids(&deployment, DeploymentPackageFamily::Publishing)
        .into_iter()
        .map(|package_id| format!("{package_id}::publishing::ArtifactStatusChangedEvent"))
        .collect::<Vec<_>>();
    let query = mock_query_client(calls.clone(), expected.len());
    let watch = PaperProofWatchClient::new(query);
    let page = watch
        .watch_verified_publishing_events(
            "ArtifactStatusChangedEvent",
            WatchOptions {
                limit: Some(1),
                ..Default::default()
            },
            true,
            false,
        )
        .next()
        .await
        .expect("verified typed watch");
    assert_eq!(*calls.lock().unwrap(), expected);
    assert_eq!(page.trust, EventTrustLevel::Verified);
    assert_eq!(page.incomplete.len(), deployment_package_ids(&deployment, DeploymentPackageFamily::Publishing).len());
}

#[tokio::test]
async fn watch_publishing_and_comments_helpers_use_move_event_types() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let deployment = mainnet_deployment();
    let expected = deployment_package_ids(&deployment, DeploymentPackageFamily::Publishing)
        .into_iter()
        .map(|package_id| format!("{package_id}::publishing::ArtifactPublishedEvent"))
        .chain(
            deployment_package_ids(&deployment, DeploymentPackageFamily::Publishing)
                .into_iter()
                .map(|package_id| format!("{package_id}::publishing::ArtifactVersionAddedEvent")),
        )
        .chain(
            deployment_package_ids(&deployment, DeploymentPackageFamily::Comments)
                .into_iter()
                .map(|package_id| format!("{package_id}::comments::CommentAddedEvent")),
        )
        .chain(
            deployment_package_ids(&deployment, DeploymentPackageFamily::Comments)
                .into_iter()
                .map(|package_id| format!("{package_id}::comments::PaperLikedEvent")),
        )
        .collect::<Vec<_>>();
    let query = mock_query_client(calls.clone(), expected.len());
    let watch = PaperProofWatchClient::new(query);
    watch
        .watch_artifact_published_events(WatchOptions {
            limit: Some(1),
            ..Default::default()
        })
        .next()
        .await
        .expect("artifact published");
    watch
        .watch_artifact_version_added_events(WatchOptions {
            limit: Some(1),
            ..Default::default()
        })
        .next()
        .await
        .expect("version added");
    watch
        .watch_comment_added_events(WatchOptions {
            limit: Some(1),
            ..Default::default()
        })
        .next()
        .await
        .expect("comment");
    watch
        .watch_paper_liked_events(WatchOptions {
            limit: Some(1),
            ..Default::default()
        })
        .next()
        .await
        .expect("like");
    assert_eq!(*calls.lock().expect("calls"), expected);
}

#[tokio::test]
async fn watch_aggregate_helpers_query_multiple_event_types_and_dedupe() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let deployment = mainnet_deployment();
    let publishing = deployment_package_ids(&deployment, DeploymentPackageFamily::Publishing);
    let comments = deployment_package_ids(&deployment, DeploymentPackageFamily::Comments);
    let expected = publishing
        .iter()
        .map(|package_id| format!("{package_id}::publishing::ArtifactStatusChangedEvent"))
        .chain(
            publishing
                .iter()
                .map(|package_id| format!("{package_id}::publishing::ProtocolPausedChangedEvent")),
        )
        .chain(
            comments
                .iter()
                .map(|package_id| format!("{package_id}::comments::TreeStatusChangedEvent")),
        )
        .chain(
            comments
                .iter()
                .map(|package_id| format!("{package_id}::comments::CommentStatusChangedEvent")),
        )
        .chain(
            publishing
                .iter()
                .map(|package_id| format!("{package_id}::publishing::ArtifactOwnerTransferredEvent")),
        )
        .chain(
            comments
                .iter()
                .map(|package_id| format!("{package_id}::comments::TreeOwnerTransferredEvent")),
        )
        .collect::<Vec<_>>();
    let query = mock_query_client(calls.clone(), expected.len());
    let watch = PaperProofWatchClient::new(query);
    let status = watch
        .watch_status_changed_events(WatchOptions {
            limit: Some(2),
            ..Default::default()
        })
        .next()
        .await
        .expect("status");
    let owner = watch
        .watch_owner_transferred_events(WatchOptions {
            limit: Some(2),
            ..Default::default()
        })
        .next()
        .await
        .expect("owner");
    assert_eq!(*calls.lock().expect("calls"), expected);
    assert_eq!(status.data.len(), 1);
    assert_eq!(owner.data.len(), 1);
}

#[tokio::test]
async fn watch_governance_helper_queries_current_and_original_packages() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let deployment = mainnet_deployment();
    let expected = deployment_package_ids(&deployment, DeploymentPackageFamily::Governance)
        .into_iter()
        .map(|package_id| format!("{package_id}::governance_voting::VoteCastEvent"))
        .collect::<Vec<_>>();
    let query = mock_query_client(calls.clone(), expected.len());
    let watch = PaperProofWatchClient::new(query);
    let page = watch
        .watch_governance_vote_cast_events(WatchOptions {
            limit: Some(2),
            ..Default::default()
        })
        .next()
        .await
        .expect("vote");
    assert_eq!(*calls.lock().expect("calls"), expected);
    assert_eq!(page.data.len(), 1);
}
