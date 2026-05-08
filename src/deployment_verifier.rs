// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    deployment::Deployment, error::Result, read::PaperProofReadClient, types::DecodedObject,
    validation::validate_package_id,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DeploymentCheckStatus {
    Pass,
    Fail,
    Warn,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentCheck {
    pub name: String,
    pub status: DeploymentCheckStatus,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentVerification {
    pub ok: bool,
    pub deployment: String,
    pub network: String,
    pub checks: Vec<DeploymentCheck>,
}

pub async fn verify_deployment(read: &PaperProofReadClient) -> Result<DeploymentVerification> {
    let deployment = &read.deployment;
    let mut checks = validate_static_deployment(deployment);

    let root = read.get_root().await;
    let type_registry = read.get_type_registry().await;
    let fee_manager = read.get_fee_manager().await;
    let governance_vault = read.get_governance_vault().await;
    let governance_config = read.get_governance_config().await;

    let Ok(root) = root else {
        checks.push(fail(
            "objects.root.readable",
            format!("Could not read root object: {}", root.unwrap_err()),
            Some(deployment.objects.root.clone()),
            None,
        ));
        return Ok(result(deployment, checks));
    };
    let Ok(type_registry) = type_registry else {
        checks.push(fail(
            "objects.typeRegistry.readable",
            format!(
                "Could not read TypeRegistry object: {}",
                type_registry.unwrap_err()
            ),
            Some(deployment.objects.type_registry.clone()),
            None,
        ));
        return Ok(result(deployment, checks));
    };
    let Ok(fee_manager) = fee_manager else {
        checks.push(fail(
            "objects.feeManager.readable",
            format!(
                "Could not read FeeManager object: {}",
                fee_manager.unwrap_err()
            ),
            Some(deployment.objects.fee_manager.clone()),
            None,
        ));
        return Ok(result(deployment, checks));
    };
    let Ok(governance_vault) = governance_vault else {
        checks.push(fail(
            "objects.governanceVault.readable",
            format!(
                "Could not read GovernanceVault object: {}",
                governance_vault.unwrap_err()
            ),
            Some(deployment.objects.governance_vault.clone()),
            None,
        ));
        return Ok(result(deployment, checks));
    };
    let Ok(governance_config) = governance_config else {
        checks.push(fail(
            "objects.governanceConfig.readable",
            format!(
                "Could not read GovernanceConfig object: {}",
                governance_config.unwrap_err()
            ),
            Some(deployment.objects.governance_config.clone()),
            None,
        ));
        return Ok(result(deployment, checks));
    };

    checks.push(check_package(
        "root.package",
        &root,
        &[&deployment.packages.publishing],
        "Root object is from the configured publishing package.",
    ));
    checks.push(check_package(
        "typeRegistry.package",
        &type_registry,
        &[&deployment.packages.publishing],
        "TypeRegistry object is from the configured publishing package.",
    ));
    checks.push(check_package(
        "feeManager.package",
        &fee_manager,
        &[
            &deployment.packages.governance,
            &deployment.packages.governance_original,
        ],
        "FeeManager object is from a configured governance package.",
    ));
    checks.push(check_package(
        "governanceVault.package",
        &governance_vault,
        &[
            &deployment.packages.governance,
            &deployment.packages.governance_original,
        ],
        "GovernanceVault object is from a configured governance package.",
    ));
    checks.push(check_package(
        "governanceConfig.package",
        &governance_config,
        &[
            &deployment.packages.governance,
            &deployment.packages.governance_original,
        ],
        "GovernanceConfig object is from a configured governance package.",
    ));

    let root_view = crate::views::view_root(&root);
    let fee_view = crate::views::view_fee_manager(&fee_manager);
    let vault_view = crate::views::view_governance_vault(&governance_vault);
    let config_view = crate::views::view_governance_config(&governance_config);

    checks.push(check_id(
        "root.typeRegistryId",
        root_view.type_registry_id.as_deref(),
        Some(&deployment.objects.type_registry),
        "Root points to the configured TypeRegistry.",
    ));
    checks.push(check_id(
        "root.feeManagerId",
        root_view.fee_manager_id.as_deref(),
        Some(&deployment.objects.fee_manager),
        "Root points to the configured FeeManager.",
    ));
    checks.push(check_id(
        "root.governanceVaultId",
        root_view.governance_vault_id.as_deref(),
        Some(&deployment.objects.governance_vault),
        "Root points to the configured GovernanceVault.",
    ));
    checks.push(check_id(
        "feeManager.registryId",
        fee_view.registry_id.as_deref(),
        Some(&deployment.objects.root),
        "FeeManager is bound to the configured root registry.",
    ));
    checks.push(check_id(
        "governanceVault.registryId",
        vault_view.registry_id.as_deref(),
        Some(&deployment.objects.root),
        "GovernanceVault is bound to the configured root registry.",
    ));
    checks.push(check_id(
        "governanceConfig.registryId",
        config_view.registry_id.as_deref(),
        Some(&deployment.objects.root),
        "GovernanceConfig is bound to the configured root registry.",
    ));
    checks.push(check_id(
        "governanceVault.configId",
        vault_view.governance_config_id.as_deref(),
        Some(&deployment.objects.governance_config),
        "GovernanceVault points to the configured GovernanceConfig.",
    ));

    if root_view.paused == Some(true) {
        checks.push(warn(
            "root.paused",
            "PaperProof publishing is currently paused. Publish/add-version calls are expected to fail.",
            Some("true".to_string()),
        ));
    }
    if config_view.proposal_creation_paused == Some(true) {
        checks.push(warn(
            "governance.proposalCreationPaused",
            "Governance proposal creation is currently paused.",
            Some("true".to_string()),
        ));
    }

    Ok(result(deployment, checks))
}

pub fn validate_static_deployment(deployment: &Deployment) -> Vec<DeploymentCheck> {
    let mut checks = Vec::new();
    for (name, id) in [
        ("packages.pprf", &deployment.packages.pprf),
        ("packages.publishing", &deployment.packages.publishing),
        ("packages.comments", &deployment.packages.comments),
        ("packages.governance", &deployment.packages.governance),
        ("objects.root", &deployment.objects.root),
        ("objects.typeRegistry", &deployment.objects.type_registry),
        ("objects.feeManager", &deployment.objects.fee_manager),
        (
            "objects.governanceVault",
            &deployment.objects.governance_vault,
        ),
        (
            "objects.governanceConfig",
            &deployment.objects.governance_config,
        ),
        ("objects.clock", &deployment.objects.clock),
    ] {
        checks.push(if validate_package_id(id).is_ok() {
            pass(name, "Configured id is a valid Sui object id.")
        } else {
            fail(
                name,
                "Configured id is not a valid Sui object id.",
                None,
                Some(id.clone()),
            )
        });
    }
    let expected_prefix = format!("{}::", deployment.packages.pprf);
    if deployment.coin_types.pprf.starts_with(&expected_prefix) {
        checks.push(pass(
            "coinTypes.pprf",
            "Configured PPRF coin type matches the configured PPRF package.",
        ));
    } else {
        checks.push(fail(
            "coinTypes.pprf",
            "Configured PPRF coin type is not from the configured PPRF package.",
            Some(format!("{expected_prefix}*")),
            Some(deployment.coin_types.pprf.clone()),
        ));
    }
    checks
}

pub fn format_deployment_verification(verification: &DeploymentVerification) -> String {
    let mut lines = vec![format!(
        "PaperProof deployment verification for {} on {}: {}",
        verification.deployment,
        verification.network,
        if verification.ok { "ok" } else { "failed" }
    )];
    for check in &verification.checks {
        lines.push(format!(
            "[{:?}] {}: {}",
            check.status, check.name, check.message
        ));
    }
    lines.join("\n")
}

fn check_package(
    name: &str,
    object: &DecodedObject,
    expected_packages: &[&String],
    message: &str,
) -> DeploymentCheck {
    let actual = object.object_type.split("::").next().unwrap_or_default();
    if expected_packages
        .iter()
        .any(|expected| same_id(actual, expected))
    {
        pass(name, message)
    } else {
        fail(
            name,
            format!(
                "{message} Object type {} is not expected.",
                object.object_type
            ),
            Some(
                expected_packages
                    .iter()
                    .map(|item| item.as_str())
                    .collect::<Vec<_>>()
                    .join(" or "),
            ),
            Some(object.object_type.clone()),
        )
    }
}

fn check_id(
    name: &str,
    actual: Option<&str>,
    expected: Option<&str>,
    message: &str,
) -> DeploymentCheck {
    if let (Some(actual), Some(expected)) = (actual, expected)
        && same_id(actual, expected)
    {
        return pass(name, message);
    }
    fail(
        name,
        format!(
            "{message} Expected {}, got {}.",
            expected.unwrap_or("unknown"),
            actual.unwrap_or("missing")
        ),
        expected.map(ToString::to_string),
        actual.map(ToString::to_string),
    )
}

fn pass(name: &str, message: impl Into<String>) -> DeploymentCheck {
    DeploymentCheck {
        name: name.to_string(),
        status: DeploymentCheckStatus::Pass,
        message: message.into(),
        expected: None,
        actual: None,
    }
}

fn warn(name: &str, message: impl Into<String>, actual: Option<String>) -> DeploymentCheck {
    DeploymentCheck {
        name: name.to_string(),
        status: DeploymentCheckStatus::Warn,
        message: message.into(),
        expected: None,
        actual,
    }
}

fn fail(
    name: &str,
    message: impl Into<String>,
    expected: Option<String>,
    actual: Option<String>,
) -> DeploymentCheck {
    DeploymentCheck {
        name: name.to_string(),
        status: DeploymentCheckStatus::Fail,
        message: message.into(),
        expected,
        actual,
    }
}

fn result(deployment: &Deployment, checks: Vec<DeploymentCheck>) -> DeploymentVerification {
    DeploymentVerification {
        ok: checks
            .iter()
            .all(|check| check.status != DeploymentCheckStatus::Fail),
        deployment: deployment.name.clone(),
        network: deployment.network.clone(),
        checks,
    }
}

fn same_id(left: &str, right: &str) -> bool {
    normalize_id(left) == normalize_id(right)
}

fn normalize_id(value: &str) -> String {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", raw.trim_start_matches('0').to_ascii_lowercase())
}
