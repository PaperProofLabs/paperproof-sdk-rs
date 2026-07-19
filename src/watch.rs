// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde_json::{Value, json};

use crate::{
    deployment::{DeploymentPackageFamily, deployment_package_ids},
    error::Result,
    events::SuiEventEnvelope,
    events_trust::EventTrustLevel,
    query::{
        EventPage, EventQueryInput, PaginationInput, PaperProofQueryClient, TrustedEventPage,
        TrustedEventQueryInput, dedupe_events, event_dedupe_key,
    },
};

#[derive(Clone, Debug)]
struct TrustedWatchConfig {
    trust: EventTrustLevel,
    include_rejected: bool,
    verify_walrus: bool,
}

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
pub struct PaperProofTrustedEventWatcher {
    query: PaperProofQueryClient,
    pub options: WatchOptions,
    pub filter: EventQueryInput,
    event_types: Option<Vec<String>>,
    pub trust: EventTrustLevel,
    pub include_rejected: bool,
    pub verify_walrus: bool,
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

impl PaperProofTrustedEventWatcher {
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub async fn next(&mut self) -> Result<TrustedEventPage> {
        if self.stopped {
            return Ok(TrustedEventPage {
                data: Vec::new(),
                next_cursor: self.cursor.clone(),
                has_next_page: false,
                raw: Value::Null,
                trust: self.trust.clone(),
                verification: Vec::new(),
                rejected: Vec::new(),
                incomplete: Vec::new(),
            });
        }
        let mut pages = Vec::new();
        let mut next_cursor = self.cursor.clone();
        for _ in 0..self.options.max_pages_per_tick.max(1) {
            let base_input = EventQueryInput {
                sender: self.filter.sender.clone(),
                package_id: self.filter.package_id.clone(),
                module: self.filter.module.clone(),
                event_type: self.filter.event_type.clone(),
                move_event_type: self.filter.move_event_type.clone(),
                start_time_ms: self.filter.start_time_ms,
                end_time_ms: self.filter.end_time_ms,
                pagination: PaginationInput {
                    cursor: next_cursor.clone(),
                    limit: self.options.limit,
                    descending_order: Some(self.options.descending_order),
                },
            };
            let mut page = if let Some(event_types) = &self.event_types {
                let mut trusted_pages = Vec::new();
                for event_type in event_types {
                    let mut input = base_input.clone();
                    input.move_event_type = Some(event_type.clone());
                    trusted_pages.push(
                        self.query
                            .query_trusted_events(TrustedEventQueryInput {
                                query: input,
                                trust: self.trust.clone(),
                                include_rejected: self.include_rejected,
                                verify_walrus: self.verify_walrus,
                            })
                            .await?,
                    );
                }
                combine_trusted_pages(trusted_pages, self.trust.clone())
            } else {
                self.query
                    .query_trusted_events(TrustedEventQueryInput {
                        query: base_input,
                        trust: self.trust.clone(),
                        include_rejected: self.include_rejected,
                        verify_walrus: self.verify_walrus,
                    })
                    .await?
            };
            if self.options.dedupe {
                let fresh = page
                    .data
                    .into_iter()
                    .filter(|event| {
                        let key = event_dedupe_key(&event.event);
                        if self.seen.contains(&key) {
                            false
                        } else {
                            self.seen.push(key);
                            true
                        }
                    })
                    .collect();
                page.data = fresh;
                while self.seen.len() > self.options.max_seen_events.max(1) {
                    self.seen.remove(0);
                }
            }
            next_cursor = page.next_cursor.clone();
            let done = !page.has_next_page || next_cursor.is_none();
            pages.push(page);
            if done {
                break;
            }
        }
        self.cursor = next_cursor.clone();
        Ok(combine_trusted_pages(pages, self.trust.clone()))
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

    pub fn watch_trusted_events(
        &self,
        trust: EventTrustLevel,
        options: WatchOptions,
    ) -> PaperProofTrustedEventWatcher {
        PaperProofTrustedEventWatcher {
            query: self.query.clone(),
            cursor: options.cursor.clone(),
            options,
            filter: EventQueryInput::default(),
            event_types: None,
            trust,
            include_rejected: false,
            verify_walrus: false,
            stopped: false,
            seen: Vec::new(),
        }
    }

    pub fn watch_verified_events(&self, options: WatchOptions) -> PaperProofTrustedEventWatcher {
        self.watch_trusted_events(EventTrustLevel::Verified, options)
    }

    pub fn watch_trusted_events_with_verification_options(
        &self,
        trust: EventTrustLevel,
        options: WatchOptions,
        include_rejected: bool,
        verify_walrus: bool,
    ) -> PaperProofTrustedEventWatcher {
        PaperProofTrustedEventWatcher {
            query: self.query.clone(),
            cursor: options.cursor.clone(),
            options,
            filter: EventQueryInput::default(),
            event_types: None,
            trust,
            include_rejected,
            verify_walrus,
            stopped: false,
            seen: Vec::new(),
        }
    }

    pub fn watch_publishing_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watcher(
            WatchFetch::EventTypes(
                deployment_package_ids(&self.query.deployment, DeploymentPackageFamily::Publishing)
                    .into_iter()
                    .map(|package_id| move_event_type(&package_id, "publishing", struct_name))
                    .collect(),
            ),
            options,
        )
    }

    pub fn watch_comments_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
    ) -> PaperProofEventWatcher {
        self.watcher(
            WatchFetch::EventTypes(
                deployment_package_ids(&self.query.deployment, DeploymentPackageFamily::Comments)
                    .into_iter()
                    .map(|package_id| move_event_type(&package_id, "comments", struct_name))
                    .collect(),
            ),
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

    pub fn watch_verified_publishing_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
        include_rejected: bool,
        verify_walrus: bool,
    ) -> PaperProofTrustedEventWatcher {
        self.watch_trusted_event_types(
            TrustedWatchConfig {
                trust: EventTrustLevel::Verified,
                include_rejected,
                verify_walrus,
            },
            deployment_package_ids(&self.query.deployment, DeploymentPackageFamily::Publishing)
                .into_iter()
                .map(|package_id| move_event_type(&package_id, "publishing", struct_name))
                .collect(),
            options,
        )
    }

    pub fn watch_verified_comments_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
        include_rejected: bool,
        verify_walrus: bool,
    ) -> PaperProofTrustedEventWatcher {
        self.watch_trusted_event_types(
            TrustedWatchConfig {
                trust: EventTrustLevel::Verified,
                include_rejected,
                verify_walrus,
            },
            deployment_package_ids(&self.query.deployment, DeploymentPackageFamily::Comments)
                .into_iter()
                .map(|package_id| move_event_type(&package_id, "comments", struct_name))
                .collect(),
            options,
        )
    }

    pub fn watch_verified_governance_events(
        &self,
        struct_name: &str,
        options: WatchOptions,
        include_rejected: bool,
        verify_walrus: bool,
    ) -> PaperProofTrustedEventWatcher {
        self.watch_trusted_event_types(
            TrustedWatchConfig {
                trust: EventTrustLevel::Verified,
                include_rejected,
                verify_walrus,
            },
            deployment_package_ids(&self.query.deployment, DeploymentPackageFamily::Governance)
                .into_iter()
                .map(|package_id| move_event_type(&package_id, "governance_voting", struct_name))
                .collect(),
            options,
        )
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
        let publishing = deployment_package_ids(
            &self.query.deployment,
            DeploymentPackageFamily::Publishing,
        );
        let comments = deployment_package_ids(
            &self.query.deployment,
            DeploymentPackageFamily::Comments,
        );
        self.watcher(
            WatchFetch::EventTypes(
                publishing
                    .iter()
                    .map(|package_id| {
                        move_event_type(package_id, "publishing", "ArtifactStatusChangedEvent")
                    })
                    .chain(publishing.iter().map(|package_id| {
                        move_event_type(package_id, "publishing", "ProtocolPausedChangedEvent")
                    }))
                    .chain(comments.iter().map(|package_id| {
                        move_event_type(package_id, "comments", "TreeStatusChangedEvent")
                    }))
                    .chain(comments.iter().map(|package_id| {
                        move_event_type(package_id, "comments", "CommentStatusChangedEvent")
                    }))
                    .collect(),
            ),
            options,
        )
    }

    pub fn watch_owner_transferred_events(&self, options: WatchOptions) -> PaperProofEventWatcher {
        let publishing = deployment_package_ids(
            &self.query.deployment,
            DeploymentPackageFamily::Publishing,
        );
        let comments = deployment_package_ids(
            &self.query.deployment,
            DeploymentPackageFamily::Comments,
        );
        self.watcher(
            WatchFetch::EventTypes(
                publishing
                    .iter()
                    .map(|package_id| {
                        move_event_type(package_id, "publishing", "ArtifactOwnerTransferredEvent")
                    })
                    .chain(comments.iter().map(|package_id| {
                        move_event_type(package_id, "comments", "TreeOwnerTransferredEvent")
                    }))
                    .collect(),
            ),
            options,
        )
    }

    fn watch_trusted_event_types(
        &self,
        config: TrustedWatchConfig,
        event_types: Vec<String>,
        options: WatchOptions,
    ) -> PaperProofTrustedEventWatcher {
        PaperProofTrustedEventWatcher {
            query: self.query.clone(),
            cursor: None,
            options,
            filter: EventQueryInput::default(),
            event_types: Some(event_types),
            trust: config.trust,
            include_rejected: config.include_rejected,
            verify_walrus: config.verify_walrus,
            stopped: false,
            seen: Vec::new(),
        }
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

fn combine_trusted_pages(pages: Vec<TrustedEventPage>, trust: EventTrustLevel) -> TrustedEventPage {
    let next_cursor = pages.last().and_then(|page| page.next_cursor.clone());
    let has_next_page = pages.iter().any(|page| page.has_next_page);
    let raw = json!({ "pages": pages.iter().map(|page| page.raw.clone()).collect::<Vec<_>>() });
    TrustedEventPage {
        data: pages.iter().flat_map(|page| page.data.clone()).collect(),
        next_cursor,
        has_next_page,
        raw,
        trust,
        verification: pages
            .iter()
            .flat_map(|page| page.verification.clone())
            .collect(),
        rejected: pages
            .iter()
            .flat_map(|page| page.rejected.clone())
            .collect(),
        incomplete: pages
            .iter()
            .flat_map(|page| page.incomplete.clone())
            .collect(),
    }
}
