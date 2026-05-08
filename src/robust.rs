// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::{thread::sleep, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{
    abort_explainer::{PaperProofErrorExplanation, explain_paperproof_error},
    error::{PaperProofError, Result},
    executor::{CliExecutionOptions, CliExecutionOutput, SuiCliExecutor},
    providers::{PaperProofExecutionProvider, ProviderExecutionOptions, ProviderExecutionOutput},
    transaction::TransactionPlan,
};

#[derive(Clone)]
pub struct RetryOptions {
    pub attempts: usize,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub retryable: fn(&PaperProofError) -> bool,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            attempts: 4,
            base_delay_ms: 1_500,
            max_delay_ms: 12_000,
            retryable: default_retryable,
        }
    }
}

impl std::fmt::Debug for RetryOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RetryOptions")
            .field("attempts", &self.attempts)
            .field("base_delay_ms", &self.base_delay_ms)
            .field("max_delay_ms", &self.max_delay_ms)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RobustExecution {
    pub output: CliExecutionOutput,
    pub success: bool,
    pub digest: Option<String>,
    pub explanation: Option<PaperProofErrorExplanation>,
    pub attempts: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RobustProviderExecution {
    pub output: ProviderExecutionOutput,
    pub success: bool,
    pub digest: Option<String>,
    pub explanation: Option<PaperProofErrorExplanation>,
    pub attempts: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NormalizedExecutionResult {
    pub digest: Option<String>,
    pub success: bool,
    pub expected_failure: bool,
    pub error: Option<String>,
    pub explanation: Option<PaperProofErrorExplanation>,
    pub events: Vec<crate::events::SuiEventEnvelope>,
    pub object_changes: Option<serde_json::Value>,
    pub balance_changes: Option<serde_json::Value>,
    pub raw: serde_json::Value,
}

#[derive(Clone, Debug)]
pub struct RobustProviderExecuteOptions {
    pub retry: RetryOptions,
    pub expect_failure: bool,
    pub rebuild_retry: bool,
}

impl Default for RobustProviderExecuteOptions {
    fn default() -> Self {
        Self {
            retry: RetryOptions::default(),
            expect_failure: false,
            rebuild_retry: true,
        }
    }
}

pub fn default_retryable(error: &PaperProofError) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    [
        "fetch failed",
        "timeout",
        "econnreset",
        "socket hang up",
        "429",
        "503",
        "504",
        "temporarily unavailable",
        "unavailable for consumption",
        "current version",
        "needs to be rebuilt",
        "dynamic field not found",
        "object not found",
    ]
    .iter()
    .any(|pattern| text.contains(pattern))
}

pub fn with_retries<T, F>(label: &str, options: &RetryOptions, mut operation: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let attempts = options.attempts.max(1);
    let mut last_error = None;
    for attempt in 1..=attempts {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= attempts || !(options.retryable)(&error) {
                    return Err(error);
                }
                last_error = Some(error);
                let delay = options
                    .base_delay_ms
                    .saturating_mul(attempt as u64)
                    .min(options.max_delay_ms);
                eprintln!(
                    "PaperProof retry {attempt}/{attempts} for {label} after {delay}ms: {}",
                    last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default()
                );
                sleep(Duration::from_millis(delay));
            }
        }
    }
    Err(
        last_error.unwrap_or_else(|| PaperProofError::TransactionExecution {
            message: format!("{label} failed without an error"),
        }),
    )
}

pub fn robust_execute_plan(
    executor: &SuiCliExecutor,
    plan: &TransactionPlan,
    options: &CliExecutionOptions,
    label: &str,
    retry: RetryOptions,
) -> Result<RobustExecution> {
    let mut attempts_used = 0usize;
    let output = with_retries(label, &retry, || {
        attempts_used += 1;
        executor.run(plan, options)
    })?;
    let explanation = if output.status_success {
        None
    } else {
        Some(explain_paperproof_error(format!(
            "{} {}",
            output.raw_stderr, output.raw_stdout
        )))
    };
    Ok(RobustExecution {
        success: output.status_success,
        digest: output.digest.clone(),
        output,
        explanation,
        attempts: attempts_used,
    })
}

pub async fn robust_execute_with_provider<P>(
    provider: &P,
    plan: &TransactionPlan,
    options: &ProviderExecutionOptions,
    label: &str,
    retry: RetryOptions,
) -> Result<RobustProviderExecution>
where
    P: PaperProofExecutionProvider,
{
    let attempts = retry.attempts.max(1);
    let mut last_error = None;
    for attempt in 1..=attempts {
        match provider.sign_and_execute(plan, options).await {
            Ok(output) => {
                let explanation = if output.status_success() {
                    None
                } else {
                    let raw = output
                        .json()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "transaction returned unsuccessful status".to_string());
                    Some(explain_paperproof_error(raw))
                };
                return Ok(RobustProviderExecution {
                    success: output.status_success(),
                    digest: output.digest().map(ToString::to_string),
                    output,
                    explanation,
                    attempts: attempt,
                });
            }
            Err(error) => {
                if attempt >= attempts || !(retry.retryable)(&error) {
                    return Err(error);
                }
                last_error = Some(error);
                let delay = retry
                    .base_delay_ms
                    .saturating_mul(attempt as u64)
                    .min(retry.max_delay_ms);
                eprintln!(
                    "PaperProof retry {attempt}/{attempts} for {label} after {delay}ms: {}",
                    last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default()
                );
                sleep(Duration::from_millis(delay));
            }
        }
    }
    Err(
        last_error.unwrap_or_else(|| PaperProofError::TransactionExecution {
            message: format!("{label} failed without an error"),
        }),
    )
}

pub fn normalize_provider_execution_output(
    output: &ProviderExecutionOutput,
) -> NormalizedExecutionResult {
    let raw = output.json().cloned().unwrap_or(serde_json::Value::Null);
    let error = execution_error(&raw);
    let explanation = error.as_ref().map(explain_paperproof_error);
    NormalizedExecutionResult {
        digest: output
            .digest()
            .map(ToString::to_string)
            .or_else(|| raw_digest(&raw)),
        success: output.status_success(),
        expected_failure: false,
        error,
        explanation,
        events: normalize_events(&raw),
        object_changes: raw
            .get("objectChanges")
            .or_else(|| raw.pointer("/Transaction/objectChanges"))
            .or_else(|| raw.pointer("/FailedTransaction/objectChanges"))
            .cloned(),
        balance_changes: raw
            .get("balanceChanges")
            .or_else(|| raw.pointer("/Transaction/balanceChanges"))
            .or_else(|| raw.pointer("/FailedTransaction/balanceChanges"))
            .cloned(),
        raw,
    }
}

pub async fn robust_execute_plan_normalized<P>(
    provider: &P,
    build: impl Fn() -> Result<TransactionPlan>,
    options: &ProviderExecutionOptions,
    label: &str,
    robust_options: RobustProviderExecuteOptions,
) -> Result<NormalizedExecutionResult>
where
    P: PaperProofExecutionProvider,
{
    let attempts = robust_options.retry.attempts.max(1);
    let mut last_error = None;
    for attempt in 1..=attempts {
        let plan = build()?;
        match provider.sign_and_execute(&plan, options).await {
            Ok(output) => {
                let mut normalized = normalize_provider_execution_output(&output);
                if robust_options.expect_failure {
                    if !normalized.success {
                        normalized.expected_failure = true;
                        return Ok(normalized);
                    }
                    return Err(PaperProofError::TransactionExecution {
                        message: format!(
                            "{label} was expected to fail but succeeded with {:?}",
                            normalized.digest
                        ),
                    });
                }
                if normalized.success {
                    return Ok(normalized);
                }
                return Err(PaperProofError::TransactionExecution {
                    message: transaction_failure_message(label, &normalized),
                });
            }
            Err(error) => {
                if robust_options.expect_failure {
                    let text = error.to_string();
                    return Ok(NormalizedExecutionResult {
                        digest: None,
                        success: false,
                        expected_failure: true,
                        error: Some(text.clone()),
                        explanation: Some(explain_paperproof_error(text)),
                        events: Vec::new(),
                        object_changes: None,
                        balance_changes: None,
                        raw: serde_json::Value::Null,
                    });
                }
                let retryable = (robust_options.retry.retryable)(&error)
                    || (robust_options.rebuild_retry && is_rebuild_retryable(&error));
                if attempt >= attempts || !retryable {
                    return Err(error);
                }
                last_error = Some(error);
                let delay = robust_options
                    .retry
                    .base_delay_ms
                    .saturating_mul(attempt as u64)
                    .min(robust_options.retry.max_delay_ms);
                eprintln!(
                    "PaperProof rebuild retry {attempt}/{attempts} for {label} after {delay}ms: {}",
                    last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default()
                );
                sleep(Duration::from_millis(delay));
            }
        }
    }
    Err(
        last_error.unwrap_or_else(|| PaperProofError::TransactionExecution {
            message: format!("{label} failed without an error"),
        }),
    )
}

pub fn is_rebuild_retryable(error: &PaperProofError) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("needs to be rebuilt")
        || text.contains("unavailable for consumption")
        || text.contains("current version")
        || text.contains("object version")
}

fn transaction_failure_message(label: &str, normalized: &NormalizedExecutionResult) -> String {
    let event_types = if normalized.events.is_empty() {
        "none".to_string()
    } else {
        normalized
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{label} failed on-chain. digest={}. error={}. events={event_types}.",
        normalized.digest.as_deref().unwrap_or("unknown"),
        normalized.error.as_deref().unwrap_or("unknown error")
    )
}

fn normalize_events(raw: &serde_json::Value) -> Vec<crate::events::SuiEventEnvelope> {
    raw.get("events")
        .or_else(|| raw.pointer("/Transaction/events"))
        .or_else(|| raw.pointer("/FailedTransaction/events"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect()
}

fn raw_digest(raw: &serde_json::Value) -> Option<String> {
    raw.get("digest")
        .or_else(|| raw.pointer("/Transaction/digest"))
        .or_else(|| raw.pointer("/FailedTransaction/digest"))
        .or_else(|| raw.pointer("/effects/transactionDigest"))
        .or_else(|| raw.pointer("/Transaction/effects/transactionDigest"))
        .or_else(|| raw.pointer("/FailedTransaction/effects/transactionDigest"))
        .or_else(|| raw.pointer("/transaction/digest"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn execution_error(raw: &serde_json::Value) -> Option<String> {
    raw.pointer("/effects/status/error")
        .or_else(|| raw.pointer("/Transaction/effects/status/error"))
        .or_else(|| raw.pointer("/FailedTransaction/effects/status/error"))
        .or_else(|| raw.pointer("/transaction/effects/status/error"))
        .or_else(|| raw.pointer("/transaction/effects/status/error/description"))
        .or_else(|| raw.pointer("/transaction/effects/status/error/message"))
        .or_else(|| raw.pointer("/transaction/effects/status/error/error"))
        .and_then(|value| match value {
            serde_json::Value::String(value) => Some(value.clone()),
            serde_json::Value::Null => None,
            other => Some(other.to_string()),
        })
}
