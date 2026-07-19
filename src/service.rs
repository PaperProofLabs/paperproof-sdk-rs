// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::PaperProofClient,
    deployment::Deployment,
    error::Result,
    events::{
        AddVersionResult, CommentResult, LikeResult, PreprintReservationResult,
        ProposalExecutedResult, ProposalFinalizedResult, ProposalResult, PublishResult,
    },
    executor::{CliExecutionOptions, CliExecutionOutput, SuiCliExecutor},
    providers::{PaperProofExecutionProvider, ProviderExecutionOptions, ProviderExecutionOutput},
    read::PaperProofReadClient,
    transaction::TransactionPlan,
    types::{
        AddBlobCommentInput, AddOnchainCommentInput, AddVersionInput, BlogPostInput,
        CreateExecutableProposalInput, CreateSignalProposalInput, DatasetInput,
        GenericFileInput, PreprintInput, SetCommentStatusInput, SetTreeStatusInput,
        SoftwareReleaseInput, TechnicalReportInput, TransferArtifactOwnerInput,
        TransferTreeOwnerInput, UpdateSeriesDescriptionInput, UpdateSeriesMetadataInput,
        VoteInput,
    },
};

#[derive(Clone, Debug)]
pub struct ExecutedResult<T> {
    pub execution: CliExecutionOutput,
    pub result: T,
}

#[derive(Clone, Debug)]
pub struct PaperProofService {
    pub client: PaperProofClient,
    pub read: PaperProofReadClient,
    pub executor: SuiCliExecutor,
    pub default_options: CliExecutionOptions,
}

#[derive(Clone, Debug)]
pub struct ProviderExecutedResult<T> {
    pub execution: ProviderExecutionOutput,
    pub result: T,
}

#[derive(Clone, Debug)]
pub struct PaperProofProviderService<P> {
    pub client: PaperProofClient,
    pub read: PaperProofReadClient,
    pub execution_provider: P,
    pub default_options: ProviderExecutionOptions,
}

impl PaperProofService {
    pub fn new(deployment: Deployment) -> Self {
        let read = PaperProofReadClient::new(
            crate::client::JsonRpcClient::new(deployment.rpc_url.clone()),
            deployment.clone(),
        );
        Self {
            client: PaperProofClient::new(deployment.clone()),
            read,
            executor: SuiCliExecutor::new(deployment),
            default_options: CliExecutionOptions::default(),
        }
    }

    pub fn mainnet() -> Self {
        Self::new(crate::deployment::mainnet_deployment())
    }

    pub fn with_options(mut self, options: CliExecutionOptions) -> Self {
        self.default_options = options;
        self
    }

    pub fn execute_plan(
        &self,
        plan: &TransactionPlan,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.executor
            .run(plan, options.unwrap_or(&self.default_options))
    }

    pub fn publish_preprint(
        &self,
        input: &PreprintInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_preprint(input)?,
            "publish preprint",
            options,
        )
    }

    pub fn reserve_preprint_code(
        &self,
        owner: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PreprintReservationResult>> {
        let execution = self.execute_plan(
            &self.client.publishing.reserve_preprint_code(owner)?,
            options,
        )?;
        let result = execution.preprint_reservation_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn finalize_reserved_preprint(
        &self,
        reservation_id: &str,
        input: &PreprintInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client
                .publishing
                .finalize_reserved_preprint(reservation_id, input)?,
            "finalize reserved preprint",
            options,
        )
    }

    pub fn publish_blog_post(
        &self,
        input: &BlogPostInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_blog_post(input)?,
            "publish blog post",
            options,
        )
    }

    pub fn publish_technical_report(
        &self,
        input: &TechnicalReportInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_technical_report(input)?,
            "publish technical report",
            options,
        )
    }

    pub fn publish_dataset(
        &self,
        input: &DatasetInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_dataset(input)?,
            "publish dataset",
            options,
        )
    }

    pub fn publish_software_release(
        &self,
        input: &SoftwareReleaseInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_software_release(input)?,
            "publish software release",
            options,
        )
    }

    pub fn publish_generic_file(
        &self,
        input: &GenericFileInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client.publishing.publish_generic_file(input)?,
            "publish generic file",
            options,
        )
    }

    pub fn add_preprint_version(
        &self,
        input: &AddVersionInput<PreprintInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_preprint_version(input)?,
            "add preprint version",
            options,
        )
    }

    pub fn add_blog_post_version(
        &self,
        input: &AddVersionInput<BlogPostInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_blog_post_version(input)?,
            "add blog post version",
            options,
        )
    }

    pub fn add_technical_report_version(
        &self,
        input: &AddVersionInput<TechnicalReportInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_technical_report_version(input)?,
            "add technical report version",
            options,
        )
    }

    pub fn add_dataset_version(
        &self,
        input: &AddVersionInput<DatasetInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_dataset_version(input)?,
            "add dataset version",
            options,
        )
    }

    pub fn add_software_release_version(
        &self,
        input: &AddVersionInput<SoftwareReleaseInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_software_release_version(input)?,
            "add software release version",
            options,
        )
    }

    pub fn add_generic_file_version(
        &self,
        input: &AddVersionInput<GenericFileInput>,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_generic_file_version(input)?,
            "add generic file version",
            options,
        )
    }

    pub fn add_onchain_comment(
        &self,
        input: &AddOnchainCommentInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<CommentResult>> {
        let execution =
            self.execute_plan(&self.client.comments.add_onchain_comment(input)?, options)?;
        let result = execution.comment_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn add_blob_comment(
        &self,
        input: &AddBlobCommentInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<CommentResult>> {
        let execution =
            self.execute_plan(&self.client.comments.add_blob_comment(input)?, options)?;
        let result = execution.comment_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn set_tree_status(
        &self,
        input: &SetTreeStatusInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.comments.set_tree_status(input)?, options)
    }

    pub fn set_comment_status(
        &self,
        input: &SetCommentStatusInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.comments.set_comment_status(input)?, options)
    }

    pub fn like_paper(
        &self,
        likes_book_id: &str,
        pprf_proof_coin_id: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<Option<LikeResult>>> {
        let execution = self.execute_plan(
            &self
                .client
                .comments
                .like_paper(likes_book_id, pprf_proof_coin_id)?,
            options,
        )?;
        let result = execution.like_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn unlike_paper(
        &self,
        likes_book_id: &str,
        pprf_proof_coin_id: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<Option<LikeResult>>> {
        let execution = self.execute_plan(
            &self
                .client
                .comments
                .unlike_paper(likes_book_id, pprf_proof_coin_id)?,
            options,
        )?;
        let result = execution.unlike_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn update_series_metadata(
        &self,
        input: &UpdateSeriesMetadataInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.publishing.update_series_metadata(input)?, options)
    }

    pub fn update_series_description(
        &self,
        input: &UpdateSeriesDescriptionInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(
            &self.client.publishing.update_series_description(input)?,
            options,
        )
    }

    pub fn transfer_artifact_owner(
        &self,
        input: &TransferArtifactOwnerInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(
            &self.client.publishing.transfer_artifact_owner(input)?,
            options,
        )
    }

    pub fn transfer_tree_owner(
        &self,
        input: &TransferTreeOwnerInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.comments.transfer_tree_owner(input)?, options)
    }

    pub fn create_proposal(
        &self,
        input: &CreateExecutableProposalInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<ProposalResult>> {
        let execution =
            self.execute_plan(&self.client.governance.create_proposal(input)?, options)?;
        let result = execution.proposal_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn create_signal_proposal(
        &self,
        input: &CreateSignalProposalInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<ProposalResult>> {
        let execution = self.execute_plan(
            &self.client.governance.create_signal_proposal(input)?,
            options,
        )?;
        let result = execution.proposal_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn vote_yes(
        &self,
        input: &VoteInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.governance.vote_yes(input)?, options)
    }

    pub fn vote_no(
        &self,
        input: &VoteInput,
        options: Option<&CliExecutionOptions>,
    ) -> Result<CliExecutionOutput> {
        self.execute_plan(&self.client.governance.vote_no(input)?, options)
    }

    pub fn finalize_proposal(
        &self,
        proposal_id: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<Option<ProposalFinalizedResult>>> {
        let execution = self.execute_plan(
            &self.client.governance.finalize_proposal(proposal_id)?,
            options,
        )?;
        let result = execution.proposal_finalized_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    pub fn execute_proposal(
        &self,
        proposal_id: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<Option<ProposalExecutedResult>>> {
        let execution = self.execute_plan(
            &self.client.governance.execute_proposal(proposal_id)?,
            options,
        )?;
        let result = execution.proposal_executed_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    fn execute_publish(
        &self,
        plan: TransactionPlan,
        _label: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<PublishResult>> {
        let execution = self.execute_plan(&plan, options)?;
        let result = execution.publish_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }

    fn execute_add_version(
        &self,
        plan: TransactionPlan,
        _label: &str,
        options: Option<&CliExecutionOptions>,
    ) -> Result<ExecutedResult<AddVersionResult>> {
        let execution = self.execute_plan(&plan, options)?;
        let result = execution.add_version_result(&self.client.deployment)?;
        Ok(ExecutedResult { execution, result })
    }
}

impl<P> PaperProofProviderService<P>
where
    P: PaperProofExecutionProvider,
{
    pub fn new(deployment: Deployment, execution_provider: P) -> Self {
        let read = PaperProofReadClient::new(
            crate::client::JsonRpcClient::new(deployment.rpc_url.clone()),
            deployment.clone(),
        );
        Self {
            client: PaperProofClient::new(deployment),
            read,
            execution_provider,
            default_options: ProviderExecutionOptions::default(),
        }
    }

    pub fn with_options(mut self, options: ProviderExecutionOptions) -> Self {
        self.default_options = options;
        self
    }

    pub async fn build_transaction(
        &self,
        plan: &TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<crate::providers::BuiltTransaction> {
        self.execution_provider
            .build_transaction(plan, options.unwrap_or(&self.default_options))
            .await
    }

    pub async fn dry_run(
        &self,
        plan: &TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execution_provider
            .dry_run(plan, options.unwrap_or(&self.default_options))
            .await
    }

    pub async fn dev_inspect(
        &self,
        plan: &TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execution_provider
            .dev_inspect(plan, options.unwrap_or(&self.default_options))
            .await
    }

    pub async fn execute_plan(
        &self,
        plan: &TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execution_provider
            .sign_and_execute(plan, options.unwrap_or(&self.default_options))
            .await
    }

    pub async fn publish_preprint(
        &self,
        input: &PreprintInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<PublishResult>> {
        self.execute_publish(self.client.publishing.publish_preprint(input)?, options)
            .await
    }

    pub async fn reserve_preprint_code(
        &self,
        owner: &str,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<PreprintReservationResult>> {
        let execution = self
            .execute_plan(
                &self.client.publishing.reserve_preprint_code(owner)?,
                options,
            )
            .await?;
        let cli = execution.clone().into_cli_output()?;
        let result = cli.preprint_reservation_result(&self.client.deployment)?;
        Ok(ProviderExecutedResult { execution, result })
    }

    pub async fn finalize_reserved_preprint(
        &self,
        reservation_id: &str,
        input: &PreprintInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<PublishResult>> {
        self.execute_publish(
            self.client
                .publishing
                .finalize_reserved_preprint(reservation_id, input)?,
            options,
        )
        .await
    }

    pub async fn add_preprint_version(
        &self,
        input: &AddVersionInput<PreprintInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_preprint_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_blog_post_version(
        &self,
        input: &AddVersionInput<BlogPostInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_blog_post_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_technical_report_version(
        &self,
        input: &AddVersionInput<TechnicalReportInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_technical_report_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_dataset_version(
        &self,
        input: &AddVersionInput<DatasetInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_dataset_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_software_release_version(
        &self,
        input: &AddVersionInput<SoftwareReleaseInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_software_release_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_generic_file_version(
        &self,
        input: &AddVersionInput<GenericFileInput>,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        self.execute_add_version(
            self.client.publishing.add_generic_file_version(input)?,
            options,
        )
        .await
    }

    pub async fn add_onchain_comment(
        &self,
        input: &AddOnchainCommentInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<CommentResult>> {
        let execution = self
            .execute_plan(&self.client.comments.add_onchain_comment(input)?, options)
            .await?;
        let cli = execution.clone().into_cli_output()?;
        let result = cli.comment_result(&self.client.deployment)?;
        Ok(ProviderExecutedResult { execution, result })
    }

    pub async fn add_blob_comment(
        &self,
        input: &AddBlobCommentInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<CommentResult>> {
        let execution = self
            .execute_plan(&self.client.comments.add_blob_comment(input)?, options)
            .await?;
        let cli = execution.clone().into_cli_output()?;
        let result = cli.comment_result(&self.client.deployment)?;
        Ok(ProviderExecutedResult { execution, result })
    }

    pub async fn set_tree_status(
        &self,
        input: &SetTreeStatusInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.comments.set_tree_status(input)?, options)
        .await
    }

    pub async fn set_comment_status(
        &self,
        input: &SetCommentStatusInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.comments.set_comment_status(input)?, options)
        .await
    }

    pub async fn transfer_artifact_owner(
        &self,
        input: &TransferArtifactOwnerInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.publishing.transfer_artifact_owner(input)?, options)
        .await
    }

    pub async fn transfer_tree_owner(
        &self,
        input: &TransferTreeOwnerInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.comments.transfer_tree_owner(input)?, options)
        .await
    }

    pub async fn update_series_metadata(
        &self,
        input: &UpdateSeriesMetadataInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.publishing.update_series_metadata(input)?, options)
        .await
    }

    pub async fn update_series_description(
        &self,
        input: &UpdateSeriesDescriptionInput,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutionOutput> {
        self.execute_plan(&self.client.publishing.update_series_description(input)?, options)
        .await
    }

    async fn execute_publish(
        &self,
        plan: TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<PublishResult>> {
        let execution = self.execute_plan(&plan, options).await?;
        let cli = execution.clone().into_cli_output()?;
        let result = cli.publish_result(&self.client.deployment)?;
        Ok(ProviderExecutedResult { execution, result })
    }

    async fn execute_add_version(
        &self,
        plan: TransactionPlan,
        options: Option<&ProviderExecutionOptions>,
    ) -> Result<ProviderExecutedResult<AddVersionResult>> {
        let execution = self.execute_plan(&plan, options).await?;
        let cli = execution.clone().into_cli_output()?;
        let result = cli.add_version_result(&self.client.deployment)?;
        Ok(ProviderExecutedResult { execution, result })
    }
}
