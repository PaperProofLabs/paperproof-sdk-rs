// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{deployment::Deployment, transaction::MoveArgument};

#[derive(Clone, Debug)]
pub struct BaseBuilder {
    pub deployment: Deployment,
}

impl BaseBuilder {
    pub fn new(deployment: Deployment) -> Self {
        Self { deployment }
    }

    pub fn target(&self, package: &str, module: &str, function: &str) -> String {
        format!("{package}::{module}::{function}")
    }

    pub fn publishing_target(&self, function: &str) -> String {
        self.target(&self.deployment.packages.publishing, "publishing", function)
    }

    pub fn comments_target(&self, function: &str) -> String {
        self.target(&self.deployment.packages.comments, "comments", function)
    }

    pub fn governance_target(&self, module: &str, function: &str) -> String {
        self.target(&self.deployment.packages.governance, module, function)
    }

    pub fn sui_payment(&self, payment_coin_id: Option<&String>) -> MoveArgument {
        MoveArgument::OptionalObject(payment_coin_id.cloned())
    }
}
