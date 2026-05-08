// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    deployment::Deployment,
    error::Result,
    events::{PaperProofEventKind, SuiEventEnvelope, parse_event},
    events_trust::{EventTrustResult, check_canonical_paperproof_event},
    query::{EventPage, EventQueryInput, PaginationInput, PaperProofQueryClient},
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PackageModuleFilter {
    pub package_id: String,
    pub module: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct StreamId(pub String);

impl StreamId {
    pub fn checkpoint() -> Self {
        Self("checkpoint".to_string())
    }

    pub fn module(module: &PackageModuleFilter) -> Self {
        Self(format!("{}::{}", module.package_id, module.module))
    }
}

impl From<&PackageModuleFilter> for StreamId {
    fn from(value: &PackageModuleFilter) -> Self {
        Self::module(value)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct EventId {
    pub checkpoint: Option<u64>,
    pub transaction_digest: Option<String>,
    pub event_seq: Option<u64>,
    pub package_id: String,
    pub module: String,
    pub event_type: String,
}

impl EventId {
    pub fn key(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.checkpoint
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.transaction_digest.as_deref().unwrap_or("-"),
            self.event_seq
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.package_id,
            self.module,
            self.event_type
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CheckpointCursor {
    pub next_checkpoint: u64,
}

impl CheckpointCursor {
    pub fn new(next_checkpoint: u64) -> Self {
        Self { next_checkpoint }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StoredIndexerCursor {
    pub event_cursor: Option<Value>,
    pub checkpoint_cursor: Option<CheckpointCursor>,
}

#[async_trait]
pub trait IndexerCursorStore: Send + Sync {
    async fn load_cursor(&self, stream: &StreamId) -> Result<Option<StoredIndexerCursor>>;

    async fn save_cursor(&self, stream: &StreamId, cursor: StoredIndexerCursor) -> Result<()>;

    async fn mark_processed(&self, event_id: &EventId) -> Result<bool>;
}

#[derive(Clone, Debug, Default)]
pub struct MemoryIndexerCursorStore {
    cursors: std::sync::Arc<std::sync::Mutex<BTreeMap<StreamId, StoredIndexerCursor>>>,
    processed: std::sync::Arc<std::sync::Mutex<BTreeSet<EventId>>>,
}

#[async_trait]
impl IndexerCursorStore for MemoryIndexerCursorStore {
    async fn load_cursor(&self, stream: &StreamId) -> Result<Option<StoredIndexerCursor>> {
        Ok(self
            .cursors
            .lock()
            .expect("cursor store poisoned")
            .get(stream)
            .cloned())
    }

    async fn save_cursor(&self, stream: &StreamId, cursor: StoredIndexerCursor) -> Result<()> {
        self.cursors
            .lock()
            .expect("cursor store poisoned")
            .insert(stream.clone(), cursor);
        Ok(())
    }

    async fn mark_processed(&self, event_id: &EventId) -> Result<bool> {
        Ok(self
            .processed
            .lock()
            .expect("cursor store poisoned")
            .insert(event_id.clone()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexerProgress {
    pub cursor: Option<Value>,
    pub has_next_page: bool,
    pub scanned_pages: u64,
    pub scanned_events: u64,
    pub accepted_events: u64,
    pub rejected_events: u64,
    pub last_event_id: Option<Value>,
    pub last_timestamp_ms: Option<u64>,
}

impl Default for IndexerProgress {
    fn default() -> Self {
        Self {
            cursor: None,
            has_next_page: true,
            scanned_pages: 0,
            scanned_events: 0,
            accepted_events: 0,
            rejected_events: 0,
            last_event_id: None,
            last_timestamp_ms: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexedPaperProofEvent {
    pub id: EventId,
    pub event: SuiEventEnvelope,
    pub kind: PaperProofEventKind,
    pub trust: EventTrustResult,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RejectedPaperProofEvent {
    pub event: SuiEventEnvelope,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexerEventBatch {
    pub accepted: Vec<IndexedPaperProofEvent>,
    pub rejected: Vec<RejectedPaperProofEvent>,
    pub progress: IndexerProgress,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PaperProofDomainChange {
    RootCreated {
        root_id: String,
        governance_vault_id: Option<String>,
        fee_manager_id: Option<String>,
        type_registry_id: Option<String>,
    },
    TypeRegistryCreated {
        root_id: String,
        type_registry_id: String,
    },
    TypeIndexCreated {
        root_id: String,
        artifact_type: u64,
        type_index_id: String,
    },
    TreeCreated {
        tree_id: String,
        registry_id: String,
        target_series_id: Option<String>,
        likes_book_id: Option<String>,
    },
    GovernanceObjectCreated {
        registry_id: String,
        object_id: String,
        kind: String,
    },
    GovernanceConfigBound {
        registry_id: String,
        governance_config_id: String,
    },
    SeriesCreated {
        series_id: String,
        version_id: String,
        comments_tree_id: Option<String>,
        likes_book_id: Option<String>,
    },
    VersionAdded {
        series_id: String,
        version_id: String,
        version: Option<u64>,
    },
    SeriesMetadataUpdated {
        series_id: String,
    },
    ArtifactTypeStatusChanged {
        registry_id: Option<String>,
        artifact_type: Option<u64>,
        enabled: Option<bool>,
    },
    CommentAdded {
        tree_id: String,
        comment_id: u64,
        parent_comment_id: Option<u64>,
    },
    CommentStatusChanged {
        tree_id: Option<String>,
        comment_id: Option<u64>,
        new_status: Option<u64>,
    },
    TreeStatusChanged {
        tree_id: Option<String>,
        new_status: Option<u64>,
    },
    ObjectMigrated {
        registry_id: Option<String>,
        object_id: Option<String>,
        kind: String,
        new_version: Option<u64>,
    },
    LikeChanged {
        likes_book_id: String,
        like_count: Option<u64>,
        liked: bool,
    },
    ProposalCreated {
        proposal_id: u64,
        proposal_object_id: Option<String>,
    },
    VoteCast {
        proposal_id: u64,
        voter: Option<String>,
        side: Option<u64>,
        voting_power: Option<u64>,
    },
    ProposalResolved {
        proposal_id: u64,
        status: Option<u64>,
    },
    ProposalExecuted {
        proposal_id: u64,
        action_type: Option<u64>,
    },
    StakeClaimed {
        proposal_id: u64,
        voter: Option<String>,
    },
    GovernanceParameterChanged {
        registry_id: Option<String>,
        parameter: String,
        old_value: Option<Value>,
        new_value: Option<Value>,
    },
    FeeCollected {
        registry_id: Option<String>,
        payer: Option<String>,
        recipient: Option<String>,
        amount: Option<u64>,
    },
    ManagedUpgradeChanged {
        registry_id: Option<String>,
        package_id: Option<String>,
        status: String,
    },
    ArtifactStatusChanged {
        series_id: Option<String>,
        new_status: Option<u64>,
    },
    ProtocolPausedChanged {
        root_id: Option<String>,
        new_paused: Option<bool>,
    },
    OwnerTransferred {
        series_id: Option<String>,
        tree_id: Option<String>,
        new_owner: Option<String>,
    },
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CheckpointData {
    pub sequence_number: u64,
    pub digest: Option<String>,
    pub events: Vec<SuiEventEnvelope>,
    pub raw: Value,
}

#[async_trait]
pub trait CheckpointDataProvider: Send + Sync {
    async fn get_checkpoint_data(&self, sequence_number: u64) -> Result<CheckpointData>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CheckpointScanOptions {
    pub start_checkpoint: u64,
    pub limit: u64,
    pub canonical_only: bool,
}

impl Default for CheckpointScanOptions {
    fn default() -> Self {
        Self {
            start_checkpoint: 0,
            limit: 10,
            canonical_only: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexerScanOptions {
    pub filter: EventQueryInput,
    pub canonical_only: bool,
}

impl Default for IndexerScanOptions {
    fn default() -> Self {
        Self {
            filter: EventQueryInput {
                pagination: PaginationInput {
                    limit: Some(50),
                    descending_order: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
            canonical_only: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct PaperProofIndexerState {
    pub total_events: u64,
    pub published_series: u64,
    pub versions_added: u64,
    pub comments_added: u64,
    pub likes: u64,
    pub unlikes: u64,
    pub proposals_created: u64,
    pub votes_cast: u64,
    pub proposals_resolved: u64,
    pub proposals_executed: u64,
    pub stakes_claimed: u64,
    pub owner_transfers: u64,
    pub status_changes: u64,
    pub type_status_changes: u64,
    pub migrations: u64,
    pub governance_parameter_changes: u64,
    pub fees_collected: u64,
    pub managed_upgrade_events: u64,
    pub latest_series_versions: BTreeMap<String, String>,
    pub tree_to_series: BTreeMap<String, String>,
    pub latest_comment_by_tree: BTreeMap<String, u64>,
    pub latest_like_count_by_book: BTreeMap<String, u64>,
    pub latest_proposal_status: BTreeMap<u64, u64>,
    pub latest_owner_by_series: BTreeMap<String, String>,
    pub latest_owner_by_tree: BTreeMap<String, String>,
    pub artifact_type_enabled: BTreeMap<u64, bool>,
    pub protocol_paused_by_root: BTreeMap<String, bool>,
    pub governance_object_by_kind: BTreeMap<String, String>,
}

impl PaperProofIndexerState {
    pub fn apply_event(&mut self, event: &IndexedPaperProofEvent) {
        self.apply_change(&domain_change_from_event(event));
    }

    pub fn apply_change(&mut self, change: &PaperProofDomainChange) {
        self.total_events += 1;
        match change {
            PaperProofDomainChange::RootCreated {
                root_id,
                governance_vault_id,
                fee_manager_id,
                type_registry_id,
            } => {
                self.governance_object_by_kind
                    .insert("root".to_string(), root_id.clone());
                if let Some(id) = governance_vault_id {
                    self.governance_object_by_kind
                        .insert("governance_vault".to_string(), id.clone());
                }
                if let Some(id) = fee_manager_id {
                    self.governance_object_by_kind
                        .insert("fee_manager".to_string(), id.clone());
                }
                if let Some(id) = type_registry_id {
                    self.governance_object_by_kind
                        .insert("type_registry".to_string(), id.clone());
                }
            }
            PaperProofDomainChange::TypeRegistryCreated {
                type_registry_id, ..
            } => {
                self.governance_object_by_kind
                    .insert("type_registry".to_string(), type_registry_id.clone());
            }
            PaperProofDomainChange::TypeIndexCreated { artifact_type, .. } => {
                self.artifact_type_enabled
                    .entry(*artifact_type)
                    .or_insert(true);
            }
            PaperProofDomainChange::TreeCreated {
                tree_id,
                target_series_id,
                likes_book_id,
                ..
            } => {
                if let Some(series_id) = target_series_id {
                    self.tree_to_series
                        .insert(tree_id.clone(), series_id.clone());
                }
                if let Some(likes_book_id) = likes_book_id {
                    self.latest_like_count_by_book
                        .entry(likes_book_id.clone())
                        .or_insert(0);
                }
            }
            PaperProofDomainChange::GovernanceObjectCreated {
                object_id, kind, ..
            } => {
                self.governance_object_by_kind
                    .insert(kind.clone(), object_id.clone());
            }
            PaperProofDomainChange::GovernanceConfigBound {
                governance_config_id,
                ..
            } => {
                self.governance_object_by_kind.insert(
                    "governance_config".to_string(),
                    governance_config_id.clone(),
                );
            }
            PaperProofDomainChange::SeriesCreated {
                series_id,
                version_id,
                comments_tree_id,
                likes_book_id,
                ..
            } => {
                self.published_series += 1;
                self.latest_series_versions
                    .insert(series_id.clone(), version_id.clone());
                if let Some(tree_id) = comments_tree_id {
                    self.tree_to_series
                        .insert(tree_id.clone(), series_id.clone());
                }
                if let Some(likes_book_id) = likes_book_id {
                    self.latest_like_count_by_book
                        .entry(likes_book_id.clone())
                        .or_insert(0);
                }
            }
            PaperProofDomainChange::VersionAdded {
                series_id,
                version_id,
                ..
            } => {
                self.versions_added += 1;
                self.latest_series_versions
                    .insert(series_id.clone(), version_id.clone());
            }
            PaperProofDomainChange::SeriesMetadataUpdated { .. } => {}
            PaperProofDomainChange::ArtifactTypeStatusChanged {
                artifact_type,
                enabled,
                ..
            } => {
                self.type_status_changes += 1;
                if let (Some(artifact_type), Some(enabled)) = (artifact_type, enabled) {
                    self.artifact_type_enabled.insert(*artifact_type, *enabled);
                }
            }
            PaperProofDomainChange::CommentAdded {
                tree_id,
                comment_id,
                ..
            } => {
                self.comments_added += 1;
                self.latest_comment_by_tree
                    .insert(tree_id.clone(), *comment_id);
            }
            PaperProofDomainChange::CommentStatusChanged { .. }
            | PaperProofDomainChange::TreeStatusChanged { .. }
            | PaperProofDomainChange::ArtifactStatusChanged { .. } => {
                self.status_changes += 1;
            }
            PaperProofDomainChange::ObjectMigrated { .. } => {
                self.migrations += 1;
            }
            PaperProofDomainChange::LikeChanged {
                likes_book_id,
                like_count,
                liked,
            } => {
                if *liked {
                    self.likes += 1;
                } else {
                    self.unlikes += 1;
                }
                if let Some(count) = like_count {
                    self.latest_like_count_by_book
                        .insert(likes_book_id.clone(), *count);
                }
            }
            PaperProofDomainChange::ProposalCreated { .. } => self.proposals_created += 1,
            PaperProofDomainChange::VoteCast { .. } => self.votes_cast += 1,
            PaperProofDomainChange::ProposalResolved {
                proposal_id,
                status,
            } => {
                self.proposals_resolved += 1;
                if let Some(status) = status {
                    self.latest_proposal_status.insert(*proposal_id, *status);
                }
            }
            PaperProofDomainChange::ProposalExecuted { .. } => self.proposals_executed += 1,
            PaperProofDomainChange::StakeClaimed { .. } => self.stakes_claimed += 1,
            PaperProofDomainChange::GovernanceParameterChanged { .. } => {
                self.governance_parameter_changes += 1;
            }
            PaperProofDomainChange::FeeCollected { .. } => self.fees_collected += 1,
            PaperProofDomainChange::ManagedUpgradeChanged { .. } => {
                self.managed_upgrade_events += 1;
            }
            PaperProofDomainChange::OwnerTransferred {
                series_id,
                tree_id,
                new_owner,
            } => {
                self.owner_transfers += 1;
                if let Some(owner) = new_owner {
                    if let Some(series_id) = series_id {
                        self.latest_owner_by_series
                            .insert(series_id.clone(), owner.clone());
                    }
                    if let Some(tree_id) = tree_id {
                        self.latest_owner_by_tree
                            .insert(tree_id.clone(), owner.clone());
                    }
                }
            }
            PaperProofDomainChange::ProtocolPausedChanged {
                root_id,
                new_paused,
            } => {
                self.status_changes += 1;
                if let (Some(root_id), Some(new_paused)) = (root_id, new_paused) {
                    self.protocol_paused_by_root
                        .insert(root_id.clone(), *new_paused);
                }
            }
            PaperProofDomainChange::Unknown => {}
        }
    }

    pub fn domain_changes(batch: &IndexerEventBatch) -> Vec<PaperProofDomainChange> {
        batch
            .accepted
            .iter()
            .map(domain_change_from_event)
            .collect()
    }

    pub fn apply_batch(&mut self, batch: &IndexerEventBatch) {
        for event in &batch.accepted {
            self.apply_event(event);
        }
    }
}

pub fn domain_change_from_event(event: &IndexedPaperProofEvent) -> PaperProofDomainChange {
    let fields = &event.event.parsed_json;
    match event.kind {
        PaperProofEventKind::RootCreated => {
            if let Some(root_id) = string_field(fields, "root_id") {
                PaperProofDomainChange::RootCreated {
                    root_id,
                    governance_vault_id: string_field(fields, "governance_vault_id"),
                    fee_manager_id: string_field(fields, "fee_manager_id"),
                    type_registry_id: string_field(fields, "type_registry_id"),
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::TypeRegistryCreated => {
            if let (Some(root_id), Some(type_registry_id)) = (
                string_field(fields, "root_id"),
                string_field(fields, "type_registry_id"),
            ) {
                PaperProofDomainChange::TypeRegistryCreated {
                    root_id,
                    type_registry_id,
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::TypeIndexCreated => {
            if let (Some(root_id), Some(artifact_type), Some(type_index_id)) = (
                string_field(fields, "root_id"),
                u64_field(fields, "artifact_type"),
                string_field(fields, "type_index_id"),
            ) {
                PaperProofDomainChange::TypeIndexCreated {
                    root_id,
                    artifact_type,
                    type_index_id,
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::TreeCreated => {
            if let (Some(tree_id), Some(registry_id)) = (
                string_field(fields, "tree_id"),
                string_field(fields, "registry_id"),
            ) {
                PaperProofDomainChange::TreeCreated {
                    tree_id,
                    registry_id,
                    target_series_id: string_field(fields, "target_series_id"),
                    likes_book_id: string_field(fields, "likes_book_id"),
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::GovernanceVaultCreated => {
            governance_object_created(fields, "governance_vault_id", "governance_vault")
        }
        PaperProofEventKind::FeeManagerCreated => {
            governance_object_created(fields, "fee_manager_id", "fee_manager")
        }
        PaperProofEventKind::GovernanceConfigCreated => {
            governance_object_created(fields, "governance_config_id", "governance_config")
        }
        PaperProofEventKind::GovernanceConfigBound => {
            if let (Some(registry_id), Some(governance_config_id)) = (
                string_field(fields, "registry_id"),
                string_field(fields, "governance_config_id"),
            ) {
                PaperProofDomainChange::GovernanceConfigBound {
                    registry_id,
                    governance_config_id,
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::ArtifactPublished => {
            if let (Some(series_id), Some(version_id)) = (
                string_field(fields, "series_id"),
                string_field(fields, "version_id"),
            ) {
                PaperProofDomainChange::SeriesCreated {
                    series_id,
                    version_id,
                    comments_tree_id: string_field(fields, "comments_tree_id"),
                    likes_book_id: string_field(fields, "likes_book_id"),
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::ArtifactVersionAdded => {
            if let (Some(series_id), Some(version_id)) = (
                string_field(fields, "series_id"),
                string_field(fields, "version_id")
                    .or_else(|| string_field(fields, "new_version_id")),
            ) {
                PaperProofDomainChange::VersionAdded {
                    series_id,
                    version_id,
                    version: u64_field(fields, "version"),
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::SeriesMetadataUpdated => {
            PaperProofDomainChange::SeriesMetadataUpdated {
                series_id: string_field(fields, "series_id").unwrap_or_default(),
            }
        }
        PaperProofEventKind::ArtifactTypeStatusChanged => {
            PaperProofDomainChange::ArtifactTypeStatusChanged {
                registry_id: string_field(fields, "registry_id"),
                artifact_type: u64_field(fields, "artifact_type"),
                enabled: fields.get("enabled").and_then(Value::as_bool),
            }
        }
        PaperProofEventKind::CommentAdded => {
            if let (Some(tree_id), Some(comment_id)) = (
                string_field(fields, "tree_id"),
                u64_field(fields, "comment_id"),
            ) {
                PaperProofDomainChange::CommentAdded {
                    tree_id,
                    comment_id,
                    parent_comment_id: u64_field(fields, "parent_comment_id"),
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::CommentStatusChanged => PaperProofDomainChange::CommentStatusChanged {
            tree_id: string_field(fields, "tree_id"),
            comment_id: u64_field(fields, "comment_id"),
            new_status: u64_field(fields, "new_status"),
        },
        PaperProofEventKind::TreeStatusChanged => PaperProofDomainChange::TreeStatusChanged {
            tree_id: string_field(fields, "tree_id"),
            new_status: u64_field(fields, "new_status"),
        },
        PaperProofEventKind::CommentsTreeMigrated => PaperProofDomainChange::ObjectMigrated {
            registry_id: string_field(fields, "registry_id"),
            object_id: string_field(fields, "tree_id"),
            kind: "comments_tree".to_string(),
            new_version: u64_field(fields, "new_version"),
        },
        PaperProofEventKind::PaperLiked | PaperProofEventKind::PaperUnliked => {
            if let Some(likes_book_id) = string_field(fields, "likes_book_id") {
                PaperProofDomainChange::LikeChanged {
                    likes_book_id,
                    like_count: u64_field(fields, "like_count"),
                    liked: event.kind == PaperProofEventKind::PaperLiked,
                }
            } else {
                PaperProofDomainChange::Unknown
            }
        }
        PaperProofEventKind::ProposalCreated => PaperProofDomainChange::ProposalCreated {
            proposal_id: u64_field(fields, "proposal_id").unwrap_or_default(),
            proposal_object_id: string_field(fields, "proposal_object_id"),
        },
        PaperProofEventKind::ProposalVoted => PaperProofDomainChange::VoteCast {
            proposal_id: u64_field(fields, "proposal_id").unwrap_or_default(),
            voter: string_field(fields, "voter"),
            side: u64_field(fields, "side"),
            voting_power: u64_field(fields, "voting_power"),
        },
        PaperProofEventKind::ProposalFinalized | PaperProofEventKind::ProposalExpired => {
            PaperProofDomainChange::ProposalResolved {
                proposal_id: u64_field(fields, "proposal_id").unwrap_or_default(),
                status: u64_field(fields, "status"),
            }
        }
        PaperProofEventKind::ProposalExecuted => PaperProofDomainChange::ProposalExecuted {
            proposal_id: u64_field(fields, "proposal_id").unwrap_or_default(),
            action_type: u64_field(fields, "action_type"),
        },
        PaperProofEventKind::VoteClaimed => PaperProofDomainChange::StakeClaimed {
            proposal_id: u64_field(fields, "proposal_id").unwrap_or_default(),
            voter: string_field(fields, "voter"),
        },
        PaperProofEventKind::GovernanceConfigMigrated => PaperProofDomainChange::ObjectMigrated {
            registry_id: string_field(fields, "registry_id"),
            object_id: string_field(fields, "governance_config_id"),
            kind: "governance_config".to_string(),
            new_version: u64_field(fields, "new_version"),
        },
        PaperProofEventKind::ProposalMigrated => PaperProofDomainChange::ObjectMigrated {
            registry_id: string_field(fields, "registry_id"),
            object_id: u64_field(fields, "proposal_id").map(|value| value.to_string()),
            kind: "proposal".to_string(),
            new_version: u64_field(fields, "new_version"),
        },
        PaperProofEventKind::ProposalCreationPausedChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "proposal_creation_paused".to_string(),
                old_value: fields.get("old_paused").cloned(),
                new_value: fields.get("paused").cloned(),
            }
        }
        PaperProofEventKind::ProposerThresholdChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "proposer_threshold".to_string(),
                old_value: fields.get("old_threshold").cloned(),
                new_value: fields.get("new_threshold").cloned(),
            }
        }
        PaperProofEventKind::ProposalDurationChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "proposal_duration_epochs".to_string(),
                old_value: fields.get("old_duration_epochs").cloned(),
                new_value: fields.get("new_duration_epochs").cloned(),
            }
        }
        PaperProofEventKind::GovernanceActionStatusChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: format!(
                    "governance_action_{}",
                    u64_field(fields, "action_type")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                old_value: fields.get("old_enabled").cloned(),
                new_value: fields.get("enabled").cloned(),
            }
        }
        PaperProofEventKind::ArtifactStatusChanged => {
            PaperProofDomainChange::ArtifactStatusChanged {
                series_id: string_field(fields, "series_id"),
                new_status: u64_field(fields, "new_status"),
            }
        }
        PaperProofEventKind::ProtocolPausedChanged => {
            PaperProofDomainChange::ProtocolPausedChanged {
                root_id: string_field(fields, "root_id"),
                new_paused: fields.get("new_paused").and_then(Value::as_bool),
            }
        }
        PaperProofEventKind::FeeRecipientChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "fee_recipient".to_string(),
                old_value: fields.get("old_fee_recipient").cloned(),
                new_value: fields.get("new_fee_recipient").cloned(),
            }
        }
        PaperProofEventKind::GovernanceAuthorityChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "governance_authority".to_string(),
                old_value: fields.get("old_governance_authority").cloned(),
                new_value: fields.get("new_governance_authority").cloned(),
            }
        }
        PaperProofEventKind::CommentsFeeLevelChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "comments_fee_level".to_string(),
                old_value: fields.get("old_level").cloned(),
                new_value: fields.get("new_level").cloned(),
            }
        }
        PaperProofEventKind::ArtifactFeeLevelChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: format!(
                    "artifact_fee_level_{}",
                    u64_field(fields, "artifact_type")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ),
                old_value: fields.get("old_fee_level").cloned(),
                new_value: fields.get("fee_level").cloned(),
            }
        }
        PaperProofEventKind::UpgradeAuthorityChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "upgrade_authority".to_string(),
                old_value: fields.get("old_upgrade_authority").cloned(),
                new_value: fields.get("new_upgrade_authority").cloned(),
            }
        }
        PaperProofEventKind::FeeCollected => PaperProofDomainChange::FeeCollected {
            registry_id: string_field(fields, "registry_id"),
            payer: string_field(fields, "payer"),
            recipient: string_field(fields, "recipient"),
            amount: u64_field(fields, "amount"),
        },
        PaperProofEventKind::DirectAuthorityModeChanged => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "direct_authority_mode".to_string(),
                old_value: fields.get("old_mode").cloned(),
                new_value: fields.get("new_mode").cloned(),
            }
        }
        PaperProofEventKind::OperatorNominated => PaperProofDomainChange::OwnerTransferred {
            series_id: None,
            tree_id: None,
            new_owner: string_field(fields, "new_operator"),
        },
        PaperProofEventKind::OperatorTransferCancelled => {
            PaperProofDomainChange::GovernanceParameterChanged {
                registry_id: string_field(fields, "registry_id"),
                parameter: "operator_transfer_cancelled".to_string(),
                old_value: fields.get("pending_operator").cloned(),
                new_value: None,
            }
        }
        PaperProofEventKind::ManagedUpgradeCapRegistered => {
            managed_upgrade_changed(fields, "registered")
        }
        PaperProofEventKind::ManagedUpgradeAuthorized => {
            managed_upgrade_changed(fields, "authorized")
        }
        PaperProofEventKind::ManagedUpgradeCommitted => {
            managed_upgrade_changed(fields, "committed")
        }
        PaperProofEventKind::GovernanceVaultMigrated => PaperProofDomainChange::ObjectMigrated {
            registry_id: string_field(fields, "registry_id"),
            object_id: string_field(fields, "governance_vault_id"),
            kind: "governance_vault".to_string(),
            new_version: u64_field(fields, "new_version"),
        },
        PaperProofEventKind::OwnerTransferred => PaperProofDomainChange::OwnerTransferred {
            series_id: string_field(fields, "series_id"),
            tree_id: string_field(fields, "tree_id"),
            new_owner: string_field(fields, "new_owner"),
        },
        PaperProofEventKind::Unknown => PaperProofDomainChange::Unknown,
    }
}

#[derive(Clone, Debug)]
pub struct PaperProofIndexerClient {
    pub query: PaperProofQueryClient,
}

impl PaperProofIndexerClient {
    pub fn new(query: PaperProofQueryClient) -> Self {
        Self { query }
    }

    pub fn mainnet() -> Self {
        Self::new(PaperProofQueryClient::mainnet())
    }

    pub fn canonical_module_filters(deployment: &Deployment) -> Vec<PackageModuleFilter> {
        vec![
            PackageModuleFilter {
                package_id: deployment.packages.publishing.clone(),
                module: "publishing".to_string(),
            },
            PackageModuleFilter {
                package_id: deployment.packages.comments.clone(),
                module: "comments".to_string(),
            },
            PackageModuleFilter {
                package_id: deployment.packages.governance.clone(),
                module: "governance_voting".to_string(),
            },
        ]
    }

    pub async fn scan_once(&self, options: IndexerScanOptions) -> Result<IndexerEventBatch> {
        #[cfg(feature = "tracing")]
        tracing::info!(
            canonical_only = options.canonical_only,
            "paperproof indexer scan_once started"
        );
        let page = self.query.query_events(options.filter).await?;
        let batch = indexer_batch_from_page(page, &self.query.deployment, options.canonical_only);
        emit_batch_metrics("event_query", &batch);
        Ok(batch)
    }

    pub async fn scan_module_once(
        &self,
        module: PackageModuleFilter,
        progress: Option<IndexerProgress>,
        limit: Option<u64>,
    ) -> Result<IndexerEventBatch> {
        self.scan_once(IndexerScanOptions {
            filter: EventQueryInput {
                package_id: Some(module.package_id),
                module: Some(module.module),
                pagination: PaginationInput {
                    cursor: progress.and_then(|progress| progress.cursor),
                    limit: limit.or(Some(50)),
                    descending_order: Some(false),
                },
                ..Default::default()
            },
            canonical_only: true,
        })
        .await
    }

    pub async fn scan_checkpoint_range_once<P>(
        &self,
        provider: &P,
        options: CheckpointScanOptions,
    ) -> Result<IndexerEventBatch>
    where
        P: CheckpointDataProvider,
    {
        #[cfg(feature = "tracing")]
        tracing::info!(
            start_checkpoint = options.start_checkpoint,
            limit = options.limit,
            canonical_only = options.canonical_only,
            "paperproof indexer checkpoint scan started"
        );
        let mut events = Vec::new();
        let mut raw_checkpoints = Vec::new();
        let mut last_checkpoint = options.start_checkpoint;
        for offset in 0..options.limit {
            let checkpoint = options.start_checkpoint + offset;
            let data = provider.get_checkpoint_data(checkpoint).await?;
            last_checkpoint = data.sequence_number;
            events.extend(data.events);
            raw_checkpoints.push(data.raw);
        }
        let page = EventPage {
            data: events,
            next_cursor: Some(json!({ "checkpoint": last_checkpoint + 1 })),
            has_next_page: true,
            raw: json!({
                "source": "checkpoint",
                "startCheckpoint": options.start_checkpoint,
                "nextCheckpoint": last_checkpoint + 1,
                "checkpoints": raw_checkpoints,
            }),
        };
        let batch = indexer_batch_from_page(page, &self.query.deployment, options.canonical_only);
        emit_batch_metrics("checkpoint", &batch);
        Ok(batch)
    }
}

pub fn indexer_batch_from_page(
    page: EventPage<SuiEventEnvelope>,
    deployment: &Deployment,
    canonical_only: bool,
) -> IndexerEventBatch {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut last_event_id = None;
    let mut last_timestamp_ms = None;
    for event in page.data {
        last_event_id = event.id.clone();
        last_timestamp_ms = event
            .timestamp_ms
            .as_deref()
            .and_then(|value| value.parse::<u64>().ok())
            .or(last_timestamp_ms);
        let trust = check_canonical_paperproof_event(&event, deployment);
        if trust.trusted || !canonical_only {
            let id = event_id(&event);
            accepted.push(IndexedPaperProofEvent {
                kind: parse_event(&event).kind,
                event,
                trust,
                id,
            });
        } else {
            rejected.push(RejectedPaperProofEvent {
                event,
                reason: trust
                    .reason
                    .unwrap_or_else(|| "event is not canonical".to_string()),
            });
        }
    }
    IndexerEventBatch {
        progress: IndexerProgress {
            cursor: page.next_cursor,
            has_next_page: page.has_next_page,
            scanned_pages: 1,
            scanned_events: (accepted.len() + rejected.len()) as u64,
            accepted_events: accepted.len() as u64,
            rejected_events: rejected.len() as u64,
            last_event_id,
            last_timestamp_ms,
        },
        accepted,
        rejected,
        raw: page.raw,
    }
}

pub fn event_kind_counts(events: &[IndexedPaperProofEvent]) -> BTreeMap<PaperProofEventKind, u64> {
    let mut counts = BTreeMap::new();
    for event in events {
        *counts.entry(event.kind.clone()).or_default() += 1;
    }
    counts
}

pub fn event_id(event: &SuiEventEnvelope) -> EventId {
    let (transaction_digest, event_seq) = event
        .id
        .as_ref()
        .map(parse_sui_event_id)
        .unwrap_or((None, None));
    let checkpoint = event
        .id
        .as_ref()
        .and_then(|id| json_u64_field(id, "checkpoint"))
        .or_else(|| json_u64_field(&event.parsed_json, "checkpoint"));
    EventId {
        checkpoint,
        transaction_digest,
        event_seq,
        package_id: event.package_id.clone(),
        module: event.transaction_module.clone(),
        event_type: event.event_type.clone(),
    }
}

fn parse_sui_event_id(value: &Value) -> (Option<String>, Option<u64>) {
    if let Some(text) = value.as_str() {
        let mut parts = text.split(':');
        return (
            parts.next().map(ToString::to_string),
            parts.next().and_then(|item| item.parse().ok()),
        );
    }
    let digest = value
        .get("txDigest")
        .or_else(|| value.get("transactionDigest"))
        .or_else(|| value.get("transaction_digest"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let seq = json_u64_field(value, "eventSeq").or_else(|| json_u64_field(value, "event_seq"));
    (digest, seq)
}

fn json_u64_field(value: &Value, key: &str) -> Option<u64> {
    value
        .get(key)
        .and_then(|item| item.as_u64().or_else(|| item.as_str()?.parse().ok()))
}

fn string_field(fields: &Value, field: &str) -> Option<String> {
    fields.get(field)?.as_str().map(ToString::to_string)
}

fn u64_field(fields: &Value, field: &str) -> Option<u64> {
    let value = fields.get(field)?;
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn governance_object_created(
    fields: &Value,
    object_field: &str,
    kind: &str,
) -> PaperProofDomainChange {
    if let (Some(registry_id), Some(object_id)) = (
        string_field(fields, "registry_id"),
        string_field(fields, object_field),
    ) {
        PaperProofDomainChange::GovernanceObjectCreated {
            registry_id,
            object_id,
            kind: kind.to_string(),
        }
    } else {
        PaperProofDomainChange::Unknown
    }
}

fn managed_upgrade_changed(fields: &Value, status: &str) -> PaperProofDomainChange {
    PaperProofDomainChange::ManagedUpgradeChanged {
        registry_id: string_field(fields, "registry_id"),
        package_id: string_field(fields, "package_id"),
        status: status.to_string(),
    }
}

fn emit_batch_metrics(source: &str, batch: &IndexerEventBatch) {
    #[cfg(feature = "tracing")]
    tracing::info!(
        source,
        scanned_events = batch.progress.scanned_events,
        accepted_events = batch.progress.accepted_events,
        rejected_events = batch.progress.rejected_events,
        has_next_page = batch.progress.has_next_page,
        "paperproof indexer batch"
    );
    let _ = (source, batch);
}
