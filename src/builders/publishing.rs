// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    builders::base::BaseBuilder,
    constants::{PROTOCOL_LIMITS, reserved_metadata_keys},
    deployment::Deployment,
    error::{PaperProofError, Result},
    transaction::{
        MoveArgument as Arg, MoveCall, TransactionPlan, TransactionValueRef, TransferObjects,
    },
    types::{
        AddVersionInput, AddVersionWithControllerInput, BlogPostInput, CommonContentInput,
        DatasetInput, GenericFileInput, MetadataAttribute, PreprintInput,
        PromoteExistingSeriesControllerModeInput, PromoteExistingSeriesToDualModeInput,
        SoftwareReleaseInput, TechnicalReportInput, TransferArtifactOwnerInput,
        TransferArtifactOwnerWithControllerInput, UpdateSeriesDescriptionWithControllerInput,
        UpdateSeriesMetadataWithControllerInput,
    },
    validation::{
        validate_address, validate_blog_post_input, validate_dataset_input,
        validate_generic_file_input, validate_metadata_attributes, validate_object_id,
        validate_preprint_input, validate_required_text, validate_software_release_input,
        validate_technical_report_input,
    },
};

#[derive(Clone, Debug)]
pub struct PublishingBuilder {
    base: BaseBuilder,
}

impl PublishingBuilder {
    pub fn new(deployment: Deployment) -> Self {
        Self {
            base: BaseBuilder::new(deployment),
        }
    }

    pub fn publish_preprint(&self, input: &PreprintInput) -> Result<TransactionPlan> {
        validate_preprint_input(input)?;
        Err(PaperProofError::invalid_input(
            "publish_preprint",
            "direct preprint publishing is disabled by the upgraded contract; reserve a preprint code, stamp the PDF, then call finalize_reserved_preprint",
        ))
    }

    pub fn reserve_preprint_code(&self, owner: &str) -> Result<TransactionPlan> {
        crate::validation::validate_address(owner)?;
        let mut plan = TransactionPlan::single(MoveCall {
            target: self.base.publishing_target("reserve_preprint_code"),
            arguments: vec![
                Arg::Object(self.base.deployment.objects.root.clone()),
                Arg::Object(self.base.deployment.objects.type_registry.clone()),
                Arg::Object(self.base.deployment.objects.governance_vault.clone()),
                Arg::Object(self.base.deployment.objects.fee_manager.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        });
        plan.transfers.push(TransferObjects {
            objects: vec![TransactionValueRef::LastResult],
            recipient: owner.to_string(),
        });
        Ok(plan)
    }

    pub fn finalize_reserved_preprint(
        &self,
        reservation_id: &str,
        input: &PreprintInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(reservation_id)?;
        validate_preprint_input(input)?;
        self.publish_call_with_context(
            "finalize_reserved_preprint",
            PublishCallContext {
                prefix: vec![Arg::Object(reservation_id.to_string())],
                args: vec![
                    Arg::String(input.title.clone()),
                    Arg::String(input.abstract_text.clone()),
                    Arg::StringVector(input.authors.clone()),
                    Arg::StringVector(input.keywords.clone()),
                    Arg::String(input.field.clone()),
                    Arg::String(input.license.clone()),
                    Arg::U64(input.page_count),
                ],
                content: &input.content,
                series_metadata: &input.series_metadata,
                series_description: input.series_description.as_deref(),
                version_metadata: &input.version_metadata,
                version_change_note: input.version_change_note.as_deref(),
                require_version_change_note: false,
                payment_coin_id: input.payment_coin_id.as_ref(),
            },
        )
    }

    pub fn publish_blog_post(&self, input: &BlogPostInput) -> Result<TransactionPlan> {
        validate_blog_post_input(input)?;
        self.publish_call(
            "publish_blog_post",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.summary.clone()),
                Arg::StringVector(input.tags.clone()),
                Arg::String(input.language.clone()),
            ],
            &input.content,
            &input.series_metadata,
            input.series_description.as_deref(),
            &input.version_metadata,
            input.version_change_note.as_deref(),
            input.payment_coin_id.as_ref(),
        )
    }

    pub fn publish_technical_report(
        &self,
        input: &TechnicalReportInput,
    ) -> Result<TransactionPlan> {
        validate_technical_report_input(input)?;
        self.publish_call(
            "publish_technical_report",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.abstract_text.clone()),
                Arg::StringVector(input.authors.clone()),
                Arg::String(input.organization.clone()),
                Arg::String(input.report_number.clone()),
                Arg::StringVector(input.keywords.clone()),
                Arg::String(input.license.clone()),
            ],
            &input.content,
            &input.series_metadata,
            input.series_description.as_deref(),
            &input.version_metadata,
            input.version_change_note.as_deref(),
            input.payment_coin_id.as_ref(),
        )
    }

    pub fn publish_dataset(&self, input: &DatasetInput) -> Result<TransactionPlan> {
        validate_dataset_input(input)?;
        self.publish_call(
            "publish_dataset",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.description.clone()),
                Arg::String(input.format.clone()),
                Arg::U64(input.file_count),
                Arg::U64(input.size_bytes),
                Arg::String(input.license.clone()),
                Arg::StringVector(input.keywords.clone()),
            ],
            &input.content,
            &input.series_metadata,
            input.series_description.as_deref(),
            &input.version_metadata,
            input.version_change_note.as_deref(),
            input.payment_coin_id.as_ref(),
        )
    }

    pub fn publish_software_release(
        &self,
        input: &SoftwareReleaseInput,
    ) -> Result<TransactionPlan> {
        validate_software_release_input(input)?;
        self.publish_call(
            "publish_software_release",
            vec![
                Arg::String(input.project_name.clone()),
                Arg::String(input.version_name.clone()),
                Arg::String(input.source_hash.clone()),
                Arg::String(input.package_hash.clone()),
                Arg::String(input.changelog.clone()),
                Arg::String(input.license.clone()),
                Arg::String(input.repository_url.clone()),
            ],
            &input.content,
            &input.series_metadata,
            input.series_description.as_deref(),
            &input.version_metadata,
            input.version_change_note.as_deref(),
            input.payment_coin_id.as_ref(),
        )
    }

    pub fn publish_generic_file(&self, input: &GenericFileInput) -> Result<TransactionPlan> {
        validate_generic_file_input(input)?;
        self.publish_call(
            "publish_generic_file",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.description.clone()),
                Arg::String(input.filename.clone()),
                Arg::U64(input.file_size),
                Arg::String(input.license.clone()),
            ],
            &input.content,
            &input.series_metadata,
            input.series_description.as_deref(),
            &input.version_metadata,
            input.version_change_note.as_deref(),
            input.payment_coin_id.as_ref(),
        )
    }

    pub fn add_preprint_version(
        &self,
        input: &AddVersionInput<PreprintInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_preprint_input(&input.body)?;
        self.add_version_call(
            "add_preprint_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.abstract_text.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::StringVector(input.body.keywords.clone()),
                Arg::String(input.body.field.clone()),
                Arg::String(input.body.license.clone()),
                Arg::U64(input.body.page_count),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_preprint_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<PreprintInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_preprint_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_preprint_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.abstract_text.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::StringVector(input.body.keywords.clone()),
                Arg::String(input.body.field.clone()),
                Arg::String(input.body.license.clone()),
                Arg::U64(input.body.page_count),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_blog_post_version(
        &self,
        input: &AddVersionInput<BlogPostInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_blog_post_input(&input.body)?;
        self.add_version_call(
            "add_blog_post_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.summary.clone()),
                Arg::StringVector(input.body.tags.clone()),
                Arg::String(input.body.language.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_blog_post_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<BlogPostInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_blog_post_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_blog_post_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.summary.clone()),
                Arg::StringVector(input.body.tags.clone()),
                Arg::String(input.body.language.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_technical_report_version(
        &self,
        input: &AddVersionInput<TechnicalReportInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_technical_report_input(&input.body)?;
        self.add_version_call(
            "add_technical_report_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.abstract_text.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::String(input.body.organization.clone()),
                Arg::String(input.body.report_number.clone()),
                Arg::StringVector(input.body.keywords.clone()),
                Arg::String(input.body.license.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_technical_report_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<TechnicalReportInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_technical_report_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_technical_report_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.abstract_text.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::String(input.body.organization.clone()),
                Arg::String(input.body.report_number.clone()),
                Arg::StringVector(input.body.keywords.clone()),
                Arg::String(input.body.license.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_dataset_version(
        &self,
        input: &AddVersionInput<DatasetInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_dataset_input(&input.body)?;
        self.add_version_call(
            "add_dataset_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.description.clone()),
                Arg::String(input.body.format.clone()),
                Arg::U64(input.body.file_count),
                Arg::U64(input.body.size_bytes),
                Arg::String(input.body.license.clone()),
                Arg::StringVector(input.body.keywords.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_dataset_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<DatasetInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_dataset_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_dataset_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.description.clone()),
                Arg::String(input.body.format.clone()),
                Arg::U64(input.body.file_count),
                Arg::U64(input.body.size_bytes),
                Arg::String(input.body.license.clone()),
                Arg::StringVector(input.body.keywords.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_software_release_version(
        &self,
        input: &AddVersionInput<SoftwareReleaseInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_software_release_input(&input.body)?;
        self.add_version_call(
            "add_software_release_version",
            &input.series_id,
            vec![
                Arg::String(input.body.project_name.clone()),
                Arg::String(input.body.version_name.clone()),
                Arg::String(input.body.source_hash.clone()),
                Arg::String(input.body.package_hash.clone()),
                Arg::String(input.body.changelog.clone()),
                Arg::String(input.body.license.clone()),
                Arg::String(input.body.repository_url.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_software_release_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<SoftwareReleaseInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_software_release_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_software_release_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.project_name.clone()),
                Arg::String(input.body.version_name.clone()),
                Arg::String(input.body.source_hash.clone()),
                Arg::String(input.body.package_hash.clone()),
                Arg::String(input.body.changelog.clone()),
                Arg::String(input.body.license.clone()),
                Arg::String(input.body.repository_url.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_generic_file_version(
        &self,
        input: &AddVersionInput<GenericFileInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_generic_file_input(&input.body)?;
        self.add_version_call(
            "add_generic_file_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.description.clone()),
                Arg::String(input.body.filename.clone()),
                Arg::U64(input.body.file_size),
                Arg::String(input.body.license.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            false,
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn add_generic_file_version_with_controller(
        &self,
        input: &AddVersionWithControllerInput<GenericFileInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_generic_file_input(&input.body)?;
        self.add_version_with_controller_call(
            "add_generic_file_version_with_controller",
            &input.series_id,
            &input.control_record_id,
            &input.controller_nft_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.description.clone()),
                Arg::String(input.body.filename.clone()),
                Arg::U64(input.body.file_size),
                Arg::String(input.body.license.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.version_change_note.as_deref(),
            input.body.payment_coin_id.as_ref(),
        )
    }

    pub fn update_series_metadata(
        &self,
        series_id: &str,
        metadata: Vec<MetadataAttribute>,
    ) -> Result<TransactionPlan> {
        validate_object_id(series_id)?;
        validate_metadata_attributes(&metadata)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .publishing_target("update_series_metadata_extensions"),
            arguments: vec![
                Arg::Object(series_id.to_string()),
                Arg::MetadataVector(metadata),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn update_series_metadata_with_controller(
        &self,
        input: &UpdateSeriesMetadataWithControllerInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_metadata_attributes(&input.metadata)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .publishing_target("update_series_metadata_extensions_with_controller"),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.control_record_id.clone()),
                Arg::Object(input.controller_nft_id.clone()),
                Arg::MetadataVector(input.metadata.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn update_series_description(
        &self,
        series_id: &str,
        description: &str,
    ) -> Result<TransactionPlan> {
        validate_object_id(series_id)?;
        validate_required_text(
            "description",
            description,
            PROTOCOL_LIMITS.max_long_text_bytes,
        )?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target("update_series_description"),
            arguments: vec![
                Arg::Object(series_id.to_string()),
                Arg::String(description.to_string()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn update_series_description_with_controller(
        &self,
        input: &UpdateSeriesDescriptionWithControllerInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_required_text(
            "description",
            &input.description,
            PROTOCOL_LIMITS.max_long_text_bytes,
        )?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .publishing_target("update_series_description_with_controller"),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.control_record_id.clone()),
                Arg::Object(input.controller_nft_id.clone()),
                Arg::String(input.description.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn transfer_artifact_owner(
        &self,
        input: &TransferArtifactOwnerInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.comments_tree_id)?;
        validate_address(&input.new_owner)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target("transfer_artifact_owner"),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.comments_tree_id.clone()),
                Arg::Address(input.new_owner.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn transfer_artifact_owner_with_controller(
        &self,
        input: &TransferArtifactOwnerWithControllerInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.comments_tree_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        validate_address(&input.new_owner)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .publishing_target("transfer_artifact_owner_with_controller"),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.comments_tree_id.clone()),
                Arg::Object(input.control_record_id.clone()),
                Arg::Object(input.controller_nft_id.clone()),
                Arg::Address(input.new_owner.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn promote_existing_series_to_dual_mode(
        &self,
        input: &PromoteExistingSeriesToDualModeInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.comments_tree_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self
                .base
                .publishing_target("promote_existing_series_to_dual_mode"),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.comments_tree_id.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    pub fn promote_existing_series_to_controller_primary(
        &self,
        input: &PromoteExistingSeriesControllerModeInput,
    ) -> Result<TransactionPlan> {
        self.controller_promotion_call("promote_existing_series_to_controller_primary", input)
    }

    pub fn promote_existing_series_to_controller_only(
        &self,
        input: &PromoteExistingSeriesControllerModeInput,
    ) -> Result<TransactionPlan> {
        self.controller_promotion_call("promote_existing_series_to_controller_only", input)
    }

    pub fn sync_existing_series_control_mirrors(
        &self,
        input: &PromoteExistingSeriesControllerModeInput,
    ) -> Result<TransactionPlan> {
        self.controller_promotion_call("sync_existing_series_control_mirrors", input)
    }

    pub fn repair_existing_series_control_mirrors(
        &self,
        input: &PromoteExistingSeriesControllerModeInput,
    ) -> Result<TransactionPlan> {
        self.controller_promotion_call("repair_existing_series_control_mirrors", input)
    }

    fn controller_promotion_call(
        &self,
        function: &str,
        input: &PromoteExistingSeriesControllerModeInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.comments_tree_id)?;
        validate_object_id(&input.control_record_id)?;
        validate_object_id(&input.controller_nft_id)?;
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments: vec![
                Arg::Object(input.series_id.clone()),
                Arg::Object(input.comments_tree_id.clone()),
                Arg::Object(input.control_record_id.clone()),
                Arg::Object(input.controller_nft_id.clone()),
                Arg::Object(self.base.deployment.objects.clock.clone()),
            ],
        }))
    }

    fn publish_call(
        &self,
        function: &str,
        args: Vec<Arg>,
        content: &CommonContentInput,
        series_metadata: &[MetadataAttribute],
        series_description: Option<&str>,
        version_metadata: &[MetadataAttribute],
        version_change_note: Option<&str>,
        payment_coin_id: Option<&String>,
    ) -> Result<TransactionPlan> {
        self.publish_call_with_context(
            function,
            PublishCallContext {
                prefix: Vec::new(),
                args,
                content,
                series_metadata,
                series_description,
                version_metadata,
                version_change_note,
                require_version_change_note: false,
                payment_coin_id,
            },
        )
    }

    fn publish_call_with_context(
        &self,
        function: &str,
        mut context: PublishCallContext<'_>,
    ) -> Result<TransactionPlan> {
        let mut arguments = Vec::new();
        arguments.append(&mut context.prefix);
        arguments.extend([
            Arg::Object(self.base.deployment.objects.root.clone()),
            Arg::Object(self.base.deployment.objects.type_registry.clone()),
            Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            Arg::Object(self.base.deployment.objects.fee_manager.clone()),
        ]);
        arguments.append(&mut context.args);
        append_content_args(&mut arguments, context.content);
        arguments.extend([
            Arg::MetadataVector(self.series_metadata_vector(
                context.series_metadata,
                context.series_description,
            )?),
            Arg::MetadataVector(self.version_metadata_vector(
                context.version_metadata,
                context.version_change_note,
                context.require_version_change_note,
            )?),
            self.base.sui_payment(context.payment_coin_id),
            Arg::Object(self.base.deployment.objects.clock.clone()),
        ]);
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments,
        }))
    }

    fn add_version_call(
        &self,
        function: &str,
        series_id: &str,
        mut args: Vec<Arg>,
        content: &CommonContentInput,
        version_metadata: &[MetadataAttribute],
        version_change_note: Option<&str>,
        require_version_change_note: bool,
        payment_coin_id: Option<&String>,
    ) -> Result<TransactionPlan> {
        let mut arguments = vec![
            Arg::Object(self.base.deployment.objects.root.clone()),
            Arg::Object(self.base.deployment.objects.type_registry.clone()),
            Arg::Object(series_id.to_string()),
            Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            Arg::Object(self.base.deployment.objects.fee_manager.clone()),
        ];
        arguments.append(&mut args);
        append_content_args(&mut arguments, content);
        arguments.extend([
            Arg::MetadataVector(self.version_metadata_vector(
                version_metadata,
                version_change_note,
                require_version_change_note,
            )?),
            self.base.sui_payment(payment_coin_id),
            Arg::Object(self.base.deployment.objects.clock.clone()),
        ]);
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments,
        }))
    }

    fn add_version_with_controller_call(
        &self,
        function: &str,
        series_id: &str,
        control_record_id: &str,
        controller_nft_id: &str,
        mut args: Vec<Arg>,
        content: &CommonContentInput,
        version_metadata: &[MetadataAttribute],
        version_change_note: Option<&str>,
        payment_coin_id: Option<&String>,
    ) -> Result<TransactionPlan> {
        let mut arguments = vec![
            Arg::Object(self.base.deployment.objects.root.clone()),
            Arg::Object(self.base.deployment.objects.type_registry.clone()),
            Arg::Object(series_id.to_string()),
            Arg::Object(control_record_id.to_string()),
            Arg::Object(controller_nft_id.to_string()),
            Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            Arg::Object(self.base.deployment.objects.fee_manager.clone()),
        ];
        arguments.append(&mut args);
        append_content_args(&mut arguments, content);
        arguments.extend([
            Arg::MetadataVector(self.version_metadata_vector(
                version_metadata,
                version_change_note,
                true,
            )?),
            self.base.sui_payment(payment_coin_id),
            Arg::Object(self.base.deployment.objects.clock.clone()),
        ]);
        Ok(TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments,
        }))
    }

    fn series_metadata_vector(
        &self,
        metadata: &[MetadataAttribute],
        description: Option<&str>,
    ) -> Result<Vec<MetadataAttribute>> {
        self.with_reserved_metadata(
            metadata,
            reserved_metadata_keys::SERIES_DESCRIPTION,
            description,
        )
    }

    fn version_metadata_vector(
        &self,
        metadata: &[MetadataAttribute],
        change_note: Option<&str>,
        require_change_note: bool,
    ) -> Result<Vec<MetadataAttribute>> {
        let merged = self.with_reserved_metadata(
            metadata,
            reserved_metadata_keys::VERSION_CHANGE_NOTE,
            change_note,
        )?;
        if require_change_note
            && !merged.iter().any(|item| {
                item.key == reserved_metadata_keys::VERSION_CHANGE_NOTE
                    && !item.value.trim().is_empty()
            })
        {
            return Err(PaperProofError::invalid_input(
                "version_change_note",
                "version_change_note is required for controller-aware add-version flows",
            ));
        }
        Ok(merged)
    }

    fn with_reserved_metadata(
        &self,
        metadata: &[MetadataAttribute],
        reserved_key: &str,
        reserved_value: Option<&str>,
    ) -> Result<Vec<MetadataAttribute>> {
        let mut next = metadata
            .iter()
            .filter(|item| item.key != reserved_key)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(value) = reserved_value {
            validate_required_text(
                reserved_key,
                value,
                PROTOCOL_LIMITS.max_metadata_value_bytes,
            )?;
            next.push(MetadataAttribute {
                key: reserved_key.to_string(),
                value: value.to_string(),
            });
        }
        validate_metadata_attributes(&next)?;
        Ok(next)
    }
}

struct PublishCallContext<'a> {
    prefix: Vec<Arg>,
    args: Vec<Arg>,
    content: &'a CommonContentInput,
    series_metadata: &'a [MetadataAttribute],
    series_description: Option<&'a str>,
    version_metadata: &'a [MetadataAttribute],
    version_change_note: Option<&'a str>,
    require_version_change_note: bool,
    payment_coin_id: Option<&'a String>,
}

fn append_content_args(args: &mut Vec<Arg>, content: &CommonContentInput) {
    args.extend([
        Arg::String(content.content_hash.clone()),
        Arg::String(content.walrus_blob_id.clone()),
        Arg::String(content.walrus_blob_object_id.clone()),
        Arg::String(content.content_type.clone()),
    ]);
}
