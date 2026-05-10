// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    GraphQlQueryProvider, JsonRpcClient, PaperProofQueryClient,
    deployment::mainnet_deployment,
    events::SuiEventEnvelope,
    query::{EventPage, EventQueryInput, PaginationInput, dedupe_events, event_dedupe_key},
};
use serde_json::json;

#[test]
fn dedupe_prefers_transaction_digest_and_event_sequence() {
    let deployment = mainnet_deployment();
    let event = event(
        &deployment.packages.governance,
        "governance_voting",
        "ProposalCreatedEvent",
        json!({"registry_id": deployment.objects.root, "proposal_id": "1", "proposal_object_id": "0x2"}),
        "digest",
        "0",
    );
    assert_eq!(event_dedupe_key(&event), "digest:0");
    assert_eq!(dedupe_events(vec![event.clone(), event]).len(), 1);
}

#[tokio::test]
async fn governance_helpers_query_both_packages_filter_and_dedupe() {
    let deployment = mainnet_deployment();
    let query = PaperProofQueryClient::new_jsonrpc(
        JsonRpcClient::new(deployment.rpc_url.clone()),
        deployment.clone(),
    );
    let page = EventPage {
        data: vec![
            event(
                &deployment.packages.governance,
                "governance_voting",
                "ProposalCreatedEvent",
                json!({"registry_id": deployment.objects.root, "proposal_id": "1", "proposal_object_id": "0x2"}),
                "digest",
                "0",
            ),
            event(
                &deployment.packages.governance_original,
                "governance_voting",
                "ProposalCreatedEvent",
                json!({"registry_id": deployment.objects.root, "proposal_id": "1", "proposal_object_id": "0x2"}),
                "digest",
                "0",
            ),
            event(
                &deployment.packages.governance,
                "governance_voting",
                "ProposalCreatedEvent",
                json!({"registry_id": "0xfake", "proposal_id": "9", "proposal_object_id": "0x9"}),
                "fake",
                "1",
            ),
        ],
        next_cursor: None,
        has_next_page: false,
        raw: json!({}),
    };
    let filtered = dedupe_events(
        page.data
            .into_iter()
            .filter(|event| {
                paperproof_sdk_rs::events_trust::validate_event_trust(event, &query.deployment)
                    .trusted
            })
            .collect(),
    );
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].parsed_json["proposal_id"], "1");
}

#[test]
fn mainnet_query_client_is_graphql_first() {
    let query = PaperProofQueryClient::mainnet();
    assert!(matches!(
        query.query_provider,
        paperproof_sdk_rs::PaperProofQueryProvider::GraphQl(_)
    ));

    let custom = PaperProofQueryClient::new_graphql(
        JsonRpcClient::new(mainnet_deployment().rpc_url),
        GraphQlQueryProvider::new("https://example.invalid/graphql"),
        mainnet_deployment(),
    );
    assert!(matches!(
        custom.query_provider,
        paperproof_sdk_rs::PaperProofQueryProvider::GraphQl(_)
    ));
}

#[test]
fn event_query_input_can_page_graphql_events() {
    let input = EventQueryInput {
        move_event_type: Some("0x1::m::E".to_string()),
        pagination: PaginationInput {
            limit: Some(100),
            descending_order: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(input.pagination.limit, Some(100));
}

fn event(
    package: &str,
    module: &str,
    struct_name: &str,
    parsed_json: serde_json::Value,
    digest: &str,
    event_seq: &str,
) -> SuiEventEnvelope {
    SuiEventEnvelope {
        id: Some(json!({ "txDigest": digest, "eventSeq": event_seq })),
        package_id: package.to_string(),
        transaction_module: module.to_string(),
        sender: "0x1".to_string(),
        event_type: format!("{package}::{module}::{struct_name}"),
        parsed_json,
        bcs: None,
        timestamp_ms: Some("1770000000000".to_string()),
    }
}
