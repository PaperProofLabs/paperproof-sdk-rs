// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

pub mod artifact_types {
    pub const PREPRINT: u8 = 1;
    pub const BLOG_POST: u8 = 2;
    pub const TECHNICAL_REPORT: u8 = 3;
    pub const DATASET: u8 = 4;
    pub const SOFTWARE_RELEASE: u8 = 5;
    pub const GENERIC_FILE: u8 = 6;
}

pub mod series_status {
    pub const ACTIVE: u8 = 0;
    pub const LOCKED: u8 = 1;
    pub const HIDDEN: u8 = 2;
}

pub mod tree_status {
    pub const OPEN: u8 = 0;
    pub const LOCKED: u8 = 1;
    pub const ARCHIVED: u8 = 2;
}

pub mod comment_mode {
    pub const ONCHAIN: u8 = 1;
    pub const BLOB: u8 = 2;
}

pub mod comment_status {
    pub const ACTIVE: u8 = 0;
    pub const HIDDEN: u8 = 1;
    pub const DELETED: u8 = 2;
}

pub mod controller_authority_mode {
    pub const LEGACY_OWNER_ONLY: u8 = 0;
    pub const DUAL_MODE: u8 = 1;
    pub const CONTROLLER_PRIMARY: u8 = 2;
    pub const CONTROLLER_ONLY: u8 = 3;
}

pub mod reserved_metadata_keys {
    pub const SERIES_DESCRIPTION: &str = "series_description";
    pub const VERSION_CHANGE_NOTE: &str = "version_change_note";
}

pub mod fee_level {
    pub const FREE: u8 = 0;
    pub const MICRO: u8 = 1;
    pub const LOW: u8 = 2;
    pub const STANDARD: u8 = 3;
    pub const HIGH: u8 = 4;
    pub const PREMIUM: u8 = 5;
}

pub mod governance {
    pub const PROPOSAL_TYPE_EXECUTABLE: u8 = 1;
    pub const PROPOSAL_TYPE_SIGNAL: u8 = 2;
    pub const ACTION_SET_COMMENTS_FEE_LEVEL: u64 = 2;
    pub const ACTION_SET_FEE_RECIPIENT: u64 = 3;
    pub const ACTION_NOMINATE_OPERATOR: u64 = 4;
    pub const ACTION_SET_PROPOSAL_CREATION_PAUSED: u64 = 5;
    pub const ACTION_SET_PROPOSER_THRESHOLD: u64 = 6;
    pub const ACTION_SET_UPGRADE_AUTHORITY: u64 = 7;
    pub const ACTION_SET_PROPOSAL_DURATION_EPOCHS: u64 = 8;
    pub const ACTION_SET_ARTIFACT_TYPE_ENABLED: u64 = 9;
    pub const ACTION_SET_ARTIFACT_FEE_LEVEL: u64 = 10;
    pub const ACTION_ACTIVATE_ARTIFACT_TYPE: u64 = 11;
    pub const ACTION_SET_GOVERNANCE_ACTION_ENABLED: u64 = 12;
    pub const ACTION_SET_DIRECT_AUTHORITY_MODE: u64 = 13;
    pub const ACTION_CANCEL_OPERATOR_TRANSFER: u64 = 14;
    pub const ACTION_SET_GOVERNANCE_AUTHORITY: u64 = 15;
    pub const ACTION_SIGNAL_TEXT: u64 = 101;
    pub const ACTION_SIGNAL_ADDRESS: u64 = 102;
    pub const ACTION_SIGNAL_OBJECT: u64 = 103;
    pub const STATUS_ACTIVE: u8 = 1;
    pub const STATUS_PASSED: u8 = 2;
    pub const STATUS_REJECTED: u8 = 3;
    pub const STATUS_EXECUTED: u8 = 4;
    pub const STATUS_EXPIRED: u8 = 5;
}

pub const PPRF_DECIMALS: u8 = 9;
pub const ONE_PPRF: u64 = 1_000_000_000;
pub const MIN_LIKE_BALANCE: u64 = ONE_PPRF;
pub const MIN_VOTE_STAKE: u64 = 100 * ONE_PPRF;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolLimits {
    pub max_versions_per_series: usize,
    pub max_metadata_attributes: usize,
    pub max_metadata_key_bytes: usize,
    pub max_metadata_value_bytes: usize,
    pub max_keywords: usize,
    pub max_authors: usize,
    pub max_tags: usize,
    pub max_title_bytes: usize,
    pub max_long_text_bytes: usize,
    pub max_medium_text_bytes: usize,
    pub max_short_text_bytes: usize,
    pub max_vector_item_bytes: usize,
    pub max_content_hash_bytes: usize,
    pub max_walrus_blob_id_bytes: usize,
    pub max_walrus_blob_object_id_bytes: usize,
    pub max_content_type_bytes: usize,
    pub max_onchain_comment_bytes_default: usize,
    pub max_blob_id_bytes: usize,
    pub max_blob_digest_bytes: usize,
    pub max_content_preview_bytes: usize,
    pub max_proposal_title_bytes: usize,
    pub max_proposal_description_bytes: usize,
    pub min_proposal_duration_epochs: u64,
    pub max_proposal_duration_epochs: u64,
    pub execution_validity_epochs: u64,
}

pub const PROTOCOL_LIMITS: ProtocolLimits = ProtocolLimits {
    max_versions_per_series: 168,
    max_metadata_attributes: 4,
    max_metadata_key_bytes: 64,
    max_metadata_value_bytes: 511,
    max_keywords: 10,
    max_authors: 20,
    max_tags: 20,
    max_title_bytes: 256,
    max_long_text_bytes: 4096,
    max_medium_text_bytes: 1024,
    max_short_text_bytes: 256,
    max_vector_item_bytes: 128,
    max_content_hash_bytes: 128,
    max_walrus_blob_id_bytes: 128,
    max_walrus_blob_object_id_bytes: 128,
    max_content_type_bytes: 64,
    max_onchain_comment_bytes_default: 512,
    max_blob_id_bytes: 128,
    max_blob_digest_bytes: 128,
    max_content_preview_bytes: 256,
    max_proposal_title_bytes: 256,
    max_proposal_description_bytes: 4096,
    min_proposal_duration_epochs: 7,
    max_proposal_duration_epochs: 14,
    execution_validity_epochs: 3,
};
