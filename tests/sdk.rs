// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    CreatePaperProofSdkOptions, PaperProofSdk, PaperProofSdkQuery, PaperProofSdkRead,
    PaperProofTransport, create_paperproof_sdk,
};

#[test]
#[cfg(not(feature = "sui-native"))]
fn default_sdk_transport_requires_sui_native_feature() {
    let error = PaperProofSdk::mainnet().unwrap_err();
    assert!(error.to_string().contains("sui-native"));
}

#[tokio::test]
#[cfg(feature = "sui-native")]
async fn default_sdk_transport_is_grpc() {
    let sdk = PaperProofSdk::mainnet().unwrap();
    assert_eq!(sdk.transport, PaperProofTransport::Grpc);
    assert!(matches!(sdk.read, PaperProofSdkRead::Grpc(_)));
    assert!(matches!(sdk.query, Some(PaperProofSdkQuery::GraphQl(_))));
}

#[test]
fn jsonrpc_sdk_transport_is_explicit_fallback() {
    let sdk = create_paperproof_sdk(CreatePaperProofSdkOptions {
        transport: Some(PaperProofTransport::JsonRpc),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(sdk.transport, PaperProofTransport::JsonRpc);
    assert!(matches!(sdk.read, PaperProofSdkRead::JsonRpc(_)));
    assert!(matches!(sdk.query, Some(PaperProofSdkQuery::JsonRpc(_))));
}
