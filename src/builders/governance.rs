// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    builders::base::BaseBuilder,
    constants::governance,
    deployment::Deployment,
    error::Result,
    transaction::{MoveArgument as Arg, MoveCall, TransactionPlan},
    types::{CreateExecutableProposalInput, CreateSignalProposalInput, VoteInput},
    validation::{validate_executable_proposal, validate_object_id, validate_signal_proposal},
};

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Debug)]
pub struct GovernanceBuilder {
    base: BaseBuilder,
}

impl GovernanceBuilder {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            base: BaseBuilder::new(deployment),
        }
    }

    pub fn create_signal_proposal(
        &self,
        input: &CreateSignalProposalInput,
    ) -> Result<TransactionPlan> {
        validate_signal_proposal(input)?;
        self.create_proposal(&CreateExecutableProposalInput {
            proposal_type: Some(governance::PROPOSAL_TYPE_SIGNAL),
            action_type: input.action_type,
            title: input.title.clone(),
            description: input.description.clone(),
            payload_u64_1: Some(0),
            payload_u64_2: Some(0),
            payload_address: input.payload_address.clone(),
            payload_object_id: None,
            payload_bytes: input.payload_text.clone().unwrap_or_default().into_bytes(),
            stake_coin_id: input.stake_coin_id.clone(),
        })
    }

    pub fn create_proposal(
        &self,
        input: &CreateExecutableProposalInput,
    ) -> Result<TransactionPlan> {
        validate_executable_proposal(input)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "create_proposal"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_config.clone()),
                Arg::U8(
                    input
                        .proposal_type
                        .unwrap_or(governance::PROPOSAL_TYPE_EXECUTABLE),
                ),
                Arg::U8(input.action_type as u8),
                Arg::String(input.title.clone()),
                Arg::String(input.description.clone()),
                Arg::U64(input.payload_u64_1.unwrap_or(0)),
                Arg::U64(input.payload_u64_2.unwrap_or(0)),
                Arg::Address(
                    input
                        .payload_address
                        .clone()
                        .unwrap_or_else(|| ZERO_ADDRESS.to_string()),
                ),
                Arg::OptionalObjectId(input.payload_object_id.clone()),
                Arg::Bytes(input.payload_bytes.clone()),
                Arg::Object(input.stake_coin_id.clone()),
            ],
        }))
    }

    pub fn vote_yes(&self, input: &VoteInput) -> Result<TransactionPlan> {
        self.vote_call("vote_yes", input)
    }

    pub fn vote_no(&self, input: &VoteInput) -> Result<TransactionPlan> {
        self.vote_call("vote_no", input)
    }

    pub fn finalize_proposal(&self, proposal_id: &str) -> Result<TransactionPlan> {
        self.config_proposal_call("finalize_proposal", proposal_id)
    }

    pub fn resolve_proposal_early(&self, proposal_id: &str) -> Result<TransactionPlan> {
        self.config_proposal_call("resolve_proposal_early", proposal_id)
    }

    pub fn execute_proposal(&self, proposal_id: &str) -> Result<TransactionPlan> {
        validate_object_id(proposal_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "execute_proposal"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_config.clone()),
                Arg::Object(proposal_id.to_string()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn claim_locked_tokens(&self, proposal_id: &str) -> Result<TransactionPlan> {
        validate_object_id(proposal_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .governance_target("governance_voting", "claim_locked_tokens"),
            arguments: vec![Arg::Object(proposal_id.to_string())],
        }))
    }

    fn vote_call(&self, function: &str, input: &VoteInput) -> Result<TransactionPlan> {
        validate_object_id(&input.proposal_id)?;
        validate_object_id(&input.coin_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.governance_target("governance_voting", function),
            arguments: vec![
                Arg::Object(input.proposal_id.clone()),
                Arg::Object(input.coin_id.clone()),
            ],
        }))
    }

    fn config_proposal_call(&self, function: &str, proposal_id: &str) -> Result<TransactionPlan> {
        validate_object_id(proposal_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.governance_target("governance_voting", function),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.governance_config.clone()),
                Arg::Object(proposal_id.to_string()),
            ],
        }))
    }
}
