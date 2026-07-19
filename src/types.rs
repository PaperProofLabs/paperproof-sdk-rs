// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MetadataAttribute {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CommonContentInput {
    pub content_hash: String,
    pub walrus_blob_id: String,
    pub walrus_blob_object_id: String,
    pub content_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PreprintInput {
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub keywords: Vec<String>,
    pub field: String,
    pub license: String,
    pub page_count: u64,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct BlogPostInput {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub language: String,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TechnicalReportInput {
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub organization: String,
    pub report_number: String,
    pub keywords: Vec<String>,
    pub license: String,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DatasetInput {
    pub title: String,
    pub description: String,
    pub format: String,
    pub file_count: u64,
    pub size_bytes: u64,
    pub license: String,
    pub keywords: Vec<String>,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SoftwareReleaseInput {
    pub project_name: String,
    pub version_name: String,
    pub source_hash: String,
    pub package_hash: String,
    pub changelog: String,
    pub license: String,
    pub repository_url: String,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct GenericFileInput {
    pub title: String,
    pub description: String,
    pub filename: String,
    pub file_size: u64,
    pub license: String,
    pub content: CommonContentInput,
    pub series_description: Option<String>,
    pub version_change_note: Option<String>,
    pub series_metadata: Vec<MetadataAttribute>,
    pub version_metadata: Vec<MetadataAttribute>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AddVersionInput<T> {
    pub series_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub body: T,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ControllerBoundInput {
    pub control_record_id: String,
    pub controller_nft_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ControllerSeriesBoundInput {
    pub series_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ControllerSeriesAndTreeBoundInput {
    pub series_id: String,
    pub comments_tree_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AddOnchainCommentInput {
    pub tree_id: String,
    pub parent_comment_id: u64,
    pub content: Vec<u8>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AddBlobCommentInput {
    pub tree_id: String,
    pub parent_comment_id: u64,
    pub blob_id: Vec<u8>,
    pub blob_object_id: Option<String>,
    pub blob_digest: Vec<u8>,
    pub preview: Vec<u8>,
    pub payment_coin_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SetCommentStatusInput {
    pub tree_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub comment_id: u64,
    pub status: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct SetTreeStatusInput {
    pub tree_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub status: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransferArtifactOwnerInput {
    pub series_id: String,
    pub comments_tree_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub new_owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransferTreeOwnerInput {
    pub tree_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub new_owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct UpdateSeriesMetadataInput {
    pub series_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub metadata: Vec<MetadataAttribute>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct UpdateSeriesDescriptionInput {
    pub series_id: String,
    pub control_record_id: String,
    pub controller_nft_id: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateSignalProposalInput {
    pub title: String,
    pub description: String,
    pub action_type: u64,
    pub payload_text: Option<String>,
    pub payload_address: Option<String>,
    pub stake_coin_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreateExecutableProposalInput {
    pub proposal_type: Option<u8>,
    pub action_type: u64,
    pub title: String,
    pub description: String,
    pub payload_u64_1: Option<u64>,
    pub payload_u64_2: Option<u64>,
    pub payload_address: Option<String>,
    pub payload_object_id: Option<String>,
    pub payload_bytes: Vec<u8>,
    pub stake_coin_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VoteInput {
    pub proposal_id: String,
    pub coin_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecodedObject {
    pub id: String,
    pub object_type: String,
    pub owner: Option<Value>,
    pub fields: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PaperProofRootView {
    pub id: String,
    pub version: Option<u64>,
    pub paused: Option<bool>,
    pub governance_vault_id: Option<String>,
    pub fee_manager_id: Option<String>,
    pub type_registry_id: Option<String>,
    pub comments_tree_factory_cap_registry_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArtifactSeriesView {
    pub id: String,
    pub artifact_type: Option<u8>,
    pub artifact_code: Option<String>,
    pub owner: Option<String>,
    pub current_version: Option<u64>,
    pub current_version_id: Option<String>,
    pub comments_tree_id: Option<String>,
    pub likes_book_id: Option<String>,
    pub status: Option<u8>,
    pub ui_status: Option<u8>,
    pub series_description: Option<String>,
    pub series_control_enabled: Option<bool>,
    pub series_authority_mode: Option<u64>,
    pub series_authority_mode_name: Option<String>,
    pub series_control_record_id: Option<String>,
    pub series_controller_nft_id: Option<String>,
    pub metadata_extensions: Vec<MetadataAttribute>,
    pub version_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArtifactVersionView {
    pub id: String,
    pub series_id: Option<String>,
    pub artifact_type: Option<u8>,
    pub version: Option<u64>,
    pub content_hash: Option<String>,
    pub version_change_note: Option<String>,
    pub metadata_extensions: Vec<MetadataAttribute>,
    pub raw_fields: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommentsTreeView {
    pub id: String,
    pub version: Option<u64>,
    pub creator: Option<String>,
    pub owner: Option<String>,
    pub registry_id: Option<String>,
    pub governance_vault_id: Option<String>,
    pub fee_manager_id: Option<String>,
    pub target_key: Option<String>,
    pub target_series_id: Option<String>,
    pub target_artifact_type: Option<u8>,
    pub root_comment_id: Option<u64>,
    pub next_comment_id: Option<u64>,
    pub total_comments: Option<u64>,
    pub status: Option<u8>,
    pub max_onchain_comment_bytes: Option<u64>,
    pub max_comment_depth: Option<u8>,
    pub likes_book_id: Option<String>,
    pub tree_control_enabled: Option<bool>,
    pub tree_authority_mode: Option<u64>,
    pub tree_authority_mode_name: Option<String>,
    pub tree_control_record_id: Option<String>,
    pub tree_controller_nft_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ControllerNFTView {
    pub id: String,
    pub version: Option<u64>,
    pub series_id: Option<String>,
    pub artifact_code: Option<String>,
    pub artifact_type_name: Option<String>,
    pub control_right: Option<String>,
    pub authority_mode_name: Option<String>,
    pub image_url: Option<String>,
    pub artifact_type: Option<u8>,
    pub control_record_id: Option<String>,
    pub issued_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ArtifactControlRecordView {
    pub id: String,
    pub version: Option<u64>,
    pub series_id: Option<String>,
    pub comments_tree_id: Option<String>,
    pub artifact_type: Option<u8>,
    pub controller_nft_id: Option<String>,
    pub authority_mode: Option<u64>,
    pub authority_mode_name: Option<String>,
    pub transfer_locked: Option<bool>,
    pub created_at_ms: Option<u64>,
    pub updated_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ControllerStateSnapshot {
    pub control_enabled: bool,
    pub authority_mode: Option<u64>,
    pub authority_mode_name: Option<String>,
    pub control_record_id: Option<String>,
    pub controller_nft_id: Option<String>,
    pub controller_holder: Option<String>,
    pub transfer_locked: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SeriesControlSnapshot {
    pub control_enabled: bool,
    pub series_id: String,
    pub tree_id: Option<String>,
    pub authority_mode: Option<u64>,
    pub authority_mode_name: Option<String>,
    pub control_record_id: Option<String>,
    pub controller_nft_id: Option<String>,
    pub controller_holder: Option<String>,
    pub transfer_locked: Option<bool>,
    pub series_owner: Option<String>,
    pub tree_owner: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommentNodeView {
    pub comment_id: Option<u64>,
    pub parent_comment_id: Option<u64>,
    pub author: Option<String>,
    pub depth: Option<u8>,
    pub content_mode: Option<u8>,
    pub inline_content: Vec<u8>,
    pub content_preview: Vec<u8>,
    pub blob_id: Vec<u8>,
    pub blob_object_id: Option<String>,
    pub blob_digest: Vec<u8>,
    pub children_count: Option<u64>,
    pub created_at_ms: Option<u64>,
    pub edited_at_ms: Option<u64>,
    pub status: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LikesBookView {
    pub id: String,
    pub version: Option<u64>,
    pub registry_id: Option<String>,
    pub comments_tree_id: Option<String>,
    pub target_series_id: Option<String>,
    pub target_artifact_type: Option<u8>,
    pub like_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProposalView {
    pub id: String,
    pub version: Option<u64>,
    pub registry_id: Option<String>,
    pub proposal_id: Option<u64>,
    pub proposer: Option<String>,
    pub proposal_type: Option<u8>,
    pub action_type: Option<u8>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub payload_u64_1: Option<u64>,
    pub payload_u64_2: Option<u64>,
    pub payload_address: Option<String>,
    pub payload_object_id: Option<String>,
    pub yes_votes: Option<u64>,
    pub no_votes: Option<u64>,
    pub status: Option<u8>,
    pub executed: Option<bool>,
    pub start_epoch: Option<u64>,
    pub end_epoch: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GovernanceConfigView {
    pub id: String,
    pub version: Option<u64>,
    pub registry_id: Option<String>,
    pub total_supply: Option<u64>,
    pub proposer_threshold: Option<u64>,
    pub proposal_duration_epochs: Option<u64>,
    pub next_proposal_id: Option<u64>,
    pub proposal_creation_paused: Option<bool>,
    pub active_proposal_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GovernanceVaultView {
    pub id: String,
    pub version: Option<u64>,
    pub registry_id: Option<String>,
    pub governance_config_id: Option<String>,
    pub governance_authority: Option<String>,
    pub upgrade_authority: Option<String>,
    pub active_operator: Option<String>,
    pub fee_recipient: Option<String>,
    pub direct_authority_mode: Option<u8>,
    pub direct_authority_permanently_disabled: Option<bool>,
    pub has_pending_operator_transfer: Option<bool>,
    pub pending_operator: Option<String>,
    pub pending_operator_epoch: Option<u64>,
    pub pending_operator_wrapper_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeeManagerView {
    pub id: String,
    pub version: Option<u64>,
    pub registry_id: Option<String>,
    pub comments_fee_level: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionResult {
    pub digest: String,
    pub confirmed_local_execution: Option<bool>,
}
