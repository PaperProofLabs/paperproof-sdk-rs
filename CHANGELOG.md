# Changelog

All notable changes to the PaperProof Rust SDK are documented here.

## 0.2.3 - 2026-05-12

### Added

- Added verified event querying infrastructure with explicit verification reports, trusted event envelopes, and fail-closed page guards.
- Added `assert_no_incomplete` and `require_verified_page` helpers so indexers and applications can reject incomplete or downgraded verified pages with one call.
- Added a mainnet verified-events example that exercises trusted querying and guard behavior against real PaperProof events.

### Changed

- Hardened query and watch APIs so `Verified` trust keeps rejected and incomplete verification reports visible instead of silently treating failures as empty history.
- Strengthened indexer trust policy handling, including explicit rejection of checkpoint-based verified ingestion when object-binding reads are unavailable.
- Updated README/API documentation to describe canonical versus verified trust levels and when verified data is required for statistics, governance history, rewards, and airdrop snapshots.

### Fixed

- Broadened Sui object/id byte parsing so provider responses that encode bytes as numeric-key JSON objects are handled consistently.
- Improved proposal/object verification compatibility across provider response shapes used by GraphQL and gRPC-style adapters.

## 0.2.2 - 2026-05-11

First crates.io-ready release candidate for the PaperProof Rust SDK.

### Added

- gRPC-oriented Sui provider support and CLI fallback execution paths.
- High-level service APIs, typed result helpers, query provider helpers and watch APIs.
- Walrus ContentService helpers for native write, extend, transfer, read and verify flows.
- Indexer-focused checkpoint ingestion, persistent cursors, idempotent event IDs and canonical filters.
- SQLite/Postgres cursor stores and event sinks with batch upsert support.
- Backfill/tail CLI examples, tracing metrics, benchmark scaffolding and deployment examples.

### Notes

- Mainnet write examples remain explicit opt-in.
- SDK configuration supports deployment manifests instead of hardcoding one network.

## 0.1.0 - 2026-05-09

Initial Rust SDK release for the PaperProof protocol.

### Added

- Mainnet deployment constants for PaperProof packages, canonical objects and coin types.
- Typed builders for publishing, versioning, comments, likes, governance and protocol operations.
- Neutral `TransactionPlan`, `MoveCall` and `MoveArgument` structures for build-only workflows.
- `SuiCliExecutor` fallback for preview, dry-run, dev-inspect and explicit execution through the Sui CLI.
- Provider traits: `PaperProofDataProvider`, `PaperProofExecutionProvider` and `PaperProofProvider`.
- Feature-gated native Sui provider scaffolding through `sui-rpc` and `sui-sdk-types`.
- Read clients for canonical objects, series, versions, comments, likes, proposals, balances, coins and dynamic fields.
- `PaperProofQueryClient` with event pagination, canonical event filtering and typed event extraction helpers.
- Deployment verification and deployment drift/update checking.
- Robust execution helpers with normalized results, expected-failure handling and rebuild retry classification.
- Move abort explanation layer for common PaperProof and Sui failure cases.
- Coin selection and amount helpers for SUI, WAL and PPRF workflows.
- Walrus HTTP helper for content read/write shape and digest verification.
- Mainnet read examples and write examples gated by explicit opt-in.

### Safety

- Default tests and examples do not write to mainnet.
- Mainnet write example requires both `PAPERPROOF_RS_MAINNET_WRITE=1` and `--execute`.
- No private keys or `.env` files are required for default tests.
