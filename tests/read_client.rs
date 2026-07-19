// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::read::{Balance, CoinObject, Page};
use serde_json::json;

#[test]
fn deserializes_sui_coin_page_shape() {
    let page: Page<CoinObject> = serde_json::from_value(json!({
        "data": [{
            "coinType": "0x2::sui::SUI",
            "coinObjectId": "0xcoin",
            "version": "1",
            "digest": "abc",
            "balance": "42"
        }],
        "nextCursor": null,
        "hasNextPage": false
    }))
    .unwrap();
    assert_eq!(page.data[0].coin_object_id, "0xcoin");
    assert!(!page.has_next_page);
}

#[test]
fn deserializes_balance_shape() {
    let balance: Balance = serde_json::from_value(json!({
        "coinType": "0x2::sui::SUI",
        "coinObjectCount": 3,
        "totalBalance": "1000",
        "lockedBalance": {}
    }))
    .unwrap();
    assert_eq!(balance.total_balance, "1000");
    assert_eq!(balance.coin_object_count, 3);
}
