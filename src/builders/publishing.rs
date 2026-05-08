// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use crate::{
    builders::base::BaseBuilder,
    deployment::Deployment,
    error::Result,
    transaction::{MoveArgument as Arg, MoveCall, TransactionPlan},
    types::{
        AddVersionInput, BlogPostInput, DatasetInput, GenericFileInput, MetadataAttribute,
        PreprintInput, SoftwareReleaseInput, TechnicalReportInput, TransferArtifactOwnerInput,
    },
    validation::{
        validate_blog_post_input, validate_dataset_input, validate_generic_file_input,
        validate_metadata_attributes, validate_object_id, validate_preprint_input,
        validate_software_release_input, validate_technical_report_input,
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
        Ok(self.publish_call(
            "publish_preprint",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.abstract_text.clone()),
                Arg::StringVector(input.authors.clone()),
                Arg::StringVector(input.keywords.clone()),
                Arg::String(input.field.clone()),
                Arg::String(input.license.clone()),
                Arg::U64(input.page_count),
            ],
            &input.content,
            &input.series_metadata,
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn publish_blog_post(&self, input: &BlogPostInput) -> Result<TransactionPlan> {
        validate_blog_post_input(input)?;
        Ok(self.publish_call(
            "publish_blog_post",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.summary.clone()),
                Arg::String(input.author_name.clone()),
                Arg::StringVector(input.tags.clone()),
                Arg::String(input.license.clone()),
            ],
            &input.content,
            &input.series_metadata,
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn publish_technical_report(
        &self,
        input: &TechnicalReportInput,
    ) -> Result<TransactionPlan> {
        validate_technical_report_input(input)?;
        Ok(self.publish_call(
            "publish_technical_report",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.abstract_text.clone()),
                Arg::StringVector(input.authors.clone()),
                Arg::String(input.organization.clone()),
                Arg::String(input.report_number.clone()),
                Arg::String(input.field.clone()),
                Arg::String(input.license.clone()),
                Arg::U64(input.page_count),
            ],
            &input.content,
            &input.series_metadata,
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn publish_dataset(&self, input: &DatasetInput) -> Result<TransactionPlan> {
        validate_dataset_input(input)?;
        Ok(self.publish_call(
            "publish_dataset",
            vec![
                Arg::String(input.title.clone()),
                Arg::String(input.description.clone()),
                Arg::StringVector(input.authors.clone()),
                Arg::String(input.field.clone()),
                Arg::String(input.license.clone()),
                Arg::String(input.schema_hash.clone()),
                Arg::U64(input.record_count),
            ],
            &input.content,
            &input.series_metadata,
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn publish_software_release(
        &self,
        input: &SoftwareReleaseInput,
    ) -> Result<TransactionPlan> {
        validate_software_release_input(input)?;
        Ok(self.publish_call(
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
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn publish_generic_file(&self, input: &GenericFileInput) -> Result<TransactionPlan> {
        validate_generic_file_input(input)?;
        Ok(self.publish_call(
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
            &input.version_metadata,
            input.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_preprint_version(
        &self,
        input: &AddVersionInput<PreprintInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_preprint_input(&input.body)?;
        Ok(self.add_version_call(
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
            input.body.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_software_release_version(
        &self,
        input: &AddVersionInput<SoftwareReleaseInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_software_release_input(&input.body)?;
        Ok(self.add_version_call(
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
            input.body.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_blog_post_version(
        &self,
        input: &AddVersionInput<BlogPostInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_blog_post_input(&input.body)?;
        Ok(self.add_version_call(
            "add_blog_post_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.summary.clone()),
                Arg::String(input.body.author_name.clone()),
                Arg::StringVector(input.body.tags.clone()),
                Arg::String(input.body.license.clone()),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_technical_report_version(
        &self,
        input: &AddVersionInput<TechnicalReportInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_technical_report_input(&input.body)?;
        Ok(self.add_version_call(
            "add_technical_report_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.abstract_text.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::String(input.body.organization.clone()),
                Arg::String(input.body.report_number.clone()),
                Arg::String(input.body.field.clone()),
                Arg::String(input.body.license.clone()),
                Arg::U64(input.body.page_count),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_dataset_version(
        &self,
        input: &AddVersionInput<DatasetInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_dataset_input(&input.body)?;
        Ok(self.add_version_call(
            "add_dataset_version",
            &input.series_id,
            vec![
                Arg::String(input.body.title.clone()),
                Arg::String(input.body.description.clone()),
                Arg::StringVector(input.body.authors.clone()),
                Arg::String(input.body.field.clone()),
                Arg::String(input.body.license.clone()),
                Arg::String(input.body.schema_hash.clone()),
                Arg::U64(input.body.record_count),
            ],
            &input.body.content,
            &input.body.version_metadata,
            input.body.payment_coin_id.as_ref(),
        ))
    }

    pub fn add_generic_file_version(
        &self,
        input: &AddVersionInput<GenericFileInput>,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_generic_file_input(&input.body)?;
        Ok(self.add_version_call(
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
            input.body.payment_coin_id.as_ref(),
        ))
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

    pub fn transfer_artifact_owner(
        &self,
        input: &TransferArtifactOwnerInput,
    ) -> Result<TransactionPlan> {
        validate_object_id(&input.series_id)?;
        validate_object_id(&input.comments_tree_id)?;
        crate::validation::validate_address(&input.new_owner)?;
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

    fn publish_call(
        &self,
        function: &str,
        mut args: Vec<Arg>,
        content: &crate::types::CommonContentInput,
        series_metadata: &[MetadataAttribute],
        version_metadata: &[MetadataAttribute],
        payment_coin_id: Option<&String>,
    ) -> TransactionPlan {
        let mut arguments = vec![
            Arg::Object(self.base.deployment.objects.root.clone()),
            Arg::Object(self.base.deployment.objects.type_registry.clone()),
            Arg::Object(self.base.deployment.objects.governance_vault.clone()),
            Arg::Object(self.base.deployment.objects.fee_manager.clone()),
        ];
        arguments.append(&mut args);
        append_content_args(&mut arguments, content);
        arguments.extend([
            Arg::MetadataVector(series_metadata.to_vec()),
            Arg::MetadataVector(version_metadata.to_vec()),
            self.base.sui_payment(payment_coin_id),
            Arg::Object(self.base.deployment.objects.clock.clone()),
        ]);
        TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments,
        })
    }

    fn add_version_call(
        &self,
        function: &str,
        series_id: &str,
        mut args: Vec<Arg>,
        content: &crate::types::CommonContentInput,
        version_metadata: &[MetadataAttribute],
        payment_coin_id: Option<&String>,
    ) -> TransactionPlan {
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
            Arg::MetadataVector(version_metadata.to_vec()),
            self.base.sui_payment(payment_coin_id),
            Arg::Object(self.base.deployment.objects.clock.clone()),
        ]);
        TransactionPlan::single(MoveCall {
            target: self.base.publishing_target(function),
            arguments,
        })
    }
}

fn append_content_args(args: &mut Vec<Arg>, content: &crate::types::CommonContentInput) {
    args.extend([
        Arg::String(content.content_hash.clone()),
        Arg::String(content.walrus_blob_id.clone()),
        Arg::String(content.walrus_blob_object_id.clone()),
        Arg::String(content.content_type.clone()),
    ]);
}
