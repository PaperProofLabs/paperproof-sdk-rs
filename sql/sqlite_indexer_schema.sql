-- Copyright (c) 2026 PaperProof Labs
-- SPDX-License-Identifier: Apache-2.0

create table if not exists paperproof_indexer_cursors (
    stream_id text primary key,
    event_cursor text,
    checkpoint_cursor integer,
    updated_at text not null default current_timestamp
);

create table if not exists paperproof_events (
    event_key text primary key,
    checkpoint integer,
    transaction_digest text,
    event_seq integer,
    package_id text not null,
    module text not null,
    event_type text not null,
    kind text not null,
    sender text,
    timestamp_ms integer,
    parsed_json text not null,
    inserted_at text not null default current_timestamp
);

create table if not exists paperproof_rejected_events (
    event_key text primary key,
    checkpoint integer,
    transaction_digest text,
    event_seq integer,
    package_id text not null,
    module text not null,
    event_type text not null,
    sender text,
    timestamp_ms integer,
    reason text not null,
    parsed_json text not null,
    inserted_at text not null default current_timestamp
);

create index if not exists paperproof_events_checkpoint_idx on paperproof_events(checkpoint);
create index if not exists paperproof_events_kind_idx on paperproof_events(kind);
create index if not exists paperproof_events_package_module_idx on paperproof_events(package_id, module);
