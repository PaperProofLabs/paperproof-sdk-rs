// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

mod common;

use paperproof_sdk_rs::{
    BuiltTransaction, CliExecutionOptions, ExecutionMode, PaperProofClient,
    PaperProofExecutionProvider, ProviderExecutionOptions, SuiCliExecutor,
};

#[tokio::test]
async fn cli_executor_implements_execution_provider_build() {
    let client = PaperProofClient::mainnet();
    let executor = SuiCliExecutor::mainnet();
    let plan = client
        .publishing
        .finalize_reserved_preprint("0xabcd", &common::sample_preprint())
        .unwrap();
    let built = executor
        .build_transaction(
            &plan,
            &ProviderExecutionOptions {
                mode: ExecutionMode::Preview,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let BuiltTransaction::SuiCliArgs(args) = built else {
        panic!("expected CLI args fallback");
    };
    assert!(args.contains(&"--preview".to_string()));
    assert!(args.contains(&"--json".to_string()));
}

#[test]
fn provider_options_roundtrip_from_cli_options() {
    let cli = CliExecutionOptions {
        sender: Some("0x1234".to_string()),
        gas_budget: Some(100),
        gas_coin: Some("0xabcd".to_string()),
        mode: ExecutionMode::DevInspect,
        ..Default::default()
    };
    let provider = ProviderExecutionOptions::from(&cli);
    assert_eq!(provider.sender.as_deref(), Some("0x1234"));
    assert_eq!(provider.gas_budget, Some(100));
    assert_eq!(provider.gas_coin.as_deref(), Some("0xabcd"));
    assert_eq!(provider.mode, ExecutionMode::DevInspect);
}
