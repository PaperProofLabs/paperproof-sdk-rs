// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    deployment::mainnet_deployment,
    deployment_verifier::{
        DeploymentCheckStatus, format_deployment_verification, validate_static_deployment,
    },
};

#[test]
fn static_deployment_verification_accepts_mainnet_config() {
    let deployment = mainnet_deployment();
    let checks = validate_static_deployment(&deployment);
    assert!(
        checks
            .iter()
            .all(|check| check.status != DeploymentCheckStatus::Fail)
    );
}

#[test]
fn static_deployment_verification_rejects_wrong_pprf_coin_type() {
    let mut deployment = mainnet_deployment();
    deployment.coin_types.pprf = "0x2::sui::SUI".to_string();
    let checks = validate_static_deployment(&deployment);
    assert!(
        checks
            .iter()
            .any(|check| check.name == "coinTypes.pprf"
                && check.status == DeploymentCheckStatus::Fail)
    );
}

#[test]
fn formats_deployment_verification() {
    let deployment = mainnet_deployment();
    let verification = paperproof_sdk_rs::DeploymentVerification {
        ok: true,
        deployment: deployment.name,
        network: deployment.network,
        checks: vec![],
    };
    assert!(format_deployment_verification(&verification).contains("PaperProof deployment"));
}
