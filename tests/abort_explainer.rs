// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::abort_explainer::{
    explain_paperproof_error, format_paperproof_error_explanation, parse_move_abort,
};

#[test]
fn parses_module_and_code_from_move_abort_text() {
    let abort = parse_move_abort(
        "MoveAbort(MoveLocation { module: ModuleId { address: 0xabc, name: publishing } }, 9)",
    )
    .unwrap();
    assert_eq!(abort.module.as_deref(), Some("publishing"));
    assert_eq!(abort.code, Some(9));
}

#[test]
fn explains_known_paperproof_abort() {
    let explanation = explain_paperproof_error(
        "Execution failed with MoveAbort: 0xabc::comments::add_onchain_comment aborted with code 4",
    );
    assert!(explanation.matched);
    assert_eq!(explanation.title, "Empty on-chain comment");
    assert!(format_paperproof_error_explanation(&explanation).contains("Suggestion"));
}

#[test]
fn explains_coin_and_gas_failures_without_move_abort() {
    let explanation = explain_paperproof_error("No valid gas coins found for the signer");
    assert!(explanation.matched);
    assert_eq!(explanation.title, "Gas or coin selection problem");
}
