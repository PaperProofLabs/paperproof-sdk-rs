# Copyright (c) 2026 PaperProof Labs
# SPDX-License-Identifier: Apache-2.0

FROM rust:1-bookworm AS builder

WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY examples ./examples
COPY sql ./sql

RUN cargo build --release --features async,tracing --example indexer_backfill \
    && cargo build --release --features async,tracing --example indexer_tail \
    && cargo build --release --features async,tracing --example verify_deployment \
    && cargo build --release --features async,tracing --example check_deployment_update

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 paperproof

WORKDIR /app

COPY --from=builder /workspace/target/release/examples/indexer_backfill /usr/local/bin/paperproof-indexer-backfill
COPY --from=builder /workspace/target/release/examples/indexer_tail /usr/local/bin/paperproof-indexer-tail
COPY --from=builder /workspace/target/release/examples/verify_deployment /usr/local/bin/paperproof-verify-deployment
COPY --from=builder /workspace/target/release/examples/check_deployment_update /usr/local/bin/paperproof-check-deployment-update
COPY sql ./sql

RUN mkdir -p /var/lib/paperproof/indexer \
    && chown -R paperproof:paperproof /var/lib/paperproof /app

USER paperproof

ENV RUST_LOG=paperproof_sdk_rs=info
ENV PAPERPROOF_RS_INDEXER_OUT=/var/lib/paperproof/indexer
ENV PAPERPROOF_RS_TAIL=1
ENV PAPERPROOF_RS_TAIL_INTERVAL_MS=10000
ENV PAPERPROOF_RS_TAIL_LIMIT=50

VOLUME ["/var/lib/paperproof"]

ENTRYPOINT ["/usr/local/bin/paperproof-indexer-tail"]
