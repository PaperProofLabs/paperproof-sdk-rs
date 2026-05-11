// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

//! Rust SDK for the PaperProof protocol.
//!
//! This crate mirrors the TypeScript SDK at the API boundary while staying
//! conservative about execution. It provides typed deployment metadata,
//! validation, Move-call transaction plans, event parsing and lightweight
//! read/Walrus helpers. The transaction plans are intentionally neutral so
//! applications can adapt them to the official Sui Rust SDK, custom CLIs, or
//! service-side transaction pipelines.

pub mod abort_explainer;
pub mod builders;
pub mod client;
pub mod coin_utils;
pub mod constants;
pub mod deployment;
pub mod deployment_update;
pub mod deployment_verifier;
pub mod error;
pub mod event_verifier;
pub mod events;
pub mod events_trust;
pub mod executor;
pub mod indexer;
pub mod providers;
pub mod query;
pub mod read;
pub mod robust;
pub mod sdk;
pub mod service;
pub mod sink;
#[cfg(feature = "sui-native")]
pub mod sui_native;
pub mod transaction;
pub mod types;
pub mod validation;
pub mod views;
pub mod walrus;
pub mod watch;

pub use builders::{
    comments::CommentsBuilder, governance::GovernanceBuilder, ops::OpsBuilder,
    publishing::PublishingBuilder,
};
pub use client::{JsonRpcClient, PaperProofClient};
pub use constants::*;
pub use deployment::{Deployment, DeploymentObjects, DeploymentPackages, MAINNET_DEPLOYMENT};
pub use deployment_update::{
    DEFAULT_DEPLOYMENT_MANIFEST_BASE_URL, DEFAULT_MAINNET_DEPLOYMENT_MANIFEST_URL,
    DeploymentDriftPolicy, DeploymentManifest, DeploymentManifestStatus, DeploymentUpdateCheck,
    DeploymentUpdateDifference, check_deployment_update_from_url,
    check_deployment_update_with_manifest, default_deployment_manifest_url, diff_deployment,
    enforce_deployment_update_policy, format_deployment_update_check, manifest_from_value,
};
pub use deployment_verifier::{
    DeploymentCheck, DeploymentCheckStatus, DeploymentVerification, format_deployment_verification,
    verify_deployment,
};
pub use error::{PaperProofError, Result};
pub use event_verifier::{PaperProofEventVerifier, VerifyEventOptions};
pub use events::{
    AddVersionResult, CommentResult, LikeResult, ProposalExecutedResult, ProposalFinalizedResult,
    ProposalResult, PublishResult, VoteCastResult,
};
pub use events_trust::{
    EventIssueSeverity, EventTrustLevel, EventTrustResult, EventVerificationIssue,
    EventVerificationReport, EventVerificationStatus, TrustedSuiEventEnvelope,
    VerifiedEventPageGuard, assert_no_incomplete, attach_event_verification, require_verified_page,
    verification_report_from_canonical_check,
};
pub use executor::{CliExecutionOptions, CliExecutionOutput, ExecutionMode, SuiCliExecutor};
#[cfg(feature = "async")]
pub use indexer::CheckpointIngestionOptions;
pub use indexer::{
    CheckpointCursor, CheckpointData, CheckpointDataProvider, CheckpointIngestionReport,
    CheckpointScanOptions, EventId, IndexedPaperProofEvent, IndexerCursorStore, IndexerEventBatch,
    IndexerMetrics, IndexerProgress, IndexerScanOptions, IndexerTrustPolicy,
    MemoryIndexerCursorStore, PackageModuleFilter, PaperProofDomainChange, PaperProofIndexerClient,
    PaperProofIndexerState, RejectedPaperProofEvent, StoredIndexerCursor, StreamId,
    domain_change_from_event, event_id, event_kind_counts, indexer_batch_from_page,
    indexer_batch_from_page_with_policy, indexer_batch_from_trusted_page,
};
pub use providers::{
    BuiltTransaction, DynamicFieldName, DynamicFieldObject, PaperProofDataProvider,
    PaperProofExecutionProvider, PaperProofProvider, ProviderExecutionOptions,
    ProviderExecutionOutput,
};
pub use query::{
    EventPage, EventQueryInput, GraphQlQueryProvider, MAINNET_GRAPHQL_ENDPOINT, PaginationInput,
    PaperProofQueryClient, PaperProofQueryProvider, SeriesDetails, TESTNET_GRAPHQL_ENDPOINT,
    TrustedEventPage, TrustedEventQueryInput,
};
pub use read::{Balance, CoinObject, Page, PaperProofProviderReadClient, PaperProofReadClient};
pub use robust::{
    NormalizedExecutionResult, RetryOptions, RobustExecution, RobustProviderExecuteOptions,
    RobustProviderExecution, default_retryable, is_rebuild_retryable,
    normalize_provider_execution_output, robust_execute_plan, robust_execute_plan_normalized,
    robust_execute_with_provider,
};
pub use sdk::{
    CreatePaperProofSdkOptions, PaperProofSdk, PaperProofSdkQuery, PaperProofSdkRead,
    PaperProofTransport, create_paperproof_sdk,
};
pub use service::{
    ExecutedResult, PaperProofProviderService, PaperProofService, ProviderExecutedResult,
};
pub use sink::{
    JsonlEventSink, POSTGRES_SCHEMA_SQL, PaperProofEventSink, SQLITE_SCHEMA_SQL, SinkWriteSummary,
    accepted_event_to_sql_params, rejected_event_to_sql_params,
};
#[cfg(feature = "postgres")]
pub use sink::{PostgresCursorStore, PostgresEventSink};
#[cfg(feature = "sqlite")]
pub use sink::{SqliteCursorStore, SqliteEventSink};
#[cfg(feature = "sui-native")]
pub use sui_native::{
    NativeBuildOptions, NativeTransaction, NativeTransactionBuilder, NativeTransactionSigner,
    NoopNativeSigner, SuiNativeProvider, UnsupportedNativeBuilder, execute_native_transaction,
    simulate_native_transaction,
};
pub use transaction::{MoveArgument, MoveCall, TransactionPlan};
pub use types::*;
pub use walrus::{
    ContentPublishOptions, ContentPublishResult, ContentReadResult, PaperProofContentBackend,
    PaperProofContentService, WalrusExtendOptions, WalrusExtendResult, WalrusTransferOptions,
    WalrusTransferResult, WalrusWriteOptions,
};
pub use watch::{
    PaperProofEventWatcher, PaperProofTrustedEventWatcher, PaperProofWatchClient, WatchOptions,
};
