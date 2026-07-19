// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    constants::PROTOCOL_LIMITS,
    types::MetadataAttribute,
    validation::{validate_metadata_attributes, validate_object_id, validate_preprint_input},
};

#[test]
fn rejects_empty_preprint_title() {
    let mut input = common::sample_preprint();
    input.title.clear();
    let err = validate_preprint_input(&input).unwrap_err().to_string();
    assert!(err.contains("title"));
}

#[test]
fn rejects_duplicate_metadata_keys() {
    let metadata = vec![
        MetadataAttribute {
            key: "x".to_string(),
            value: "1".to_string(),
        },
        MetadataAttribute {
            key: "x".to_string(),
            value: "2".to_string(),
        },
    ];
    let err = validate_metadata_attributes(&metadata)
        .unwrap_err()
        .to_string();
    assert!(err.contains("duplicate"));
}

#[test]
fn rejects_too_many_metadata_attributes() {
    let metadata = (0..=PROTOCOL_LIMITS.max_metadata_attributes)
        .map(|index| MetadataAttribute {
            key: format!("k{index}"),
            value: "v".to_string(),
        })
        .collect::<Vec<_>>();
    assert!(validate_metadata_attributes(&metadata).is_err());
}

#[test]
fn accepts_short_object_id_and_rejects_non_hex() {
    assert!(validate_object_id("0x6").is_ok());
    assert!(validate_object_id("0xnothex").is_err());
}
