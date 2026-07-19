// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{PaperProofError, Result},
    events::SuiEventEnvelope,
    executor::{CliExecutionOptions, CliExecutionOutput, ExecutionMode},
    read::{Balance, CoinObject, Page},
    transaction::TransactionPlan,
    types::DecodedObject,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DynamicFieldName {
    pub type_: String,
    pub value: Value,
}

impl DynamicFieldName {
    pub fn new(type_: impl Into<String>, value: Value) -> Self {
        Self {
            type_: type_.into(),
            value,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DynamicFieldObject {
    pub data: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProviderExecutionOptions {
    pub sender: Option<String>,
    pub gas_budget: Option<u64>,
    pub gas_coin: Option<String>,
    pub mode: ExecutionMode,
}

impl Default for ProviderExecutionOptions {
    fn default() -> Self {
        Self {
            sender: None,
            gas_budget: None,
            gas_coin: None,
            mode: ExecutionMode::Preview,
        }
    }
}

impl From<&CliExecutionOptions> for ProviderExecutionOptions {
    fn from(value: &CliExecutionOptions) -> Self {
        Self {
            sender: value.sender.clone(),
            gas_budget: value.gas_budget,
            gas_coin: value.gas_coin.clone(),
            mode: value.mode.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "backend", content = "value")]
pub enum BuiltTransaction {
    NeutralPlan(TransactionPlan),
    SuiCliArgs(Vec<String>),
    NativeTransactionBytes(Vec<u8>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "backend", content = "value")]
pub enum ProviderExecutionOutput {
    SuiCli(CliExecutionOutput),
    NativeJson {
        status_success: bool,
        digest: Option<String>,
        json: Value,
    },
}

impl ProviderExecutionOutput {
    pub fn status_success(&self) -> bool {
        match self {
            Self::SuiCli(output) => output.status_success,
            Self::NativeJson { status_success, .. } => *status_success,
        }
    }

    pub fn digest(&self) -> Option<&str> {
        match self {
            Self::SuiCli(output) => output.digest.as_deref(),
            Self::NativeJson { digest, .. } => digest.as_deref(),
        }
    }

    pub fn json(&self) -> Option<&Value> {
        match self {
            Self::SuiCli(output) => output.json.as_ref(),
            Self::NativeJson { json, .. } => Some(json),
        }
    }

    pub fn into_cli_output(self) -> Result<CliExecutionOutput> {
        match self {
            Self::SuiCli(output) => Ok(output),
            Self::NativeJson { .. } => Err(PaperProofError::TransactionExecution {
                message: "provider returned native output where CLI output was required"
                    .to_string(),
            }),
        }
    }
}

#[async_trait]
pub trait PaperProofDataProvider: Send + Sync {
    async fn get_object(&self, object_id: &str) -> Result<Option<DecodedObject>>;

    async fn get_dynamic_field_object(
        &self,
        parent_id: &str,
        name: DynamicFieldName,
    ) -> Result<DynamicFieldObject>;

    async fn get_balance(&self, owner: &str, coin_type: &str) -> Result<Balance>;

    async fn get_coins_page(
        &self,
        owner: &str,
        coin_type: &str,
        cursor: Option<&str>,
        limit: Option<u64>,
    ) -> Result<Page<CoinObject>>;

    async fn query_events(
        &self,
        query: Value,
        cursor: Option<Value>,
        limit: Option<u64>,
        descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>>;

    async fn get_coins(&self, owner: &str, coin_type: &str) -> Result<Vec<CoinObject>> {
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
}

#[async_trait]
pub trait PaperProofExecutionProvider: Send + Sync {
    async fn build_transaction(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<BuiltTransaction>;

    async fn dry_run(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput>;

    async fn dev_inspect(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput>;

    async fn sign_and_execute(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput>;
}

pub trait PaperProofProvider: PaperProofDataProvider + PaperProofExecutionProvider {}

impl<T> PaperProofProvider for T where T: PaperProofDataProvider + PaperProofExecutionProvider {}
