// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{coin_utils, walrus};

#[test]
fn converts_pprf_amounts() {
    assert_eq!(coin_utils::pprf_to_base_units("1").unwrap(), 1_000_000_000);
    assert_eq!(
        coin_utils::pprf_to_base_units("1.25").unwrap(),
        1_250_000_000
    );
    assert_eq!(coin_utils::base_units_to_pprf(1_250_000_000), "1.25");
    assert!(coin_utils::pprf_to_base_units("1.1234567891").is_err());
}

#[test]
fn computes_walrus_sha256() {
    assert_eq!(
        walrus::sha256_hex(b"paperproof"),
        "4df34b1c2fb3e472842dab3779be631aab4978fb6f6af7e533dff8d2bf6aae06"
    );
}

#[test]
fn detects_shared_walrus_blob_objects() {
    assert_eq!(
        walrus::owner_address(&serde_json::json!({ "owner": { "AddressOwner": "0xabc" }})),
        Some("0xabc")
    );
    assert!(!walrus::is_shared_blob_object(&serde_json::json!({
        "owner": { "AddressOwner": "0xabc" }
    })));
    assert!(walrus::is_shared_blob_object(&serde_json::json!({
        "owner": { "Shared": { "initial_shared_version": "1" } }
    })));
    assert!(walrus::is_shared_blob_object(&serde_json::json!({
        "shared": true
    })));
}

#[test]
fn walrus_write_options_default_to_share_as_non_deletable() {
    let default_options = walrus::WalrusWriteOptions::default();
    assert_eq!(default_options.epochs, 5);
    assert!(!default_options.share);
    assert_eq!(default_options.deletable, None);

    let shared_options = walrus::WalrusWriteOptions {
        epochs: 2,
        share: true,
        deletable: None,
    };
    assert!(shared_options.share);
}

#[test]
fn walrus_cli_client_defaults_to_mainnet_reader() {
    let client = walrus::WalrusCliClient::new("walrus-test");
    assert_eq!(client.cli_path, "walrus-test");
    assert!(client.aggregator_url.contains("mainnet"));
}

#[test]
fn walrus_owned_blob_transfer_preflight_blocks_shared_or_non_owner() {
    let owned = serde_json::json!({ "owner": { "AddressOwner": "0xabc" } });
    assert!(walrus::assert_transferable_owned_blob("0x1", Some("0xabc"), &owned).is_ok());
    assert!(walrus::assert_transferable_owned_blob("0x1", Some("0x000abc"), &owned).is_ok());
    assert!(walrus::assert_transferable_owned_blob("0x1", Some("0xdef"), &owned).is_err());

    let shared = serde_json::json!({ "owner": { "Shared": { "initial_shared_version": "1" } } });
    assert!(walrus::assert_transferable_owned_blob("0x1", Some("0xabc"), &shared).is_err());

    let options = walrus::WalrusTransferOptions {
        signer_address: Some("0xabc".to_string()),
        skip_owner_check: false,
    };
    assert_eq!(options.signer_address.as_deref(), Some("0xabc"));
    assert!(!options.skip_owner_check);
}

#[test]
fn parses_walrus_write_response_shapes() {
    let parsed = walrus::parse_walrus_write_response(&serde_json::json!({
        "newlyCreated": {
            "blobObject": {
                "blobId": "blob-1",
                "id": "0x1"
            }
        }
    }))
    .unwrap();
    assert_eq!(parsed.0, "blob-1");
    assert_eq!(parsed.1.as_deref(), Some("0x1"));

    let parsed = walrus::parse_walrus_write_response(&serde_json::json!({
        "blobObject": {
            "blob_id": "blob-2",
            "objectId": "0x2"
        }
    }))
    .unwrap();
    assert_eq!(parsed.0, "blob-2");
    assert_eq!(parsed.1.as_deref(), Some("0x2"));

    assert!(walrus::parse_walrus_write_response(&serde_json::json!({})).is_err());
}

#[test]
fn content_publish_options_default_to_owned_blob() {
    let options = walrus::ContentPublishOptions::default();
    assert_eq!(options.epochs, 5);
    assert!(!options.share);
    assert_eq!(options.deletable, None);
    assert_eq!(options.content_type, None);
}
