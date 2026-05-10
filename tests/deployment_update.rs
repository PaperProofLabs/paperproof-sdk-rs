// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    deployment::mainnet_deployment,
    deployment_update::{
        DeploymentDriftPolicy, DeploymentManifest, DeploymentManifestStatus, DeploymentUpdateCheck,
        check_deployment_update_with_manifest, diff_deployment, enforce_deployment_update_policy,
        format_deployment_update_check, default_deployment_manifest_url, manifest_from_value,
    },
};
use serde_json::json;

#[test]
fn detects_deployment_drift() {
    let current = mainnet_deployment();
    let mut latest = current.clone();
    latest.packages.comments = "0x9999".to_string();
    let diff = diff_deployment(&current, &latest);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].path, "packages.comments");
}

#[test]
fn default_manifest_url_is_network_specific_and_contracts_hosted() {
    assert_eq!(
        default_deployment_manifest_url("mainnet"),
        "https://raw.githubusercontent.com/PaperProofLabs/paperproof-contracts/main/docs/deployments/mainnet.json"
    );
    assert_eq!(
        default_deployment_manifest_url("testnet"),
        "https://raw.githubusercontent.com/PaperProofLabs/paperproof-contracts/main/docs/deployments/testnet.json"
    );
}

#[test]
fn parses_contracts_repository_manifest_shape() {
    let current = mainnet_deployment();
    let value = json!({
        "schemaVersion": 1,
        "deployment": {
            "name": current.name,
            "network": current.network,
            "rpcUrl": current.rpc_url,
            "protocolVersion": current.protocol_version,
            "packages": {
                "pprf": current.packages.pprf,
                "governanceOriginal": current.packages.governance_original,
                "governance": current.packages.governance,
                "comments": current.packages.comments,
                "publishing": current.packages.publishing
            },
            "objects": {
                "root": current.objects.root,
                "typeRegistry": current.objects.type_registry,
                "feeManager": current.objects.fee_manager,
                "governanceVault": current.objects.governance_vault,
                "governanceConfig": current.objects.governance_config,
                "clock": current.objects.clock
            },
            "coinTypes": {
                "pprf": current.coin_types.pprf,
                "wal": current.coin_types.wal,
                "sui": current.coin_types.sui
            }
        },
        "packageHistory": {
            "governance": [current.packages.governance_original, current.packages.governance]
        },
        "updatedAt": "2026-05-08T00:00:00+08:00",
        "minSdkVersion": "0.1.0"
    });
    let manifest = manifest_from_value(value).expect("manifest should parse");
    assert_eq!(manifest.deployment, mainnet_deployment());
    assert_eq!(manifest.updated_at.as_deref(), Some("2026-05-08T00:00:00+08:00"));
    assert_eq!(manifest.min_sdk_version.as_deref(), Some("0.1.0"));
}

#[test]
fn reports_update_available_from_manifest() {
    let current = mainnet_deployment();
    let mut latest = current.clone();
    latest.objects.root = "0x8888".to_string();
    let result = check_deployment_update_with_manifest(
        current,
        DeploymentManifest {
            deployment: latest,
            min_sdk_version: Some("0.2.0".to_string()),
            updated_at: Some("2026-05-09".to_string()),
            release_notes_url: None,
            message: Some("test manifest".to_string()),
        },
        Some("https://example.com/mainnet.json".to_string()),
    );
    assert_eq!(result.status, DeploymentManifestStatus::UpdateAvailable);
    assert!(format_deployment_update_check(&result).contains("objects.root"));
}

#[test]
fn deployment_drift_policy_can_hard_fail() {
    let current = mainnet_deployment();
    let mut latest = current.clone();
    latest.objects.root = "0x9999".to_string();
    let check = check_deployment_update_with_manifest(
        current,
        DeploymentManifest {
            deployment: latest,
            min_sdk_version: None,
            updated_at: None,
            release_notes_url: None,
            message: None,
        },
        None,
    );
    assert!(enforce_deployment_update_policy(&check, DeploymentDriftPolicy::Warn).is_ok());
    assert!(
        enforce_deployment_update_policy(&check, DeploymentDriftPolicy::HardFailOnUpdate).is_err()
    );
}

#[test]
fn deployment_drift_policy_can_hard_fail_on_unchecked_manifest() {
    let check = DeploymentUpdateCheck {
        status: DeploymentManifestStatus::Unchecked,
        current: mainnet_deployment(),
        latest: None,
        manifest_url: Some("https://example.com/missing.json".to_string()),
        min_sdk_version: None,
        updated_at: None,
        release_notes_url: None,
        message: "could not check manifest".to_string(),
        differences: vec![],
        error: Some("network unavailable".to_string()),
    };

    assert!(enforce_deployment_update_policy(&check, DeploymentDriftPolicy::Warn).is_ok());
    assert!(
        enforce_deployment_update_policy(&check, DeploymentDriftPolicy::HardFailOnUpdate).is_ok()
    );
    assert!(
        enforce_deployment_update_policy(&check, DeploymentDriftPolicy::HardFailOnUnchecked)
            .is_err()
    );
    assert!(
        enforce_deployment_update_policy(&check, DeploymentDriftPolicy::HardFailOnAnyProblem)
            .is_err()
    );
}
