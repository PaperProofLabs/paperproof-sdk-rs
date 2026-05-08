-- Copyright (c) 2026 PaperProof Labs
-- SPDX-License-Identifier: Apache-2.0

create table if not exists paperproof_indexer_cursors (
    stream_id text primary key,
    event_cursor jsonb,
    checkpoint_cursor bigint,
    updated_at timestamptz not null default now()
);

create table if not exists paperproof_events (
    event_key text primary key,
    checkpoint bigint,
    transaction_digest text,
    event_seq bigint,
    package_id text not null,
    module text not null,
    event_type text not null,
    kind text not null,
    sender text,
    timestamp_ms bigint,
    parsed_json jsonb not null,
    inserted_at timestamptz not null default now()
);

create table if not exists paperproof_rejected_events (
    event_key text primary key,
    checkpoint bigint,
    transaction_digest text,
    event_seq bigint,
    package_id text not null,
    module text not null,
    event_type text not null,
    sender text,
    timestamp_ms bigint,
    reason text not null,
    parsed_json jsonb not null,
    inserted_at timestamptz not null default now()
);

create index if not exists paperproof_events_checkpoint_idx on paperproof_events(checkpoint);
create index if not exists paperproof_events_kind_idx on paperproof_events(kind);
create index if not exists paperproof_events_package_module_idx on paperproof_events(package_id, module);
