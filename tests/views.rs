// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    types::DecodedObject,
    views::{
        bytes_value, id_value, parse_metadata_attributes, view_comment_node, view_series,
        view_version,
    },
};
use serde_json::json;

#[test]
fn parses_series_view_with_metadata_and_versions() {
    let object = DecodedObject {
        id: "0xseries".to_string(),
        object_type: "ArtifactSeries".to_string(),
        owner: None,
        fields: json!({
            "artifact_type": "1",
            "artifact_code": "preprint",
            "owner": "0xowner",
            "current_version": "2",
            "current_version_id": { "fields": { "id": "0xv2" } },
            "comments_tree_id": "0xtree",
            "likes_book_id": "0xlikes",
            "status": 1,
            "ui_status": 1,
            "metadata_extensions": {
                "fields": {
                    "contents": [
                        { "fields": { "key": "source", "value": "test" } }
                    ]
                }
            },
            "version_ids": { "vec": ["0xv1", { "fields": { "id": "0xv2" } }] }
        }),
    };
    let view = view_series(&object);
    assert_eq!(view.artifact_type, Some(1));
    assert_eq!(view.current_version, Some(2));
    assert_eq!(view.metadata_extensions[0].key, "source");
    assert_eq!(view.version_ids, vec!["0xv1", "0xv2"]);
}

#[test]
fn parses_version_header_view() {
    let object = DecodedObject {
        id: "0xversion".to_string(),
        object_type: "ArtifactVersion".to_string(),
        owner: None,
        fields: json!({
            "header": {
                "fields": {
                    "series_id": "0xseries",
                    "artifact_type": 1,
                    "version": "3",
                    "content_hash": "sha256:abc",
                    "metadata_extensions": [
                        { "fields": { "key": "k", "value": "v" } }
                    ]
                }
            }
        }),
    };
    let view = view_version(&object);
    assert_eq!(view.series_id.as_deref(), Some("0xseries"));
    assert_eq!(view.version, Some(3));
    assert_eq!(view.metadata_extensions.len(), 1);
}

#[test]
fn parses_comment_node_option_fields() {
    let node = json!({
        "fields": {
            "comment_id": "8",
            "parent_comment_id": { "vec": ["1"] },
            "author": "0xauthor",
            "depth": 2,
            "content_mode": 1,
            "inline_content": [104, 105],
            "content_preview": { "fields": { "contents": [112] } },
            "blob_id": [],
            "blob_object_id": { "vec": ["0xblob"] },
            "blob_digest": [1, 2],
            "children_count": "0",
            "created_at_ms": "100",
            "edited_at_ms": { "vec": [] },
            "status": 1
        }
    });
    let view = view_comment_node(&node);
    assert_eq!(view.comment_id, Some(8));
    assert_eq!(view.parent_comment_id, Some(1));
    assert_eq!(view.inline_content, b"hi");
    assert_eq!(view.blob_object_id.as_deref(), Some("0xblob"));
    assert_eq!(view.edited_at_ms, None);
}

#[test]
fn metadata_parser_accepts_plain_vectors() {
    let metadata = parse_metadata_attributes(&json!([
        { "key": "a", "value": "b" },
        { "fields": { "key": "c", "value": "d" } }
    ]));
    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[1].key, "c");
}

#[test]
fn id_value_accepts_numeric_key_byte_objects() {
    let value = json!({
        "2": 204,
        "0": 170,
        "1": 187
    });
    assert_eq!(id_value(&value).as_deref(), Some("0xaabbcc"));
}

#[test]
fn bytes_value_accepts_numeric_key_byte_objects() {
    let value = json!({
        "1": 2,
        "0": 1,
        "2": 3
    });
    assert_eq!(bytes_value(&value), vec![1, 2, 3]);
}
