// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    PaperProofClient,
    types::{CommonContentInput, PreprintInput},
};

fn main() -> paperproof_sdk_rs::Result<()> {
    let client = PaperProofClient::mainnet();
    let reservation_id = "0x1234";
    let plan = client.publishing.finalize_reserved_preprint(
        reservation_id,
        &PreprintInput {
            title: "Example preprint".to_string(),
            abstract_text: "A minimal PaperProof Rust SDK example.".to_string(),
            authors: vec!["PaperProof Labs".to_string()],
            keywords: vec!["example".to_string()],
            field: "computer science".to_string(),
            license: "CC-BY-4.0".to_string(),
            page_count: 1,
            content: CommonContentInput {
                content_hash: "sha256:example".to_string(),
                walrus_blob_id: "example-blob".to_string(),
                walrus_blob_object_id: "0x6".to_string(),
                content_type: "application/pdf".to_string(),
            },
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
