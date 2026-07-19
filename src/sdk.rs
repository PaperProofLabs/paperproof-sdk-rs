// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    client::{JsonRpcClient, PaperProofClient},
    deployment::Deployment,
    error::{PaperProofError, Result},
    query::PaperProofQueryClient,
    read::PaperProofReadClient,
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PaperProofTransport {
    Grpc,
    JsonRpc,
    Custom,
}

#[derive(Clone, Debug, Default)]
pub struct CreatePaperProofSdkOptions {
    pub deployment: Option<Deployment>,
    pub transport: Option<PaperProofTransport>,
    pub rpc_url: Option<String>,
}

#[derive(Clone, Debug)]
pub enum PaperProofSdkRead {
    JsonRpc(Box<PaperProofReadClient>),
    #[cfg(feature = "sui-native")]
    Grpc(Box<crate::read::PaperProofProviderReadClient<crate::sui_native::SuiNativeProvider>>),
}

#[derive(Clone, Debug)]
pub enum PaperProofSdkQuery {
    JsonRpc(PaperProofQueryClient),
    GraphQl(PaperProofQueryClient),
}

#[derive(Clone, Debug)]
pub struct PaperProofSdk {
    pub deployment: Deployment,
    pub transport: PaperProofTransport,
    pub client: PaperProofClient,
    pub read: PaperProofSdkRead,
    pub query: Option<PaperProofSdkQuery>,
}

impl PaperProofSdk {
    pub fn mainnet() -> Result<Self> {
        create_paperproof_sdk(CreatePaperProofSdkOptions::default())
    }

    #[deprecated(
        since = "0.1.0",
        note = "Sui JSON-RPC is deprecated and not supported by the official sui-rust-sdk; use mainnet() with the sui-native feature for gRPC"
    )]
    pub fn mainnet_jsonrpc() -> Result<Self> {
        create_paperproof_sdk(CreatePaperProofSdkOptions {
            transport: Some(PaperProofTransport::JsonRpc),
            ..Default::default()
        })
    }
}

pub fn create_paperproof_sdk(options: CreatePaperProofSdkOptions) -> Result<PaperProofSdk> {
    let deployment = options
        .deployment
        .unwrap_or_else(crate::deployment::mainnet_deployment);
    let transport = options.transport.unwrap_or(PaperProofTransport::Grpc);
    let rpc_url = options
        .rpc_url
        .clone()
        .unwrap_or_else(|| deployment.rpc_url.clone());
    let client = PaperProofClient::new(deployment.clone());

    match transport {
        PaperProofTransport::Grpc => create_grpc_sdk(deployment, client, rpc_url),
        PaperProofTransport::JsonRpc => {
            let rpc = JsonRpcClient::new(rpc_url);
            let read = PaperProofReadClient::new(rpc.clone(), deployment.clone());
            let query = PaperProofQueryClient::new_jsonrpc(rpc, deployment.clone());
            Ok(PaperProofSdk {
                deployment,
                transport: PaperProofTransport::JsonRpc,
                client,
                read: PaperProofSdkRead::JsonRpc(Box::new(read)),
                query: Some(PaperProofSdkQuery::JsonRpc(query)),
            })
        }
        PaperProofTransport::Custom => Err(PaperProofError::invalid_input(
            "transport",
            "custom transport requires constructing provider/read/query clients directly",
        )),
    }
}

#[cfg(feature = "sui-native")]
fn create_grpc_sdk(
    deployment: Deployment,
    client: PaperProofClient,
    rpc_url: String,
) -> Result<PaperProofSdk> {
    use crate::query::{GraphQlQueryProvider, MAINNET_GRAPHQL_ENDPOINT};

    let provider = crate::sui_native::SuiNativeProvider::new(rpc_url)?;
    let read = crate::read::PaperProofProviderReadClient::new(provider, deployment.clone());
    let query = PaperProofQueryClient::new_graphql(
        JsonRpcClient::new(deployment.rpc_url.clone()),
        GraphQlQueryProvider::new(MAINNET_GRAPHQL_ENDPOINT),
        deployment.clone(),
    );
    Ok(PaperProofSdk {
        deployment,
        transport: PaperProofTransport::Grpc,
        client,
        read: PaperProofSdkRead::Grpc(Box::new(read)),
        query: Some(PaperProofSdkQuery::GraphQl(query)),
    })
}

#[cfg(not(feature = "sui-native"))]
fn create_grpc_sdk(
    _deployment: Deployment,
    _client: PaperProofClient,
    _rpc_url: String,
) -> Result<PaperProofSdk> {
    Err(PaperProofError::invalid_input(
        "transport",
        "grpc is the default PaperProof SDK transport, but this crate was built without the `sui-native` feature; enable `features = [\"sui-native\"]`. The JsonRpc transport is a deprecated compatibility fallback and is not backed by the official sui-rust-sdk.",
    ))
}
