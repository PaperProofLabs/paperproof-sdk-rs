// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    PaperProofError,
    coin_utils::{CoinLike, select_coins_covering, summarize_coins},
};

fn coin(id: &str, balance: u64) -> CoinLike {
    CoinLike {
        coin_object_id: id.to_string(),
        balance,
        version: None,
        digest: None,
    }
}

#[test]
fn summarizes_and_sorts_coins() {
    let summary = summarize_coins(
        "0xowner",
        "0x2::sui::SUI",
        &[coin("0x1", 2), coin("0x2", 9)],
    );
    assert_eq!(summary.total_balance, 11);
    assert_eq!(summary.largest_coin.unwrap().coin_object_id, "0x2");
    assert_eq!(summary.coins[0].balance, 9);
}

#[test]
fn selects_single_largest_covering_coin() {
    let selection = select_coins_covering(
        "0xowner",
        "0x2::sui::SUI",
        &[coin("0xsmall", 3), coin("0xbig", 10)],
        8,
    )
    .unwrap();
    assert_eq!(selection.coins.len(), 1);
    assert_eq!(selection.coins[0].coin_object_id, "0xbig");
    assert_eq!(selection.total_selected, 10);
    assert!(!selection.exact);
}

#[test]
fn combines_coins_when_no_single_coin_covers() {
    let selection = select_coins_covering(
        "0xowner",
        "0x2::sui::SUI",
        &[coin("0x1", 3), coin("0x2", 4), coin("0x3", 5)],
        9,
    )
    .unwrap();
    assert_eq!(selection.coins.len(), 2);
    assert_eq!(selection.total_selected, 9);
    assert!(selection.exact);
}

#[test]
fn returns_structured_insufficient_balance() {
    let error =
        select_coins_covering("0xowner", "0x2::sui::SUI", &[coin("0x1", 3)], 9).unwrap_err();
    assert!(matches!(
        error,
        PaperProofError::InsufficientBalance {
            required: 9,
            available: 3,
            ..
        }
    ));
}
