// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde_json::{Value, json};

use crate::{
    error::Result,
    events::SuiEventEnvelope,
    query::{
        EventPage, EventQueryInput, PaginationInput, PaperProofQueryClient, dedupe_events,
        event_dedupe_key,
    },
};

#[derive(Clone, Debug)]
pub struct WatchOptions {
    pub cursor: Option<Value>,
    pub limit: Option<u64>,
    pub descending_order: bool,
    pub max_pages_per_tick: usize,
    pub dedupe: bool,
    pub max_seen_events: usize,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: None,
            descending_order: false,
            max_pages_per_tick: 1,
            dedupe: true,
            max_seen_events: 10_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaperProofEventWatcher {
    query: PaperProofQueryClient,
    fetch: WatchFetch,
    pub options: WatchOptions,
    pub cursor: Option<Value>,
    pub stopped: bool,
    seen: Vec<String>,
}

#[derive(Clone, Debug)]
enum WatchFetch {
    Events,
    CanonicalEvents,
    MoveEventType(String),
    Governance(String),
    EventTypes(Vec<String>),
}

impl PaperProofEventWatcher {
    pub fn new(query: PaperProofQueryClient, options: WatchOptions) -> Self {
        let cursor = options.cursor.clone();
        Self {
            query,
            fetch: WatchFetch::CanonicalEvents,
            options,
            cursor,
            stopped: false,
            seen: Vec::new(),
        }
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub async fn next(&mut self) -> Result<EventPage<SuiEventEnvelope>> {
        if self.stopped {
            return Ok(EventPage {
                data: Vec::new(),
                next_cursor: self.cursor.clone(),
                has_next_page: false,
                raw: Value::Null,
            });
        }
        let mut events = Vec::new();
        let mut raw_pages = Vec::new();
        let mut next_cursor = self.cursor.clone();
        let mut has_next_page = false;
        for _ in 0..self.options.max_pages_per_tick.max(1) {
            let mut input = EventQueryInput {
                pagination: PaginationInput {
                    cursor: next_cursor.clone(),
                    limit: self.options.limit,
                    descending_order: Some(self.options.descending_order),
                },
                ..Default::default()
            };
            let page = self.fetch_page(&mut input).await?;
            let fresh = if self.options.dedupe {
                self.fresh_events(page.data.clone())
            } else {
                page.data.clone()
            };
            events.extend(fresh);
            raw_pages.push(page.raw);
            next_cursor = page.next_cursor;
            has_next_page = page.has_next_page;
            if !has_next_page || next_cursor.is_none() {
                break;
            }
        }
        self.cursor = next_cursor.clone();
        Ok(EventPage {
            data: events,
            next_cursor,
            has_next_page,
            raw: json!({ "pages": raw_pages }),
        })
    }

    async fn fetch_page(&self, input: &mut EventQueryInput) -> Result<EventPage<SuiEventEnvelope>> {
        match &self.fetch {
            WatchFetch::Events => self.query.query_events(input.clone()).await,
            WatchFetch::CanonicalEvents => self.query.query_canonical_events(input.clone()).await,
            WatchFetch::MoveEventType(move_event_type) => {
                input.move_event_type = Some(move_event_type.clone());
                self.query.query_canonical_events(input.clone()).await
            }
            WatchFetch::Governance(struct_name) => {
                self.query
                    .query_governance_events(struct_name, Some(input.clone()))
                    .await
            }
            WatchFetch::EventTypes(event_types) => {
                let mut pages = Vec::new();
                for event_type in event_types {
                    let mut query = input.clone();
                    query.move_event_type = Some(event_type.clone());
                    pages.push(self.query.query_canonical_events(query).await?);
                }
                Ok(combine_pages(pages))
            }
        }
    }

    fn fresh_events(&mut self, events: Vec<SuiEventEnvelope>) -> Vec<SuiEventEnvelope> {
        let mut fresh = Vec::new();
        for event in events {
            let key = event_dedupe_key(&event);
            if self.seen.contains(&key) {
                continue;
            }
            self.seen.push(key);
            fresh.push(event);
        }
        while self.seen.len() > self.options.max_seen_events.max(1) {
            self.seen.remove(0);
        }
        fresh
    }
}

#[derive(Clone, Debug)]
pub struct PaperProofWatchClient {
    pub query: PaperProofQueryClient,
}

impl PaperProofWatchClient {
    pub fn new(query: PaperProofQueryClient) -> Self {
        Self { query }
    }

    pub fn mainnet() -> Self {
        Self::new(PaperProofQueryClient::mainnet())
    }

    pub fn watch_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watcher(WatchFetch::Events, options)
    }

    pub fn watch_canonical_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watcher(WatchFetch::CanonicalEvents, options)
    }

    pub fn watch_publishing_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_move_event_type(
            "publishing",
            struct_name,
            &self.query.deployment.packages.publishing,
            options,
        )
    }

    pub fn watch_comments_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_move_event_type(
            "comments",
            struct_name,
            &self.query.deployment.packages.comments,
            options,
        )
    }

    pub fn watch_governance_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watcher(WatchFetch::Governance(struct_name.to_string()), options)
    }

    pub fn watch_artifact_published_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watch_publishing_events("ArtifactPublishedEvent", options)
    }

    pub fn watch_artifact_version_added_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_publishing_events("ArtifactVersionAddedEvent", options)
    }

    pub fn watch_artifact_status_changed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_publishing_events("ArtifactStatusChangedEvent", options)
    }

    pub fn watch_protocol_paused_changed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_publishing_events("ProtocolPausedChangedEvent", options)
    }

    pub fn watch_artifact_owner_transferred_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_publishing_events("ArtifactOwnerTransferredEvent", options)
    }

    pub fn watch_comment_added_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watch_comments_events("CommentAddedEvent", options)
    }

    pub fn watch_paper_liked_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watch_comments_events("PaperLikedEvent", options)
    }

    pub fn watch_paper_unliked_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watch_comments_events("PaperUnlikedEvent", options)
    }

    pub fn watch_tree_status_changed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_comments_events("TreeStatusChangedEvent", options)
    }

    pub fn watch_comment_status_changed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_comments_events("CommentStatusChangedEvent", options)
    }

    pub fn watch_tree_owner_transferred_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_comments_events("TreeOwnerTransferredEvent", options)
    }

    pub fn watch_governance_proposal_created_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_governance_events("ProposalCreatedEvent", options)
    }

    pub fn watch_governance_vote_cast_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_governance_events("VoteCastEvent", options)
    }

    pub fn watch_governance_finalized_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_governance_events("ProposalFinalizedEvent", options)
    }

    pub fn watch_governance_executed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_governance_events("ProposalExecutedEvent", options)
    }

    pub fn watch_governance_expired_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watch_governance_events("ProposalExpiredEvent", options)
    }

    pub fn watch_governance_vote_claimed_events(
        &self,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watch_governance_events("VoteClaimedEvent", options)
    }

    pub fn watch_status_changed_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watcher(
            WatchFetch::EventTypes(vec![
                move_event_type(
                    &self.query.deployment.packages.publishing,
                    "publishing",
                    "ArtifactStatusChangedEvent",
                ),
                move_event_type(
                    &self.query.deployment.packages.publishing,
                    "publishing",
                    "ProtocolPausedChangedEvent",
                ),
                move_event_type(
                    &self.query.deployment.packages.comments,
                    "comments",
                    "TreeStatusChangedEvent",
                ),
                move_event_type(
                    &self.query.deployment.packages.comments,
                    "comments",
                    "CommentStatusChangedEvent",
                ),
            ]),
            options,
        )
    }

    pub fn watch_owner_transferred_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        self.watcher(
            WatchFetch::EventTypes(vec![
                move_event_type(
                    &self.query.deployment.packages.publishing,
                    "publishing",
                    "ArtifactOwnerTransferredEvent",
                ),
                move_event_type(
                    &self.query.deployment.packages.comments,
                    "comments",
                    "TreeOwnerTransferredEvent",
                ),
            ]),
            options,
        )
    }

    fn watch_move_event_type(
        &self,
        module: &str,
        struct_name: &str,
        package_id: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watcher(
            WatchFetch::MoveEventType(move_event_type(package_id, module, struct_name)),
            options,
        )
    }

    fn watcher(&self, fetch: WatchFetch, options: WatchOptions) -> PaperProofEventWatcher {
        let cursor = options.cursor.clone();
        PaperProofEventWatcher {
            query: self.query.clone(),
            fetch,
            options,
            cursor,
            stopped: false,
            seen: Vec::new(),
        }
    }
}

fn move_event_type(package_id: &str, module: &str, struct_name: &str) -> String {
    format!("{package_id}::{module}::{struct_name}")
}

fn combine_pages(pages: Vec<EventPage<SuiEventEnvelope>>) -> EventPage<SuiEventEnvelope> {
    EventPage {
        data: dedupe_events(pages.iter().flat_map(|page| page.data.clone()).collect()),
        next_cursor: None,
        has_next_page: pages.iter().any(|page| page.has_next_page),
        raw: json!({ "pages": pages.into_iter().map(|page| page.raw).collect::<Vec<_>>() }),
    }
}
