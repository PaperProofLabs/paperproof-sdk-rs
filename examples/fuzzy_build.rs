// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    PaperProofClient,
    types::{AddOnchainCommentInput, AddVersionInput, MetadataAttribute},
};

fn main() -> paperproof_sdk_rs::Result<()> {
    let client = PaperProofClient::mainnet();
    let mut built = 0usize;
    let seed = std::env::var("PAPERPROOF_RS_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(20260509);
    let rounds = std::env::var("PAPERPROOF_RS_FUZZ_ROUNDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(128);
    let mut rng = TinyRng(seed);

    for index in 0..rounds {
        let mut preprint = paperproof_sdk_rs::types::PreprintInput {
            title: format!("Rust SDK fuzzy preprint {index} {}", rng.next()),
            abstract_text: format!("Fuzzy local build test {}", rng.next()),
            authors: vec![format!("Author {}", rng.next() % 10)],
            keywords: vec!["sdk".to_string(), format!("k{}", rng.next() % 100)],
            field: "computer science".to_string(),
            license: "CC-BY-4.0".to_string(),
            page_count: 1 + (rng.next() % 20),
            content: paperproof_sdk_rs::types::CommonContentInput {
                content_hash: format!("sha256:{:x}", rng.next()),
                walrus_blob_id: format!("walrus-{:x}", rng.next()),
                walrus_blob_object_id: "0x6".to_string(),
                content_type: "text/plain".to_string(),
            },
            series_metadata: vec![MetadataAttribute {
                key: "fuzz".to_string(),
                value: index.to_string(),
            }],
            version_metadata: vec![],
            payment_coin_id: None,
        };
        if rng.next().is_multiple_of(7) {
            preprint.version_metadata.push(MetadataAttribute {
                key: "note".to_string(),
                value: "extra".to_string(),
            });
        }
        client.publishing.publish_preprint(&preprint)?;
        built += 1;

        client.publishing.add_preprint_version(&AddVersionInput {
            series_id: "0x1234".to_string(),
            body: preprint,
        })?;
        built += 1;

        client
            .comments
            .add_onchain_comment(&AddOnchainCommentInput {
                tree_id: "0x1234".to_string(),
                parent_comment_id: rng.next() % 3,
                content: format!("fuzzy comment {}", rng.next()).into_bytes(),
                payment_coin_id: None,
            })?;
        built += 1;
    }

    println!("Built {built} PaperProof transaction plans with seed {seed}.");
    Ok(())
}

struct TinyRng(u64);

impl TinyRng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.0
    }
}
