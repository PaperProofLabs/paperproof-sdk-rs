// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    EventTrustLevel, EventVerificationStatus, JsonRpcClient, PaperProofError,
    PaperProofQueryClient, assert_no_incomplete,
    deployment::mainnet_deployment,
    query::{EventQueryInput, PaginationInput, TrustedEventQueryInput, build_event_filter},
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn builds_package_and_module_event_filter() {
    let filter = build_event_filter(&EventQueryInput {
        package_id: Some("0x1234".to_string()),
        module: Some("publishing".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        filter,
        json!({ "MoveModule": { "package": "0x1234", "module": "publishing" } })
    );
}

#[test]
fn rejects_incompatible_event_filters() {
    let error = build_event_filter(&EventQueryInput {
        sender: Some("0x1234".to_string()),
        event_type: Some("0x1::m::E".to_string()),
        pagination: PaginationInput::default(),
        ..Default::default()
    })
    .unwrap_err();
    assert!(error.to_string().contains("sender cannot be combined"));
}

#[test]
fn builds_move_event_type_filter() {
    let filter = build_event_filter(&EventQueryInput {
        move_event_type: Some("0x1::m::E".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(filter, json!({ "MoveEventType": "0x1::m::E" }));
}

#[tokio::test]
async fn query_verified_events_reports_binding_failures() {
    let deployment = mainnet_deployment();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock rpc");
    let addr = listener.local_addr().expect("local addr");
    let deployment_for_thread = deployment.clone();
    thread::spawn(move || {
        for _ in 0..5 {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0_u8; 8192];
            let size = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..size]);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("{}");
            let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            let method = value
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let result = if method == "suix_queryEvents" {
                json!({
                    "data": [{
                        "id": { "txDigest": "digest", "eventSeq": "1" },
                        "packageId": deployment_for_thread.packages.publishing,
                        "transactionModule": "publishing",
                        "sender": "0x1",
                        "type": format!("{}::publishing::ArtifactPublishedEvent", deployment_for_thread.packages.publishing),
                        "parsedJson": {
                            "root_id": deployment_for_thread.objects.root,
                            "series_id": "0x1111",
                            "version_id": "0x2222",
                            "comments_tree_id": "0x3333",
                            "likes_book_id": "0x4444"
                        }
                    }],
                    "nextCursor": null,
                    "hasNextPage": false
                })
            } else {
                let object_id = value
                    .pointer("/params/0")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                object_response(object_id, &deployment_for_thread)
            };
            let response = json!({ "jsonrpc": "2.0", "id": value.get("id").cloned().unwrap_or(json!(1)), "result": result });
            let text = response.to_string();
            let http = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                text.len(),
                text
            );
            stream.write_all(http.as_bytes()).expect("write response");
        }
    });
    let query = PaperProofQueryClient::new_jsonrpc(
        JsonRpcClient::new(format!("http://{addr}")),
        deployment,
    );
    let page = query
        .query_trusted_events(TrustedEventQueryInput {
            query: EventQueryInput::default(),
            trust: EventTrustLevel::Verified,
            include_rejected: true,
            verify_walrus: false,
        })
        .await
        .unwrap();
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.incomplete.len(), 1);
    assert_eq!(
        page.verification[0].status,
        EventVerificationStatus::Incomplete
    );
    assert!(
        page.verification[0]
            .issues
            .iter()
            .any(|issue| issue.code == "LIKES_SERIES_MISMATCH")
    );
    let error = assert_no_incomplete(&page).unwrap_err();
    assert!(matches!(error, PaperProofError::EventVerification { .. }));
}

fn object_response(object_id: &str, deployment: &paperproof_sdk_rs::Deployment) -> Value {
    let fields = match object_id {
        "0x1111" => json!({
            "current_version_id": "0x2222",
            "comments_tree_id": "0x3333",
            "likes_book_id": "0x4444",
            "version_ids": ["0x2222"]
        }),
        "0x2222" => json!({
            "header": {
                "fields": {
                    "series_id": "0x1111",
                    "content_hash": "sha256:abc",
                    "metadata_extensions": []
                }
            }
        }),
        "0x3333" => json!({
            "registry_id": deployment.objects.root,
            "target_series_id": "0x1111",
            "likes_book_id": "0x4444"
        }),
        "0x4444" => json!({
            "registry_id": deployment.objects.root,
            "comments_tree_id": "0x3333",
            "target_series_id": "0x9999"
        }),
        _ => json!({}),
    };
    json!({
        "data": {
            "objectId": object_id,
            "owner": { "AddressOwner": "0x1" },
            "content": {
                "dataType": "moveObject",
                "type": "0xpaper::mock::Object",
                "fields": fields
            }
        }
    })
}
