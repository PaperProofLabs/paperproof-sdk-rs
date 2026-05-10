// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Map, Number, Value};
use std::fmt;

use crate::{
    error::{PaperProofError, Result},
    events::SuiEventEnvelope,
    executor::ExecutionMode,
    indexer::{CheckpointData, CheckpointDataProvider},
    providers::{
        BuiltTransaction, DynamicFieldName, DynamicFieldObject, PaperProofDataProvider,
        PaperProofExecutionProvider, ProviderExecutionOptions, ProviderExecutionOutput,
    },
    read::{Balance, CoinObject, Page},
    transaction::TransactionPlan,
    types::DecodedObject,
};

#[derive(Clone, Debug)]
pub struct NativeTransaction {
    pub transaction: sui_sdk_types::Transaction,
    pub signatures: Vec<sui_sdk_types::UserSignature>,
}

#[derive(Clone, Debug)]
pub struct NativeBuildOptions {
    pub sender: String,
    pub gas_budget: u64,
}

pub trait NativeTransactionBuilder: Send + Sync {
    fn build_native_transaction(
        &self,
        plan: &TransactionPlan,
        options: &NativeBuildOptions,
    ) -> Result<sui_sdk_types::Transaction>;
}

pub trait NativeTransactionSigner: Send + Sync {
    fn sign_transaction(
        &self,
        transaction: &sui_sdk_types::Transaction,
    ) -> Result<sui_sdk_types::UserSignature>;
}

#[derive(Clone)]
pub struct SuiNativeProvider<B = UnsupportedNativeBuilder, S = NoopNativeSigner> {
    pub client: sui_rpc::Client,
    pub builder: B,
    pub signer: S,
}

impl<B, S> fmt::Debug for SuiNativeProvider<B, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SuiNativeProvider")
            .field("endpoint", &self.client.uri().to_string())
            .finish_non_exhaustive()
    }
}

impl SuiNativeProvider<UnsupportedNativeBuilder, NoopNativeSigner> {
    pub fn new<T>(endpoint: T) -> Result<Self>
    where
        T: TryInto<http::Uri>,
        T::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        let client = sui_rpc::Client::new(endpoint).map_err(|err| PaperProofError::Network {
            endpoint: "sui-native".to_string(),
            message: err.to_string(),
        })?;
        Ok(Self {
            client,
            builder: UnsupportedNativeBuilder,
            signer: NoopNativeSigner,
        })
    }
}

impl<B, S> SuiNativeProvider<B, S> {
    pub fn with_parts(client: sui_rpc::Client, builder: B, signer: S) -> Self {
        Self {
            client,
            builder,
            signer,
        }
    }

    pub fn with_builder<NB>(self, builder: NB) -> SuiNativeProvider<NB, S> {
        SuiNativeProvider {
            client: self.client,
            builder,
            signer: self.signer,
        }
    }

    pub fn with_signer<NS>(self, signer: NS) -> SuiNativeProvider<B, NS> {
        SuiNativeProvider {
            client: self.client,
            builder: self.builder,
            signer,
        }
    }
}

#[async_trait]
impl<B, S> CheckpointDataProvider for SuiNativeProvider<B, S>
where
    B: Send + Sync,
    S: Send + Sync,
{
    async fn get_checkpoint_data(&self, sequence_number: u64) -> Result<CheckpointData> {
        use sui_rpc::proto::sui::rpc::v2::{
            GetCheckpointRequest, get_checkpoint_request::CheckpointId,
        };

        let mut client = self.client.clone();
        let mut request = GetCheckpointRequest::default();
        request.checkpoint_id = Some(CheckpointId::SequenceNumber(sequence_number));
        let response = client
            .ledger_client()
            .get_checkpoint(request)
            .await
            .map(|response| response.into_inner())
            .map_err(|err| PaperProofError::Network {
                endpoint: "sui-native get_checkpoint".to_string(),
                message: err.to_string(),
            })?;
        let raw = serde_json::to_value(&response).map_err(|err| PaperProofError::EventParse {
            message: format!("failed to encode native checkpoint response as JSON: {err}"),
        })?;
        Ok(CheckpointData {
            sequence_number,
            digest: raw
                .pointer("/checkpoint/digest")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            events: checkpoint_events_from_raw(&raw)?,
            raw,
        })
    }
}

#[async_trait]
impl<B, S> PaperProofDataProvider for SuiNativeProvider<B, S>
where
    B: Send + Sync,
    S: Send + Sync,
{
    async fn get_object(&self, object_id: &str) -> Result<Option<DecodedObject>> {
        crate::validation::validate_object_id(object_id)?;
        use sui_rpc::proto::sui::rpc::v2::GetObjectRequest;

        let mut client = self.client.clone();
        let mut request = GetObjectRequest::default();
        request.object_id = Some(object_id.to_string());
        request.read_mask = Some(field_mask(&["object_id", "object_type", "owner", "json"]));
        let response = client
            .ledger_client()
            .get_object(request)
            .await
            .map(|response| response.into_inner())
            .map_err(|err| PaperProofError::Network {
                endpoint: "sui-native get_object".to_string(),
                message: err.to_string(),
            })?;
        let Some(object) = response.object else {
            return Ok(None);
        };
        native_object_to_decoded(object).map(Some)
    }

    async fn get_dynamic_field_object(
        &self,
        parent_id: &str,
        _name: DynamicFieldName,
    ) -> Result<DynamicFieldObject> {
        crate::validation::validate_object_id(parent_id)?;
        Err(PaperProofError::TransactionBuild {
            message: "Sui Rust gRPC 0.3 exposes ListDynamicFields but not JSON-RPC-compatible getDynamicFieldObject by name; prefer scanning dynamic fields via native gRPC or a custom provider. JsonRpcClient is only a deprecated compatibility fallback for short-term point lookups.".to_string(),
        })
    }

    async fn get_balance(&self, owner: &str, coin_type: &str) -> Result<Balance> {
        crate::validation::validate_address(owner)?;
        use sui_rpc::proto::sui::rpc::v2::ListBalancesRequest;

        let mut client = self.client.clone();
        let mut request = ListBalancesRequest::default();
        request.owner = Some(owner.to_string());
        request.page_size = Some(1000);
        let response = client
            .state_client()
            .list_balances(request)
            .await
            .map(|response| response.into_inner())
            .map_err(|err| PaperProofError::Network {
                endpoint: "sui-native list_balances".to_string(),
                message: err.to_string(),
            })?;
        let total_balance = response
            .balances
            .into_iter()
            .find(|balance| balance.coin_type.as_deref() == Some(coin_type))
            .and_then(|balance| balance.balance)
            .unwrap_or_default();
        Ok(Balance {
            coin_type: coin_type.to_string(),
            coin_object_count: 0,
            total_balance: total_balance.to_string(),
            locked_balance: Value::Null,
        })
    }

    async fn get_coins_page(
        &self,
        owner: &str,
        coin_type: &str,
        cursor: Option<&str>,
        limit: Option<u64>,
    ) -> Result<Page<CoinObject>> {
        crate::validation::validate_address(owner)?;
        use sui_rpc::proto::sui::rpc::v2::ListOwnedObjectsRequest;

        let mut client = self.client.clone();
        let mut request = ListOwnedObjectsRequest::default();
        request.owner = Some(owner.to_string());
        request.page_size = Some(limit.unwrap_or(50).min(1000) as u32);
        request.page_token = cursor.map(|cursor| BASE64.decode(cursor).unwrap_or_default().into());
        request.read_mask = Some(field_mask(&[
            "object_id",
            "version",
            "digest",
            "object_type",
            "balance",
        ]));
        request.object_type = Some(format!("0x2::coin::Coin<{coin_type}>"));
        let response = client
            .state_client()
            .list_owned_objects(request)
            .await
            .map(|response| response.into_inner())
            .map_err(|err| PaperProofError::Network {
                endpoint: "sui-native list_owned_objects".to_string(),
                message: err.to_string(),
            })?;
        let next_cursor = response
            .next_page_token
            .as_ref()
            .map(|bytes| BASE64.encode(bytes));
        let data = response
            .objects
            .into_iter()
            .filter_map(|object| {
                Some(CoinObject {
                    coin_type: coin_type.to_string(),
                    coin_object_id: object.object_id?,
                    version: object.version.unwrap_or_default().to_string(),
                    digest: object.digest.unwrap_or_default(),
                    balance: object.balance.unwrap_or_default().to_string(),
                })
            })
            .collect();
        Ok(Page {
            data,
            has_next_page: next_cursor.is_some(),
            next_cursor,
        })
    }

    async fn query_events(
        &self,
        _query: Value,
        _cursor: Option<Value>,
        _limit: Option<u64>,
        _descending_order: bool,
    ) -> Result<Vec<SuiEventEnvelope>> {
        Err(PaperProofError::EventParse {
            message: "Sui Rust gRPC 0.3 does not provide a JSON-RPC-style historical queryEvents API; prefer a checkpoint/subscription indexer on top of SuiNativeProvider or a custom provider. PaperProofQueryClient::mainnet_jsonrpc is a deprecated compatibility fallback for short-term historical backfill.".to_string(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnsupportedNativeBuilder;

impl NativeTransactionBuilder for UnsupportedNativeBuilder {
    fn build_native_transaction(
        &self,
        _plan: &TransactionPlan,
        _options: &NativeBuildOptions,
    ) -> Result<sui_sdk_types::Transaction> {
        Err(PaperProofError::TransactionBuild {
            message: "native Sui Rust transaction building needs object references, shared-object initial versions, gas payment and BCS pure-argument typing; use SuiCliExecutor fallback or provide a custom NativeTransactionBuilder".to_string(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoopNativeSigner;

impl NativeTransactionSigner for NoopNativeSigner {
    fn sign_transaction(
        &self,
        _transaction: &sui_sdk_types::Transaction,
    ) -> Result<sui_sdk_types::UserSignature> {
        Err(PaperProofError::WalletNotConnected)
    }
}

#[async_trait]
impl<B, S> PaperProofExecutionProvider for SuiNativeProvider<B, S>
where
    B: NativeTransactionBuilder + Send + Sync,
    S: NativeTransactionSigner + Send + Sync,
{
    async fn build_transaction(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<BuiltTransaction> {
        let build_options = native_build_options(options)?;
        let transaction = self
            .builder
            .build_native_transaction(plan, &build_options)?;
        let bytes =
            bcs::to_bytes(&transaction).map_err(|err| PaperProofError::TransactionBuild {
                message: format!("failed to BCS-encode native Sui transaction: {err}"),
            })?;
        Ok(BuiltTransaction::NativeTransactionBytes(bytes))
    }

    async fn dry_run(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let mut options = options.clone();
        options.mode = ExecutionMode::DryRun;
        let transaction = self
            .builder
            .build_native_transaction(plan, &native_build_options(&options)?)?;
        let response = simulate_transaction(self.client.clone(), transaction, true).await?;
        Ok(native_response(response))
    }

    async fn dev_inspect(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let mut options = options.clone();
        options.mode = ExecutionMode::DevInspect;
        let transaction = self
            .builder
            .build_native_transaction(plan, &native_build_options(&options)?)?;
        let response = simulate_transaction(self.client.clone(), transaction, false).await?;
        Ok(native_response(response))
    }

    async fn sign_and_execute(
        &self,
        plan: &TransactionPlan,
        options: &ProviderExecutionOptions,
    ) -> Result<ProviderExecutionOutput> {
        let transaction = self
            .builder
            .build_native_transaction(plan, &native_build_options(options)?)?;
        let signature = self.signer.sign_transaction(&transaction)?;
        execute_transaction(self.client.clone(), transaction, vec![signature]).await
    }
}

pub async fn simulate_native_transaction(
    client: sui_rpc::Client,
    transaction: sui_sdk_types::Transaction,
    checks_enabled: bool,
) -> Result<ProviderExecutionOutput> {
    let response = simulate_transaction(client, transaction, checks_enabled).await?;
    Ok(native_response(response))
}

pub async fn execute_native_transaction(
    client: sui_rpc::Client,
    transaction: sui_sdk_types::Transaction,
    signatures: Vec<sui_sdk_types::UserSignature>,
) -> Result<ProviderExecutionOutput> {
    execute_transaction(client, transaction, signatures).await
}

fn native_build_options(options: &ProviderExecutionOptions) -> Result<NativeBuildOptions> {
    let sender = options
        .sender
        .clone()
        .ok_or_else(|| PaperProofError::TransactionBuild {
            message: "native transaction building requires ProviderExecutionOptions.sender"
                .to_string(),
        })?;
    crate::validation::validate_address(&sender)?;
    let gas_budget = options
        .gas_budget
        .ok_or_else(|| PaperProofError::TransactionBuild {
            message: "native transaction building requires ProviderExecutionOptions.gas_budget"
                .to_string(),
        })?;
    Ok(NativeBuildOptions { sender, gas_budget })
}

async fn simulate_transaction(
    mut client: sui_rpc::Client,
    transaction: sui_sdk_types::Transaction,
    checks_enabled: bool,
) -> Result<sui_rpc::proto::sui::rpc::v2::SimulateTransactionResponse> {
    use sui_rpc::proto::sui::rpc::v2::{
        SimulateTransactionRequest, simulate_transaction_request::TransactionChecks,
    };

    let checks = if checks_enabled {
        TransactionChecks::Enabled
    } else {
        TransactionChecks::Disabled
    };
    let mut request = SimulateTransactionRequest::default();
    request.transaction = Some(transaction.into());
    request.checks = Some(checks as i32);
    request.do_gas_selection = Some(true);
    client
        .execution_client()
        .simulate_transaction(request)
        .await
        .map(|response| response.into_inner())
        .map_err(|err| PaperProofError::Network {
            endpoint: "sui-native simulate_transaction".to_string(),
            message: err.to_string(),
        })
}

async fn execute_transaction(
    mut client: sui_rpc::Client,
    transaction: sui_sdk_types::Transaction,
    signatures: Vec<sui_sdk_types::UserSignature>,
) -> Result<ProviderExecutionOutput> {
    use sui_rpc::proto::sui::rpc::v2::ExecuteTransactionRequest;

    let mut request = ExecuteTransactionRequest::default();
    request.transaction = Some(transaction.into());
    request.signatures = signatures
        .into_iter()
        .map(sui_rpc::proto::sui::rpc::v2::UserSignature::from)
        .collect();
    let response = client
        .execution_client()
        .execute_transaction(request)
        .await
        .map(|response| response.into_inner())
        .map_err(|err| PaperProofError::Network {
            endpoint: "sui-native execute_transaction".to_string(),
            message: err.to_string(),
        })?;
    let json = serde_json::to_value(&response).map_err(|err| PaperProofError::EventParse {
        message: format!("failed to encode native execute response as JSON: {err}"),
    })?;
    let digest = response
        .transaction
        .as_ref()
        .and_then(|tx| tx.digest.clone());
    let status_success = response
        .transaction
        .as_ref()
        .and_then(|tx| tx.effects.as_ref())
        .and_then(|effects| effects.status.as_ref())
        .and_then(|status| status.success)
        .unwrap_or(false);
    Ok(ProviderExecutionOutput::NativeJson {
        status_success,
        digest,
        json,
    })
}

fn native_response(
    response: sui_rpc::proto::sui::rpc::v2::SimulateTransactionResponse,
) -> ProviderExecutionOutput {
    let json = serde_json::to_value(&response).unwrap_or(Value::Null);
    let digest = response
        .transaction
        .as_ref()
        .and_then(|tx| tx.digest.clone());
    let status_success = response
        .transaction
        .as_ref()
        .and_then(|tx| tx.effects.as_ref())
        .and_then(|effects| effects.status.as_ref())
        .and_then(|status| status.success)
        .unwrap_or(false);
    ProviderExecutionOutput::NativeJson {
        status_success,
        digest,
        json,
    }
}

fn checkpoint_events_from_raw(raw: &Value) -> Result<Vec<SuiEventEnvelope>> {
    let arrays = [
        "/checkpoint/events",
        "/checkpoint/transactions/events",
        "/checkpoint/transaction_events",
        "/events",
    ];
    let mut events = Vec::new();
    for pointer in arrays {
        if let Some(items) = raw.pointer(pointer).and_then(Value::as_array) {
            for item in items {
                if let Ok(event) = serde_json::from_value::<SuiEventEnvelope>(item.clone()) {
                    events.push(event);
                }
            }
        }
    }
    Ok(events)
}

fn field_mask(paths: &[&str]) -> prost_types::FieldMask {
    prost_types::FieldMask {
        paths: paths.iter().map(|path| (*path).to_string()).collect(),
    }
}

fn native_object_to_decoded(object: sui_rpc::proto::sui::rpc::v2::Object) -> Result<DecodedObject> {
    let id = object.object_id.unwrap_or_default();
    let object_type = object.object_type.unwrap_or_default();
    let owner = object
        .owner
        .map(serde_json::to_value)
        .transpose()
        .map_err(|err| PaperProofError::EventParse {
            message: format!("failed to encode native object owner as JSON: {err}"),
        })?;
    let fields = object
        .json
        .map(|value| prost_value_to_json(*value))
        .transpose()?
        .unwrap_or(Value::Null);
    Ok(DecodedObject {
        id,
        object_type,
        owner,
        fields,
    })
}

fn prost_value_to_json(value: prost_types::Value) -> Result<Value> {
    Ok(prost_value_kind_to_json(value.kind))
}

fn prost_value_kind_to_json(kind: Option<prost_types::value::Kind>) -> Value {
    match kind {
        Some(prost_types::value::Kind::NullValue(_)) | None => Value::Null,
        Some(prost_types::value::Kind::NumberValue(number)) => Number::from_f64(number)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Some(prost_types::value::Kind::StringValue(text)) => Value::String(text),
        Some(prost_types::value::Kind::BoolValue(value)) => Value::Bool(value),
        Some(prost_types::value::Kind::StructValue(value)) => {
            let mut map = Map::new();
            for (key, value) in value.fields {
                map.insert(key, prost_value_kind_to_json(value.kind));
            }
            Value::Object(map)
        }
        Some(prost_types::value::Kind::ListValue(value)) => Value::Array(
            value
                .values
                .into_iter()
                .map(|value| prost_value_kind_to_json(value.kind))
                .collect(),
        ),
    }
}
