// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    deployment::{Deployment, DeploymentPackageFamily, deployment_package_ids},
    error::{PaperProofError, Result},
    events::{SuiEventEnvelope, event_struct_name},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventTrustLevel {
    Raw,
    #[default]
    Canonical,
    Verified,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventVerificationStatus {
    Raw,
    Canonical,
    Verified,
    Rejected,
    Incomplete,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventIssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EventVerificationIssue {
    pub code: String,
    pub reason: String,
    pub severity: EventIssueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EventTrustResult {
    pub trusted: bool,
    pub reason: Option<String>,
    #[serde(default = "default_canonical_status")]
    pub status: EventVerificationStatus,
    #[serde(default)]
    pub issues: Vec<EventVerificationIssue>,
}

impl EventTrustResult {
    pub fn trusted() -> Self {
        trusted()
    }

    pub fn rejected(reason: impl Into<String>) -> Self {
        untrusted(reason)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventVerificationReport {
    pub event: SuiEventEnvelope,
    pub requested_trust: EventTrustLevel,
    pub status: EventVerificationStatus,
    pub trusted: bool,
    pub canonical: bool,
    pub verified: bool,
    pub issues: Vec<EventVerificationIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub bindings: serde_json::Map<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustedSuiEventEnvelope {
    pub event: SuiEventEnvelope,
    pub trust: EventVerificationStatus,
    pub verification: EventVerificationReport,
}

pub trait VerifiedEventPageGuard {
    fn trust_level(&self) -> EventTrustLevel;
    fn verification_reports(&self) -> &[EventVerificationReport];
}

pub fn assert_no_incomplete<P>(page: &P) -> Result<()>
where
    P: VerifiedEventPageGuard,
{
    let reports = page.verification_reports();
    let incomplete = reports
        .iter()
        .filter(|report| report.status == EventVerificationStatus::Incomplete)
        .count();
    let rejected = reports
        .iter()
        .filter(|report| report.status == EventVerificationStatus::Rejected)
        .count();
    let unverified = reports.iter().filter(|report| !report.verified).count();
    if page.trust_level() != EventTrustLevel::Verified
        || incomplete > 0
        || rejected > 0
        || unverified > 0
    {
        return Err(PaperProofError::event_verification(format!(
            "PaperProof page is not fully verified: trust={:?}, incomplete={}, rejected={}, unverified={}",
            page.trust_level(),
            incomplete,
            rejected,
            unverified
        )));
    }
    Ok(())
}

pub fn require_verified_page<P>(page: P) -> Result<P>
where
    P: VerifiedEventPageGuard,
{
    assert_no_incomplete(&page)?;
    Ok(page)
}

pub fn validate_event_trust(event: &SuiEventEnvelope, deployment: &Deployment) -> EventTrustResult {
    check_canonical_paperproof_event(event, deployment)
}

pub fn is_canonical_paperproof_event(event: &SuiEventEnvelope, deployment: &Deployment) -> bool {
    check_canonical_paperproof_event(event, deployment).trusted
}

pub fn check_canonical_paperproof_event(
    event: &SuiEventEnvelope,
    deployment: &Deployment,
) -> EventTrustResult {
    if !package_trusted(event, deployment) {
        return untrusted_with_code(
            "PACKAGE_NOT_CONFIGURED",
            format!(
                "event package {} is not a configured PaperProof package",
                event.package_id
            ),
            Some(json!({ "package_id": event.package_id })),
        );
    }

    if field_as_str(&event.parsed_json, "root_id")
        .or_else(|| field_as_str(&event.parsed_json, "root"))
        .is_some_and(|root| !same_id(root, &deployment.objects.root))
    {
        return untrusted_with_code(
            "ROOT_MISMATCH",
            "event root id does not match the deployment root",
            Some(json!({ "expected": deployment.objects.root })),
        );
    }

    if let Some(registry) = field_as_str(&event.parsed_json, "registry_id")
        && !same_id(registry, &deployment.objects.root)
    {
        return untrusted_with_code(
            "REGISTRY_MISMATCH",
            "event registry id does not match the deployment root",
            Some(json!({ "expected": deployment.objects.root, "actual": registry })),
        );
    }

    let fields = &event.parsed_json;
    match event_struct_name(&event.event_type) {
        "ArtifactPublishedEvent" => require_fields(
            fields,
            &[
                "series_id",
                "version_id",
                "comments_tree_id",
                "likes_book_id",
            ],
            "artifact publish event is missing canonical object ids",
        ),
        "ArtifactVersionAddedEvent" => {
            if field_present(fields, "series_id")
                && (field_present(fields, "version_id") || field_present(fields, "new_version_id"))
            {
                trusted()
            } else {
                untrusted("artifact version event is missing series/version ids")
            }
        }
        "PreprintCodeReservedEvent" => require_fields(
            fields,
            &["reservation_id", "series_id", "reserver", "artifact_code"],
            "preprint reservation event is missing reservation/series/code fields",
        ),
        "CommentAddedEvent" => require_fields(
            fields,
            &["tree_id", "comment_id"],
            "comment event is missing tree_id/comment_id",
        ),
        "PaperLikedEvent" | "PaperUnlikedEvent" => require_fields(
            fields,
            &["likes_book_id", "target_series_id"],
            "like event is missing likes_book_id/target_series_id",
        ),
        "ProposalCreatedEvent" => require_fields(
            fields,
            &["registry_id", "proposal_object_id"],
            "proposal event is missing registry_id/proposal_object_id",
        ),
        _ => trusted(),
    }
}

pub fn verification_report_from_canonical_check(
    event: &SuiEventEnvelope,
    deployment: &Deployment,
    requested_trust: EventTrustLevel,
) -> EventVerificationReport {
    if requested_trust == EventTrustLevel::Raw {
        return EventVerificationReport {
            event: event.clone(),
            requested_trust,
            status: EventVerificationStatus::Raw,
            trusted: true,
            canonical: false,
            verified: false,
            issues: Vec::new(),
            provider: None,
            bindings: serde_json::Map::new(),
        };
    }
    let check = check_canonical_paperproof_event(event, deployment);
    EventVerificationReport {
        event: event.clone(),
        requested_trust,
        status: check.status.clone(),
        trusted: check.trusted,
        canonical: check.trusted,
        verified: false,
        issues: check.issues,
        provider: None,
        bindings: serde_json::Map::new(),
    }
}

pub fn attach_event_verification(report: EventVerificationReport) -> TrustedSuiEventEnvelope {
    TrustedSuiEventEnvelope {
        trust: report.status.clone(),
        event: report.event.clone(),
        verification: report,
    }
}

pub fn filter_canonical_paperproof_events(
    events: &[SuiEventEnvelope],
    deployment: &Deployment,
) -> Vec<SuiEventEnvelope> {
    events
        .iter()
        .filter(|event| is_canonical_paperproof_event(event, deployment))
        .cloned()
        .collect()
}

pub fn explain_untrusted_paperproof_events(
    events: &[SuiEventEnvelope],
    deployment: &Deployment,
) -> Vec<(SuiEventEnvelope, String)> {
    events
        .iter()
        .filter_map(|event| {
            let check = check_canonical_paperproof_event(event, deployment);
            (!check.trusted).then(|| {
                (
                    event.clone(),
                    check
                        .reason
                        .unwrap_or_else(|| "event is not trusted".to_string()),
                )
            })
        })
        .collect()
}

fn field_as_str<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn package_trusted(event: &SuiEventEnvelope, deployment: &Deployment) -> bool {
    [
        deployment_package_ids(deployment, DeploymentPackageFamily::Publishing),
        deployment_package_ids(deployment, DeploymentPackageFamily::Comments),
        deployment_package_ids(deployment, DeploymentPackageFamily::Governance),
    ]
    .into_iter()
    .flatten()
    .any(|package| same_id(&event.package_id, &package))
}

fn require_fields(value: &Value, fields: &[&str], reason: &str) -> EventTrustResult {
    if fields.iter().all(|field| field_present(value, field)) {
        trusted()
    } else {
        untrusted(reason)
    }
}

fn field_present(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
}

fn trusted() -> EventTrustResult {
    EventTrustResult {
        trusted: true,
        reason: None,
        status: EventVerificationStatus::Canonical,
        issues: Vec::new(),
    }
}

fn untrusted(reason: impl Into<String>) -> EventTrustResult {
    untrusted_with_code("CANONICAL_REJECTED", reason, None)
}

fn untrusted_with_code(
    code: impl Into<String>,
    reason: impl Into<String>,
    details: Option<Value>,
) -> EventTrustResult {
    let reason = reason.into();
    EventTrustResult {
        trusted: false,
        reason: Some(reason.clone()),
        status: EventVerificationStatus::Rejected,
        issues: vec![EventVerificationIssue {
            code: code.into(),
            reason,
            severity: EventIssueSeverity::Error,
            details,
        }],
    }
}

fn default_canonical_status() -> EventVerificationStatus {
    EventVerificationStatus::Canonical
}

fn same_id(left: &str, right: &str) -> bool {
    normalize_id(left) == normalize_id(right)
}

fn normalize_id(value: &str) -> String {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", raw.trim_start_matches('0').to_ascii_lowercase())
}
