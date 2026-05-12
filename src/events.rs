// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    deployment::Deployment,
    error::{PaperProofError, Result},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SuiEventEnvelope {
    pub id: Option<Value>,
    #[serde(rename = "packageId")]
    pub package_id: String,
    #[serde(rename = "transactionModule")]
    pub transaction_module: String,
    pub sender: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(rename = "parsedJson")]
    pub parsed_json: Value,
    pub bcs: Option<String>,
    pub timestamp_ms: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub enum PaperProofEventKind {
    RootCreated,
    TypeRegistryCreated,
    TypeIndexCreated,
    TreeCreated,
    GovernanceVaultCreated,
    FeeManagerCreated,
    GovernanceConfigCreated,
    GovernanceConfigBound,
    PreprintCodeReserved,
    ArtifactPublished,
    ArtifactVersionAdded,
    SeriesMetadataUpdated,
    ArtifactTypeStatusChanged,
    CommentAdded,
    CommentStatusChanged,
    TreeStatusChanged,
    CommentsTreeMigrated,
    PaperLiked,
    PaperUnliked,
    ProposalCreated,
    ProposalVoted,
    ProposalFinalized,
    ProposalExecuted,
    ProposalExpired,
    VoteClaimed,
    GovernanceConfigMigrated,
    ProposalMigrated,
    ProposalCreationPausedChanged,
    ProposerThresholdChanged,
    ProposalDurationChanged,
    GovernanceActionStatusChanged,
    ArtifactStatusChanged,
    ProtocolPausedChanged,
    FeeRecipientChanged,
    GovernanceAuthorityChanged,
    CommentsFeeLevelChanged,
    ArtifactFeeLevelChanged,
    UpgradeAuthorityChanged,
    FeeCollected,
    DirectAuthorityModeChanged,
    OperatorNominated,
    OperatorTransferCancelled,
    ManagedUpgradeCapRegistered,
    ManagedUpgradeAuthorized,
    ManagedUpgradeCommitted,
    GovernanceVaultMigrated,
    OwnerTransferred,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ParsedPaperProofEvent {
    pub kind: PaperProofEventKind,
    pub package_id: String,
    pub event_type: String,
    pub fields: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PublishResult {
    pub series_id: String,
    pub version_id: String,
    pub comments_tree_id: String,
    pub likes_book_id: String,
    pub artifact_code: String,
    pub artifact_type: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PreprintReservationResult {
    pub reservation_id: String,
    pub series_id: String,
    pub reserver: String,
    pub artifact_code: String,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AddVersionResult {
    pub series_id: String,
    pub version_id: String,
    pub artifact_type: u64,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CommentResult {
    pub tree_id: String,
    pub comment_id: u64,
    pub parent_comment_id: u64,
    pub content_mode: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProposalResult {
    pub proposal_id: u64,
    pub proposal_object_id: String,
    pub action_type: u64,
    pub proposal_type: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct LikeResult {
    pub tree_id: String,
    pub likes_book_id: String,
    pub target_series_id: String,
    pub liker: String,
    pub like_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VoteCastResult {
    pub registry_id: String,
    pub proposal_id: u64,
    pub voter: String,
    pub side: u64,
    pub voting_power: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProposalFinalizedResult {
    pub registry_id: String,
    pub proposal_id: u64,
    pub yes_votes: u64,
    pub no_votes: u64,
    pub status: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProposalExecutedResult {
    pub registry_id: String,
    pub proposal_id: u64,
    pub action_type: u64,
    pub executed_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProposalExpiredResult {
    pub registry_id: String,
    pub proposal_id: u64,
    pub expired_at_epoch: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VoteClaimedResult {
    pub registry_id: String,
    pub proposal_id: u64,
    pub voter: String,
    pub side: u64,
    pub voting_power: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StatusChangedResult {
    pub registry_id: Option<String>,
    pub root_id: Option<String>,
    pub series_id: Option<String>,
    pub tree_id: Option<String>,
    pub comment_id: Option<u64>,
    pub changed_by: String,
    pub old_status: Option<u64>,
    pub new_status: Option<u64>,
    pub old_paused: Option<bool>,
    pub new_paused: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct OwnerTransferredResult {
    pub registry_id: Option<String>,
    pub series_id: Option<String>,
    pub tree_id: Option<String>,
    pub changed_by: String,
    pub old_owner: String,
    pub new_owner: String,
}

pub fn parse_event(event: &SuiEventEnvelope) -> ParsedPaperProofEvent {
    ParsedPaperProofEvent {
        kind: classify_event_type(&event.event_type),
        package_id: event.package_id.clone(),
        event_type: event.event_type.clone(),
        fields: event.parsed_json.clone(),
    }
}

pub fn classify_event_type(event_type: &str) -> PaperProofEventKind {
    let name = event_type.rsplit("::").next().unwrap_or(event_type);
    match name {
        "PaperProofRootCreated" | "PaperProofRootCreatedEvent" => PaperProofEventKind::RootCreated,
        "TypeRegistryCreated" | "TypeRegistryCreatedEvent" => {
            PaperProofEventKind::TypeRegistryCreated
        }
        "TypeIndexCreated" | "TypeIndexCreatedEvent" => PaperProofEventKind::TypeIndexCreated,
        "TreeCreated" | "TreeCreatedEvent" => PaperProofEventKind::TreeCreated,
        "GovernanceVaultCreated" | "GovernanceVaultCreatedEvent" => {
            PaperProofEventKind::GovernanceVaultCreated
        }
        "FeeManagerCreated" | "FeeManagerCreatedEvent" => PaperProofEventKind::FeeManagerCreated,
        "GovernanceConfigCreated" | "GovernanceConfigCreatedEvent" => {
            PaperProofEventKind::GovernanceConfigCreated
        }
        "GovernanceConfigBound" | "GovernanceConfigBoundEvent" => {
            PaperProofEventKind::GovernanceConfigBound
        }
        "PreprintCodeReserved" | "PreprintCodeReservedEvent" => {
            PaperProofEventKind::PreprintCodeReserved
        }
        "ArtifactPublished" | "ArtifactPublishedEvent" => PaperProofEventKind::ArtifactPublished,
        "ArtifactVersionAdded" | "ArtifactVersionAddedEvent" => {
            PaperProofEventKind::ArtifactVersionAdded
        }
        "SeriesMetadataUpdated"
        | "SeriesMetadataUpdatedEvent"
        | "ArtifactSeriesMetadataUpdated"
        | "ArtifactSeriesMetadataUpdatedEvent" => PaperProofEventKind::SeriesMetadataUpdated,
        "ArtifactTypeStatusChanged" | "ArtifactTypeStatusChangedEvent" => {
            PaperProofEventKind::ArtifactTypeStatusChanged
        }
        "CommentAdded" | "CommentAddedEvent" => PaperProofEventKind::CommentAdded,
        "CommentStatusChanged" | "CommentStatusChangedEvent" => {
            PaperProofEventKind::CommentStatusChanged
        }
        "TreeStatusChanged" | "TreeStatusChangedEvent" => PaperProofEventKind::TreeStatusChanged,
        "CommentsTreeMigrated" | "CommentsTreeMigratedEvent" => {
            PaperProofEventKind::CommentsTreeMigrated
        }
        "PaperLiked" | "PaperLikedEvent" => PaperProofEventKind::PaperLiked,
        "PaperUnliked" | "PaperUnlikedEvent" => PaperProofEventKind::PaperUnliked,
        "ProposalCreated" | "ProposalCreatedEvent" => PaperProofEventKind::ProposalCreated,
        "VoteCast" | "VoteCastEvent" | "ProposalVoted" => PaperProofEventKind::ProposalVoted,
        "ProposalFinalized" | "ProposalFinalizedEvent" => PaperProofEventKind::ProposalFinalized,
        "ProposalExecuted" | "ProposalExecutedEvent" => PaperProofEventKind::ProposalExecuted,
        "ProposalExpired" | "ProposalExpiredEvent" => PaperProofEventKind::ProposalExpired,
        "VoteClaimed" | "VoteClaimedEvent" => PaperProofEventKind::VoteClaimed,
        "GovernanceConfigMigrated" | "GovernanceConfigMigratedEvent" => {
            PaperProofEventKind::GovernanceConfigMigrated
        }
        "ProposalMigrated" | "ProposalMigratedEvent" => PaperProofEventKind::ProposalMigrated,
        "ProposalCreationPausedChanged" | "ProposalCreationPausedChangedEvent" => {
            PaperProofEventKind::ProposalCreationPausedChanged
        }
        "ProposerThresholdChanged" | "ProposerThresholdChangedEvent" => {
            PaperProofEventKind::ProposerThresholdChanged
        }
        "ProposalDurationChanged" | "ProposalDurationChangedEvent" => {
            PaperProofEventKind::ProposalDurationChanged
        }
        "GovernanceActionStatusChanged" | "GovernanceActionStatusChangedEvent" => {
            PaperProofEventKind::GovernanceActionStatusChanged
        }
        "ArtifactStatusChanged" | "ArtifactStatusChangedEvent" => {
            PaperProofEventKind::ArtifactStatusChanged
        }
        "ProtocolPausedChanged" | "ProtocolPausedChangedEvent" => {
            PaperProofEventKind::ProtocolPausedChanged
        }
        "FeeRecipientChanged" | "FeeRecipientChangedEvent" => {
            PaperProofEventKind::FeeRecipientChanged
        }
        "GovernanceAuthorityChanged" | "GovernanceAuthorityChangedEvent" => {
            PaperProofEventKind::GovernanceAuthorityChanged
        }
        "CommentsFeeLevelChanged" | "CommentsFeeLevelChangedEvent" => {
            PaperProofEventKind::CommentsFeeLevelChanged
        }
        "ArtifactFeeLevelChanged" | "ArtifactFeeLevelChangedEvent" => {
            PaperProofEventKind::ArtifactFeeLevelChanged
        }
        "UpgradeAuthorityChanged" | "UpgradeAuthorityChangedEvent" => {
            PaperProofEventKind::UpgradeAuthorityChanged
        }
        "FeeCollected" | "FeeCollectedEvent" => PaperProofEventKind::FeeCollected,
        "DirectAuthorityModeChanged" | "DirectAuthorityModeChangedEvent" => {
            PaperProofEventKind::DirectAuthorityModeChanged
        }
        "OperatorNominated" | "OperatorNominatedEvent" => PaperProofEventKind::OperatorNominated,
        "OperatorTransferCancelled" | "OperatorTransferCancelledEvent" => {
            PaperProofEventKind::OperatorTransferCancelled
        }
        "ManagedUpgradeCapRegistered" | "ManagedUpgradeCapRegisteredEvent" => {
            PaperProofEventKind::ManagedUpgradeCapRegistered
        }
        "ManagedUpgradeAuthorized" | "ManagedUpgradeAuthorizedEvent" => {
            PaperProofEventKind::ManagedUpgradeAuthorized
        }
        "ManagedUpgradeCommitted" | "ManagedUpgradeCommittedEvent" => {
            PaperProofEventKind::ManagedUpgradeCommitted
        }
        "GovernanceVaultMigrated" | "GovernanceVaultMigratedEvent" => {
            PaperProofEventKind::GovernanceVaultMigrated
        }
        "ArtifactOwnerTransferred"
        | "ArtifactOwnerTransferredEvent"
        | "TreeOwnerTransferred"
        | "TreeOwnerTransferredEvent"
        | "OperatorTransferAccepted"
        | "OperatorTransferAcceptedEvent" => PaperProofEventKind::OwnerTransferred,
        _ => PaperProofEventKind::Unknown,
    }
}

pub fn event_field_string(event: &ParsedPaperProofEvent, field: &str) -> Result<Option<String>> {
    match event.fields.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(PaperProofError::EventParse {
            message: format!("field `{field}` is not a string"),
        }),
    }
}

pub fn event_struct_name(event_type: &str) -> &str {
    event_type.rsplit("::").next().unwrap_or(event_type)
}

pub fn events_from_value(value: &Value) -> Result<Vec<SuiEventEnvelope>> {
    let Some(events) = value.get("events").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    events
        .iter()
        .cloned()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn find_events(
    events: &[SuiEventEnvelope],
    struct_name: &str,
    deployment: Option<&Deployment>,
) -> Vec<SuiEventEnvelope> {
    events
        .iter()
        .filter(|event| {
            event_struct_name(&event.event_type) == struct_name
                && deployment
                    .map(|deployment| is_from_known_package(event, deployment))
                    .unwrap_or(true)
        })
        .cloned()
        .collect()
}

pub fn require_first_event(
    events: &[SuiEventEnvelope],
    struct_name: &str,
    deployment: Option<&Deployment>,
) -> Result<SuiEventEnvelope> {
    find_events(events, struct_name, deployment)
        .into_iter()
        .next()
        .ok_or_else(|| PaperProofError::EventParse {
            message: format!(
                "transaction output does not contain {struct_name}; available events: {}",
                events
                    .iter()
                    .map(|event| event.event_type.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
}

pub fn extract_publish_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<PublishResult> {
    let event = require_first_event(events, "ArtifactPublishedEvent", deployment)?;
    let fields = &event.parsed_json;
    Ok(PublishResult {
        series_id: required_string(fields, "series_id")?,
        version_id: required_string(fields, "version_id")?,
        comments_tree_id: required_string(fields, "comments_tree_id")?,
        likes_book_id: required_string(fields, "likes_book_id")?,
        artifact_code: required_string(fields, "artifact_code")?,
        artifact_type: required_u64(fields, "artifact_type")?,
    })
}

pub fn extract_preprint_reservation_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<PreprintReservationResult> {
    let event = require_first_event(events, "PreprintCodeReservedEvent", deployment)?;
    let fields = &event.parsed_json;
    Ok(PreprintReservationResult {
        reservation_id: required_string(fields, "reservation_id")?,
        series_id: required_string(fields, "series_id")?,
        reserver: required_string(fields, "reserver")?,
        artifact_code: required_string(fields, "artifact_code")?,
        created_at_ms: required_u64(fields, "created_at_ms")?,
    })
}

pub fn extract_add_version_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<AddVersionResult> {
    let event = require_first_event(events, "ArtifactVersionAddedEvent", deployment)?;
    let fields = &event.parsed_json;
    Ok(AddVersionResult {
        series_id: required_string(fields, "series_id")?,
        version_id: optional_string(fields, "version_id")
            .or_else(|| optional_string(fields, "new_version_id"))
            .ok_or_else(|| PaperProofError::EventParse {
                message: "ArtifactVersionAddedEvent lacks version_id/new_version_id".to_string(),
            })?,
        artifact_type: required_u64(fields, "artifact_type")?,
        version: required_u64(fields, "version")?,
    })
}

pub fn extract_comment_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<CommentResult> {
    let event = require_first_event(events, "CommentAddedEvent", deployment)?;
    let fields = &event.parsed_json;
    Ok(CommentResult {
        tree_id: required_string(fields, "tree_id")?,
        comment_id: required_u64(fields, "comment_id")?,
        parent_comment_id: required_u64(fields, "parent_comment_id")?,
        content_mode: required_u64(fields, "content_mode")?,
    })
}

pub fn extract_proposal_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<ProposalResult> {
    let event = require_first_event(events, "ProposalCreatedEvent", deployment)?;
    let fields = &event.parsed_json;
    Ok(ProposalResult {
        proposal_id: required_u64(fields, "proposal_id")?,
        proposal_object_id: required_string(fields, "proposal_object_id")?,
        action_type: required_u64(fields, "action_type")?,
        proposal_type: required_u64(fields, "proposal_type")?,
    })
}

pub fn extract_like_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<LikeResult>> {
    extract_like_by_struct(events, "PaperLikedEvent", deployment)
}

pub fn extract_unlike_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<LikeResult>> {
    extract_like_by_struct(events, "PaperUnlikedEvent", deployment)
}

pub fn extract_vote_cast_results(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Vec<VoteCastResult>> {
    find_events(events, "VoteCastEvent", deployment)
        .iter()
        .map(|event| {
            let fields = &event.parsed_json;
            Ok(VoteCastResult {
                registry_id: required_string(fields, "registry_id")?,
                proposal_id: required_u64(fields, "proposal_id")?,
                voter: required_string(fields, "voter")?,
                side: required_u64(fields, "side")?,
                voting_power: required_u64(fields, "voting_power")?,
            })
        })
        .collect()
}

pub fn extract_proposal_finalized_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<ProposalFinalizedResult>> {
    let Some(event) = find_events(events, "ProposalFinalizedEvent", deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(ProposalFinalizedResult {
        registry_id: required_string(fields, "registry_id")?,
        proposal_id: required_u64(fields, "proposal_id")?,
        yes_votes: required_u64(fields, "yes_votes")?,
        no_votes: required_u64(fields, "no_votes")?,
        status: required_u64(fields, "status")?,
    }))
}

pub fn extract_proposal_executed_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<ProposalExecutedResult>> {
    let Some(event) = find_events(events, "ProposalExecutedEvent", deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(ProposalExecutedResult {
        registry_id: required_string(fields, "registry_id")?,
        proposal_id: required_u64(fields, "proposal_id")?,
        action_type: required_u64(fields, "action_type")?,
        executed_by: required_string(fields, "executed_by")?,
    }))
}

pub fn extract_proposal_expired_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<ProposalExpiredResult>> {
    let Some(event) = find_events(events, "ProposalExpiredEvent", deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(ProposalExpiredResult {
        registry_id: required_string(fields, "registry_id")?,
        proposal_id: required_u64(fields, "proposal_id")?,
        expired_at_epoch: required_u64(fields, "expired_at_epoch")?,
    }))
}

pub fn extract_vote_claimed_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<VoteClaimedResult>> {
    let Some(event) = find_events(events, "VoteClaimedEvent", deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(VoteClaimedResult {
        registry_id: required_string(fields, "registry_id")?,
        proposal_id: required_u64(fields, "proposal_id")?,
        voter: required_string(fields, "voter")?,
        side: required_u64(fields, "side")?,
        voting_power: required_u64(fields, "voting_power")?,
    }))
}

pub fn extract_status_changed_result(
    events: &[SuiEventEnvelope],
    struct_name: &str,
    deployment: Option<&Deployment>,
) -> Result<Option<StatusChangedResult>> {
    let Some(event) = find_events(events, struct_name, deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(StatusChangedResult {
        registry_id: optional_string(fields, "registry_id"),
        root_id: optional_string(fields, "root_id"),
        series_id: optional_string(fields, "series_id"),
        tree_id: optional_string(fields, "tree_id"),
        comment_id: optional_u64(fields, "comment_id"),
        changed_by: required_string(fields, "changed_by")?,
        old_status: optional_u64(fields, "old_status"),
        new_status: optional_u64(fields, "new_status"),
        old_paused: optional_bool(fields, "old_paused"),
        new_paused: optional_bool(fields, "new_paused"),
    }))
}

pub fn extract_owner_transferred_result(
    events: &[SuiEventEnvelope],
    deployment: Option<&Deployment>,
) -> Result<Option<OwnerTransferredResult>> {
    for struct_name in [
        "ArtifactOwnerTransferredEvent",
        "TreeOwnerTransferredEvent",
        "OperatorTransferAcceptedEvent",
    ] {
        if let Some(event) = find_events(events, struct_name, deployment)
            .into_iter()
            .next()
        {
            let fields = &event.parsed_json;
            return Ok(Some(OwnerTransferredResult {
                registry_id: optional_string(fields, "registry_id"),
                series_id: optional_string(fields, "series_id"),
                tree_id: optional_string(fields, "tree_id"),
                changed_by: required_string(fields, "changed_by")?,
                old_owner: required_string(fields, "old_owner")?,
                new_owner: required_string(fields, "new_owner")?,
            }));
        }
    }
    Ok(None)
}

pub fn extract_events_by_struct<T>(
    events: &[SuiEventEnvelope],
    struct_name: &str,
    deployment: Option<&Deployment>,
) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    find_events(events, struct_name, deployment)
        .into_iter()
        .map(|event| serde_json::from_value(event.parsed_json).map_err(Into::into))
        .collect()
}

fn extract_like_by_struct(
    events: &[SuiEventEnvelope],
    struct_name: &str,
    deployment: Option<&Deployment>,
) -> Result<Option<LikeResult>> {
    let Some(event) = find_events(events, struct_name, deployment)
        .into_iter()
        .next()
    else {
        return Ok(None);
    };
    let fields = &event.parsed_json;
    Ok(Some(LikeResult {
        tree_id: required_string(fields, "tree_id")?,
        likes_book_id: required_string(fields, "likes_book_id")?,
        target_series_id: required_string(fields, "target_series_id")?,
        liker: required_string(fields, "liker")?,
        like_count: required_u64(fields, "like_count")?,
    }))
}

fn required_string(fields: &Value, field: &str) -> Result<String> {
    optional_string(fields, field).ok_or_else(|| PaperProofError::EventParse {
        message: format!("event field `{field}` is missing or not a string"),
    })
}

fn optional_string(fields: &Value, field: &str) -> Option<String> {
    fields.get(field)?.as_str().map(ToString::to_string)
}

fn optional_u64(fields: &Value, field: &str) -> Option<u64> {
    let value = fields.get(field)?;
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .or_else(|| value.as_f64().map(|number| number as u64))
}

fn optional_bool(fields: &Value, field: &str) -> Option<bool> {
    fields.get(field)?.as_bool()
}

fn required_u64(fields: &Value, field: &str) -> Result<u64> {
    let value = fields
        .get(field)
        .ok_or_else(|| PaperProofError::EventParse {
            message: format!("event field `{field}` is missing"),
        })?;
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .or_else(|| value.as_f64().map(|number| number as u64))
        .ok_or_else(|| PaperProofError::EventParse {
            message: format!("event field `{field}` is not a u64-compatible value"),
        })
}

pub fn is_from_known_package(event: &SuiEventEnvelope, deployment: &Deployment) -> bool {
    let package = event.package_id.as_str();
    package == deployment.packages.publishing
        || package == deployment.packages.comments
        || package == deployment.packages.governance
}
