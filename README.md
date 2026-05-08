# PaperProof SDK for Rust

Rust SDK for the PaperProof protocol on Sui. This release focuses on typed
configuration, input validation, transaction-plan construction, high-level service
APIs, event parsing, canonical event filtering, gRPC-first reads, deployment
verification, robust retry helpers and lightweight Walrus content helpers.

It is designed to mirror the public surface of `@paperproof/sdk-ts` while giving
Rust applications a conservative integration point: builders return neutral
`TransactionPlan` values that can be adapted to the official Sui Rust SDK, a CLI
pipeline, a backend signer, or a custom transaction service.

The high-level SDK constructor follows the TypeScript SDK transport policy:
gRPC is the default transport. The official Sui Rust SDK path does not support
JSON-RPC. This crate keeps a small `reqwest` JSON-RPC compatibility adapter only
for short-term historical event backfills and migration paths where equivalent
Rust gRPC/GraphQL APIs are not yet wired into this SDK.

## Install

```toml
[dependencies]
paperproof-sdk-rs = { git = "https://github.com/PaperProofLabs/paperproof-sdk-rs" }
```

After the crate is published to crates.io:

```toml
[dependencies]
paperproof-sdk-rs = "0.1"
```

Optional native Sui RPC integration:

```toml
[dependencies]
paperproof-sdk-rs = { version = "0.1", features = ["sui-native"] }
```

## Quick Start

```rust
use paperproof_sdk_rs::{PaperProofClient, types::{CommonContentInput, PreprintInput}};

fn main() -> paperproof_sdk_rs::Result<()> {
    let client = PaperProofClient::mainnet();
    let plan = client.publishing.publish_preprint(&PreprintInput {
        title: "Example preprint".into(),
        abstract_text: "A minimal PaperProof Rust SDK example.".into(),
        authors: vec!["PaperProof Labs".into()],
        keywords: vec!["example".into()],
        field: "computer science".into(),
        license: "CC-BY-4.0".into(),
        page_count: 1,
        content: CommonContentInput {
            content_hash: "sha256:example".into(),
            walrus_blob_id: "example-blob".into(),
            walrus_blob_object_id: "0x6".into(),
            content_type: "application/pdf".into(),
        },
        series_metadata: vec![],
        version_metadata: vec![],
        payment_coin_id: None,
    })?;
    println!("{plan:#?}");
    Ok(())
}
```

High-level gRPC-first SDK:

```rust
use paperproof_sdk_rs::{PaperProofSdk, PaperProofTransport};

fn main() -> paperproof_sdk_rs::Result<()> {
    let sdk = PaperProofSdk::mainnet()?;
    assert_eq!(sdk.transport, PaperProofTransport::Grpc);
    Ok(())
}
```

Deprecated JSON-RPC compatibility fallback:

```rust
use paperproof_sdk_rs::{CreatePaperProofSdkOptions, PaperProofTransport, create_paperproof_sdk};

let sdk = create_paperproof_sdk(CreatePaperProofSdkOptions {
    transport: Some(PaperProofTransport::JsonRpc),
    ..Default::default()
})?;
# Ok::<(), paperproof_sdk_rs::PaperProofError>(())
```

This fallback is not backed by the official Sui Rust SDK and should not be used
as the default transport for new services.

## Capabilities

- Deployment constants for the current PaperProof mainnet deployment.
- Deployment drift checking against a remote deployment manifest.
- Protocol constants for artifact types, statuses, fee levels and governance actions.
- Strong local validation for addresses, object ids, metadata, content fields,
  comments and proposal inputs.
- Transaction builders for publishing, versioning, comments, likes, governance
  voting and operational controls.
- Typed `TransactionPlan`, `MoveCall` and `MoveArgument` structures for build-only
  workflows and batch/PTB preparation.
- A `SuiCliExecutor` adapter that can preview, dry-run, dev-inspect or execute a
  `TransactionPlan` through the official Sui CLI. This gives Rust users real
  signing and mainnet write support while keeping the core plan format stable.
- Deprecated JSON-RPC compatibility helper for object reads and event queries
  during migration/backfill work. It is a direct HTTP adapter, not official
  `sui-rust-sdk` JSON-RPC support.
- Full `PaperProofReadClient` for canonical objects, series, versions, comments,
  likes, governance proposals, dynamic fields, balances and coin pages.
- `PaperProofQueryClient` for paginated event queries, canonical event filtering,
  all-page collection helpers and typed event extraction.
- Typed view structs for common PaperProof on-chain objects.
- `PaperProofService` for script-friendly high-level operations backed by the
  Sui CLI executor.
- Provider traits for custom data and execution backends:
  `PaperProofDataProvider`, `PaperProofExecutionProvider` and `PaperProofProvider`.
- Feature-gated native Sui adapter scaffolding based on `sui-rpc` and
  `sui-sdk-types`; `create_paperproof_sdk` defaults to this gRPC transport when
  the `sui-native` feature is enabled. CLI execution remains the conservative
  fallback for write flows.
- Deployment verification that checks canonical object/package bindings before
  indexers or services trust a configuration.
- Checkpoint ingestion abstractions for high-throughput indexers:
  `CheckpointDataProvider`, `CheckpointScanOptions`, persistent cursor traits,
  `EventId` idempotency keys and canonical-by-default filtering.
- Indexer sinks and schemas: `PaperProofEventSink`, `JsonlEventSink`,
  `POSTGRES_SCHEMA_SQL`, `SQLITE_SCHEMA_SQL`, plus SQL parameter helpers.
- Domain reducer output via `PaperProofDomainChange` for production-oriented
  upsert/update pipelines.
- Deployment drift hard-fail policy for long-running services.
- Optional `tracing` metrics for indexer scan batches.
- Robust retry helpers and Move abort explanations for friendlier failure logs.
- Event parser and trust filter so indexers can reject events from fake packages
  or wrong canonical objects.
- Walrus HTTP helper for blob reads, writes and SHA-256 verification.
- Coin amount helpers and coin selection utilities for PPRF/SUI/WAL workflows.

See [docs/API.md](docs/API.md) for the API map.
See [docs/ENTERPRISE_DEPLOYMENT.md](docs/ENTERPRISE_DEPLOYMENT.md) for Docker,
systemd and Kubernetes deployment examples.

## Local Validation

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

The default tests are local/offline and do not require private keys or mainnet
funds.

Optional mainnet read-only verification:

```powershell
$env:PAPERPROOF_RS_MAINNET_READ='1'
cargo test --test integration_mainnet
cargo run --example verify_deployment
cargo run --example mainnet_read
cargo run --example check_deployment_update
cargo run --example indexer_backfill
```

Indexer deployment examples:

```powershell
docker build -t paperproof-sdk-rs:local .
cargo run --example indexer_sql_sinks
```

Production operators can start from:

- `Dockerfile`
- `deploy/systemd/paperproof-indexer-tail.service`
- `deploy/systemd/paperproof-indexer-backfill.service`
- `deploy/kubernetes/paperproof-indexer.yaml`
- `docs/ENTERPRISE_DEPLOYMENT.md`

Local fuzzy builder exercise:

```powershell
cargo run --example fuzzy_build
```

## Sui Execution and Mainnet Writes

The SDK exposes two layers:

- `TransactionPlan` for build-only and custom PTB workflows.
- `SuiCliExecutor` for real signing and execution through `sui client ptb`.
- `PaperProofExecutionProvider` for native, CLI, remote signer or custom backend
  integrations.

Example:

```rust
use paperproof_sdk_rs::{CliExecutionOptions, ExecutionMode, PaperProofClient, SuiCliExecutor};

let client = PaperProofClient::mainnet();
let executor = SuiCliExecutor::mainnet();
let plan = client.ops.set_paused(false);
let output = executor.run(&plan, &CliExecutionOptions {
    mode: ExecutionMode::DryRun,
    sender: Some("0x...".into()),
    gas_budget: Some(30_000_000),
    ..Default::default()
})?;
println!("{:?}", output.digest);
# Ok::<(), paperproof_sdk_rs::PaperProofError>(())
```

The executor supports:

- `ExecutionMode::Preview`
- `ExecutionMode::DryRun`
- `ExecutionMode::DevInspect`
- `ExecutionMode::Execute`

Real writes use the active Sui CLI keystore. Make sure `sui client active-env`
is `mainnet` and `sui client active-address` or `CliExecutionOptions.sender`
is the intended signer.

The `mainnet_write_smoke` example is opt-in:

```powershell
$env:PAPERPROOF_RS_MAINNET_WRITE='1'
$env:PAPERPROOF_RS_SENDER='0x...'
cargo run --example mainnet_write_smoke
```

The command above dry-runs by default. Add `-- --execute` to send a real mainnet
transaction:

```powershell
$env:PAPERPROOF_RS_MAINNET_WRITE='1'
$env:PAPERPROOF_RS_SENDER='0x...'
cargo run --example mainnet_write_smoke -- --execute
```

This double guard is deliberate: examples and tests should never write to
mainnet by accident. To only comment on an existing tree, set
`PAPERPROOF_RS_TREE_ID`; otherwise the example builds a preprint publish
transaction.

## Query API

`PaperProofQueryClient` is the event/indexer-friendly read surface.

```rust
use paperproof_sdk_rs::{EventQueryInput, PaginationInput, PaperProofQueryClient};

# async fn run() -> paperproof_sdk_rs::Result<()> {
let query = PaperProofQueryClient::mainnet();
let page = query.query_canonical_events(EventQueryInput {
    package_id: Some(query.deployment.packages.publishing.clone()),
    module: Some("publishing".into()),
    pagination: PaginationInput {
        limit: Some(10),
        descending_order: Some(true),
        ..Default::default()
    },
    ..Default::default()
}).await?;
println!("events={}", page.data.len());
# Ok(())
# }
```

For long-running indexers, prefer the dedicated indexer helpers:

```rust
use paperproof_sdk_rs::{PaperProofIndexerClient, PaperProofIndexerState};

# async fn run() -> paperproof_sdk_rs::Result<()> {
let indexer = PaperProofIndexerClient::mainnet();
let filters = PaperProofIndexerClient::canonical_module_filters(&indexer.query.deployment);
let batch = indexer.scan_module_once(filters[0].clone(), None, Some(100)).await?;

let mut state = PaperProofIndexerState::default();
state.apply_batch(&batch);
println!(
    "accepted={} rejected={}",
    batch.progress.accepted_events,
    batch.progress.rejected_events
);
# Ok(())
# }
```

Indexer guidance:

- Treat official package ids plus canonical root/registry ids as the trust boundary.
- Prefer `CheckpointDataProvider` and `scan_checkpoint_range_once` for production
  gRPC/checkpoint ingestion; keep JSON-RPC event queries for compatibility
  backfill only.
- Persist `StoredIndexerCursor` with an implementation of `IndexerCursorStore`.
- Use `EventId::key()` as the default idempotency key for database upserts.
- Use `PaperProofEventSink` to write accepted/rejected batches. The SDK ships a
  JSONL sink and Postgres/SQLite starter schemas; production users can implement
  the trait with their own connection pool and upsert strategy.
- Map `PaperProofDomainChange` into your application schema instead of relying on
  raw events alone.
- Enable `features = ["tracing"]` to emit batch metrics through `tracing`.
- Keep `IndexerProgress.cursor` per module stream so services can resume safely.
- Persist rejected events and reasons; they help diagnose fake events and integration mistakes.
- Use `PaperProofIndexerState` as a reference reducer, not as a replacement for a durable database.

Sui event filters have protocol-level combination limits. The SDK returns a
clear `EventParse` error when a user combines incompatible filters such as
`sender` with package/module/event-type filters.

For new indexers, prefer checkpoint/subscription ingestion through the native
Sui gRPC stack or a custom provider. Use JSON-RPC only as an explicitly chosen,
deprecated backfill compatibility path.

## Deployment Drift Checks

Use deployment drift checks when running a website, indexer or long-lived service
that might outlive an SDK release.

```rust
use paperproof_sdk_rs::{check_deployment_update_from_url, format_deployment_update_check};

# async fn run() {
let result = check_deployment_update_from_url(None, None).await;
println!("{}", format_deployment_update_check(&result));
# }
```

If the manifest differs from the compiled deployment constants, update the SDK or
pass the latest `Deployment` override explicitly.

Services can enforce this at startup:

```rust
use paperproof_sdk_rs::{DeploymentDriftPolicy, enforce_deployment_update_policy};

# async fn run() -> paperproof_sdk_rs::Result<()> {
let check = paperproof_sdk_rs::check_deployment_update_from_url(None, None).await;
enforce_deployment_update_policy(&check, DeploymentDriftPolicy::HardFailOnAnyProblem)?;
# Ok(())
# }
```

Benchmarks:

```powershell
cargo bench --bench indexer
```

## Deployment Verification

```rust
use paperproof_sdk_rs::{PaperProofReadClient, verify_deployment};

# async fn run() -> paperproof_sdk_rs::Result<()> {
let read = PaperProofReadClient::mainnet();
let verification = verify_deployment(&read).await?;
assert!(verification.ok);
# Ok(())
# }
```

This checks that the configured root, type registry, fee manager, governance
vault and governance config are readable, come from expected packages, and point
back to the canonical PaperProof deployment objects.

## Robust Retry And Diagnostics

Use `robust_execute_plan` for transient CLI/RPC failures and
`abort_explainer::explain_paperproof_error` for user-facing Move abort messages.
The SDK classifies common failures such as insufficient balance, wrong object
types, duplicate votes, duplicate likes, paused publishing and fake object
bindings.

Provider-based robust helpers also expose normalized results:

- `normalize_provider_execution_output`
- `robust_execute_plan_normalized`
- `RobustProviderExecuteOptions { expect_failure, rebuild_retry, retry }`

`expect_failure` is useful for negative integration tests. `rebuild_retry`
classifies common object-version and transaction-rebuild failures so callers can
rebuild a fresh transaction plan before retrying.

## Walrus

There is no official Rust Walrus SDK at the time this crate was created. The
crate uses the public HTTP aggregator/publisher shape directly and does not
depend on third-party Walrus crates. The helper always exposes digest checks so
callers can verify content before binding it to PaperProof metadata.

For application code, prefer `PaperProofContentService`. It gives PaperProof
users a single content lifecycle API: publish bytes, read and verify bytes,
extend storage, and transfer owned blob objects. That makes the PaperProof SDK a
practical Walrus simplification layer instead of forcing users to learn a
separate toolchain before they can publish.

```rust
use paperproof_sdk_rs::{
    ContentPublishOptions, PaperProofContentService, WalrusClient,
};

let walrus = WalrusClient::new(
    "https://aggregator.walrus-mainnet.walrus.space",
    Some("https://publisher.walrus-mainnet.walrus.space".to_string()),
);
let content = PaperProofContentService::new(walrus);
let published = content
    .publish_content(b"paperproof content".to_vec(), ContentPublishOptions::default())
    .await?;
```

Native Rust read/verify and owned/shared preflight checks are implemented in
this crate. Fully native Walrus write/certify is intentionally tracked as a
separate backend milestone because it requires the full Walrus encoding,
storage-node upload, certificate, and Sui register/certify flow. The
`PaperProofContentService` API is stable so that backend can be swapped in
without changing downstream application code.

## Security Notes

- Treat `MAINNET_DEPLOYMENT` as the trusted entry point for package and canonical
  object ids.
- For indexers, filter by official package id and canonical object ids. Event
  names alone are not sufficient.
- Local validation is a developer-experience and safety layer. The Move
  contracts remain the source of truth.
- Do not commit private keys, `.env` files, or signer material into applications
  using this SDK.

## Release Checklist

Before publishing:

```powershell
.\scripts\publish-check.ps1
```

The script runs:

- `cargo fmt --check`
- `cargo test`
- `cargo test --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo doc --no-deps --all-features`
- `cargo package --allow-dirty`

For crates.io publication:

```powershell
cargo login
cargo publish --dry-run
cargo publish
```

Keep mainnet write examples opt-in and never commit signer material.
