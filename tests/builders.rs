// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    PaperProofClient, TransactionPlan,
    constants::{comment_status, tree_status},
    deployment::mainnet_deployment,
    governance,
    transaction::MoveArgument,
    types::{
        AddOnchainCommentInput, AddVersionInput, CreateExecutableProposalInput,
        CreateSignalProposalInput, SetCommentStatusInput,
    },
};

#[test]
fn builds_publish_preprint_call() {
    let client = PaperProofClient::mainnet();
    let plan = client
        .publishing
        .publish_preprint(&common::sample_preprint())
        .unwrap();
    assert_eq!(plan.calls.len(), 1);
    let call = &plan.calls[0];
    assert!(call.target.ends_with("::publishing::publish_preprint"));
    assert_eq!(
        call.arguments[0],
        MoveArgument::Object(mainnet_deployment().objects.root)
    );
    assert_eq!(call.arguments.len(), 19);
}

#[test]
fn builds_add_software_release_version_call() {
    let client = PaperProofClient::mainnet();
    let plan = client
        .publishing
        .add_software_release_version(&AddVersionInput {
            series_id: "0x1234".to_string(),
            body: common::sample_software_release(),
        })
        .unwrap();
    assert!(
        plan.calls[0]
            .target
            .ends_with("::publishing::add_software_release_version")
    );
    assert_eq!(
        plan.calls[0].arguments[2],
        MoveArgument::Object("0x1234".to_string())
    );
}

#[test]
fn builds_all_add_version_calls() {
    let client = PaperProofClient::mainnet();
    let cases = [
        client
            .publishing
            .add_blog_post_version(&AddVersionInput {
                series_id: "0x1234".to_string(),
                body: common::sample_blog_post(),
            })
            .unwrap(),
        client
            .publishing
            .add_technical_report_version(&AddVersionInput {
                series_id: "0x1234".to_string(),
                body: common::sample_technical_report(),
            })
            .unwrap(),
        client
            .publishing
            .add_dataset_version(&AddVersionInput {
                series_id: "0x1234".to_string(),
                body: common::sample_dataset(),
            })
            .unwrap(),
        client
            .publishing
            .add_generic_file_version(&AddVersionInput {
                series_id: "0x1234".to_string(),
                body: common::sample_generic_file(),
            })
            .unwrap(),
    ];
    let expected = [
        "add_blog_post_version",
        "add_technical_report_version",
        "add_dataset_version",
        "add_generic_file_version",
    ];
    for (plan, expected) in cases.iter().zip(expected) {
        assert!(plan.calls[0].target.ends_with(expected));
        assert_eq!(
            plan.calls[0].arguments[2],
            MoveArgument::Object("0x1234".to_string())
        );
    }
}

#[test]
fn builds_executable_proposal_with_ts_argument_layout() {
    let client = PaperProofClient::mainnet();
    let plan = client
        .governance
        .create_proposal(&CreateExecutableProposalInput {
            proposal_type: None,
            action_type: governance::ACTION_SET_COMMENTS_FEE_LEVEL,
            title: "Set comments fee".to_string(),
            description: "Set comments fee to micro".to_string(),
            payload_u64_1: Some(1),
            payload_u64_2: Some(0),
            payload_address: None,
            payload_object_id: Some("0x9999".to_string()),
            payload_bytes: vec![1, 2, 3],
            stake_coin_id: "0x5678".to_string(),
        })
        .unwrap();
    let args = &plan.calls[0].arguments;
    assert!(
        plan.calls[0]
            .target
            .ends_with("::governance_voting::create_proposal")
    );
    assert_eq!(
        args[1],
        MoveArgument::U8(governance::PROPOSAL_TYPE_EXECUTABLE)
    );
    assert_eq!(
        args[2],
        MoveArgument::U8(governance::ACTION_SET_COMMENTS_FEE_LEVEL as u8)
    );
    assert!(matches!(args[8], MoveArgument::OptionalObjectId(Some(_))));
    assert_eq!(args[9], MoveArgument::Bytes(vec![1, 2, 3]));
    assert_eq!(args.len(), 11);
}

#[test]
fn builds_signal_proposal_via_executable_layout() {
    let client = PaperProofClient::mainnet();
    let plan = client
        .governance
        .create_signal_proposal(&CreateSignalProposalInput {
            title: "Signal".to_string(),
            description: "Signal description".to_string(),
            action_type: governance::ACTION_SIGNAL_TEXT,
            payload_text: Some("hello".to_string()),
            payload_address: None,
            stake_coin_id: "0x5678".to_string(),
        })
        .unwrap();
    let args = &plan.calls[0].arguments;
    assert_eq!(args[1], MoveArgument::U8(governance::PROPOSAL_TYPE_SIGNAL));
    assert_eq!(args[9], MoveArgument::Bytes(b"hello".to_vec()));
}

#[test]
fn execute_proposal_matches_contract_argument_order() {
    let client = PaperProofClient::mainnet();
    let deployment = mainnet_deployment();
    let plan = client.governance.execute_proposal("0x9999").unwrap();
    let args = &plan.calls[0].arguments;
    assert_eq!(
        args,
        &vec![
            MoveArgument::Object(deployment.objects.governance_config),
            MoveArgument::Object("0x9999".to_string()),
            MoveArgument::Object(deployment.objects.governance_vault),
            MoveArgument::Object(deployment.objects.clock),
        ]
    );
}

#[test]
fn builds_comment_and_status_calls() {
    let client = PaperProofClient::mainnet();
    let comment = client
        .comments
        .add_onchain_comment(&AddOnchainCommentInput {
            tree_id: "0x1234".to_string(),
            parent_comment_id: 0,
            content: b"hello".to_vec(),
            payment_coin_id: None,
        })
        .unwrap();
    assert!(
        comment.calls[0]
            .target
            .ends_with("::comments::add_onchain_comment")
    );
    assert_eq!(comment.calls[0].arguments.len(), 7);

    let status = client
        .comments
        .set_comment_status(&SetCommentStatusInput {
            tree_id: "0x1234".to_string(),
            comment_id: 1,
            status: comment_status::HIDDEN,
        })
        .unwrap();
    assert_eq!(
        status.calls[0].arguments[2],
        MoveArgument::U8(comment_status::HIDDEN)
    );

    let tree = client
        .comments
        .set_tree_status("0x1234", tree_status::LOCKED)
        .unwrap();
    assert_eq!(
        tree.calls[0].arguments[1],
        MoveArgument::U8(tree_status::LOCKED)
    );
}

#[test]
fn transaction_plan_can_batch_calls() {
    let client = PaperProofClient::mainnet();
    let mut plan = TransactionPlan::new();
    plan.calls.extend(client.ops.set_paused(true).calls);
    plan.calls.extend(client.ops.set_paused(false).calls);
    assert_eq!(plan.calls.len(), 2);
}
