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
        ArtifactSeriesView, ArtifactVersionView, CommentNodeView, CommentsTreeView, DecodedObject,
        FeeManagerView, GovernanceConfigView, GovernanceVaultView, LikesBookView,
        PaperProofRootView, ProposalView,
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
        self.get_object(&self.deployment.objects.type_registry)
            .await
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
        Ok(views::view_series(&self.get_series(series_id).await?))
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
        Ok(views::view_comments_tree(
            &self.get_comments_tree(tree_id).await?,
        ))
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
