// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    CliExecutionOptions, ExecutionMode, PaperProofClient, SuiCliExecutor,
    types::{AddOnchainCommentInput, CommonContentInput, PreprintInput},
};

fn main() -> paperproof_sdk_rs::Result<()> {
    if std::env::var("PAPERPROOF_RS_MAINNET_WRITE").ok().as_deref() != Some("1") {
        println!(
            "Set PAPERPROOF_RS_MAINNET_WRITE=1 to execute real mainnet writes. Default is safe/no-op."
        );
        return Ok(());
    }

    let sender = std::env::var("PAPERPROOF_RS_SENDER").ok();
    let execute = std::env::args().any(|arg| arg == "--execute");
    let mode = if execute {
        ExecutionMode::Execute
    } else {
        ExecutionMode::DryRun
    };

    let client = PaperProofClient::mainnet();
    let deployment = client.deployment.clone();
    let executor = SuiCliExecutor::mainnet();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    let tree_id = std::env::var("PAPERPROOF_RS_TREE_ID").ok();
    if let Some(tree_id) = tree_id {
        let comment = client
            .comments
            .add_onchain_comment(&AddOnchainCommentInput {
                tree_id,
                parent_comment_id: 0,
                content: format!("Rust SDK smoke comment {nonce}").into_bytes(),
                payment_coin_id: None,
            })?;
        let output = executor.run(
            &comment,
            &CliExecutionOptions {
                sender,
                gas_budget: Some(30_000_000),
                mode,
                ..Default::default()
            },
        )?;
        println!("comment output: {}", output.raw_stdout);
        if let Ok(result) = output.comment_result(&deployment) {
            println!("comment_id={}", result.comment_id);
        }
        return Ok(());
    }

    let owner = sender.clone().ok_or_else(|| {
        paperproof_sdk_rs::PaperProofError::invalid_input(
            "PAPERPROOF_RS_SENDER",
            "preprint reserve/finalize smoke requires an explicit sender address",
        )
    })?;
    let reserve = client.publishing.reserve_preprint_code(&owner)?;
    let reserve_output = executor.run(
        &reserve,
        &CliExecutionOptions {
            sender: sender.clone(),
            gas_budget: Some(50_000_000),
            mode: mode.clone(),
            ..Default::default()
        },
    )?;
    println!("reserve output: {}", reserve_output.raw_stdout);
    if !execute {
        println!(
            "Reserve dry run completed. Sui CLI dry-run output is text in recent CLI versions, so event parsing and finalize are only exercised with --execute."
        );
        return Ok(());
    }
    let reservation = reserve_output.preprint_reservation_result(&deployment)?;

    let publish = client.publishing.finalize_reserved_preprint(
        &reservation.reservation_id,
        &PreprintInput {
            title: format!("PaperProof Rust SDK smoke {nonce}"),
            abstract_text:
                "Rust SDK mainnet smoke artifact. Created by an explicit opt-in example."
                    .to_string(),
            authors: vec!["PaperProof Labs".to_string()],
            keywords: vec!["rust".to_string(), "sdk".to_string()],
            field: "computer science".to_string(),
            license: "CC-BY-4.0".to_string(),
            page_count: 1,
            content: CommonContentInput {
                content_hash: format!("sha256:paperproof-rs-smoke-{nonce}"),
                walrus_blob_id: format!("paperproof-rs-smoke-{nonce}"),
                walrus_blob_object_id: "0x6".to_string(),
                content_type: "text/plain".to_string(),
            },
            series_metadata: vec![],
            version_metadata: vec![],
            payment_coin_id: None,
        },
    )?;

    let output = executor.run(
        &publish,
        &CliExecutionOptions {
            sender: sender.clone(),
            gas_budget: Some(50_000_000),
            mode: mode.clone(),
            ..Default::default()
        },
    )?;
    println!("publish output: {}", output.raw_stdout);
    if let Ok(result) = output.publish_result(&deployment) {
        println!(
            "series_id={} version_id={} comments_tree_id={} likes_book_id={}",
            result.series_id, result.version_id, result.comments_tree_id, result.likes_book_id
        );
    }

    if !execute {
        println!("Dry run completed. Re-run with --execute to send the transaction.");
    }

    Ok(())
}
