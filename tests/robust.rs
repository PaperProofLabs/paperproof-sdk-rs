// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;

use paperproof_sdk_rs::{
    PaperProofError, ProviderExecutionOutput,
    robust::{
        RetryOptions, default_retryable, is_rebuild_retryable, normalize_provider_execution_output,
        with_retries,
    },
};
use serde_json::json;

#[test]
fn retries_retryable_errors_and_returns_value() {
    let attempts = Cell::new(0usize);
    let result = with_retries(
        "flaky test",
        &RetryOptions {
            base_delay_ms: 0,
            max_delay_ms: 0,
            ..Default::default()
        },
        || {
            attempts.set(attempts.get() + 1);
            if attempts.get() < 3 {
                Err(PaperProofError::network("rpc", "timeout"))
            } else {
                Ok(42)
            }
        },
    )
    .unwrap();
    assert_eq!(result, 42);
    assert_eq!(attempts.get(), 3);
}

#[test]
fn does_not_retry_non_retryable_validation_error() {
    let attempts = Cell::new(0usize);
    let error = with_retries::<(), _>("bad input", &RetryOptions::default(), || {
        attempts.set(attempts.get() + 1);
        Err(PaperProofError::invalid_input("title", "must not be empty"))
    })
    .unwrap_err();
    assert_eq!(attempts.get(), 1);
    assert!(error.to_string().contains("title"));
}

#[test]
fn classifies_common_retryable_errors() {
    assert!(default_retryable(&PaperProofError::network(
        "rpc",
        "503 temporarily unavailable"
    )));
    assert!(default_retryable(&PaperProofError::network(
        "rpc",
        "object not found"
    )));
    assert!(!default_retryable(&PaperProofError::invalid_input(
        "x", "bad"
    )));
}

#[test]
fn normalizes_provider_execution_output() {
    let output = ProviderExecutionOutput::NativeJson {
        status_success: false,
        digest: Some("abc".to_string()),
        json: json!({
            "effects": { "status": { "error": "MoveAbort in PaperProof" } },
            "events": [{
                "id": null,
                "packageId": "0x1",
                "transactionModule": "publishing",
                "sender": "0x2",
                "type": "0x1::publishing::ArtifactPublishedEvent",
                "parsedJson": {}
            }],
            "objectChanges": [],
            "balanceChanges": []
        }),
    };
    let normalized = normalize_provider_execution_output(&output);
    assert!(!normalized.success);
    assert_eq!(normalized.digest.as_deref(), Some("abc"));
    assert_eq!(normalized.events.len(), 1);
    assert!(normalized.error.unwrap().contains("MoveAbort"));
}

#[test]
fn classifies_rebuild_retryable_errors() {
    assert!(is_rebuild_retryable(
        &PaperProofError::TransactionExecution {
            message: "transaction needs to be rebuilt because object current version changed"
                .to_string()
        }
    ));
    assert!(!is_rebuild_retryable(&PaperProofError::invalid_input(
        "title", "empty"
    )));
}
