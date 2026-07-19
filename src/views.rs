// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde_json::Value;

use crate::{
    constants::{controller_authority_mode, reserved_metadata_keys},
    error::Result,
    types::{
        ArtifactControlRecordView, ArtifactSeriesView, ArtifactVersionView, CommentNodeView,
        CommentsTreeView, ControllerNFTView, DecodedObject, FeeManagerView,
        GovernanceConfigView, GovernanceVaultView, LikesBookView, MetadataAttribute,
        PaperProofRootView, ProposalView,
    },
};

pub fn decode_sui_object(value: &Value) -> Result<Option<DecodedObject>> {
    let Some(data) = value.get("data") else {
        return Ok(None);
    };
    let Some(content) = data.get("content") else {
        return Ok(None);
    };
    let Some(object_type) = content.get("type").and_then(Value::as_str) else {
        return Ok(None);
    };
    let id = data
        .get("objectId")
        .or_else(|| data.get("object_id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let fields = content.get("fields").cloned().unwrap_or(Value::Null);
    Ok(Some(DecodedObject {
        id,
        object_type: object_type.to_string(),
        owner: data.get("owner").cloned(),
        fields,
    }))
}

pub fn object_field_string(object: &DecodedObject, field: &str) -> Option<String> {
    object
        .fields
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub fn object_field_u64(object: &DecodedObject, field: &str) -> Option<u64> {
    object
        .fields
        .get(field)
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
}

pub fn id_value(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(text) = value.pointer("/fields/id").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    if let Some(text) = value.pointer("/fields/id/id").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    if let Some(bytes) = value.pointer("/fields/bytes").and_then(Value::as_array) {
        let hex = bytes
            .iter()
            .filter_map(|byte| byte.as_u64())
            .map(|byte| format!("{:02x}", byte as u8))
            .collect::<String>();
        if !hex.is_empty() {
            return Some(format!("0x{}", hex));
        }
    }
    if let Some(bytes) = numeric_key_bytes(value)
        && !bytes.is_empty()
    {
        return Some(format!("0x{}", hex::encode(bytes)));
    }
    value
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub fn u64_value(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

pub fn u8_value(value: &Value) -> Option<u8> {
    u64_value(value).and_then(|number| u8::try_from(number).ok())
}

pub fn bool_value(value: &Value) -> Option<bool> {
    value.as_bool()
}

pub fn string_value(value: &Value) -> Option<String> {
    value.as_str().map(ToString::to_string)
}

pub fn bytes_value(value: &Value) -> Vec<u8> {
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .filter_map(|item| item.as_u64().map(|number| number as u8))
            .collect();
    }
    if let Some(items) = value.pointer("/fields/contents").and_then(Value::as_array) {
        return items
            .iter()
            .filter_map(|item| item.as_u64().map(|number| number as u8))
            .collect();
    }
    if let Some(items) = value.pointer("/fields/bytes").and_then(Value::as_array) {
        return items
            .iter()
            .filter_map(|item| item.as_u64().map(|number| number as u8))
            .collect();
    }
    if let Some(bytes) = numeric_key_bytes(value) {
        return bytes;
    }
    Vec::new()
}

fn numeric_key_bytes(value: &Value) -> Option<Vec<u8>> {
    let object = value.as_object()?;
    let mut entries = object
        .iter()
        .filter_map(|(key, byte)| Some((key.parse::<usize>().ok()?, byte.as_u64()? as u8)))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return None;
    }
    entries.sort_by_key(|(index, _)| *index);
    Some(entries.into_iter().map(|(_, byte)| byte).collect())
}

pub fn parse_option_field(value: &Value) -> Option<&Value> {
    if value.is_null() {
        return None;
    }
    if let Some(vec) = value.get("vec").and_then(Value::as_array) {
        return vec.first();
    }
    if let Some(vec) = value.pointer("/fields/vec").and_then(Value::as_array) {
        return vec.first();
    }
    Some(value)
}

pub fn vector_items(value: &Value) -> Vec<&Value> {
    if let Some(items) = value.as_array() {
        return items.iter().collect();
    }
    if let Some(items) = value.get("contents").and_then(Value::as_array) {
        return items.iter().collect();
    }
    if let Some(items) = value.get("vec").and_then(Value::as_array) {
        return items.iter().collect();
    }
    if let Some(items) = value.pointer("/fields/contents").and_then(Value::as_array) {
        return items.iter().collect();
    }
    if let Some(items) = value.pointer("/fields/vec").and_then(Value::as_array) {
        return items.iter().collect();
    }
    Vec::new()
}

pub fn parse_metadata_attributes(value: &Value) -> Vec<MetadataAttribute> {
    vector_items(value)
        .into_iter()
        .map(|item| {
            let fields = item.get("fields").unwrap_or(item);
            MetadataAttribute {
                key: fields
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                value: fields
                    .get("value")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }
        })
        .collect()
}

pub fn metadata_value(items: &[MetadataAttribute], key: &str) -> Option<String> {
    items.iter()
        .find(|item| item.key == key)
        .map(|item| item.value.clone())
}

pub fn authority_mode_name(value: Option<u64>) -> Option<String> {
    match value {
        Some(number) if number == controller_authority_mode::LEGACY_OWNER_ONLY as u64 => {
            Some("legacy_owner_only".to_string())
        }
        Some(number) if number == controller_authority_mode::DUAL_MODE as u64 => {
            Some("dual_mode".to_string())
        }
        Some(number) if number == controller_authority_mode::CONTROLLER_PRIMARY as u64 => {
            Some("controller_primary".to_string())
        }
        Some(number) if number == controller_authority_mode::CONTROLLER_ONLY as u64 => {
            Some("controller_only".to_string())
        }
        _ => None,
    }
}

pub fn parse_id_vector(value: &Value) -> Vec<String> {
    vector_items(value)
        .into_iter()
        .filter_map(id_value)
        .collect()
}

pub fn table_id(value: &Value) -> Option<String> {
    id_value(value)
        .or_else(|| value.pointer("/fields/id").and_then(id_value))
        .or_else(|| {
            value
                .pointer("/fields/id/id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
}

pub fn view_root(object: &DecodedObject) -> PaperProofRootView {
    let f = &object.fields;
    PaperProofRootView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        paused: f.get("paused").and_then(bool_value),
        governance_vault_id: f.get("governance_vault_id").and_then(id_value),
        fee_manager_id: f.get("fee_manager_id").and_then(id_value),
        type_registry_id: f.get("type_registry_id").and_then(id_value),
        comments_tree_factory_cap_registry_id: f
            .get("comments_tree_factory_cap_registry_id")
            .and_then(id_value),
    }
}

pub fn view_series(object: &DecodedObject) -> ArtifactSeriesView {
    let f = &object.fields;
    let metadata_extensions = f
        .get("metadata_extensions")
        .map(parse_metadata_attributes)
        .unwrap_or_default();
    ArtifactSeriesView {
        id: object.id.clone(),
        artifact_type: f.get("artifact_type").and_then(u8_value),
        artifact_code: f.get("artifact_code").and_then(string_value),
        owner: f.get("owner").and_then(string_value),
        current_version: f.get("current_version").and_then(u64_value),
        current_version_id: f.get("current_version_id").and_then(id_value),
        comments_tree_id: f.get("comments_tree_id").and_then(id_value),
        likes_book_id: f.get("likes_book_id").and_then(id_value),
        status: f.get("status").and_then(u8_value),
        ui_status: f.get("ui_status").and_then(u8_value),
        series_description: metadata_value(
            &metadata_extensions,
            reserved_metadata_keys::SERIES_DESCRIPTION,
        ),
        series_control_enabled: None,
        series_authority_mode: None,
        series_authority_mode_name: None,
        series_control_record_id: None,
        series_controller_nft_id: None,
        metadata_extensions,
        version_ids: f
            .get("version_ids")
            .map(parse_id_vector)
            .unwrap_or_default(),
    }
}

pub fn view_version(object: &DecodedObject) -> ArtifactVersionView {
    let f = &object.fields;
    let header = f
        .get("header")
        .and_then(|value| value.get("fields"))
        .or_else(|| f.get("header"));
    let header = header.unwrap_or(&Value::Null);
    let metadata_extensions = header
        .get("metadata_extensions")
        .map(parse_metadata_attributes)
        .unwrap_or_default();
    ArtifactVersionView {
        id: object.id.clone(),
        series_id: header.get("series_id").and_then(id_value),
        artifact_type: header.get("artifact_type").and_then(u8_value),
        version: header.get("version").and_then(u64_value),
        content_hash: header.get("content_hash").and_then(string_value),
        version_change_note: metadata_value(
            &metadata_extensions,
            reserved_metadata_keys::VERSION_CHANGE_NOTE,
        ),
        metadata_extensions,
        raw_fields: f.clone(),
    }
}

pub fn view_comments_tree(object: &DecodedObject) -> CommentsTreeView {
    let f = &object.fields;
    CommentsTreeView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        creator: f.get("creator").and_then(string_value),
        owner: f.get("owner").and_then(string_value),
        registry_id: f.get("registry_id").and_then(id_value),
        governance_vault_id: f.get("governance_vault_id").and_then(id_value),
        fee_manager_id: f.get("fee_manager_id").and_then(id_value),
        target_key: f.get("target_key").and_then(string_value),
        target_series_id: f.get("target_series_id").and_then(id_value),
        target_artifact_type: f.get("target_artifact_type").and_then(u8_value),
        root_comment_id: f.get("root_comment_id").and_then(u64_value),
        next_comment_id: f.get("next_comment_id").and_then(u64_value),
        total_comments: f.get("total_comments").and_then(u64_value),
        status: f.get("status").and_then(u8_value),
        max_onchain_comment_bytes: f.get("max_onchain_comment_bytes").and_then(u64_value),
        max_comment_depth: f.get("max_comment_depth").and_then(u8_value),
        likes_book_id: f.get("likes_book_id").and_then(id_value),
        tree_control_enabled: None,
        tree_authority_mode: None,
        tree_authority_mode_name: None,
        tree_control_record_id: None,
        tree_controller_nft_id: None,
    }
}

pub fn view_controller_nft(object: &DecodedObject) -> ControllerNFTView {
    let f = &object.fields;
    ControllerNFTView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        series_id: f.get("series_id").and_then(id_value),
        artifact_code: f.get("artifact_code").and_then(string_value),
        artifact_type_name: f.get("artifact_type_name").and_then(string_value),
        control_right: f.get("control_right").and_then(string_value),
        authority_mode_name: f.get("authority_mode_name").and_then(string_value),
        image_url: f.get("image_url").and_then(string_value),
        artifact_type: f.get("artifact_type").and_then(u8_value),
        control_record_id: f.get("control_record_id").and_then(id_value),
        issued_at_ms: f.get("issued_at_ms").and_then(u64_value),
    }
}

pub fn view_artifact_control_record(object: &DecodedObject) -> ArtifactControlRecordView {
    let f = &object.fields;
    let authority_mode = f.get("authority_mode").and_then(u64_value);
    ArtifactControlRecordView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        series_id: f.get("series_id").and_then(id_value),
        comments_tree_id: f.get("comments_tree_id").and_then(id_value),
        artifact_type: f.get("artifact_type").and_then(u8_value),
        controller_nft_id: f.get("controller_nft_id").and_then(id_value),
        authority_mode,
        authority_mode_name: authority_mode_name(authority_mode),
        transfer_locked: f.get("transfer_locked").and_then(bool_value),
        created_at_ms: f.get("created_at_ms").and_then(u64_value),
        updated_at_ms: f.get("updated_at_ms").and_then(u64_value),
    }
}

pub fn view_comment_node(fields_like: &Value) -> CommentNodeView {
    let f = fields_like.get("fields").unwrap_or(fields_like);
    let parent = f.get("parent_comment_id").and_then(parse_option_field);
    let edited = f.get("edited_at_ms").and_then(parse_option_field);
    CommentNodeView {
        comment_id: f.get("comment_id").and_then(u64_value),
        parent_comment_id: parent.and_then(u64_value),
        author: f.get("author").and_then(string_value),
        depth: f.get("depth").and_then(u8_value),
        content_mode: f.get("content_mode").and_then(u8_value),
        inline_content: f.get("inline_content").map(bytes_value).unwrap_or_default(),
        content_preview: f
            .get("content_preview")
            .map(bytes_value)
            .unwrap_or_default(),
        blob_id: f.get("blob_id").map(bytes_value).unwrap_or_default(),
        blob_object_id: f
            .get("blob_object_id")
            .and_then(parse_option_field)
            .and_then(id_value),
        blob_digest: f.get("blob_digest").map(bytes_value).unwrap_or_default(),
        children_count: f.get("children_count").and_then(u64_value),
        created_at_ms: f.get("created_at_ms").and_then(u64_value),
        edited_at_ms: edited.and_then(u64_value),
        status: f.get("status").and_then(u8_value),
    }
}

pub fn view_likes_book(object: &DecodedObject) -> LikesBookView {
    let f = &object.fields;
    LikesBookView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        registry_id: f.get("registry_id").and_then(id_value),
        comments_tree_id: f.get("comments_tree_id").and_then(id_value),
        target_series_id: f.get("target_series_id").and_then(id_value),
        target_artifact_type: f.get("target_artifact_type").and_then(u8_value),
        like_count: f.get("like_count").and_then(u64_value),
    }
}

pub fn view_proposal(object: &DecodedObject) -> ProposalView {
    let f = &object.fields;
    ProposalView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        registry_id: f.get("registry_id").and_then(id_value),
        proposal_id: f.get("proposal_id").and_then(u64_value),
        proposer: f.get("proposer").and_then(string_value),
        proposal_type: f.get("proposal_type").and_then(u8_value),
        action_type: f.get("action_type").and_then(u8_value),
        title: f.get("title").and_then(string_value),
        description: f.get("description").and_then(string_value),
        payload_u64_1: f.get("payload_u64_1").and_then(u64_value),
        payload_u64_2: f.get("payload_u64_2").and_then(u64_value),
        payload_address: f.get("payload_address").and_then(string_value),
        payload_object_id: f
            .get("payload_object_id")
            .and_then(parse_option_field)
            .and_then(id_value),
        yes_votes: f.get("yes_votes").and_then(u64_value),
        no_votes: f.get("no_votes").and_then(u64_value),
        status: f.get("status").and_then(u8_value),
        executed: f.get("executed").and_then(bool_value),
        start_epoch: f.get("start_epoch").and_then(u64_value),
        end_epoch: f.get("end_epoch").and_then(u64_value),
    }
}

pub fn view_governance_config(object: &DecodedObject) -> GovernanceConfigView {
    let f = &object.fields;
    GovernanceConfigView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        registry_id: f.get("registry_id").and_then(id_value),
        total_supply: f.get("pprf_total_supply").and_then(u64_value),
        proposer_threshold: f.get("proposer_threshold").and_then(u64_value),
        proposal_duration_epochs: f.get("proposal_duration_epochs").and_then(u64_value),
        next_proposal_id: f.get("next_proposal_id").and_then(u64_value),
        proposal_creation_paused: f.get("proposal_creation_paused").and_then(bool_value),
        active_proposal_id: f
            .get("active_proposal_id")
            .and_then(parse_option_field)
            .and_then(u64_value),
    }
}

pub fn view_governance_vault(object: &DecodedObject) -> GovernanceVaultView {
    let f = &object.fields;
    GovernanceVaultView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        registry_id: f.get("registry_id").and_then(id_value),
        governance_config_id: f.get("governance_config_id").and_then(id_value),
        governance_authority: f.get("governance_authority").and_then(string_value),
        upgrade_authority: f.get("upgrade_authority").and_then(string_value),
        active_operator: f.get("active_operator").and_then(string_value),
        fee_recipient: f.get("fee_recipient").and_then(string_value),
        direct_authority_mode: f.get("direct_authority_mode").and_then(u8_value),
        direct_authority_permanently_disabled: f
            .get("direct_authority_permanently_disabled")
            .and_then(bool_value),
        has_pending_operator_transfer: f.get("has_pending_operator_transfer").and_then(bool_value),
        pending_operator: f.get("pending_operator").and_then(string_value),
        pending_operator_epoch: f.get("pending_operator_epoch").and_then(u64_value),
        pending_operator_wrapper_id: f.get("pending_operator_wrapper_id").and_then(id_value),
    }
}

pub fn view_fee_manager(object: &DecodedObject) -> FeeManagerView {
    let f = &object.fields;
    FeeManagerView {
        id: object.id.clone(),
        version: f.get("version").and_then(u64_value),
        registry_id: f.get("registry_id").and_then(id_value),
        comments_fee_level: f.get("comments_fee_level").and_then(u8_value),
    }
}
