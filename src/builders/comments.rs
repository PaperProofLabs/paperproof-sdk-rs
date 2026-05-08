// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    builders::base::BaseBuilder,
    deployment::Deployment,
    error::Result,
    transaction::{MoveArgument as Arg, MoveCall, TransactionPlan},
    types::{AddBlobCommentInput, AddOnchainCommentInput, SetCommentStatusInput},
    validation::{
        validate_blob_comment, validate_comment_status, validate_object_id,
        validate_onchain_comment, validate_tree_status,
    },
};

#[derive(Clone, Debug)]
pub struct CommentsBuilder {
    base: BaseBuilder,
}

impl CommentsBuilder {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            base: BaseBuilder::new(deployment),
        }
    }

    pub fn add_onchain_comment(&self, input: &AddOnchainCommentInput) -> Result<TransactionPlan> {
        validate_onchain_comment(input)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("add_onchain_comment"),
            arguments: vec![
                Arg::Object(input.tree_id.clone()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.fee_manager.clone()),
                Arg::U64(input.parent_comment_id),
                Arg::Bytes(input.content.clone()),
                self.base.sui_payment(input.payment_coin_id.as_ref()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn add_blob_comment(&self, input: &AddBlobCommentInput) -> Result<TransactionPlan> {
        validate_blob_comment(input)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("add_blob_comment"),
            arguments: vec![
                Arg::Object(input.tree_id.clone()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.fee_manager.clone()),
                Arg::U64(input.parent_comment_id),
                Arg::Bytes(input.blob_id.clone()),
                Arg::OptionalAddress(input.blob_object_id.clone()),
                Arg::Bytes(input.blob_digest.clone()),
                Arg::Bytes(input.preview.clone()),
                self.base.sui_payment(input.payment_coin_id.as_ref()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
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

    pub fn set_comment_status(&self, input: &SetCommentStatusInput) -> Result<TransactionPlan> {
        validate_object_id(&input.tree_id)?;
        validate_comment_status(input.status)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("set_comment_status"),
            arguments: vec![
                Arg::Object(input.tree_id.clone()),
                Arg::U64(input.comment_id),
                Arg::U8(input.status),
            ],
        }))
    }

    pub fn like_paper(
        &self,
        likes_book_id: &str,
        pprf_proof_coin_id: &str,
    ) -> Result<TransactionPlan> {
        self.like_call("like_paper", likes_book_id, pprf_proof_coin_id)
    }

    pub fn unlike_paper(
        &self,
        likes_book_id: &str,
        pprf_proof_coin_id: &str,
    ) -> Result<TransactionPlan> {
        self.like_call("unlike_paper", likes_book_id, pprf_proof_coin_id)
    }

    pub fn transfer_tree_owner(&self, tree_id: &str, new_owner: &str) -> Result<TransactionPlan> {
        validate_object_id(tree_id)?;
        crate::validation::validate_address(new_owner)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target("transfer_tree_owner"),
            arguments: vec![
                Arg::Object(tree_id.to_string()),
                Arg::Address(new_owner.to_string()),
            ],
        }))
    }

    fn like_call(
        &self,
        function: &str,
        likes_book_id: &str,
        pprf_proof_coin_id: &str,
    ) -> Result<TransactionPlan> {
        validate_object_id(likes_book_id)?;
        validate_object_id(pprf_proof_coin_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.comments_target(function),
            arguments: vec![
                Arg::Object(likes_book_id.to_string()),
                Arg::Object(pprf_proof_coin_id.to_string()),
            ],
        }))
    }
}
