# PaperProof Rust SDK Enterprise Deployment Guide

This guide shows how to run the PaperProof Rust SDK indexer examples as
long-lived infrastructure. The examples are intentionally conservative: they
read canonical PaperProof events, write JSONL files or SQL-ready records, and do
not require private keys.

## Deployment Model

The Rust SDK supports three production shapes:

1. Embedded library: link `paperproof-sdk-rs` into your own indexer, worker or
   API service.
2. Containerized example services: run the supplied backfill and tail examples
   as lightweight indexer jobs.
3. Custom sink: use `PaperProofEventSink`, `PaperProofDomainChange` and the SQL
   schema files to persist accepted events into Postgres, SQLite or a data lake.

For production, treat the deployment manifest as a trust boundary. Services
should verify canonical package and object IDs at startup, and should hard-fail
when the manifest is stale or cannot be checked if your downstream system cannot
tolerate drift.

## Runtime Configuration

Common environment variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `RUST_LOG` | `paperproof_sdk_rs=info` | Log level for tracing output. |
| `PAPERPROOF_RS_INDEXER_OUT` | `examples/artifacts/indexer` | JSONL output directory. |
| `PAPERPROOF_RS_BACKFILL_LIMIT` | `100` | Events per query page for backfill. |
| `PAPERPROOF_RS_BACKFILL_PAGES` | `1` | Pages per module for one backfill run. |
| `PAPERPROOF_RS_TAIL` | unset | Must be `1` to run the tail loop. |
| `PAPERPROOF_RS_TAIL_INTERVAL_MS` | `10000` | Polling interval for tail mode. |
| `PAPERPROOF_RS_TAIL_LIMIT` | `25` | Events per module per tail scan. |
| `PAPERPROOF_RS_SINK` | `jsonl` | Sink backend: `jsonl`, `sqlite`, or `postgres`. |
| `PAPERPROOF_RS_SQLITE_PATH` | derived from output dir | SQLite database path when `PAPERPROOF_RS_SINK=sqlite`. |
| `PAPERPROOF_RS_POSTGRES_URL` | unset | Postgres connection string when `PAPERPROOF_RS_SINK=postgres`. |
| `PAPERPROOF_RS_CHECKPOINT` | unset | Set to `1` to use gRPC checkpoint ingestion instead of event-query pagination. |
| `PAPERPROOF_RS_CHECKPOINT_START` | stored cursor or `0` | Optional explicit checkpoint start. Omit for resume. |
| `PAPERPROOF_RS_CHECKPOINT_COUNT` | `100` backfill, `25` tail | Checkpoints scanned per run/loop. |
| `PAPERPROOF_RS_CHECKPOINT_BATCH` | `10` backfill, `5` tail | Checkpoints per worker batch. |
| `PAPERPROOF_RS_CHECKPOINT_WORKERS` | `4` | Concurrent checkpoint workers. |
| `PAPERPROOF_RS_CHECKPOINTS_PER_SECOND` | unlimited | Global checkpoint rate limit. |

Do not mount private keys into read-only indexer deployments. Mainnet write
examples remain separately opt-in and should not share the same service account
or pod.

Checkpoint mode requires a build with `--features sui-native` and is the
recommended production path as Sui JSON-RPC moves toward sunset. Cursor resume
uses `StreamId::checkpoint()` in the selected cursor store, so restarts continue
from the last persisted checkpoint unless `PAPERPROOF_RS_CHECKPOINT_START` is
set explicitly.

## Docker

Build the image:

```bash
docker build -t paperproof-sdk-rs:local .
```

Run a tail indexer:

```bash
docker run --rm \
  -e PAPERPROOF_RS_TAIL=1 \
  -e PAPERPROOF_RS_INDEXER_OUT=/var/lib/paperproof/indexer \
  -v paperproof-indexer:/var/lib/paperproof \
  paperproof-sdk-rs:local
```

Run a bounded backfill:

```bash
docker run --rm \
  -e PAPERPROOF_RS_BACKFILL_PAGES=10 \
  -e PAPERPROOF_RS_INDEXER_OUT=/var/lib/paperproof/indexer \
  -v paperproof-indexer:/var/lib/paperproof \
  --entrypoint /usr/local/bin/paperproof-indexer-backfill \
  paperproof-sdk-rs:local
```

Check deployment drift:

```bash
docker run --rm \
  --entrypoint /usr/local/bin/paperproof-check-deployment-update \
  paperproof-sdk-rs:local
```

## systemd

1. Create a service account:

```bash
sudo useradd --system --home /var/lib/paperproof --create-home paperproof
sudo mkdir -p /etc/paperproof /var/lib/paperproof/indexer
sudo chown -R paperproof:paperproof /var/lib/paperproof
```

2. Install binaries from the Docker image or from a local release build:

```bash
cargo build --release --features async,tracing --example indexer_tail
cargo build --release --features async,tracing --example indexer_backfill
cargo build --release --features async,tracing --example check_deployment_update
sudo install -m 0755 target/release/examples/indexer_tail /usr/local/bin/paperproof-indexer-tail
sudo install -m 0755 target/release/examples/indexer_backfill /usr/local/bin/paperproof-indexer-backfill
sudo install -m 0755 target/release/examples/check_deployment_update /usr/local/bin/paperproof-check-deployment-update
```

3. Copy the unit files:

```bash
sudo install -m 0644 deploy/systemd/paperproof-indexer.env.example /etc/paperproof/indexer.env
sudo install -m 0644 deploy/systemd/paperproof-indexer-tail.service /etc/systemd/system/
sudo install -m 0644 deploy/systemd/paperproof-indexer-backfill.service /etc/systemd/system/
sudo systemctl daemon-reload
```

4. Run a backfill, then start tailing:

```bash
sudo systemctl start paperproof-indexer-backfill.service
sudo systemctl enable --now paperproof-indexer-tail.service
```

Operational commands:

```bash
journalctl -u paperproof-indexer-tail.service -f
sudo systemctl restart paperproof-indexer-tail.service
sudo systemctl status paperproof-indexer-tail.service
```

## Kubernetes

Build and publish an image under your registry:

```bash
docker build -t ghcr.io/paperprooflabs/paperproof-sdk-rs:0.1.0 .
docker push ghcr.io/paperprooflabs/paperproof-sdk-rs:0.1.0
```

Apply the example manifests:

```bash
kubectl apply -f deploy/kubernetes/paperproof-indexer.yaml
```

Run the backfill job again:

```bash
kubectl -n paperproof delete job paperproof-indexer-backfill --ignore-not-found
kubectl apply -f deploy/kubernetes/paperproof-indexer.yaml
```

Inspect logs:

```bash
kubectl -n paperproof logs deploy/paperproof-indexer-tail -f
kubectl -n paperproof logs job/paperproof-indexer-backfill
```

For managed clusters, replace the single PVC with your normal storage class,
wire logs into your observability stack, and use your own image tag promotion
flow instead of `latest`.

## Database Sinks

The SDK ships schema templates:

- `sql/postgres_indexer_schema.sql`
- `sql/sqlite_indexer_schema.sql`

The example `indexer_sql_sinks` prints SQL-ready JSON parameter maps. Production
services can use `SqliteEventSink` / `PostgresEventSink` directly, or implement
`PaperProofEventSink` with their own pool. The supplied backfill and tail
examples choose the sink with `PAPERPROOF_RS_SINK` and persist cursors with the
matching `SqliteCursorStore` / `PostgresCursorStore`.

SQLite backfill:

```bash
PAPERPROOF_RS_SINK=sqlite \
PAPERPROOF_RS_SQLITE_PATH=/var/lib/paperproof/indexer/paperproof-indexer.sqlite \
cargo run --release --features sqlite --example indexer_backfill
```

Postgres tail:

```bash
PAPERPROOF_RS_TAIL=1 \
PAPERPROOF_RS_SINK=postgres \
PAPERPROOF_RS_POSTGRES_URL=postgres://paperproof:paperproof@localhost/paperproof \
cargo run --release --features postgres --example indexer_tail
```

Recommended ingestion flow:

1. Verify deployment drift policy at process startup.
2. Scan canonical events by checkpoint or module.
3. Reject non-canonical events before persistence.
4. Persist accepted raw event, idempotency key, event kind and domain change.
5. Apply `PaperProofDomainChange` into your read model inside the same database
   transaction.
6. Persist cursor after the batch commit.

## Hard-Fail Drift Policy

For production indexers:

- Use `HardFailOnUpdate` when your app can keep running if the manifest check is
  temporarily unavailable, but must stop on known object/package drift.
- Use `HardFailOnAnyProblem` when your downstream data product cannot tolerate
  unchecked deployment state.
- Use `Warn` only for development and dashboards that do not drive balances,
  rewards, access control or analytics commitments.

The supplied systemd and Kubernetes examples run `paperproof-check-deployment-update`
before the tail service starts. Applications embedding the SDK should call
`enforce_deployment_update_policy` directly.

## Security Notes

- Keep read-only indexers separate from write-capable workers.
- Never inject Sui private keys into indexer pods unless the process explicitly
  performs signed transactions.
- Pin image digests in production.
- Store cursors durably before increasing scan concurrency.
- Treat `PAPERPROOF_RS_INDEXER_OUT` as application state and back it up if JSONL
  is your source of truth.
- Keep canonical event filtering enabled by default.

## Production Readiness Checklist

- Deployment manifest check is enabled.
- Canonical event filtering is enabled.
- Cursors are durable and restored on restart.
- Event writes are idempotent by `EventId::key()`.
- Backfill and tail run under separate operational controls.
- Logs are shipped to a centralized system.
- The storage backend has backup and restore procedures.
- Alerts cover tail lag, repeated rejected events, restart loops and disk usage.
