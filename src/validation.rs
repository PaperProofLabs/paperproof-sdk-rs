// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use crate::{
    constants::{PROTOCOL_LIMITS, comment_status, tree_status},
    error::{PaperProofError, Result},
    types::{
        AddBlobCommentInput, AddOnchainCommentInput, BlogPostInput, CommonContentInput,
        CreateExecutableProposalInput, CreateSignalProposalInput, DatasetInput, GenericFileInput,
        MetadataAttribute, PreprintInput, SoftwareReleaseInput, TechnicalReportInput,
    },
};

pub fn validate_address(value: &str) -> Result<()> {
    validate_hex_id(value, "address").map_err(|e| match e {
        PaperProofError::InvalidObjectId { value, message } => {
            PaperProofError::InvalidAddress { value, message }
        }
        other => other,
    })
}

pub fn validate_object_id(value: &str) -> Result<()> {
    validate_hex_id(value, "object id")
}

pub fn validate_package_id(value: &str) -> Result<()> {
    validate_hex_id(value, "package id").map_err(|e| match e {
        PaperProofError::InvalidObjectId { value, message } => {
            PaperProofError::InvalidPackageId { value, message }
        }
        other => other,
    })
}

fn validate_hex_id(value: &str, kind: &str) -> Result<()> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    if raw.is_empty() {
        return Err(PaperProofError::InvalidObjectId {
            value: value.to_string(),
            message: format!("{kind} must not be empty"),
        });
    }
    if raw.len() > 64 {
        return Err(PaperProofError::InvalidObjectId {
            value: value.to_string(),
            message: format!("{kind} must be at most 32 bytes"),
        });
    }
    if !raw.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(PaperProofError::InvalidObjectId {
            value: value.to_string(),
            message: format!("{kind} must be hex encoded"),
        });
    }
    Ok(())
}

pub fn validate_required_bytes(field: &str, value: impl AsRef<[u8]>, max: usize) -> Result<()> {
    let bytes = value.as_ref();
    if bytes.is_empty() {
        return Err(PaperProofError::invalid_input(field, "must not be empty"));
    }
    if bytes.len() > max {
        return Err(PaperProofError::invalid_input(
            field,
            format!("must be at most {max} bytes"),
        ));
    }
    Ok(())
}

pub fn validate_required_text(field: &str, value: &str, max: usize) -> Result<()> {
    validate_required_bytes(field, value.as_bytes(), max)
}

pub fn validate_text_vector(
    field: &str,
    values: &[String],
    max_count: usize,
    max_item_bytes: usize,
) -> Result<()> {
    if values.is_empty() {
        return Err(PaperProofError::invalid_input(field, "must not be empty"));
    }
    if values.len() > max_count {
        return Err(PaperProofError::invalid_input(
            field,
            format!("must contain at most {max_count} items"),
        ));
    }
    for (index, value) in values.iter().enumerate() {
        validate_required_text(&format!("{field}[{index}]"), value, max_item_bytes)?;
    }
    Ok(())
}

pub fn validate_metadata_attributes(values: &[MetadataAttribute]) -> Result<()> {
    if values.len() > PROTOCOL_LIMITS.max_metadata_attributes {
        return Err(PaperProofError::invalid_input(
            "metadata",
            format!(
                "must contain at most {} attributes",
                PROTOCOL_LIMITS.max_metadata_attributes
            ),
        ));
    }
    let mut keys = HashSet::new();
    for attribute in values {
        validate_required_text(
            "metadata.key",
            &attribute.key,
            PROTOCOL_LIMITS.max_metadata_key_bytes,
        )?;
        validate_required_text(
            "metadata.value",
            &attribute.value,
            PROTOCOL_LIMITS.max_metadata_value_bytes,
        )?;
        if !keys.insert(attribute.key.clone()) {
            return Err(PaperProofError::invalid_input(
                "metadata.key",
                format!("duplicate key `{}`", attribute.key),
            ));
        }
    }
    Ok(())
}

pub fn validate_common_content(input: &CommonContentInput) -> Result<()> {
    validate_required_text(
        "content_hash",
        &input.content_hash,
        PROTOCOL_LIMITS.max_content_hash_bytes,
    )?;
    validate_required_text(
        "walrus_blob_id",
        &input.walrus_blob_id,
        PROTOCOL_LIMITS.max_walrus_blob_id_bytes,
    )?;
    validate_required_text(
        "walrus_blob_object_id",
        &input.walrus_blob_object_id,
        PROTOCOL_LIMITS.max_walrus_blob_object_id_bytes,
    )?;
    validate_required_text(
        "content_type",
        &input.content_type,
        PROTOCOL_LIMITS.max_content_type_bytes,
    )?;
    Ok(())
}

pub fn validate_preprint_input(input: &PreprintInput) -> Result<()> {
    validate_required_text("title", &input.title, PROTOCOL_LIMITS.max_title_bytes)?;
    validate_required_text(
        "abstract_text",
        &input.abstract_text,
        PROTOCOL_LIMITS.max_long_text_bytes,
    )?;
    validate_text_vector(
        "authors",
        &input.authors,
        PROTOCOL_LIMITS.max_authors,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_text_vector(
        "keywords",
        &input.keywords,
        PROTOCOL_LIMITS.max_keywords,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_required_text("field", &input.field, PROTOCOL_LIMITS.max_short_text_bytes)?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_blog_post_input(input: &BlogPostInput) -> Result<()> {
    validate_required_text("title", &input.title, PROTOCOL_LIMITS.max_title_bytes)?;
    validate_required_text(
        "summary",
        &input.summary,
        PROTOCOL_LIMITS.max_medium_text_bytes,
    )?;
    validate_required_text(
        "author_name",
        &input.author_name,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_text_vector(
        "tags",
        &input.tags,
        PROTOCOL_LIMITS.max_tags,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_technical_report_input(input: &TechnicalReportInput) -> Result<()> {
    validate_required_text("title", &input.title, PROTOCOL_LIMITS.max_title_bytes)?;
    validate_required_text(
        "abstract_text",
        &input.abstract_text,
        PROTOCOL_LIMITS.max_long_text_bytes,
    )?;
    validate_text_vector(
        "authors",
        &input.authors,
        PROTOCOL_LIMITS.max_authors,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_required_text(
        "organization",
        &input.organization,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text(
        "report_number",
        &input.report_number,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text("field", &input.field, PROTOCOL_LIMITS.max_short_text_bytes)?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_dataset_input(input: &DatasetInput) -> Result<()> {
    validate_required_text("title", &input.title, PROTOCOL_LIMITS.max_title_bytes)?;
    validate_required_text(
        "description",
        &input.description,
        PROTOCOL_LIMITS.max_long_text_bytes,
    )?;
    validate_text_vector(
        "authors",
        &input.authors,
        PROTOCOL_LIMITS.max_authors,
        PROTOCOL_LIMITS.max_vector_item_bytes,
    )?;
    validate_required_text("field", &input.field, PROTOCOL_LIMITS.max_short_text_bytes)?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text(
        "schema_hash",
        &input.schema_hash,
        PROTOCOL_LIMITS.max_content_hash_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_software_release_input(input: &SoftwareReleaseInput) -> Result<()> {
    validate_required_text(
        "project_name",
        &input.project_name,
        PROTOCOL_LIMITS.max_title_bytes,
    )?;
    validate_required_text(
        "version_name",
        &input.version_name,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text(
        "source_hash",
        &input.source_hash,
        PROTOCOL_LIMITS.max_content_hash_bytes,
    )?;
    validate_required_text(
        "package_hash",
        &input.package_hash,
        PROTOCOL_LIMITS.max_content_hash_bytes,
    )?;
    validate_required_text(
        "changelog",
        &input.changelog,
        PROTOCOL_LIMITS.max_long_text_bytes,
    )?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text(
        "repository_url",
        &input.repository_url,
        PROTOCOL_LIMITS.max_medium_text_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_generic_file_input(input: &GenericFileInput) -> Result<()> {
    validate_required_text("title", &input.title, PROTOCOL_LIMITS.max_title_bytes)?;
    validate_required_text(
        "description",
        &input.description,
        PROTOCOL_LIMITS.max_long_text_bytes,
    )?;
    validate_required_text(
        "filename",
        &input.filename,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_required_text(
        "license",
        &input.license,
        PROTOCOL_LIMITS.max_short_text_bytes,
    )?;
    validate_common_content(&input.content)?;
    validate_metadata_attributes(&input.series_metadata)?;
    validate_metadata_attributes(&input.version_metadata)
}

pub fn validate_onchain_comment(input: &AddOnchainCommentInput) -> Result<()> {
    validate_object_id(&input.tree_id)?;
    validate_required_bytes(
        "content",
        &input.content,
        PROTOCOL_LIMITS.max_onchain_comment_bytes_default,
    )
}

pub fn validate_blob_comment(input: &AddBlobCommentInput) -> Result<()> {
    validate_object_id(&input.tree_id)?;
    validate_required_bytes("blob_id", &input.blob_id, PROTOCOL_LIMITS.max_blob_id_bytes)?;
    if let Some(id) = &input.blob_object_id {
        validate_object_id(id)?;
    }
    validate_required_bytes(
        "blob_digest",
        &input.blob_digest,
        PROTOCOL_LIMITS.max_blob_digest_bytes,
    )?;
    validate_required_bytes(
        "preview",
        &input.preview,
        PROTOCOL_LIMITS.max_content_preview_bytes,
    )
}

pub fn validate_comment_status(status: u8) -> Result<()> {
    if matches!(
        status,
        comment_status::ACTIVE | comment_status::HIDDEN | comment_status::DELETED
    ) {
        Ok(())
    } else {
        Err(PaperProofError::invalid_input(
            "status",
            "must be active, hidden, or deleted",
        ))
    }
}

pub fn validate_tree_status(status: u8) -> Result<()> {
    if matches!(
        status,
        tree_status::OPEN | tree_status::LOCKED | tree_status::ARCHIVED
    ) {
        Ok(())
    } else {
        Err(PaperProofError::invalid_input(
            "status",
            "must be open, locked, or archived",
        ))
    }
}

pub fn validate_signal_proposal(input: &CreateSignalProposalInput) -> Result<()> {
    validate_proposal_text(&input.title, &input.description)?;
    if let Some(address) = &input.payload_address {
        validate_address(address)?;
    }
    validate_object_id(&input.stake_coin_id)
}

pub fn validate_executable_proposal(input: &CreateExecutableProposalInput) -> Result<()> {
    validate_proposal_text(&input.title, &input.description)?;
    if input.action_type > u8::MAX as u64 {
        return Err(PaperProofError::invalid_input(
            "action_type",
            "must fit in a u8 Move argument",
        ));
    }
    if let Some(address) = &input.payload_address {
        validate_address(address)?;
    }
    if let Some(object_id) = &input.payload_object_id {
        validate_object_id(object_id)?;
    }
    if input.payload_bytes.len() > PROTOCOL_LIMITS.max_proposal_description_bytes {
        return Err(PaperProofError::invalid_input(
            "payload_bytes",
            format!(
                "must be at most {} bytes",
                PROTOCOL_LIMITS.max_proposal_description_bytes
            ),
        ));
    }
    validate_object_id(&input.stake_coin_id)
}

fn validate_proposal_text(title: &str, description: &str) -> Result<()> {
    validate_required_text("title", title, PROTOCOL_LIMITS.max_proposal_title_bytes)?;
    validate_required_text(
        "description",
        description,
        PROTOCOL_LIMITS.max_proposal_description_bytes,
    )
}
