// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    builders::base::BaseBuilder,
    deployment::Deployment,
    error::Result,
    transaction::{MoveArgument as Arg, MoveCall, TransactionPlan},
    validation::{validate_address, validate_object_id, validate_tree_status},
};

#[derive(Clone, Debug)]
pub struct OpsBuilder {
    base: BaseBuilder,
}

impl OpsBuilder {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            base: BaseBuilder::new(deployment),
        }
    }

    pub fn set_paused(&self, paused: bool) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self.base.publishing_target("set_paused"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.root.clone()),
                Arg::Bool(paused),
            ],
        })
    }

    pub fn set_series_status(&self, series_id: &str, status: u8) -> Result<TransactionPlan> {
        validate_object_id(series_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target("set_series_status"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.root.clone()),
                Arg::Object(series_id.to_string()),
                Arg::U8(status),
            ],
        }))
    }

    pub fn set_fee_recipient(&self, new_fee_recipient: &str) -> Result<TransactionPlan> {
        self.governance_address_call("set_fee_recipient", new_fee_recipient)
    }

    pub fn set_governance_authority(&self, new_authority: &str) -> Result<TransactionPlan> {
        self.governance_address_call("set_governance_authority", new_authority)
    }

    pub fn set_upgrade_authority(&self, new_authority: &str) -> Result<TransactionPlan> {
        self.governance_address_call("set_upgrade_authority", new_authority)
    }

    pub fn set_comments_fee_level(&self, fee_level: u8) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "set_comments_fee_level"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.fee_manager.clone()),
                Arg::U8(fee_level),
            ],
        })
    }

    pub fn nominate_operator(&self, new_operator: &str) -> Result<TransactionPlan> {
        self.governance_address_call("nominate_operator", new_operator)
    }

    pub fn accept_operator_transfer(&self) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "accept_operator_transfer"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        })
    }

    pub fn cancel_operator_transfer(&self) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "cancel_operator_transfer"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        })
    }

    pub fn expire_passed_proposal(&self, proposal_id: &str) -> Result<TransactionPlan> {
        validate_object_id(proposal_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "expire_passed_proposal"),
            arguments: vec![Arg::Object(proposal_id.to_string())],
        }))
    }

    pub fn migrate_config(&self) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "migrate_config"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_config.clone()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            ],
        })
    }

    pub fn migrate_proposal(&self, proposal_id: &str) -> Result<TransactionPlan> {
        validate_object_id(proposal_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "migrate_proposal"),
            arguments: vec![
                Arg::Object(proposal_id.to_string()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            ],
        }))
    }

    pub fn migrate_vault(&self) -> TransactionPlan {
        TransactionPlan::single(MoveCall {
            target: self.base.governance_target("governance", "migrate_vault"),
            arguments: vec![Arg::Object(
                self.base.deployment.objects.governance_vault.clone(),
            )],
        })
    }

    pub fn migrate_tree(&self, tree_id: &str) -> Result<TransactionPlan> {
        validate_object_id(tree_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("migrate_tree"),
            arguments: vec![
                Arg::Object(tree_id.to_string()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            ],
        }))
    }

    pub fn transfer_tree_owner(&self, tree_id: &str, new_owner: &str) -> Result<TransactionPlan> {
        validate_object_id(tree_id)?;
        validate_address(new_owner)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("transfer_tree_owner"),
            arguments: vec![
                Arg::Object(tree_id.to_string()),
                Arg::Address(new_owner.to_string()),
            ],
        }))
    }

    pub fn set_tree_status(&self, tree_id: &str, status: u8) -> Result<TransactionPlan> {
        validate_object_id(tree_id)?;
        validate_tree_status(status)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("set_tree_status"),
            arguments: vec![Arg::Object(tree_id.to_string()), Arg::U8(status)],
        }))
    }

    pub fn register_managed_upgrade_cap(&self, upgrade_cap_id: &str) -> Result<TransactionPlan> {
        validate_object_id(upgrade_cap_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "register_managed_upgrade_cap"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(upgrade_cap_id.to_string()),
            ],
        }))
    }

    pub fn share_managed_upgrade_cap(&self, managed_cap_id: &str) -> Result<TransactionPlan> {
        validate_object_id(managed_cap_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "share_managed_upgrade_cap"),
            arguments: vec![Arg::Object(managed_cap_id.to_string())],
        }))
    }

    pub fn authorize_managed_upgrade(
        &self,
        managed_cap_id: &str,
        policy: u8,
        digest: Vec<u8>,
    ) -> Result<TransactionPlan> {
        validate_object_id(managed_cap_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "authorize_managed_upgrade"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(managed_cap_id.to_string()),
                Arg::U8(policy),
                Arg::Bytes(digest),
            ],
        }))
    }

    pub fn commit_managed_upgrade(&self, managed_cap_id: &str) -> Result<TransactionPlan> {
        validate_object_id(managed_cap_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance", "commit_managed_upgrade"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(managed_cap_id.to_string()),
            ],
        }))
    }

    fn governance_address_call(&self, function: &str, address: &str) -> Result<TransactionPlan> {
        validate_address(address)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.governance_target("governance", function),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Address(address.to_string()),
            ],
        }))
    }
}
