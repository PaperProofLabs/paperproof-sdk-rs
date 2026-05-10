// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Reverse;

use crate::{
    constants::{ONE_PPRF, PPRF_DECIMALS},
    error::{PaperProofError, Result},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoinLike {
    pub coin_object_id: String,
    pub balance: u64,
    pub version: Option<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoinSelection {
    pub owner: String,
    pub coin_type: String,
    pub amount: u64,
    pub total_selected: u64,
    pub exact: bool,
    pub coins: Vec<CoinLike>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoinSummary {
    pub owner: String,
    pub coin_type: String,
    pub total_balance: u64,
    pub coin_count: usize,
    pub largest_coin: Option<CoinLike>,
    pub coins: Vec<CoinLike>,
}

pub fn pprf_to_base_units(value: &str) -> Result<u64> {
    decimal_to_base_units(value, PPRF_DECIMALS)
}

pub fn decimal_to_base_units(value: &str, decimals: u8) -> Result<u64> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PaperProofError::invalid_input(
            "amount",
            "must not be empty",
        ));
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("0");
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return Err(PaperProofError::invalid_input(
            "amount",
            "must contain at most one decimal point",
        ));
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(PaperProofError::invalid_input(
            "amount",
            "must be a positive decimal number",
        ));
    }
    if fraction.len() > decimals as usize {
        return Err(PaperProofError::invalid_input(
            "amount",
            format!("supports at most {decimals} decimal places"),
        ));
    }
    let scale = 10u64.pow(decimals as u32);
    let whole_units = whole
        .parse::<u64>()
        .map_err(|_| PaperProofError::invalid_input("amount", "whole part is too large"))?
        .checked_mul(scale)
        .ok_or_else(|| PaperProofError::invalid_input("amount", "amount is too large"))?;
    let mut fraction_padded = fraction.to_string();
    while fraction_padded.len() < decimals as usize {
        fraction_padded.push('0');
    }
    let fraction_units = if fraction_padded.is_empty() {
        0
    } else {
        fraction_padded
            .parse::<u64>()
            .map_err(|_| PaperProofError::invalid_input("amount", "fraction part is too large"))?
    };
    whole_units
        .checked_add(fraction_units)
        .ok_or_else(|| PaperProofError::invalid_input("amount", "amount is too large"))
}

pub fn base_units_to_pprf(value: u64) -> String {
    let whole = value / ONE_PPRF;
    let fraction = value % ONE_PPRF;
    if fraction == 0 {
        return whole.to_string();
    }
    let mut fraction = format!("{fraction:09}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{whole}.{fraction}")
}

pub fn summarize_coins(owner: &str, coin_type: &str, coins: &[CoinLike]) -> CoinSummary {
    let mut sorted = coins.to_vec();
    sorted.sort_by_key(|coin| Reverse(coin.balance));
    let total_balance = sorted
        .iter()
        .fold(0u64, |sum, coin| sum.saturating_add(coin.balance));
    CoinSummary {
        owner: owner.to_string(),
        coin_type: coin_type.to_string(),
        total_balance,
        coin_count: sorted.len(),
        largest_coin: sorted.first().cloned(),
        coins: sorted,
    }
}

pub fn select_coins_covering(
    owner: &str,
    coin_type: &str,
    coins: &[CoinLike],
    amount: u64,
) -> Result<CoinSelection> {
    if amount == 0 {
        return Err(PaperProofError::invalid_input(
            "amount",
            "coin selection amount must be positive",
        ));
    }
    let mut sorted = coins.to_vec();
    sorted.sort_by_key(|coin| Reverse(coin.balance));
    if let Some(single) = sorted.iter().find(|coin| coin.balance >= amount) {
        return Ok(CoinSelection {
            owner: owner.to_string(),
            coin_type: coin_type.to_string(),
            amount,
            total_selected: single.balance,
            exact: single.balance == amount,
            coins: vec![single.clone()],
        });
    }

    let mut selected = Vec::new();
    let mut total = 0u64;
    for coin in sorted {
        total = total.saturating_add(coin.balance);
        selected.push(coin);
        if total >= amount {
            break;
        }
    }
    if total < amount {
        return Err(PaperProofError::InsufficientBalance {
            owner: owner.to_string(),
            coin_type: coin_type.to_string(),
            required: amount,
            available: total,
            coin_count: coins.len(),
            purpose: "building a PaperProof coin argument".to_string(),
        });
    }
    Ok(CoinSelection {
        owner: owner.to_string(),
        coin_type: coin_type.to_string(),
        amount,
        total_selected: total,
        exact: total == amount,
        coins: selected,
    })
}
