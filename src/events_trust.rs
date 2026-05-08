// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    deployment::Deployment,
    events::{SuiEventEnvelope, event_struct_name},
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EventTrustResult {
    pub trusted: bool,
    pub reason: Option<String>,
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
        return EventTrustResult {
            trusted: false,
            reason: Some(format!(
                "event package {} is not a configured PaperProof package",
                event.package_id
            )),
        };
    }

    if field_as_str(&event.parsed_json, "root_id")
        .or_else(|| field_as_str(&event.parsed_json, "root"))
        .is_some_and(|root| !same_id(root, &deployment.objects.root))
    {
        return EventTrustResult {
            trusted: false,
            reason: Some("event root id does not match the deployment root".to_string()),
        };
    }

    if let Some(registry) = field_as_str(&event.parsed_json, "registry_id")
        && !same_id(registry, &deployment.objects.root)
    {
        return EventTrustResult {
            trusted: false,
            reason: Some("event registry id does not match the deployment root".to_string()),
        };
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
        &deployment.packages.publishing,
        &deployment.packages.comments,
        &deployment.packages.governance,
        &deployment.packages.governance_original,
    ]
    .iter()
    .any(|package| same_id(&event.package_id, package))
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
    }
}

fn untrusted(reason: impl Into<String>) -> EventTrustResult {
    EventTrustResult {
        trusted: false,
        reason: Some(reason.into()),
    }
}

fn same_id(left: &str, right: &str) -> bool {
    normalize_id(left) == normalize_id(right)
}

fn normalize_id(value: &str) -> String {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", raw.trim_start_matches('0').to_ascii_lowercase())
}
