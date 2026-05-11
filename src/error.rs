// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub type Result<T> = std::result::Result<T, PaperProofError>;

#[derive(Debug, Error)]
pub enum PaperProofError {
    #[error("invalid address `{value}`: {message}")]
    InvalidAddress { value: String, message: String },

    #[error("invalid object id `{value}`: {message}")]
    InvalidObjectId { value: String, message: String },

    #[error("invalid package id `{value}`: {message}")]
    InvalidPackageId { value: String, message: String },

    #[error("invalid input `{field}`: {message}")]
    InvalidInput { field: String, message: String },

    #[error(
        "insufficient balance for {purpose}: owner {owner}, coin {coin_type}, required {required}, available {available}, coin count {coin_count}"
    )]
    InsufficientBalance {
        owner: String,
        coin_type: String,
        required: u64,
        available: u64,
        coin_count: usize,
        purpose: String,
    },

    #[error("object `{object_id}` was not found")]
    ObjectNotFound { object_id: String },

    #[error("event parse error: {message}")]
    EventParse { message: String },

    #[error("event verification failed: {message}")]
    EventVerification { message: String },

    #[error("network error while calling {endpoint}: {message}")]
    Network { endpoint: String, message: String },

    #[error("contract call `{target}` failed: {message}")]
    ContractCall { target: String, message: String },

    #[error("transaction build failed: {message}")]
    TransactionBuild { message: String },

    #[error("transaction execution failed: {message}")]
    TransactionExecution { message: String },

    #[error("wallet is not connected")]
    WalletNotConnected,

    #[error("walrus content verification failed: expected {expected}, got {actual}")]
    WalrusDigestMismatch { expected: String, actual: String },

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl PaperProofError {
    pub fn invalid_input(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn network(endpoint: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Network {
            endpoint: endpoint.into(),
            message: message.into(),
        }
    }

    pub fn event_verification(message: impl Into<String>) -> Self {
        Self::EventVerification {
            message: message.into(),
        }
    }
}
