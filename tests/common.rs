// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::types::{
    BlogPostInput, CommonContentInput, DatasetInput, GenericFileInput, MetadataAttribute,
    PreprintInput, SoftwareReleaseInput, TechnicalReportInput,
};

pub fn sample_content() -> CommonContentInput {
    CommonContentInput {
        content_hash: "sha256:abc".to_string(),
        walrus_blob_id: "walrus-blob".to_string(),
        walrus_blob_object_id: "0x1234".to_string(),
        content_type: "application/pdf".to_string(),
    }
}

pub fn sample_metadata() -> Vec<MetadataAttribute> {
    vec![MetadataAttribute {
        key: "source".to_string(),
        value: "test".to_string(),
    }]
}

pub fn sample_preprint() -> PreprintInput {
    PreprintInput {
        title: "A PaperProof Test Preprint".to_string(),
        abstract_text: "A local SDK test record.".to_string(),
        authors: vec!["PaperProof Labs".to_string()],
        keywords: vec!["sdk".to_string()],
        field: "computer science".to_string(),
        license: "CC-BY-4.0".to_string(),
        page_count: 12,
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}

#[allow(dead_code)]
pub fn sample_software_release() -> SoftwareReleaseInput {
    SoftwareReleaseInput {
        project_name: "paperproof-sdk-rs".to_string(),
        version_name: "0.1.0".to_string(),
        source_hash: "sha256:source".to_string(),
        package_hash: "sha256:package".to_string(),
        changelog: "Initial test release".to_string(),
        license: "Apache-2.0".to_string(),
        repository_url: "https://github.com/PaperProofLabs/paperproof-sdk-rs".to_string(),
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}

#[allow(dead_code)]
pub fn sample_blog_post() -> BlogPostInput {
    BlogPostInput {
        title: "PaperProof SDK blog".to_string(),
        summary: "A local SDK blog test.".to_string(),
        tags: vec!["sdk".to_string()],
        language: "en".to_string(),
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}

#[allow(dead_code)]
pub fn sample_technical_report() -> TechnicalReportInput {
    TechnicalReportInput {
        title: "PaperProof SDK report".to_string(),
        abstract_text: "A local SDK technical report test.".to_string(),
        authors: vec!["PaperProof Labs".to_string()],
        organization: "PaperProof Labs".to_string(),
        report_number: "PPRF-RS-001".to_string(),
        keywords: vec!["sdk".to_string()],
        license: "CC-BY-4.0".to_string(),
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}

#[allow(dead_code)]
pub fn sample_dataset() -> DatasetInput {
    DatasetInput {
        title: "PaperProof SDK dataset".to_string(),
        description: "A local SDK dataset test.".to_string(),
        format: "csv".to_string(),
        file_count: 1,
        size_bytes: 128,
        license: "CC-BY-4.0".to_string(),
        keywords: vec!["sdk".to_string()],
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}

#[allow(dead_code)]
pub fn sample_generic_file() -> GenericFileInput {
    GenericFileInput {
        title: "PaperProof SDK file".to_string(),
        description: "A local SDK generic file test.".to_string(),
        filename: "paperproof-sdk-rs.txt".to_string(),
        file_size: 128,
        license: "Apache-2.0".to_string(),
        content: sample_content(),
        series_metadata: sample_metadata(),
        version_metadata: sample_metadata(),
        payment_coin_id: None,
    }
}
