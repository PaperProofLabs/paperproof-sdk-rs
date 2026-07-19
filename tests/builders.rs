// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    PaperProofClient, TransactionPlan, TransactionValueRef,
    constants::{comment_status, tree_status},
    deployment::mainnet_deployment,
    governance,
    transaction::MoveArgument,
    types::{
        AddOnchainCommentInput, AddVersionInput, AddVersionWithControllerInput,
        CreateExecutableProposalInput, CreateSignalProposalInput,
        PromoteExistingSeriesControllerModeInput, SetCommentStatusInput,
        SetCommentStatusWithControllerInput, SetTreeStatusWithControllerInput,
        TransferArtifactOwnerWithControllerInput, TransferTreeOwnerWithControllerInput,
    },
};

#[test]
fn direct_publish_preprint_is_disabled() {
    let client = PaperProofClient::mainnet();
    let error = client
        .publishing
        .publish_preprint(&common::sample_preprint())
        .unwrap_err()
        .to_string();
    assert!(error.contains("direct preprint publishing is disabled"));
}

#[test]
fn builds_preprint_reserve_and_finalize_calls() {
    let client = PaperProofClient::mainnet();
    let owner = "0x1234";
    let plan = client.publishing.reserve_preprint_code(owner).unwrap();
    assert_eq!(plan.calls.len(), 1);
    assert_eq!(plan.transfers.len(), 1);
    let call = &plan.calls[0];
    assert!(call.target.ends_with("::publishing::reserve_preprint_code"));
    assert_eq!(
        call.arguments[0],
        MoveArgument::Object(mainnet_deployment().objects.root)
    );
    assert_eq!(call.arguments.len(), 5);
    assert_eq!(
        plan.transfers[0].objects,
        vec![TransactionValueRef::LastResult]
    );
    assert_eq!(plan.transfers[0].recipient, owner);

    let finalize = client
        .publishing
        .finalize_reserved_preprint("0xabcd", &common::sample_preprint())
        .unwrap();
    assert_eq!(finalize.calls.len(), 1);
    assert!(
        finalize.calls[0]
            .target
            .ends_with("::publishing::finalize_reserved_preprint")
    );
    assert_eq!(
        finalize.calls[0].arguments[0],
        MoveArgument::Object("0xabcd".to_string())
    );
    assert_eq!(finalize.calls[0].arguments.len(), 20);
}

#[test]
fn publishing_builders_match_current_contract_abi() {
    let client = PaperProofClient::mainnet();
    let deployment = mainnet_deployment();
    let preprint = common::sample_preprint();
    let finalize = client
        .publishing
        .finalize_reserved_preprint("0xabcd", &preprint)
        .unwrap();
    assert_call_shape(
        &finalize.calls[0],
        "finalize_reserved_preprint",
        20,
        &[
            (0, MoveArgument::Object("0xabcd".to_string())),
            (1, MoveArgument::Object(deployment.objects.root.clone())),
            (
                2,
                MoveArgument::Object(deployment.objects.type_registry.clone()),
            ),
            (
                3,
                MoveArgument::Object(deployment.objects.governance_vault.clone()),
            ),
            (
                4,
                MoveArgument::Object(deployment.objects.fee_manager.clone()),
            ),
            (5, MoveArgument::String(preprint.title.clone())),
            (6, MoveArgument::String(preprint.abstract_text.clone())),
            (7, MoveArgument::StringVector(preprint.authors.clone())),
            (8, MoveArgument::StringVector(preprint.keywords.clone())),
            (9, MoveArgument::String(preprint.field.clone())),
            (10, MoveArgument::String(preprint.license.clone())),
            (11, MoveArgument::U64(preprint.page_count)),
            (
                12,
                MoveArgument::String(preprint.content.content_hash.clone()),
            ),
            (
                13,
                MoveArgument::String(preprint.content.walrus_blob_id.clone()),
            ),
            (
                14,
                MoveArgument::String(preprint.content.walrus_blob_object_id.clone()),
            ),
            (
                15,
                MoveArgument::String(preprint.content.content_type.clone()),
            ),
            (19, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    assert!(matches!(
        finalize.calls[0].arguments[16],
        MoveArgument::MetadataVector(_)
    ));
    assert!(matches!(
        finalize.calls[0].arguments[17],
        MoveArgument::MetadataVector(_)
    ));
    assert!(matches!(
        finalize.calls[0].arguments[18],
        MoveArgument::OptionalObject(None)
    ));

    let content = common::sample_content();

    let blog = client
        .publishing
        .publish_blog_post(&common::sample_blog_post())
        .unwrap();
    assert_call_shape(
        &blog.calls[0],
        "publish_blog_post",
        16,
        &[
            (4, MoveArgument::String("PaperProof SDK blog".to_string())),
            (
                5,
                MoveArgument::String("A local SDK blog test.".to_string()),
            ),
            (6, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (7, MoveArgument::String("en".to_string())),
            (8, MoveArgument::String(content.content_hash.clone())),
            (9, MoveArgument::String(content.walrus_blob_id.clone())),
            (
                10,
                MoveArgument::String(content.walrus_blob_object_id.clone()),
            ),
            (11, MoveArgument::String(content.content_type.clone())),
            (15, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let report = client
        .publishing
        .publish_technical_report(&common::sample_technical_report())
        .unwrap();
    assert_call_shape(
        &report.calls[0],
        "publish_technical_report",
        19,
        &[
            (4, MoveArgument::String("PaperProof SDK report".to_string())),
            (
                5,
                MoveArgument::String("A local SDK technical report test.".to_string()),
            ),
            (
                6,
                MoveArgument::StringVector(vec!["PaperProof Labs".to_string()]),
            ),
            (7, MoveArgument::String("PaperProof Labs".to_string())),
            (8, MoveArgument::String("PPRF-RS-001".to_string())),
            (9, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (10, MoveArgument::String("CC-BY-4.0".to_string())),
            (11, MoveArgument::String(content.content_hash.clone())),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let dataset = client
        .publishing
        .publish_dataset(&common::sample_dataset())
        .unwrap();
    assert_call_shape(
        &dataset.calls[0],
        "publish_dataset",
        19,
        &[
            (
                4,
                MoveArgument::String("PaperProof SDK dataset".to_string()),
            ),
            (
                5,
                MoveArgument::String("A local SDK dataset test.".to_string()),
            ),
            (6, MoveArgument::String("csv".to_string())),
            (7, MoveArgument::U64(1)),
            (8, MoveArgument::U64(128)),
            (9, MoveArgument::String("CC-BY-4.0".to_string())),
            (10, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (11, MoveArgument::String(content.content_hash.clone())),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let software = client
        .publishing
        .publish_software_release(&common::sample_software_release())
        .unwrap();
    assert_call_shape(
        &software.calls[0],
        "publish_software_release",
        19,
        &[
            (4, MoveArgument::String("paperproof-sdk-rs".to_string())),
            (5, MoveArgument::String("0.1.0".to_string())),
            (6, MoveArgument::String("sha256:source".to_string())),
            (7, MoveArgument::String("sha256:package".to_string())),
            (8, MoveArgument::String("Initial test release".to_string())),
            (9, MoveArgument::String("Apache-2.0".to_string())),
            (
                10,
                MoveArgument::String(
                    "https://github.com/PaperProofLabs/paperproof-sdk-rs".to_string(),
                ),
            ),
            (11, MoveArgument::String(content.content_hash.clone())),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let generic = client
        .publishing
        .publish_generic_file(&common::sample_generic_file())
        .unwrap();
    assert_call_shape(
        &generic.calls[0],
        "publish_generic_file",
        17,
        &[
            (4, MoveArgument::String("PaperProof SDK file".to_string())),
            (
                5,
                MoveArgument::String("A local SDK generic file test.".to_string()),
            ),
            (6, MoveArgument::String("paperproof-sdk-rs.txt".to_string())),
            (7, MoveArgument::U64(128)),
            (8, MoveArgument::String("Apache-2.0".to_string())),
            (9, MoveArgument::String(content.content_hash.clone())),
            (16, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );
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
    let deployment = mainnet_deployment();
    let series = "0x1234".to_string();

    let preprint = client
        .publishing
        .add_preprint_version(&AddVersionInput {
            series_id: series.clone(),
            body: common::sample_preprint(),
        })
        .unwrap();
    assert_add_version_shape(
        &preprint.calls[0],
        "add_preprint_version",
        19,
        &[
            (
                5,
                MoveArgument::String("A PaperProof Test Preprint".to_string()),
            ),
            (
                6,
                MoveArgument::String("A local SDK test record.".to_string()),
            ),
            (
                7,
                MoveArgument::StringVector(vec!["PaperProof Labs".to_string()]),
            ),
            (8, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (9, MoveArgument::String("computer science".to_string())),
            (10, MoveArgument::String("CC-BY-4.0".to_string())),
            (11, MoveArgument::U64(12)),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let blog = client
        .publishing
        .add_blog_post_version(&AddVersionInput {
            series_id: series.clone(),
            body: common::sample_blog_post(),
        })
        .unwrap();
    assert_add_version_shape(
        &blog.calls[0],
        "add_blog_post_version",
        16,
        &[
            (5, MoveArgument::String("PaperProof SDK blog".to_string())),
            (
                6,
                MoveArgument::String("A local SDK blog test.".to_string()),
            ),
            (7, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (8, MoveArgument::String("en".to_string())),
            (15, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let report = client
        .publishing
        .add_technical_report_version(&AddVersionInput {
            series_id: series.clone(),
            body: common::sample_technical_report(),
        })
        .unwrap();
    assert_add_version_shape(
        &report.calls[0],
        "add_technical_report_version",
        19,
        &[
            (5, MoveArgument::String("PaperProof SDK report".to_string())),
            (
                6,
                MoveArgument::String("A local SDK technical report test.".to_string()),
            ),
            (
                7,
                MoveArgument::StringVector(vec!["PaperProof Labs".to_string()]),
            ),
            (8, MoveArgument::String("PaperProof Labs".to_string())),
            (9, MoveArgument::String("PPRF-RS-001".to_string())),
            (10, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (11, MoveArgument::String("CC-BY-4.0".to_string())),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let dataset = client
        .publishing
        .add_dataset_version(&AddVersionInput {
            series_id: series.clone(),
            body: common::sample_dataset(),
        })
        .unwrap();
    assert_add_version_shape(
        &dataset.calls[0],
        "add_dataset_version",
        19,
        &[
            (
                5,
                MoveArgument::String("PaperProof SDK dataset".to_string()),
            ),
            (
                6,
                MoveArgument::String("A local SDK dataset test.".to_string()),
            ),
            (7, MoveArgument::String("csv".to_string())),
            (8, MoveArgument::U64(1)),
            (9, MoveArgument::U64(128)),
            (10, MoveArgument::String("CC-BY-4.0".to_string())),
            (11, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let software = client
        .publishing
        .add_software_release_version(&AddVersionInput {
            series_id: series.clone(),
            body: common::sample_software_release(),
        })
        .unwrap();
    assert_add_version_shape(
        &software.calls[0],
        "add_software_release_version",
        19,
        &[
            (5, MoveArgument::String("paperproof-sdk-rs".to_string())),
            (6, MoveArgument::String("0.1.0".to_string())),
            (7, MoveArgument::String("sha256:source".to_string())),
            (8, MoveArgument::String("sha256:package".to_string())),
            (9, MoveArgument::String("Initial test release".to_string())),
            (10, MoveArgument::String("Apache-2.0".to_string())),
            (
                11,
                MoveArgument::String(
                    "https://github.com/PaperProofLabs/paperproof-sdk-rs".to_string(),
                ),
            ),
            (18, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let generic = client
        .publishing
        .add_generic_file_version(&AddVersionInput {
            series_id: series,
            body: common::sample_generic_file(),
        })
        .unwrap();
    assert_add_version_shape(
        &generic.calls[0],
        "add_generic_file_version",
        17,
        &[
            (5, MoveArgument::String("PaperProof SDK file".to_string())),
            (
                6,
                MoveArgument::String("A local SDK generic file test.".to_string()),
            ),
            (7, MoveArgument::String("paperproof-sdk-rs.txt".to_string())),
            (8, MoveArgument::U64(128)),
            (9, MoveArgument::String("Apache-2.0".to_string())),
            (16, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );
}

#[test]
fn controller_add_version_requires_version_change_note() {
    let client = PaperProofClient::mainnet();
    let mut body = common::sample_blog_post();
    body.version_change_note = None;
    let error = client
        .publishing
        .add_blog_post_version_with_controller(&AddVersionWithControllerInput {
            series_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            body,
        })
        .unwrap_err()
        .to_string();
    assert!(error.contains("version_change_note is required"));
}

#[test]
fn controller_builders_match_current_contract_abi() {
    let client = PaperProofClient::mainnet();
    let deployment = mainnet_deployment();

    let blog = client
        .publishing
        .add_blog_post_version_with_controller(&AddVersionWithControllerInput {
            series_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            body: common::sample_blog_post(),
        })
        .unwrap();
    assert_call_shape(
        &blog.calls[0],
        "add_blog_post_version_with_controller",
        18,
        &[
            (0, MoveArgument::Object(deployment.objects.root.clone())),
            (
                1,
                MoveArgument::Object(deployment.objects.type_registry.clone()),
            ),
            (2, MoveArgument::Object("0x1234".to_string())),
            (3, MoveArgument::Object("0x5678".to_string())),
            (4, MoveArgument::Object("0x9abc".to_string())),
            (
                5,
                MoveArgument::Object(deployment.objects.governance_vault.clone()),
            ),
            (
                6,
                MoveArgument::Object(deployment.objects.fee_manager.clone()),
            ),
            (7, MoveArgument::String("PaperProof SDK blog".to_string())),
            (
                8,
                MoveArgument::String("A local SDK blog test.".to_string()),
            ),
            (9, MoveArgument::StringVector(vec!["sdk".to_string()])),
            (10, MoveArgument::String("en".to_string())),
            (17, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );
    assert_reserved_note(
        &blog.calls[0].arguments[15],
        "version_change_note",
        "Initial blog fixture.",
    );
    assert_eq!(blog.calls[0].arguments[16], MoveArgument::OptionalObject(None));

    let transfer_owner = client
        .publishing
        .transfer_artifact_owner_with_controller(&TransferArtifactOwnerWithControllerInput {
            series_id: "0x1234".to_string(),
            comments_tree_id: "0x2222".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            new_owner:
                "0x1111111111111111111111111111111111111111111111111111111111111111"
                    .to_string(),
        })
        .unwrap();
    assert_call_shape(
        &transfer_owner.calls[0],
        "transfer_artifact_owner_with_controller",
        6,
        &[
            (0, MoveArgument::Object("0x1234".to_string())),
            (1, MoveArgument::Object("0x2222".to_string())),
            (2, MoveArgument::Object("0x5678".to_string())),
            (3, MoveArgument::Object("0x9abc".to_string())),
            (
                4,
                MoveArgument::Address(
                    "0x1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                ),
            ),
            (5, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let promote = client
        .publishing
        .promote_existing_series_to_controller_primary(
            &PromoteExistingSeriesControllerModeInput {
                series_id: "0x1234".to_string(),
                comments_tree_id: "0x2222".to_string(),
                control_record_id: "0x5678".to_string(),
                controller_nft_id: "0x9abc".to_string(),
            },
        )
        .unwrap();
    assert_call_shape(
        &promote.calls[0],
        "promote_existing_series_to_controller_primary",
        5,
        &[
            (0, MoveArgument::Object("0x1234".to_string())),
            (1, MoveArgument::Object("0x2222".to_string())),
            (2, MoveArgument::Object("0x5678".to_string())),
            (3, MoveArgument::Object("0x9abc".to_string())),
            (4, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );
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
fn controller_comment_builders_match_current_contract_abi() {
    let client = PaperProofClient::mainnet();
    let deployment = mainnet_deployment();

    let tree = client
        .comments
        .set_tree_status_with_controller(&SetTreeStatusWithControllerInput {
            tree_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            status: tree_status::LOCKED,
        })
        .unwrap();
    assert_call_shape(
        &tree.calls[0],
        "set_tree_status_with_controller",
        5,
        &[
            (0, MoveArgument::Object("0x1234".to_string())),
            (1, MoveArgument::Object("0x5678".to_string())),
            (2, MoveArgument::Object("0x9abc".to_string())),
            (3, MoveArgument::U8(tree_status::LOCKED)),
            (4, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let comment = client
        .comments
        .set_comment_status_with_controller(&SetCommentStatusWithControllerInput {
            tree_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            comment_id: 7,
            status: comment_status::HIDDEN,
        })
        .unwrap();
    assert_call_shape(
        &comment.calls[0],
        "set_comment_status_with_controller",
        6,
        &[
            (0, MoveArgument::Object("0x1234".to_string())),
            (1, MoveArgument::Object("0x5678".to_string())),
            (2, MoveArgument::Object("0x9abc".to_string())),
            (3, MoveArgument::U64(7)),
            (4, MoveArgument::U8(comment_status::HIDDEN)),
            (5, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
    );

    let transfer = client
        .comments
        .transfer_tree_owner_with_controller(&TransferTreeOwnerWithControllerInput {
            tree_id: "0x1234".to_string(),
            control_record_id: "0x5678".to_string(),
            controller_nft_id: "0x9abc".to_string(),
            new_owner:
                "0x1111111111111111111111111111111111111111111111111111111111111111"
                    .to_string(),
        })
        .unwrap();
    assert_call_shape(
        &transfer.calls[0],
        "transfer_tree_owner_with_controller",
        5,
        &[
            (0, MoveArgument::Object("0x1234".to_string())),
            (1, MoveArgument::Object("0x5678".to_string())),
            (2, MoveArgument::Object("0x9abc".to_string())),
            (
                3,
                MoveArgument::Address(
                    "0x1111111111111111111111111111111111111111111111111111111111111111"
                        .to_string(),
                ),
            ),
            (4, MoveArgument::Object(deployment.objects.clock.clone())),
        ],
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

fn assert_call_shape(
    call: &paperproof_sdk_rs::MoveCall,
    function: &str,
    argument_count: usize,
    expected_arguments: &[(usize, MoveArgument)],
) {
    assert!(call.target.ends_with(function), "target={}", call.target);
    assert_eq!(
        call.arguments.len(),
        argument_count,
        "target={}",
        call.target
    );
    for (index, expected) in expected_arguments {
        assert_eq!(
            call.arguments[*index], *expected,
            "target={} argument[{index}]",
            call.target
        );
    }
}

fn assert_add_version_shape(
    call: &paperproof_sdk_rs::MoveCall,
    function: &str,
    argument_count: usize,
    expected_arguments: &[(usize, MoveArgument)],
) {
    assert_call_shape(
        call,
        function,
        argument_count,
        &[(2, MoveArgument::Object("0x1234".to_string()))],
    );
    assert_call_shape(call, function, argument_count, expected_arguments);
}

fn assert_reserved_note(argument: &MoveArgument, key: &str, value: &str) {
    let MoveArgument::MetadataVector(items) = argument else {
        panic!("expected metadata vector, got {argument:?}");
    };
    assert!(
        items.iter().any(|item| item.key == key && item.value == value),
        "missing reserved metadata {key}={value} in {items:?}"
    );
}
