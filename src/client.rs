// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    builders::{
        comments::CommentsBuilder, governance::GovernanceBuilder, ops::OpsBuilder,
        publishing::PublishingBuilder,
    },
    deployment::Deployment,
    error::{PaperProofError, Result},
    events::SuiEventEnvelope,
    providers::{DynamicFieldName, DynamicFieldObject, PaperProofDataProvider},
    read::{Balance, CoinObject, Page},
    types::DecodedObject,
    views,
};

#[derive(Clone, Debug)]
pub struct PaperProofClient {
    pub deployment: Deployment,
    pub publishing: PublishingBuilder,
    pub comments: CommentsBuilder,
    pub governance: GovernanceBuilder,
    pub ops: OpsBuilder,
}

#[async_trait]
impl PaperProofDataProvider for JsonRpcClient {
    async fn get_object(&self, object_id: &str) -> Result<Option<DecodedObject>> {
        JsonRpcClient::get_object(self, object_id).await
    }

    async fn get_dynamic_field_object(
        &self,
        parent_id: &str,
        name: DynamicFieldName,
    ) -> Result<DynamicFieldObject> {
        crate::validation::validate_object_id(parent_id)?;
        let result = self
            .rpc(
                "suix_getDynamicFieldObject",
                json!([parent_id, { "type": name.type_, "value": name.value }]),
            )
            .await?;
        Ok(DynamicFieldObject {
            data: result.get("data").cloned(),
        })
    }

    async fn get_balance(&self, owner: &str, coin_type: &str) -> Result<Balance> {
        crate::validation::validate_address(owner)?;
        let result = self
            .rpc("suix_getBalance", json!([owner, coin_type]))
            .await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    async fn get_coins_page(
        &self,
        owner: &str,
        coin_type: &str,
        cursor: Option<&str>,
        limit: Option<u64>,
    ) -> Result<Page<CoinObject>> {
        crate::validation::validate_address(owner)?;
        let result = self
            .rpc(
                "suix_getCoins",
                json!([owner, coin_type, cursor, limit.unwrap_or(50)]),
            )
            .await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    async fn query_events(
        &self,
        query: Value,
        cursor: Option<Value>,
        limit: Option<u64>,
        descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>> {
        JsonRpcClient::query_events(self, query, cursor, limit, descending_order).await
    }
}

impl PaperProofClient {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            publishing: PublishingBuilder::new(deployment.clone()),
            comments: CommentsBuilder::new(deployment.clone()),
            governance: GovernanceBuilder::new(deployment.clone()),
            ops: OpsBuilder::new(deployment.clone()),
            deployment,
        }
    }

    pub fn mainnet() -> Self {
        Self::new(crate::deployment::mainnet_deployment())
    }
}

/// Deprecated compatibility adapter for Sui JSON-RPC.
///
/// This client is not backed by the official Sui Rust SDK. It is a small
/// `reqwest` wrapper kept only for temporary historical event backfill and
/// compatibility paths while equivalent gRPC/GraphQL event APIs are adopted.
/// New integrations should prefer `sui-native` gRPC providers or a custom
/// provider.
#[derive(Clone, Debug)]
pub struct JsonRpcClient {
    endpoint: String,
    http: reqwest::Client,
}

impl JsonRpcClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http: reqwest::Client::new(),
        }
    }

    #[deprecated(
        since = "0.1.0",
        note = "Sui JSON-RPC is deprecated and not supported by the official sui-rust-sdk; prefer the sui-native gRPC provider or a custom provider"
    )]
    pub fn new_deprecated_compat(endpoint: impl Into<String>) -> Self {
        Self::new(endpoint)
    }

    pub async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|err| PaperProofError::network(&self.endpoint, err.to_string()))?;
        let status = response.status();
        let value: Value = response
            .json()
            .await
            .map_err(|err| PaperProofError::network(&self.endpoint, err.to_string()))?;
        if !status.is_success() {
            return Err(PaperProofError::network(
                &self.endpoint,
                format!("HTTP {status}: {value}"),
            ));
        }
        if let Some(error) = value.get("error") {
            return Err(PaperProofError::network(
                &self.endpoint,
                format!("JSON-RPC error: {error}"),
            ));
        }
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn get_object(&self, object_id: &str) -> Result<Option<DecodedObject>> {
        crate::validation::validate_object_id(object_id)?;
        let result = self
            .rpc(
                "sui_getObject",
                json!([
                    object_id,
                    {
                        "showContent": true,
                        "showOwner": true,
                        "showType": true,
                    }
                ]),
            )
            .await?;
        views::decode_sui_object(&result)
    }

    pub async fn query_events(
        &self,
        query: Value,
        cursor: Option<Value>,
        limit: Option<u64>,
        descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>> {
        let result = self
            .rpc(
                "suix_queryEvents",
                json!([query, cursor, limit.unwrap_or(50), descending_order]),
            )
            .await?;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        data.into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}
