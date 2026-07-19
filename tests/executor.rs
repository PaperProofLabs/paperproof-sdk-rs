// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    CliExecutionOptions, ExecutionMode, PaperProofClient, SuiCliExecutor,
    transaction::MoveArgument, types::MetadataAttribute,
};

#[test]
fn renders_publish_plan_for_sui_cli_preview() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let plan = client
        .publishing
        .finalize_reserved_preprint("0xabcd", &common::sample_preprint())
        .unwrap();
    let args = executor
        .to_cli_args(
            &plan,
            &CliExecutionOptions {
                mode: ExecutionMode::Preview,
                ..Default::default()
            },
        )
        .unwrap();
    let joined = args.join(" ");
    assert!(joined.contains("finalize_reserved_preprint"));
    assert!(joined.contains("metadata_attribute"));
    assert!(joined.contains("<0x1::string::String>"));
    assert!(joined.contains("--preview"));
    assert!(joined.contains("--json"));
}

#[test]
fn renders_reserve_return_transfer_for_sui_cli_preview() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let owner = "0x1234";
    let plan = client.publishing.reserve_preprint_code(owner).unwrap();
    let args = executor
        .to_cli_args(&plan, &CliExecutionOptions::default())
        .unwrap();
    let joined = args.join(" ");
    assert!(joined.contains("reserve_preprint_code"));
    assert!(joined.contains("--assign last_result_0"));
    assert!(joined.contains("--transfer-objects [last_result_0] @0x1234"));
}

#[test]
fn renders_bytes_as_u8_move_vector() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let plan = client
        .comments
        .add_onchain_comment(&paperproof_sdk_rs::types::AddOnchainCommentInput {
            tree_id: "0x1234".to_string(),
            parent_comment_id: 0,
            content: b"hi".to_vec(),
            payment_coin_id: None,
        })
        .unwrap();
    let args = executor
        .to_cli_args(&plan, &CliExecutionOptions::default())
        .unwrap();
    assert!(args.iter().any(|arg| arg == "<u8>"));
    assert!(args.iter().any(|arg| arg == "[104,105]"));
}

#[test]
fn renders_empty_metadata_vector() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let plan = client
        .publishing
        .update_series_metadata(&paperproof_sdk_rs::types::UpdateSeriesMetadataInput {
            series_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            metadata: Vec::<MetadataAttribute>::new(),
        })
        .unwrap();
    let args = executor
        .to_cli_args(&plan, &CliExecutionOptions::default())
        .unwrap();
    assert!(args.iter().any(|arg| arg.contains("MetadataAttribute")));
    assert!(args.iter().any(|arg| arg == "[]"));
}

#[test]
fn proposal_payload_object_option_is_not_rendered_as_coin_option() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let plan = client
        .governance
        .create_proposal(&paperproof_sdk_rs::types::CreateExecutableProposalInput {
            proposal_type: None,
            title: "hello".to_string(),
            description: "world".to_string(),
            action_type: paperproof_sdk_rs::governance::ACTION_SIGNAL_ADDRESS,
            payload_u64_1: None,
            payload_u64_2: None,
            payload_address: Some("0x1234".to_string()),
            payload_object_id: Some("0x9999".to_string()),
            payload_bytes: vec![],
            stake_coin_id: "0x5678".to_string(),
        })
        .unwrap();
    assert!(
        plan.calls[0]
            .arguments
            .iter()
            .any(|arg| matches!(arg, MoveArgument::OptionalObjectId(Some(_))))
    );
    let args = executor
        .to_cli_args(&plan, &CliExecutionOptions::default())
        .unwrap();
    assert!(args.iter().any(|arg| arg == "<address>"));
}
