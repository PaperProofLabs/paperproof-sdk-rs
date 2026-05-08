// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    ExecutionMode, PaperProofProviderService, PaperProofService, SuiCliExecutor,
};

#[test]
fn service_exposes_builders_and_preview_defaults() {
    let service = PaperProofService::mainnet();
    assert_eq!(service.default_options.mode, ExecutionMode::Preview);
    let plan = service
        .client
        .publishing
        .publish_preprint(&common::sample_preprint())
        .unwrap();
    assert_eq!(plan.calls.len(), 1);
    assert!(plan.calls[0].target.ends_with("publish_preprint"));
}

#[test]
fn provider_service_can_use_cli_fallback_provider() {
    let deployment = paperproof_sdk_rs::deployment::mainnet_deployment();
    let service =
        PaperProofProviderService::new(deployment.clone(), SuiCliExecutor::new(deployment));
    let plan = service
        .client
        .publishing
        .publish_preprint(&common::sample_preprint())
        .unwrap();
    assert_eq!(plan.calls.len(), 1);
    assert!(plan.calls[0].target.ends_with("publish_preprint"));
}
