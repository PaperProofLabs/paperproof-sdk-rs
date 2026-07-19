// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    client::JsonRpcClient,
    deployment::Deployment,
    error::{PaperProofError, Result},
    events::SuiEventEnvelope,
    providers::{DynamicFieldName, PaperProofDataProvider},
    types::{
        ArtifactControlRecordView, ArtifactSeriesView, ArtifactVersionView, CommentNodeView,
        CommentsTreeView, ControllerNFTView, ControllerStateSnapshot, DecodedObject, FeeManagerView,
        GovernanceConfigView, GovernanceVaultView, LikesBookView, PaperProofRootView,
        ProposalView, SeriesControlSnapshot,
    },
    validation::{validate_address, validate_object_id},
    views,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoinObject {
    #[serde(rename = "coinType")]
    pub coin_type: String,
    #[serde(rename = "coinObjectId")]
    pub coin_object_id: String,
    pub version: String,
    pub digest: String,
    pub balance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    #[serde(rename = "coinType")]
    pub coin_type: String,
    #[serde(rename = "coinObjectCount")]
    pub coin_object_count: u64,
    #[serde(rename = "totalBalance")]
    pub total_balance: String,
    #[serde(rename = "lockedBalance")]
    pub locked_balance: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Page<T> {
    pub data: Vec<T>,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

#[derive(Clone, Debug)]
pub struct PaperProofReadClient {
    pub rpc: JsonRpcClient,
    pub deployment: Deployment,
}

impl PaperProofReadClient {
    pub fn new(rpc: JsonRpcClient, deployment: Deployment) -> Self {
        Self { rpc, deployment }
    }

    pub fn mainnet() -> Self {
        let deployment = crate::deployment::mainnet_deployment();
        Self::new(JsonRpcClient::new(deployment.rpc_url.clone()), deployment)
    }

    pub async fn get_object(&self, object_id: &str) -> Result<DecodedObject> {
        validate_object_id(object_id)?;
        PaperProofDataProvider::get_object(&self.rpc, object_id)
            .await?
            .ok_or_else(|| PaperProofError::ObjectNotFound {
                object_id: object_id.to_string(),
            })
    }

    pub async fn get_object_or_null(&self, object_id: &str) -> Result<Option<DecodedObject>> {
        validate_object_id(object_id)?;
        PaperProofDataProvider::get_object(&self.rpc, object_id).await
    }

    pub async fn get_root(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.root).await
    }

    pub async fn get_root_view(&self) -> Result<PaperProofRootView> {
        Ok(views::view_root(&self.get_root().await?))
    }

    pub async fn get_type_registry(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.type_registry).await
    }

    pub async fn get_fee_manager(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.fee_manager).await
    }

    pub async fn get_fee_manager_view(&self) -> Result<FeeManagerView> {
        Ok(views::view_fee_manager(&self.get_fee_manager().await?))
    }

    pub async fn get_governance_vault(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.governance_vault)
            .await
    }

    pub async fn get_governance_vault_view(&self) -> Result<GovernanceVaultView> {
        Ok(views::view_governance_vault(
            &self.get_governance_vault().await?,
        ))
    }

    pub async fn get_governance_config(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.governance_config)
            .await
    }

    pub async fn get_governance_config_view(&self) -> Result<GovernanceConfigView> {
        Ok(views::view_governance_config(
            &self.get_governance_config().await?,
        ))
    }

    pub async fn get_series(&self, series_id: &str) -> Result<DecodedObject> {
        self.get_object(series_id).await
    }

    pub async fn get_series_view(&self, series_id: &str) -> Result<ArtifactSeriesView> {
        let mut base = views::view_series(&self.get_series(series_id).await?);
        if let Some(state) = self.get_series_control_state(series_id).await? {
            base.series_control_enabled = Some(true);
            base.series_authority_mode = state.authority_mode;
            base.series_authority_mode_name = state.authority_mode_name;
            base.series_control_record_id = state.control_record_id;
            base.series_controller_nft_id = state.controller_nft_id;
        } else {
            base.series_control_enabled = Some(false);
            base.series_authority_mode = Some(0);
            base.series_authority_mode_name = views::authority_mode_name(Some(0));
            base.series_control_record_id = None;
            base.series_controller_nft_id = None;
        }
        Ok(base)
    }

    pub async fn get_version(&self, version_id: &str) -> Result<DecodedObject> {
        self.get_object(version_id).await
    }

    pub async fn get_version_view(&self, version_id: &str) -> Result<ArtifactVersionView> {
        Ok(views::view_version(&self.get_version(version_id).await?))
    }

    pub async fn get_comments_tree(&self, tree_id: &str) -> Result<DecodedObject> {
        self.get_object(tree_id).await
    }

    pub async fn get_comments_tree_view(&self, tree_id: &str) -> Result<CommentsTreeView> {
        let mut base = views::view_comments_tree(&self.get_comments_tree(tree_id).await?);
        if let Some(state) = self.get_tree_control_state(tree_id).await? {
            base.tree_control_enabled = Some(true);
            base.tree_authority_mode = state.authority_mode;
            base.tree_authority_mode_name = state.authority_mode_name;
            base.tree_control_record_id = state.control_record_id;
            base.tree_controller_nft_id = state.controller_nft_id;
        } else {
            base.tree_control_enabled = Some(false);
            base.tree_authority_mode = Some(0);
            base.tree_authority_mode_name = views::authority_mode_name(Some(0));
            base.tree_control_record_id = None;
            base.tree_controller_nft_id = None;
        }
        Ok(base)
    }

    pub async fn get_controller_nft(&self, controller_nft_id: &str) -> Result<DecodedObject> {
        self.get_object(controller_nft_id).await
    }

    pub async fn get_controller_nft_view(
        &self,
        controller_nft_id: &str,
    ) -> Result<ControllerNFTView> {
        Ok(views::view_controller_nft(
            &self.get_controller_nft(controller_nft_id).await?,
        ))
    }

    pub async fn get_artifact_control_record(
        &self,
        control_record_id: &str,
    ) -> Result<DecodedObject> {
        self.get_object(control_record_id).await
    }

    pub async fn get_artifact_control_record_view(
        &self,
        control_record_id: &str,
    ) -> Result<ArtifactControlRecordView> {
        Ok(views::view_artifact_control_record(
            &self.get_artifact_control_record(control_record_id).await?,
        ))
    }

    pub async fn get_controller_nft_holder(
        &self,
        controller_nft_id: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .get_controller_nft(controller_nft_id)
            .await?
            .owner
            .as_ref()
            .and_then(owner_address_value))
    }

    pub async fn get_controller_state_snapshot(
        &self,
        control_record_id: Option<&str>,
        controller_nft_id: Option<&str>,
        series_owner: Option<&str>,
        tree_owner: Option<&str>,
    ) -> Result<ControllerStateSnapshot> {
        let Some(control_record_id) = control_record_id else {
            return Ok(ControllerStateSnapshot {
                control_enabled: false,
                authority_mode: None,
                authority_mode_name: None,
                control_record_id: None,
                controller_nft_id: controller_nft_id.map(ToString::to_string),
                controller_holder: None,
                current_controller_mirror: None,
                legacy_series_owner_mirror: None,
                legacy_comments_owner_mirror: None,
                transfer_locked: None,
                mirror_stale: Some(false),
            });
        };
        let Some(controller_nft_id) = controller_nft_id else {
            return Ok(ControllerStateSnapshot {
                control_enabled: false,
                authority_mode: None,
                authority_mode_name: None,
                control_record_id: Some(control_record_id.to_string()),
                controller_nft_id: None,
                controller_holder: None,
                current_controller_mirror: None,
                legacy_series_owner_mirror: None,
                legacy_comments_owner_mirror: None,
                transfer_locked: None,
                mirror_stale: Some(false),
            });
        };
        let record = self
            .get_artifact_control_record_view(control_record_id)
            .await?;
        let holder = self.get_controller_nft_holder(controller_nft_id).await?;
        let controller_matches_mirror = holder
            .as_deref()
            .zip(record.current_controller_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        let series_owner_matches_mirror = series_owner
            .zip(record.legacy_series_owner_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        let tree_owner_matches_mirror = tree_owner
            .zip(record.legacy_comments_owner_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        let mirror_stale = Some(
            controller_matches_mirror == Some(false)
                || series_owner_matches_mirror == Some(false)
                || tree_owner_matches_mirror == Some(false),
        );
        Ok(ControllerStateSnapshot {
            control_enabled: true,
            authority_mode: record.authority_mode,
            authority_mode_name: record.authority_mode_name.clone(),
            control_record_id: Some(control_record_id.to_string()),
            controller_nft_id: Some(controller_nft_id.to_string()),
            controller_holder: holder,
            current_controller_mirror: record.current_controller_mirror.clone(),
            legacy_series_owner_mirror: record.legacy_series_owner_mirror.clone(),
            legacy_comments_owner_mirror: record.legacy_comments_owner_mirror.clone(),
            transfer_locked: record.transfer_locked,
            mirror_stale,
        })
    }

    pub async fn get_likes_book(&self, book_id: &str) -> Result<DecodedObject> {
        self.get_object(book_id).await
    }

    pub async fn get_likes_book_view(&self, book_id: &str) -> Result<LikesBookView> {
        Ok(views::view_likes_book(&self.get_likes_book(book_id).await?))
    }

    pub async fn get_proposal(&self, proposal_id: &str) -> Result<DecodedObject> {
        self.get_object(proposal_id).await
    }

    pub async fn get_proposal_view(&self, proposal_id: &str) -> Result<ProposalView> {
        Ok(views::view_proposal(&self.get_proposal(proposal_id).await?))
    }

    pub async fn get_dynamic_field_object(
        &self,
        parent_id: &str,
        name_type: &str,
        name_value: Value,
    ) -> Result<Option<Value>> {
        validate_object_id(parent_id)?;
        Ok(self
            .rpc
            .get_dynamic_field_object(parent_id, DynamicFieldName::new(name_type, name_value))
            .await?
            .data)
    }

    pub async fn get_series_control_state(
        &self,
        series_id: &str,
    ) -> Result<Option<ControlStateView>> {
        let Some(package_id) = self.deployment.packages.controller.as_deref() else {
            return Ok(None);
        };
        let field = self
            .get_dynamic_field_object(
                series_id,
                &format!("{package_id}::controller::SeriesControlStateKey"),
                json!({}),
            )
            .await?;
        Ok(field.as_ref().and_then(control_state_view))
    }

    pub async fn get_tree_control_state(
        &self,
        tree_id: &str,
    ) -> Result<Option<ControlStateView>> {
        let Some(package_id) = self.deployment.packages.controller.as_deref() else {
            return Ok(None);
        };
        let field = self
            .get_dynamic_field_object(
                tree_id,
                &format!("{package_id}::controller::TreeControlStateKey"),
                json!({}),
            )
            .await?;
        Ok(field.as_ref().and_then(control_state_view))
    }

    pub async fn get_series_control_snapshot(
        &self,
        series_id: &str,
    ) -> Result<SeriesControlSnapshot> {
        let series = self.get_series_view(series_id).await?;
        let tree = match series.comments_tree_id.as_deref() {
            Some(tree_id) => Some(self.get_comments_tree_view(tree_id).await?),
            None => None,
        };
        let snapshot = self
            .get_controller_state_snapshot(
                series.series_control_record_id.as_deref(),
                series.series_controller_nft_id.as_deref(),
                series.owner.as_deref(),
                tree.as_ref().and_then(|item| item.owner.as_deref()),
            )
            .await?;
        let controller_matches_mirror = snapshot
            .controller_holder
            .as_deref()
            .zip(snapshot.current_controller_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        let series_owner_matches_mirror = series
            .owner
            .as_deref()
            .zip(snapshot.legacy_series_owner_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        let tree_owner_matches_mirror = tree
            .as_ref()
            .and_then(|item| item.owner.as_deref())
            .zip(snapshot.legacy_comments_owner_mirror.as_deref())
            .map(|(left, right)| same_address(left, right));
        Ok(SeriesControlSnapshot {
            control_enabled: snapshot.control_enabled,
            series_id: series_id.to_string(),
            tree_id: series.comments_tree_id.clone(),
            authority_mode: snapshot.authority_mode,
            authority_mode_name: snapshot.authority_mode_name,
            control_record_id: snapshot.control_record_id,
            controller_nft_id: snapshot.controller_nft_id,
            controller_holder: snapshot.controller_holder,
            current_controller_mirror: snapshot.current_controller_mirror,
            legacy_series_owner_mirror: snapshot.legacy_series_owner_mirror,
            legacy_comments_owner_mirror: snapshot.legacy_comments_owner_mirror,
            transfer_locked: snapshot.transfer_locked,
            mirror_stale: snapshot.mirror_stale,
            series_owner: series.owner,
            tree_owner: tree.and_then(|item| item.owner),
            controller_matches_mirror,
            series_owner_matches_mirror,
            tree_owner_matches_mirror,
        })
    }

    pub async fn get_comment_node(&self, tree_id: &str, comment_id: u64) -> Result<Option<Value>> {
        let tree = self.get_comments_tree(tree_id).await?;
        let Some(nodes_table_id) = tree.fields.get("nodes").and_then(views::table_id) else {
            return Err(PaperProofError::invalid_input(
                "tree_id",
                format!("cannot resolve comments nodes table for {tree_id}"),
            ));
        };
        self.get_dynamic_field_object(&nodes_table_id, "u64", json!(comment_id.to_string()))
            .await
    }

    pub async fn get_comment_node_view(
        &self,
        tree_id: &str,
        comment_id: u64,
    ) -> Result<Option<CommentNodeView>> {
        Ok(self
            .get_comment_node(tree_id, comment_id)
            .await?
            .as_ref()
            .map(views::view_comment_node))
    }

    pub async fn has_liked(&self, likes_book_id: &str, liker: &str) -> Result<bool> {
        validate_address(liker)?;
        let book = self.get_likes_book(likes_book_id).await?;
        let Some(likes_table_id) = book.fields.get("likes").and_then(views::table_id) else {
            return Err(PaperProofError::invalid_input(
                "likes_book_id",
                format!("cannot resolve likes table for {likes_book_id}"),
            ));
        };
        Ok(self
            .get_dynamic_field_object(&likes_table_id, "address", json!(liker))
            .await?
            .is_some())
    }

    pub async fn get_proposal_object_id(&self, proposal_id: u64) -> Result<String> {
        let config = self.get_governance_config().await?;
        self.get_proposal_object_id_from_config(&config.fields, proposal_id)
            .await
    }

    pub async fn get_proposal_object_id_from_config(
        &self,
        config_fields: &Value,
        proposal_id: u64,
    ) -> Result<String> {
        let Some(table_id) = config_fields
            .get("proposal_id_to_object")
            .and_then(views::table_id)
        else {
            return Err(PaperProofError::invalid_input(
                "governance_config",
                "cannot resolve proposal_id_to_object table id",
            ));
        };
        let Some(value) = self
            .get_dynamic_field_object(&table_id, "u64", json!(proposal_id.to_string()))
            .await?
        else {
            return Err(PaperProofError::ObjectNotFound {
                object_id: format!("proposal dynamic field {proposal_id}"),
            });
        };
        views::id_value(&value)
            .or_else(|| value.get("value").and_then(views::id_value))
            .or_else(|| {
                value
                    .pointer("/content/fields/value")
                    .and_then(views::id_value)
            })
            .ok_or_else(|| PaperProofError::EventParse {
                message: format!(
                    "proposal dynamic field {proposal_id} does not contain an object id"
                ),
            })
    }

    pub async fn get_balance(&self, owner: &str, coin_type: &str) -> Result<Balance> {
        validate_address(owner)?;
        self.rpc.get_balance(owner, coin_type).await
    }

    pub async fn get_coins_page(
        &self,
        owner: &str,
        coin_type: &str,
        cursor: Option<&str>,
        limit: Option<u64>,
    ) -> Result<Page<CoinObject>> {
        validate_address(owner)?;
        self.rpc
            .get_coins_page(owner, coin_type, cursor, limit)
            .await
    }

    pub async fn get_coins(&self, owner: &str, coin_type: &str) -> Result<Vec<CoinObject>> {
        let mut coins = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .get_coins_page(owner, coin_type, cursor.as_deref(), Some(50))
                .await?;
            coins.extend(page.data);
            if !page.has_next_page {
                break;
            }
            cursor = page.next_cursor;
        }
        Ok(coins)
    }

    pub async fn query_events(
        &self,
        query: Value,
        cursor: Option<Value>,
        limit: Option<u64>,
        descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>> {
        PaperProofDataProvider::query_events(&self.rpc, query, cursor, limit, descending_order)
            .await
    }
}

#[derive(Clone, Debug)]
pub struct PaperProofProviderReadClient<P> {
    pub provider: P,
    pub deployment: Deployment,
}

impl<P> PaperProofProviderReadClient<P>
where
    P: PaperProofDataProvider,
{
    pub fn new(provider: P, deployment: Deployment) -> Self {
        Self {
            provider,
            deployment,
        }
    }

    pub async fn get_object(&self, object_id: &str) -> Result<DecodedObject> {
        validate_object_id(object_id)?;
        self.provider
            .get_object(object_id)
            .await?
            .ok_or_else(|| PaperProofError::ObjectNotFound {
                object_id: object_id.to_string(),
            })
    }

    pub async fn get_object_or_null(&self, object_id: &str) -> Result<Option<DecodedObject>> {
        validate_object_id(object_id)?;
        self.provider.get_object(object_id).await
    }

    pub async fn get_root(&self) -> Result<DecodedObject> {
        self.get_object(&self.deployment.objects.root).await
    }

    pub async fn get_root_view(&self) -> Result<PaperProofRootView> {
        Ok(views::view_root(&self.get_root().await?))
    }

    pub async fn get_series(&self, series_id: &str) -> Result<DecodedObject> {
        self.get_object(series_id).await
    }

    pub async fn get_series_view(&self, series_id: &str) -> Result<ArtifactSeriesView> {
        let mut base = views::view_series(&self.get_series(series_id).await?);
        if let Some(state) = self.get_series_control_state(series_id).await? {
            base.series_control_enabled = Some(true);
            base.series_authority_mode = state.authority_mode;
            base.series_authority_mode_name = state.authority_mode_name;
            base.series_control_record_id = state.control_record_id;
            base.series_controller_nft_id = state.controller_nft_id;
        } else {
            base.series_control_enabled = Some(false);
            base.series_authority_mode = Some(0);
            base.series_authority_mode_name = views::authority_mode_name(Some(0));
        }
        Ok(base)
    }

    pub async fn get_version(&self, version_id: &str) -> Result<DecodedObject> {
        self.get_object(version_id).await
    }

    pub async fn get_version_view(&self, version_id: &str) -> Result<ArtifactVersionView> {
        Ok(views::view_version(&self.get_version(version_id).await?))
    }

    pub async fn get_comments_tree(&self, tree_id: &str) -> Result<DecodedObject> {
        self.get_object(tree_id).await
    }

    pub async fn get_comments_tree_view(&self, tree_id: &str) -> Result<CommentsTreeView> {
        let mut base = views::view_comments_tree(&self.get_comments_tree(tree_id).await?);
        if let Some(state) = self.get_tree_control_state(tree_id).await? {
            base.tree_control_enabled = Some(true);
            base.tree_authority_mode = state.authority_mode;
            base.tree_authority_mode_name = state.authority_mode_name;
            base.tree_control_record_id = state.control_record_id;
            base.tree_controller_nft_id = state.controller_nft_id;
        } else {
            base.tree_control_enabled = Some(false);
            base.tree_authority_mode = Some(0);
            base.tree_authority_mode_name = views::authority_mode_name(Some(0));
        }
        Ok(base)
    }

    pub async fn get_likes_book(&self, book_id: &str) -> Result<DecodedObject> {
        self.get_object(book_id).await
    }

    pub async fn get_likes_book_view(&self, book_id: &str) -> Result<LikesBookView> {
        Ok(views::view_likes_book(&self.get_likes_book(book_id).await?))
    }

    pub async fn get_dynamic_field_object(
        &self,
        parent_id: &str,
        name_type: &str,
        name_value: Value,
    ) -> Result<Option<Value>> {
        validate_object_id(parent_id)?;
        Ok(self
            .provider
            .get_dynamic_field_object(parent_id, DynamicFieldName::new(name_type, name_value))
            .await?
            .data)
    }

    pub async fn get_series_control_state(
        &self,
        series_id: &str,
    ) -> Result<Option<ControlStateView>> {
        let Some(package_id) = self.deployment.packages.controller.as_deref() else {
            return Ok(None);
        };
        let field = self
            .get_dynamic_field_object(
                series_id,
                &format!("{package_id}::controller::SeriesControlStateKey"),
                json!({}),
            )
            .await?;
        Ok(field.as_ref().and_then(control_state_view))
    }

    pub async fn get_tree_control_state(
        &self,
        tree_id: &str,
    ) -> Result<Option<ControlStateView>> {
        let Some(package_id) = self.deployment.packages.controller.as_deref() else {
            return Ok(None);
        };
        let field = self
            .get_dynamic_field_object(
                tree_id,
                &format!("{package_id}::controller::TreeControlStateKey"),
                json!({}),
            )
            .await?;
        Ok(field.as_ref().and_then(control_state_view))
    }

    pub async fn get_balance(&self, owner: &str, coin_type: &str) -> Result<Balance> {
        self.provider.get_balance(owner, coin_type).await
    }

    pub async fn get_coins(&self, owner: &str, coin_type: &str) -> Result<Vec<CoinObject>> {
        self.provider.get_coins(owner, coin_type).await
    }

    pub async fn query_events(
        &self,
        query: Value,
        cursor: Option<Value>,
        limit: Option<u64>,
        descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>> {
        self.provider
            .query_events(query, cursor, limit, descending_order)
            .await
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ControlStateView {
    pub control_record_id: Option<String>,
    pub controller_nft_id: Option<String>,
    pub authority_mode: Option<u64>,
    pub authority_mode_name: Option<String>,
    pub artifact_type: Option<u8>,
    pub series_id: Option<String>,
}

fn control_state_view(value: &Value) -> Option<ControlStateView> {
    let record = value.get("value").unwrap_or(value);
    let authority_mode = record
        .get("authority_mode")
        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()));
    Some(ControlStateView {
        control_record_id: record.get("control_record_id").and_then(views::id_value),
        controller_nft_id: record.get("controller_nft_id").and_then(views::id_value),
        authority_mode,
        authority_mode_name: views::authority_mode_name(authority_mode),
        artifact_type: record
            .get("artifact_type")
            .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
            .and_then(|v| u8::try_from(v).ok()),
        series_id: record.get("series_id").and_then(views::id_value),
    })
}

fn owner_address_value(value: &Value) -> Option<String> {
    let owner = value.get("owner").unwrap_or(value);
    if let Some(text) = owner.as_str() {
        return normalize_address(text);
    }
    owner
        .get("AddressOwner")
        .or_else(|| owner.get("ObjectOwner"))
        .or_else(|| owner.get("addressOwner"))
        .or_else(|| owner.get("owner"))
        .and_then(Value::as_str)
        .and_then(normalize_address)
}

fn same_address(left: &str, right: &str) -> bool {
    normalize_address(left) == normalize_address(right)
}

fn normalize_address(value: &str) -> Option<String> {
    let raw = value.trim().trim_matches('"');
    let raw = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    if raw.is_empty() || !raw.chars().all(|char| char.is_ascii_hexdigit()) {
        return None;
    }
    if raw.len() > 64 {
        return None;
    }
    Some(format!("0x{:0>64}", raw.to_ascii_lowercase()))
}
