// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransactionPlan {
    pub calls: Vec<MoveCall>,
}

impl TransactionPlan {
    pub fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn push(&mut self, call: MoveCall) {
        self.calls.push(call);
    }

    pub fn single(call: MoveCall) -> Self {
        Self { calls: vec![call] }
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }
}

impl Default for TransactionPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MoveCall {
    pub target: String,
    pub arguments: Vec<MoveArgument>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum MoveArgument {
    Object(String),
    Address(String),
    String(String),
    U8(u8),
    U64(u64),
    Bool(bool),
    Bytes(Vec<u8>),
    StringVector(Vec<String>),
    MetadataVector(Vec<crate::types::MetadataAttribute>),
    OptionalObject(Option<String>),
    OptionalAddress(Option<String>),
    OptionalObjectId(Option<String>),
}

impl MoveArgument {
    pub fn object(value: impl Into<String>) -> Self {
        Self::Object(value.into())
    }

    pub fn address(value: impl Into<String>) -> Self {
        Self::Address(value.into())
    }
}
