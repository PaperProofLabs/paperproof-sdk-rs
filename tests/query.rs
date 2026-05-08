// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::query::{EventQueryInput, PaginationInput, build_event_filter};
use serde_json::json;

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
