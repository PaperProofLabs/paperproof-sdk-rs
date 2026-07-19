// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    PaperProofClient,
    transaction::MoveArgument,
    types::{
        BlogPostInput, CommonContentInput, DatasetInput, GenericFileInput, PreprintInput,
        SoftwareReleaseInput, TechnicalReportInput,
    },
    walrus::{WalrusCliClient, WalrusClient, parse_walrus_write_response},
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const OBJECT_ID: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SERIES_ID: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Clone, Debug)]
struct ContentPackage {
    bytes: Vec<u8>,
    reference: CommonContentInput,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "pdf" => "application/pdf",
        "md" => "text/markdown",
        "txt" => "text/plain",
        "svg" => "image/svg+xml",
        "csv" => "text/csv",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn fixture_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("content-formats")
}

fn read_fixture_files(root: &Path) -> Vec<serde_json::Value> {
    let mut paths = Vec::new();
    collect_files(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("fixture file is readable");
            serde_json::json!({
                "path": path.strip_prefix(root).expect("relative path").to_string_lossy().replace('\\', "/"),
                "contentType": content_type_for(&path),
                "sha256": sha256_hex(&bytes),
                "bytesHex": hex::encode(bytes),
            })
        })
        .collect()
}

fn collect_files(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("fixture dir is readable") {
        let entry = entry.expect("fixture entry is readable");
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, output);
        } else {
            output.push(path);
        }
    }
}

fn content_package(kind: &str, root: &Path, declared_content_type: &str) -> ContentPackage {
    let manifest = serde_json::json!({
        "schema": "paperproof-content-package-v1",
        "kind": kind,
        "declaredContentType": declared_content_type,
        "files": read_fixture_files(root),
    });
    let mut bytes = serde_json::to_vec_pretty(&manifest).expect("manifest serializes");
    bytes.push(b'\n');
    let digest = sha256_hex(&bytes);
    ContentPackage {
        bytes,
        reference: CommonContentInput {
            content_hash: format!("sha256:{digest}"),
            walrus_blob_id: format!("local-{kind}-{}", &digest[..24]),
            walrus_blob_object_id: OBJECT_ID.to_string(),
            content_type: declared_content_type.to_string(),
        },
    }
}

fn string_arg(call: &paperproof_sdk_rs::MoveCall, index: usize) -> &str {
    match &call.arguments[index] {
        MoveArgument::String(value) => value,
        other => panic!("argument {index} is not string: {other:?}"),
    }
}

fn u64_arg(call: &paperproof_sdk_rs::MoveCall, index: usize) -> u64 {
    match &call.arguments[index] {
        MoveArgument::U64(value) => *value,
        other => panic!("argument {index} is not u64: {other:?}"),
    }
}

#[test]
fn content_format_matrix_drives_all_publish_and_add_version_abi() {
    let sdk = PaperProofClient::mainnet();
    let root = fixture_root();
    let preprint = content_package("preprint", &root.join("preprint"), "application/pdf").reference;
    let report = content_package(
        "technical_report",
        &root.join("technical-report"),
        "application/pdf",
    )
    .reference;
    let blog_md = content_package(
        "blog_post_markdown",
        &root.join("blog-post"),
        "application/vnd.paperproof.markdown-package+json",
    )
    .reference;
    let mut blog_text =
        content_package("blog_post_text", &root.join("blog-post"), "text/plain").reference;
    blog_text.content_type = "text/plain".to_string();
    let dataset = content_package(
        "dataset",
        &root.join("dataset"),
        "application/vnd.paperproof.dataset-package+json",
    )
    .reference;
    let software = content_package(
        "software_release",
        &root.join("software"),
        "application/vnd.paperproof.software-package+json",
    )
    .reference;
    let generic = content_package("generic_file", &root.join("generic"), "text/plain").reference;

    let finalize = sdk
        .publishing
        .finalize_reserved_preprint(
            SERIES_ID,
            &PreprintInput {
                title: "Fixture Preprint".to_string(),
                abstract_text: "Fixture abstract".to_string(),
                authors: vec!["PaperProof Labs".to_string()],
                keywords: vec!["fixture".to_string()],
                field: "computer science".to_string(),
                license: "CC-BY-4.0".to_string(),
                page_count: 1,
                content: preprint.clone(),
                series_description: Some("Fixture preprint series.".to_string()),
                version_change_note: Some("Fixture preprint v1.".to_string()),
                series_metadata: vec![],
                version_metadata: vec![],
                payment_coin_id: None,
            },
        )
        .unwrap();
    assert!(
        finalize.calls[0]
            .target
            .ends_with("finalize_reserved_preprint")
    );
    assert_eq!(finalize.calls[0].arguments.len(), 20);
    assert_eq!(string_arg(&finalize.calls[0], 12), preprint.content_hash);
    assert_eq!(string_arg(&finalize.calls[0], 15), "application/pdf");

    let blog = sdk
        .publishing
        .publish_blog_post(&BlogPostInput {
            title: "Fixture Blog".to_string(),
            summary: "Markdown package".to_string(),
            tags: vec!["fixture".to_string(), "markdown".to_string()],
            language: "en".to_string(),
            content: blog_md.clone(),
            series_description: Some("Fixture blog series.".to_string()),
            version_change_note: Some("Fixture blog v1.".to_string()),
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        })
        .unwrap();
    assert_eq!(blog.calls[0].arguments.len(), 16);
    assert_eq!(string_arg(&blog.calls[0], 8), blog_md.content_hash);
    assert_eq!(
        string_arg(&blog.calls[0], 11),
        "application/vnd.paperproof.markdown-package+json"
    );

    let plain_blog = sdk
        .publishing
        .add_blog_post_version(&paperproof_sdk_rs::types::AddVersionInput {
            series_id: SERIES_ID.to_string(),
            body: BlogPostInput {
                title: "Plain Text Blog".to_string(),
                summary: "Plain text".to_string(),
                tags: vec!["fixture".to_string(), "text".to_string()],
                language: "en".to_string(),
                content: blog_text.clone(),
                series_description: Some("Fixture blog series.".to_string()),
                version_change_note: Some("Fixture plain text blog version.".to_string()),
                series_metadata: vec![],
                version_metadata: vec![],
                payment_coin_id: None,
            },
        })
        .unwrap();
    assert_eq!(plain_blog.calls[0].arguments.len(), 16);
    assert_eq!(string_arg(&plain_blog.calls[0], 12), "text/plain");

    let technical_report = sdk
        .publishing
        .publish_technical_report(&TechnicalReportInput {
            title: "Fixture Report".to_string(),
            abstract_text: "Report abstract".to_string(),
            authors: vec!["PaperProof Labs".to_string()],
            organization: "PaperProof Labs".to_string(),
            report_number: "PPRF-TR-FIXTURE".to_string(),
            keywords: vec!["fixture".to_string()],
            license: "CC-BY-4.0".to_string(),
            content: report.clone(),
            series_description: Some("Fixture report series.".to_string()),
            version_change_note: Some("Fixture report v1.".to_string()),
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        })
        .unwrap();
    assert_eq!(technical_report.calls[0].arguments.len(), 19);
    assert_eq!(
        string_arg(&technical_report.calls[0], 11),
        report.content_hash
    );

    let dataset_plan = sdk
        .publishing
        .publish_dataset(&DatasetInput {
            title: "Fixture Dataset".to_string(),
            description: "Dataset package".to_string(),
            format: "paperproof-package-json".to_string(),
            file_count: 2,
            size_bytes: dataset.content_hash.len() as u64,
            license: "CC0-1.0".to_string(),
            keywords: vec!["fixture".to_string(), "dataset".to_string()],
            content: dataset.clone(),
            series_description: Some("Fixture dataset series.".to_string()),
            version_change_note: Some("Fixture dataset v1.".to_string()),
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        })
        .unwrap();
    assert_eq!(dataset_plan.calls[0].arguments.len(), 19);
    assert_eq!(u64_arg(&dataset_plan.calls[0], 7), 2);
    assert_eq!(string_arg(&dataset_plan.calls[0], 11), dataset.content_hash);

    let software_plan = sdk
        .publishing
        .publish_software_release(&SoftwareReleaseInput {
            project_name: "paperproof-fixture".to_string(),
            version_name: "0.0.1".to_string(),
            source_hash: software.content_hash.clone(),
            package_hash: software.content_hash.clone(),
            changelog: "Fixture package".to_string(),
            license: "Apache-2.0".to_string(),
            repository_url: "https://github.com/PaperProofLabs/paperproof-sdk-rs".to_string(),
            content: software.clone(),
            series_description: Some("Fixture software series.".to_string()),
            version_change_note: Some("Fixture software v1.".to_string()),
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        })
        .unwrap();
    assert_eq!(software_plan.calls[0].arguments.len(), 19);
    assert_eq!(
        string_arg(&software_plan.calls[0], 11),
        software.content_hash
    );

    let generic_plan = sdk
        .publishing
        .publish_generic_file(&GenericFileInput {
            title: "Fixture Generic File".to_string(),
            description: "Plain text generic file".to_string(),
            filename: "notes.txt".to_string(),
            file_size: 64,
            license: "Apache-2.0".to_string(),
            content: generic.clone(),
            series_description: Some("Fixture generic file series.".to_string()),
            version_change_note: Some("Fixture generic file v1.".to_string()),
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        })
        .unwrap();
    assert_eq!(generic_plan.calls[0].arguments.len(), 17);
    assert_eq!(string_arg(&generic_plan.calls[0], 9), generic.content_hash);
}

#[tokio::test]
async fn real_walrus_round_trip_for_every_fixture_package() -> paperproof_sdk_rs::Result<()> {
    if std::env::var("PAPERPROOF_REAL_WALRUS").ok().as_deref() != Some("1") {
        eprintln!("Set PAPERPROOF_REAL_WALRUS=1 to run real Walrus writes.");
        return Ok(());
    }
    let root = fixture_root();
    let aggregator = std::env::var("PAPERPROOF_WALRUS_AGGREGATOR_URL")
        .unwrap_or_else(|_| "https://aggregator.walrus-mainnet.walrus.space".to_string());
    let use_cli = std::env::var("PAPERPROOF_WALRUS_USE_CLI").ok().as_deref() == Some("1");
    let http_publisher = std::env::var("PAPERPROOF_WALRUS_PUBLISHER_URL").ok();
    for (kind, dirname) in [
        ("preprint", "preprint"),
        ("technical_report", "technical-report"),
        ("blog_post", "blog-post"),
        ("dataset", "dataset"),
        ("software_release", "software"),
        ("generic_file", "generic"),
    ] {
        let package = content_package(
            kind,
            &root.join(dirname),
            "application/vnd.paperproof.content-package+json",
        );
        let (blob_id, blob_object_id) = if use_cli {
            let service = paperproof_sdk_rs::PaperProofContentService::new(
                WalrusCliClient::new(
                    std::env::var("PAPERPROOF_WALRUS_CLI").unwrap_or_else(|_| "walrus".to_string()),
                )
                .with_aggregator_url(aggregator.clone()),
            );
            let published = service
                .publish_content(
                    package.bytes.clone(),
                    paperproof_sdk_rs::ContentPublishOptions {
                        epochs: 1,
                        content_type: Some(
                            "application/vnd.paperproof.content-package+json".to_string(),
                        ),
                        share: false,
                        deletable: Some(true),
                    },
                )
                .await?;
            (published.blob_id, published.blob_object_id)
        } else if let Some(publisher) = &http_publisher {
            let client = WalrusClient::new(&aggregator, Some(publisher.clone()));
            let raw = client.write_blob(package.bytes.clone()).await?;
            parse_walrus_write_response(&raw)?
        } else {
            eprintln!(
                "set PAPERPROOF_WALRUS_PUBLISHER_URL or PAPERPROOF_WALRUS_USE_CLI=1 for real Walrus writes"
            );
            return Ok(());
        };
        let read_client = WalrusClient::new(&aggregator, None);
        let read = read_client
            .read_and_verify_sha256(
                &blob_id,
                package
                    .reference
                    .content_hash
                    .strip_prefix("sha256:")
                    .expect("sha256 prefix"),
            )
            .await?;
        assert_eq!(read.sha256_hex, package.reference.content_hash[7..]);
        assert!(!blob_object_id.unwrap_or_default().is_empty());
    }
    Ok(())
}
