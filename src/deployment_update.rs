// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    deployment::{Deployment, mainnet_deployment},
    error::{PaperProofError, Result},
};

pub const DEFAULT_DEPLOYMENT_MANIFEST_BASE_URL: &str =
    "https://raw.githubusercontent.com/PaperProofLabs/paperproof-contracts/main/docs/deployments";
pub const DEFAULT_MAINNET_DEPLOYMENT_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/PaperProofLabs/paperproof-contracts/main/docs/deployments/mainnet.json";

pub fn default_deployment_manifest_url(network: impl AsRef<str>) -> String {
    format!(
        "{}/{}.json",
        DEFAULT_DEPLOYMENT_MANIFEST_BASE_URL,
        network.as_ref()
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DeploymentManifestStatus {
    Current,
    UpdateAvailable,
    Unchecked,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentManifest {
    pub deployment: Deployment,
    pub min_sdk_version: Option<String>,
    pub updated_at: Option<String>,
    pub release_notes_url: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentUpdateDifference {
    pub path: String,
    pub current: Option<String>,
    pub latest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DeploymentUpdateCheck {
    pub status: DeploymentManifestStatus,
    pub current: Deployment,
    pub latest: Option<Deployment>,
    pub manifest_url: Option<String>,
    pub min_sdk_version: Option<String>,
    pub updated_at: Option<String>,
    pub release_notes_url: Option<String>,
    pub message: String,
    pub differences: Vec<DeploymentUpdateDifference>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DeploymentDriftPolicy {
    Warn,
    HardFailOnUpdate,
    HardFailOnUnchecked,
    HardFailOnAnyProblem,
}

const DEPLOYMENT_PATHS: &[&str] = &[
    "name",
    "network",
    "protocol_version",
    "packages.pprf",
    "packages.governance_original",
    "packages.governance",
    "packages.comments",
    "packages.publishing",
    "objects.root",
    "objects.type_registry",
    "objects.fee_manager",
    "objects.governance_vault",
    "objects.governance_config",
    "objects.clock",
    "coin_types.pprf",
    "coin_types.wal",
    "coin_types.sui",
];

pub async fn check_deployment_update_from_url(
    current: Option<Deployment>,
    manifest_url: Option<&str>,
) -> DeploymentUpdateCheck {
    let current = current.unwrap_or_else(mainnet_deployment);
    let url = manifest_url.map(ToString::to_string).or_else(|| {
        Some(default_deployment_manifest_url(&current.network))
    });
    let Some(url) = url else {
        return unchecked(
            current,
            None,
            "No deployment manifest URL is configured for this network.",
            None,
        );
    };
    match fetch_manifest(&url).await {
        Ok(manifest) => check_deployment_update_with_manifest(current, manifest, Some(url)),
        Err(error) => unchecked(
            current,
            Some(url),
            format!("Could not check PaperProof deployment updates: {error}"),
            Some(error.to_string()),
        ),
    }
}

pub fn check_deployment_update_with_manifest(
    current: Deployment,
    manifest: DeploymentManifest,
    manifest_url: Option<String>,
) -> DeploymentUpdateCheck {
    let differences = diff_deployment(&current, &manifest.deployment);
    let status = if differences.is_empty() {
        DeploymentManifestStatus::Current
    } else {
        DeploymentManifestStatus::UpdateAvailable
    };
    let message = if differences.is_empty() {
        manifest.message.clone().unwrap_or_else(|| {
            "PaperProof deployment configuration matches the latest manifest.".to_string()
        })
    } else {
        let mut text = format!(
            "PaperProof deployment configuration has {} difference(s). Update the SDK or pass the latest Deployment override.",
            differences.len()
        );
        if let Some(extra) = &manifest.message {
            text.push(' ');
            text.push_str(extra);
        }
        text
    };
    DeploymentUpdateCheck {
        status,
        current,
        latest: Some(manifest.deployment),
        manifest_url,
        min_sdk_version: manifest.min_sdk_version,
        updated_at: manifest.updated_at,
        release_notes_url: manifest.release_notes_url,
        message,
        differences,
        error: None,
    }
}

pub fn diff_deployment(
    current: &Deployment,
    latest: &Deployment,
) -> Vec<DeploymentUpdateDifference> {
    let current_value = serde_json::to_value(current).unwrap_or(Value::Null);
    let latest_value = serde_json::to_value(latest).unwrap_or(Value::Null);
    DEPLOYMENT_PATHS
        .iter()
        .filter_map(|path| {
            let current = value_at(&current_value, path);
            let latest = value_at(&latest_value, path);
            (!same_value(current.as_deref(), latest.as_deref())).then(|| {
                DeploymentUpdateDifference {
                    path: (*path).to_string(),
                    current,
                    latest,
                }
            })
        })
        .collect()
}

pub fn format_deployment_update_check(result: &DeploymentUpdateCheck) -> String {
    let mut lines = vec![format!(
        "PaperProof deployment update check: {:?}",
        result.status
    )];
    lines.push(result.message.clone());
    if let Some(url) = &result.manifest_url {
        lines.push(format!("manifest: {url}"));
    }
    if let Some(updated_at) = &result.updated_at {
        lines.push(format!("updatedAt: {updated_at}"));
    }
    if let Some(min_sdk_version) = &result.min_sdk_version {
        lines.push(format!("minSdkVersion: {min_sdk_version}"));
    }
    if let Some(release_notes_url) = &result.release_notes_url {
        lines.push(format!("releaseNotes: {release_notes_url}"));
    }
    for difference in &result.differences {
        lines.push(format!(
            "[diff] {}: current={} latest={}",
            difference.path,
            difference.current.as_deref().unwrap_or("missing"),
            difference.latest.as_deref().unwrap_or("missing")
        ));
    }
    lines.join("\n")
}

pub fn enforce_deployment_update_policy(
    result: &DeploymentUpdateCheck,
    policy: DeploymentDriftPolicy,
) -> Result<()> {
    let should_fail = match policy {
        DeploymentDriftPolicy::Warn => false,
        DeploymentDriftPolicy::HardFailOnUpdate => {
            result.status == DeploymentManifestStatus::UpdateAvailable
        }
        DeploymentDriftPolicy::HardFailOnUnchecked => {
            result.status == DeploymentManifestStatus::Unchecked
        }
        DeploymentDriftPolicy::HardFailOnAnyProblem => {
            result.status != DeploymentManifestStatus::Current
        }
    };
    if should_fail {
        Err(PaperProofError::invalid_input(
            "deployment",
            format_deployment_update_check(result),
        ))
    } else {
        Ok(())
    }
}

async fn fetch_manifest(url: &str) -> Result<DeploymentManifest> {
    let response = reqwest::Client::new()
        .get(url)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|error| PaperProofError::network(url, error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(PaperProofError::network(url, format!("HTTP {status}")));
    }
    let value = response
        .json::<Value>()
        .await
        .map_err(|error| PaperProofError::network(url, error.to_string()))?;
    manifest_from_value(value)
}

pub fn manifest_from_value(value: Value) -> Result<DeploymentManifest> {
    let value = normalize_manifest_value(value);
    if value.get("deployment").is_some() {
        serde_json::from_value(value).map_err(Into::into)
    } else {
        Ok(DeploymentManifest {
            deployment: serde_json::from_value(value)?,
            min_sdk_version: None,
            updated_at: None,
            release_notes_url: None,
            message: None,
        })
    }
}

fn normalize_manifest_value(mut value: Value) -> Value {
    if let Some(deployment) = value.get_mut("deployment") {
        normalize_deployment_value(deployment);
    } else {
        normalize_deployment_value(&mut value);
    }
    normalize_top_level_manifest_keys(&mut value);
    value
}

fn normalize_top_level_manifest_keys(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    rename_key(object, "minSdkVersion", "min_sdk_version");
    rename_key(object, "updatedAt", "updated_at");
    rename_key(object, "releaseNotesUrl", "release_notes_url");
}

fn normalize_deployment_value(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    rename_key(object, "rpcUrl", "rpc_url");
    rename_key(object, "protocolVersion", "protocol_version");
    if let Some(packages) = object.get_mut("packages").and_then(Value::as_object_mut) {
        rename_key(packages, "governanceOriginal", "governance_original");
    }
    if let Some(objects) = object.get_mut("objects").and_then(Value::as_object_mut) {
        rename_key(objects, "typeRegistry", "type_registry");
        rename_key(objects, "feeManager", "fee_manager");
        rename_key(objects, "governanceVault", "governance_vault");
        rename_key(objects, "governanceConfig", "governance_config");
    }
    rename_key(object, "coinTypes", "coin_types");
}

fn rename_key(map: &mut serde_json::Map<String, Value>, from: &str, to: &str) {
    if map.contains_key(to) {
        return;
    }
    if let Some(value) = map.remove(from) {
        map.insert(to.to_string(), value);
    }
}

fn unchecked(
    current: Deployment,
    manifest_url: Option<String>,
    message: impl Into<String>,
    error: Option<String>,
) -> DeploymentUpdateCheck {
    DeploymentUpdateCheck {
        status: DeploymentManifestStatus::Unchecked,
        current,
        latest: None,
        manifest_url,
        min_sdk_version: None,
        updated_at: None,
        release_notes_url: None,
        message: message.into(),
        differences: Vec::new(),
        error,
    }
}

fn value_at(object: &Value, path: &str) -> Option<String> {
    let mut current = object;
    for segment in path.split('.') {
        current = current
            .get(segment)
            .or_else(|| current.get(camel_case(segment)))?;
    }
    Some(match current {
        Value::String(value) => value.clone(),
        Value::Null => return None,
        other => other.to_string(),
    })
}

fn camel_case(segment: &str) -> String {
    let mut output = String::new();
    let mut upper = false;
    for ch in segment.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            output.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            output.push(ch);
        }
    }
    output
}

fn same_value(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right || left.eq_ignore_ascii_case(right),
        (None, None) => true,
        _ => false,
    }
}
