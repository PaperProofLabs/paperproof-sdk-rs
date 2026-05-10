# PaperProof Rust SDK API

The Rust SDK is organized into five layers.

## Deployment

- `Deployment` stores package ids, canonical shared object ids, RPC URL and coin types.
- `mainnet_deployment()` and `MAINNET_DEPLOYMENT` provide the current PaperProof mainnet configuration.
- `create_paperproof_sdk` and `PaperProofSdk::mainnet()` use gRPC by default, matching the TypeScript SDK transport policy.
- `PaperProofTransport::JsonRpc` is deprecated and remains only as an explicit compatibility fallback. It is not backed by the official Sui Rust SDK.
- `verify_deployment(&PaperProofReadClient)` reads canonical objects and checks package/object bindings.
- `check_deployment_update_from_url` checks whether a remote manifest has newer package/object ids.
- `diff_deployment` compares two deployment configs and reports field-level drift.

Use deployment verification before long-running services or indexers start.

## Build Layer

`PaperProofClient` exposes protocol builders:

- `client.publishing`: publish artifacts, add versions, update metadata, transfer artifact owner.
- `client.comments`: add comments, set comment/tree status, like/unlike, transfer tree owner.
- `client.governance`: create proposals, vote, finalize, resolve, execute and claim.
- `client.ops`: governance/operator/admin operation builders.

Every builder returns a neutral `TransactionPlan`. It does not sign or execute by itself.

## Execution Layer

`SuiCliExecutor` adapts `TransactionPlan` to `sui client ptb`.

Execution modes:

- `Preview`
- `DryRun`
- `DevInspect`
- `Execute`

Use `robust_execute_plan` for retry handling around transient RPC/CLI failures. Real writes still require explicit `ExecutionMode::Execute`.

Provider interfaces:

- `PaperProofDataProvider`
- `PaperProofExecutionProvider`
- `PaperProofProvider`
- `ProviderExecutionOptions`
- `ProviderExecutionOutput`

`SuiCliExecutor` implements `PaperProofExecutionProvider` and remains the default fallback. The `sui-native` feature exposes native Sui RPC adapter scaffolding for applications that provide complete native transaction builders and signers.

## Read Layer

`PaperProofReadClient` supports:

- canonical objects: root, type registry, fee manager, governance vault/config;
- business objects: series, versions, comments trees, likes books, proposals;
- dynamic fields: comments, likes and proposal id mapping;
- balances and coin pages;
- event queries when backed by a provider that supports them. The built-in JSON-RPC event path is deprecated compatibility glue for explicit short-term backfills.

`PaperProofQueryClient` adds indexer-style helpers:

- `query_events`
- `query_canonical_events`
- `query_all_events`
- `query_governance_proposal_created_events`
- `query_governance_vote_cast_events`
- `query_governance_finalized_events`
- `query_governance_executed_events`
- `query_governance_expired_events`
- `query_governance_vote_claimed_events`
- `get_series_details`
- `parse_events_by_struct`

`PaperProofQueryClient::mainnet()` is GraphQL-first. Governance helpers query both the current and original governance
packages, filter by the configured PaperProof root/registry id, and deduplicate by transaction digest plus event
sequence. Frontends and indexers should prefer these helpers over single-package hand-built event queries.

Event query filters support sender, package, package+module, event type and move event type. Incompatible Sui query combinations return `EventParse` errors before sending a request.

Note: the official Sui Rust SDK path used by this crate's `sui-native` feature is gRPC-oriented and does not support JSON-RPC. The SDK's `JsonRpcClient` is a small `reqwest` compatibility adapter kept for migration/backfill scenarios. Do not treat it as the recommended or default transport for new services.

## Watch Layer

`PaperProofWatchClient` wraps `PaperProofQueryClient` with polling watchers for scripts, bots, and lightweight services.
It is intentionally small: applications drive `next().await` on their own schedule, while the watcher retains cursor and
dedupe state.

```rust
use paperproof_sdk_rs::{PaperProofWatchClient, WatchOptions};

# async fn run() -> paperproof_sdk_rs::Result<()> {
let watch = PaperProofWatchClient::mainnet();
let mut watcher = watch.watch_artifact_published_events(WatchOptions {
    limit: Some(20),
    ..Default::default()
});

let page = watcher.next().await?;
println!("new events={}", page.data.len());
# Ok(())
# }
```

Named helpers include:

- `watch_artifact_published_events`
- `watch_artifact_version_added_events`
- `watch_comment_added_events`
- `watch_paper_liked_events`
- `watch_paper_unliked_events`
- `watch_status_changed_events`
- `watch_owner_transferred_events`
- `watch_governance_proposal_created_events`
- `watch_governance_vote_cast_events`
- `watch_governance_finalized_events`
- `watch_governance_executed_events`
- `watch_governance_expired_events`
- `watch_governance_vote_claimed_events`

Governance watchers query the current and original governance packages and canonical-filter the result, which prevents
frontends and indexers from accidentally reporting empty governance history after an upgrade.

Typed view helpers convert raw Move object fields into stable Rust structs:

- `PaperProofRootView`
- `ArtifactSeriesView`
- `ArtifactVersionView`
- `CommentsTreeView`
- `CommentNodeView`
- `LikesBookView`
- `ProposalView`
- `GovernanceConfigView`
- `GovernanceVaultView`
- `FeeManagerView`

## Service Layer

`PaperProofService` combines builders, read client and CLI executor. It is convenient for scripts and CLIs where the Sui CLI keystore is the signer.

For frontends or custom Rust transaction pipelines, prefer the build layer and adapt `TransactionPlan` to your own signer/executor.

## Events

The SDK provides:

- event classification with `parse_event`;
- canonical package filtering with `validate_event_trust`;
- typed result extraction for publish, add-version, comments, likes, votes, proposal lifecycle, status changes and owner transfers.

Indexers should filter by official package id and canonical object ids. Event names alone are not a trust boundary.

## Indexer Layer

The indexer layer is intended for high-throughput community services that need
to ingest PaperProof events without trusting arbitrary chain noise.

Core exports:

- `PaperProofIndexerClient`
- `CheckpointDataProvider`
- `CheckpointData`
- `CheckpointScanOptions`
- `IndexerCursorStore`
- `MemoryIndexerCursorStore`
- `EventId`
- `StreamId`
- `IndexerScanOptions`
- `IndexerEventBatch`
- `IndexerProgress`
- `IndexedPaperProofEvent`
- `RejectedPaperProofEvent`
- `PaperProofIndexerState`
- `indexer_batch_from_page`
- `event_kind_counts`
- `PaperProofEventSink`
- `JsonlEventSink`
- `POSTGRES_SCHEMA_SQL`
- `SQLITE_SCHEMA_SQL`

Recommended pipeline:

1. Start from `mainnet_deployment()` or a verified custom `Deployment`.
2. Run `verify_deployment` and optional deployment drift checks before a long scan.
3. Prefer checkpoint ingestion through `CheckpointDataProvider` for production backfills and tailing.
4. Query events by the canonical module filters from `PaperProofIndexerClient::canonical_module_filters` only for compatibility backfills.
5. Pass each event page through `indexer_batch_from_page`, or call `scan_checkpoint_range_once`.
6. Persist `StoredIndexerCursor` per `StreamId`.
7. Use `EventId::key()` as the default idempotency key before writing to a database.
8. Store rejected events separately with their reason for diagnostics.
9. Apply accepted events to your own durable database, or use `PaperProofIndexerState` as a lightweight reference reducer.

Checkpoint ingestion:

```rust
use paperproof_sdk_rs::{
    CheckpointScanOptions, IndexerCursorStore, MemoryIndexerCursorStore, PaperProofIndexerClient,
};

# async fn run<P: paperproof_sdk_rs::CheckpointDataProvider>(
#   provider: &P
# ) -> paperproof_sdk_rs::Result<()> {
let indexer = PaperProofIndexerClient::mainnet();
let store = MemoryIndexerCursorStore::default();
let batch = indexer.scan_checkpoint_range_once(provider, CheckpointScanOptions {
    start_checkpoint: 1_000_000,
    limit: 100,
    canonical_only: true,
}).await?;

for event in &batch.accepted {
    if store.mark_processed(&event.id).await? {
        // Upsert event into your durable sink.
    }
}
# Ok(())
# }
```

`canonical_only` defaults to `true` for indexer scans. Fake package/root events are rejected before reducers see them.

Sink and CLI helpers:

- `PaperProofEventSink` is the sink trait for durable writes.
- `JsonlEventSink` writes accepted/rejected events as JSONL and is useful for backfill smoke tests.
- `accepted_event_to_sql_params` and `rejected_event_to_sql_params` provide typed JSON parameter maps for SQL upserts.
- `POSTGRES_SCHEMA_SQL` and `SQLITE_SCHEMA_SQL` expose starter schemas with idempotent primary keys.
- `examples/indexer_backfill.rs` runs a bounded backfill into JSONL.
- `examples/indexer_tail.rs` runs a guarded polling tail loop.
- `examples/indexer_sql_sinks.rs` prints the SQL schemas and example upsert parameters.

Deployment helpers:

- `Dockerfile` builds read-only backfill/tail binaries into a minimal runtime image.
- `deploy/systemd/` contains hardened Linux service units.
- `deploy/kubernetes/paperproof-indexer.yaml` contains a PVC-backed tail deployment and a bounded backfill job.
- `docs/ENTERPRISE_DEPLOYMENT.md` explains drift policy, storage, logs, cursors and security boundaries.

Enable the `tracing` feature to emit structured batch metrics from scans:

```toml
paperproof-sdk-rs = { version = "0.1", features = ["tracing"] }
```

Domain reducer:

- `PaperProofDomainChange` turns accepted events into business-level changes such as `SeriesCreated`, `VersionAdded`, `CommentAdded`, `LikeChanged`, `ProposalResolved`, `StakeClaimed`, and `OwnerTransferred`.
- `PaperProofIndexerState::domain_changes(&batch)` returns the changes for a batch.
- `PaperProofIndexerState::apply_batch(&batch)` remains a lightweight reference reducer; production services should map domain changes into their own durable schema.

`PaperProofIndexerState` is intentionally small. It tracks counts and latest
object ids, but it is not a full database model. Production indexers should keep
raw event ids, transaction digests, checkpoints and replay-safe idempotency keys.

## Errors And Diagnostics

- Local validation returns structured `PaperProofError` variants.
- `abort_explainer` maps common PaperProof Move aborts to human-readable titles, details and suggestions.
- `coin_utils` can summarize and select coin objects before submitting transactions.
- `normalize_provider_execution_output` turns provider-specific execution output into digest/status/error/events/object-change/balance-change fields.
- `robust_execute_plan_normalized` supports expected-failure test flows and rebuild retry classification.
- `DeploymentDriftPolicy` and `enforce_deployment_update_policy` let services hard-fail on stale or unchecked deployment manifests before indexing.

## Mainnet Tests

Default tests are offline. Optional tests/examples are gated:

```powershell
$env:PAPERPROOF_RS_MAINNET_READ='1'
cargo test --test integration_mainnet

cargo run --example verify_deployment
cargo run --example mainnet_read
cargo run --example check_deployment_update
cargo run --example fuzzy_build
```

Real writes are guarded by `PAPERPROOF_RS_MAINNET_WRITE=1` and `--execute` in the write smoke example.

## Release

Run the local release gate before tagging or publishing:

```powershell
.\scripts\publish-check.ps1
```

The GitHub Actions workflow runs the same core checks on push and pull request.
