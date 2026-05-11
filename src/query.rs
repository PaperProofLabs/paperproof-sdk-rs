// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    client::JsonRpcClient,
    deployment::Deployment,
    error::{PaperProofError, Result},
    event_verifier::{PaperProofEventVerifier, VerifyEventOptions},
    events::{SuiEventEnvelope, extract_events_by_struct},
    events_trust::{
        EventTrustLevel, EventVerificationReport, TrustedSuiEventEnvelope,
        attach_event_verification, validate_event_trust, verification_report_from_canonical_check,
    },
    read::PaperProofReadClient,
    types::{ArtifactSeriesView, ArtifactVersionView, CommentsTreeView, LikesBookView},
    validation::{validate_address, validate_object_id},
};

pub const MAINNET_GRAPHQL_ENDPOINT: &str = "https://rpc.ankr.com/http/sui_graphql";
pub const TESTNET_GRAPHQL_ENDPOINT: &str = "https://rpc.ankr.com/http/sui_testnet_graphql";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PaginationInput {
    pub cursor: Option<Value>,
    pub limit: Option<u64>,
    pub descending_order: Option<bool>,
}

impl crate::events_trust::VerifiedEventPageGuard for TrustedEventPage {
    fn trust_level(&self) -> EventTrustLevel {
        self.trust.clone()
    }

    fn verification_reports(&self) -> &[EventVerificationReport] {
        &self.verification
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EventQueryInput {
    pub sender: Option<String>,
    pub package_id: Option<String>,
    pub module: Option<String>,
    pub event_type: Option<String>,
    pub move_event_type: Option<String>,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
    pub pagination: PaginationInput,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventPage<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<Value>,
    pub has_next_page: bool,
    pub raw: Value,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TrustedEventQueryInput {
    pub query: EventQueryInput,
    pub trust: EventTrustLevel,
    pub include_rejected: bool,
    pub verify_walrus: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TrustedEventPage {
    pub data: Vec<TrustedSuiEventEnvelope>,
    pub next_cursor: Option<Value>,
    pub has_next_page: bool,
    pub raw: Value,
    pub trust: EventTrustLevel,
    pub verification: Vec<EventVerificationReport>,
    pub rejected: Vec<EventVerificationReport>,
    pub incomplete: Vec<EventVerificationReport>,
}

#[derive(Clone, Debug)]
pub enum PaperProofQueryProvider {
    GraphQl(GraphQlQueryProvider),
    JsonRpc(JsonRpcClient),
    Fallback {
        graphql: GraphQlQueryProvider,
        jsonrpc: JsonRpcClient,
    },
}

#[derive(Clone, Debug)]
pub struct GraphQlQueryProvider {
    pub endpoint: String,
    http: reqwest::Client,
}

impl GraphQlQueryProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn query_events(
        &self,
        input: EventQueryInput,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        let descending = input.pagination.descending_order.unwrap_or(false);
        let limit = input.pagination.limit.unwrap_or(50).min(50) as i64;
        let variables = json!({
            "filter": build_graphql_event_filter(&input)?,
            "first": if descending { Value::Null } else { json!(limit) },
            "last": if descending { json!(limit) } else { Value::Null },
            "after": if descending { Value::Null } else { input.pagination.cursor.clone().unwrap_or(Value::Null) },
            "before": if descending { input.pagination.cursor.clone().unwrap_or(Value::Null) } else { Value::Null },
        });
        let response = self
            .http
            .post(&self.endpoint)
            .json(&json!({ "query": EVENTS_QUERY, "variables": variables }))
            .send()
            .await
            .map_err(|err| PaperProofError::network(&self.endpoint, err.to_string()))?;
        let status = response.status();
        let raw: Value = response
            .json()
            .await
            .map_err(|err| PaperProofError::network(&self.endpoint, err.to_string()))?;
        if !status.is_success() {
            return Err(PaperProofError::network(
                &self.endpoint,
                format!("HTTP {status}: {raw}"),
            ));
        }
        if let Some(errors) = raw.get("errors") {
            return Err(PaperProofError::network(
                &self.endpoint,
                format!("GraphQL event query failed: {errors}"),
            ));
        }
        let events = raw.pointer("/data/events").cloned().unwrap_or(Value::Null);
        let nodes = events
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let data = nodes
            .into_iter()
            .map(event_from_graphql_node)
            .collect::<Result<Vec<_>>>()?;
        let page_info = events.get("pageInfo").cloned().unwrap_or(Value::Null);
        Ok(EventPage {
            data: filter_by_time(data, input.start_time_ms, input.end_time_ms),
            next_cursor: page_info
                .get(if descending {
                    "startCursor"
                } else {
                    "endCursor"
                })
                .cloned()
                .filter(|value| !value.is_null()),
            has_next_page: page_info
                .get(if descending {
                    "hasPreviousPage"
                } else {
                    "hasNextPage"
                })
                .and_then(Value::as_bool)
                .unwrap_or(false),
            raw,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SeriesDetails {
    pub series: ArtifactSeriesView,
    pub current_version: Option<ArtifactVersionView>,
    pub comments_tree: Option<CommentsTreeView>,
    pub likes_book: Option<LikesBookView>,
}

#[derive(Clone, Debug)]
pub struct PaperProofQueryClient {
    pub read: PaperProofReadClient,
    pub deployment: Deployment,
    pub query_provider: PaperProofQueryProvider,
}

impl PaperProofQueryClient {
    pub fn new(rpc: JsonRpcClient, deployment: Deployment) -> Self {
        Self::new_jsonrpc(rpc, deployment)
    }

    pub fn new_jsonrpc(rpc: JsonRpcClient, deployment: Deployment) -> Self {
        Self {
            read: PaperProofReadClient::new(rpc.clone(), deployment.clone()),
            query_provider: PaperProofQueryProvider::JsonRpc(rpc),
            deployment,
        }
    }

    pub fn new_graphql(
        read_rpc: JsonRpcClient,
        graphql: GraphQlQueryProvider,
        deployment: Deployment,
    ) -> Self {
        Self {
            read: PaperProofReadClient::new(read_rpc, deployment.clone()),
            query_provider: PaperProofQueryProvider::GraphQl(graphql),
            deployment,
        }
    }

    pub fn new_fallback(
        read_rpc: JsonRpcClient,
        graphql: GraphQlQueryProvider,
        jsonrpc: JsonRpcClient,
        deployment: Deployment,
    ) -> Self {
        Self {
            read: PaperProofReadClient::new(read_rpc, deployment.clone()),
            query_provider: PaperProofQueryProvider::Fallback { graphql, jsonrpc },
            deployment,
        }
    }

    pub fn mainnet() -> Self {
        let deployment = crate::deployment::mainnet_deployment();
        Self::new_graphql(
            JsonRpcClient::new(deployment.rpc_url.clone()),
            GraphQlQueryProvider::new(MAINNET_GRAPHQL_ENDPOINT),
            deployment,
        )
    }

    pub fn mainnet_jsonrpc() -> Self {
        let deployment = crate::deployment::mainnet_deployment();
        Self::new_jsonrpc(JsonRpcClient::new(deployment.rpc_url.clone()), deployment)
    }

    pub async fn get_series_details(&self, series_id: &str) -> Result<SeriesDetails> {
        let series = self.read.get_series_view(series_id).await?;
        let current_version = match series.current_version_id.as_deref() {
            Some(version_id) => Some(self.read.get_version_view(version_id).await?),
            None => None,
        };
        let comments_tree = match series.comments_tree_id.as_deref() {
            Some(tree_id) => Some(self.read.get_comments_tree_view(tree_id).await?),
            None => None,
        };
        let likes_book = match series.likes_book_id.as_deref() {
            Some(book_id) => Some(self.read.get_likes_book_view(book_id).await?),
            None => None,
        };
        Ok(SeriesDetails {
            series,
            current_version,
            comments_tree,
            likes_book,
        })
    }

    pub async fn query_events(
        &self,
        input: EventQueryInput,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        match &self.query_provider {
            PaperProofQueryProvider::GraphQl(graphql) => graphql.query_events(input).await,
            PaperProofQueryProvider::JsonRpc(jsonrpc) => query_events_jsonrpc(jsonrpc, input).await,
            PaperProofQueryProvider::Fallback { graphql, jsonrpc } => {
                match graphql.query_events(input.clone()).await {
                    Ok(page) => Ok(page),
                    Err(_) => query_events_jsonrpc(jsonrpc, input).await,
                }
            }
        }
    }

    pub async fn query_canonical_events(
        &self,
        input: EventQueryInput,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        let mut page = self.query_events(input).await?;
        page.data
            .retain(|event| validate_event_trust(event, &self.deployment).trusted);
        Ok(page)
    }

    pub async fn query_trusted_events(
        &self,
        input: TrustedEventQueryInput,
    ) -> Result<TrustedEventPage> {
        let page = if input.trust == EventTrustLevel::Verified {
            self.query_canonical_events(input.query.clone()).await?
        } else {
            self.query_events(input.query.clone()).await?
        };
        if input.trust == EventTrustLevel::Raw {
            let reports = page
                .data
                .iter()
                .map(|event| {
                    verification_report_from_canonical_check(
                        event,
                        &self.deployment,
                        EventTrustLevel::Raw,
                    )
                })
                .collect::<Vec<_>>();
            return Ok(trusted_page_from_reports(
                page,
                EventTrustLevel::Raw,
                reports,
                input.include_rejected,
            ));
        }
        if input.trust == EventTrustLevel::Canonical {
            let reports = page
                .data
                .iter()
                .map(|event| {
                    verification_report_from_canonical_check(
                        event,
                        &self.deployment,
                        EventTrustLevel::Canonical,
                    )
                })
                .collect::<Vec<_>>();
            return Ok(trusted_page_from_reports(
                page,
                EventTrustLevel::Canonical,
                reports,
                input.include_rejected,
            ));
        }
        let verifier = PaperProofEventVerifier::new(self.read.clone());
        let mut reports = Vec::new();
        for event in &page.data {
            reports.push(
                verifier
                    .verify_event(
                        event,
                        VerifyEventOptions {
                            trust: EventTrustLevel::Verified,
                            verify_walrus: input.verify_walrus,
                            provider: None,
                        },
                    )
                    .await?,
            );
        }
        Ok(trusted_page_from_reports(
            page,
            EventTrustLevel::Verified,
            reports,
            input.include_rejected,
        ))
    }

    pub async fn query_verified_events(
        &self,
        query: EventQueryInput,
        include_rejected: bool,
    ) -> Result<TrustedEventPage> {
        self.query_trusted_events(TrustedEventQueryInput {
            query,
            trust: EventTrustLevel::Verified,
            include_rejected,
            verify_walrus: false,
        })
        .await
    }

    pub async fn query_all_events(
        &self,
        mut input: EventQueryInput,
        max_pages: usize,
    ) -> Result<Vec<SuiEventEnvelope>> {
        let mut events = Vec::new();
        for _ in 0..max_pages.max(1) {
            let page = self.query_events(input.clone()).await?;
            events.extend(page.data);
            if !page.has_next_page {
                break;
            }
            input.pagination.cursor = page.next_cursor;
        }
        Ok(events)
    }

    pub fn parse_events_by_struct<T>(
        &self,
        events: &[SuiEventEnvelope],
        struct_name: &str,
    ) -> Result<Vec<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        extract_events_by_struct(events, struct_name, Some(&self.deployment))
    }

    pub async fn query_governance_events(
        &self,
        struct_name: &str,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        if !struct_name.ends_with("Event") {
            return Err(PaperProofError::invalid_input(
                "struct_name",
                "governance event struct name must end with Event",
            ));
        }
        let mut all = Vec::new();
        let mut raw_pages = Vec::new();
        let mut next_cursor = None;
        let mut has_next_page = false;
        for package_id in [
            &self.deployment.packages.governance,
            &self.deployment.packages.governance_original,
        ] {
            let mut query = input.clone().unwrap_or_default();
            query.move_event_type = Some(format!("{package_id}::governance_voting::{struct_name}"));
            query.pagination.limit = Some(query.pagination.limit.unwrap_or(50).min(50));
            let page = self.query_events(query).await?;
            next_cursor = page.next_cursor.clone().or(next_cursor);
            has_next_page |= page.has_next_page;
            raw_pages.push(page.raw);
            all.extend(page.data);
        }
        let data = dedupe_events(
            all.into_iter()
                .filter(|event| validate_event_trust(event, &self.deployment).trusted)
                .collect(),
        );
        Ok(EventPage {
            data,
            next_cursor,
            has_next_page,
            raw: json!({ "pages": raw_pages }),
        })
    }

    pub async fn query_governance_proposal_created_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("ProposalCreatedEvent", input)
            .await
    }

    pub async fn query_governance_vote_cast_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("VoteCastEvent", input).await
    }

    pub async fn query_governance_finalized_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("ProposalFinalizedEvent", input)
            .await
    }

    pub async fn query_governance_executed_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("ProposalExecutedEvent", input)
            .await
    }

    pub async fn query_governance_expired_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("ProposalExpiredEvent", input)
            .await
    }

    pub async fn query_governance_vote_claimed_events(
        &self,
        input: Option<EventQueryInput>,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        self.query_governance_events("VoteClaimedEvent", input)
            .await
    }
}

fn trusted_page_from_reports(
    page: EventPage<SuiEventEnvelope>,
    trust: EventTrustLevel,
    reports: Vec<EventVerificationReport>,
    include_rejected: bool,
) -> TrustedEventPage {
    let rejected = reports
        .iter()
        .filter(|report| report.status == crate::events_trust::EventVerificationStatus::Rejected)
        .cloned()
        .collect::<Vec<_>>();
    let incomplete = reports
        .iter()
        .filter(|report| report.status == crate::events_trust::EventVerificationStatus::Incomplete)
        .cloned()
        .collect::<Vec<_>>();
    let data = reports
        .iter()
        .filter(|report| include_rejected || report.trusted)
        .cloned()
        .map(attach_event_verification)
        .collect();
    TrustedEventPage {
        data,
        next_cursor: page.next_cursor,
        has_next_page: page.has_next_page,
        raw: page.raw,
        trust,
        verification: reports,
        rejected,
        incomplete,
    }
}

async fn query_events_jsonrpc(
    jsonrpc: &JsonRpcClient,
    input: EventQueryInput,
) -> Result<EventPage<SuiEventEnvelope>> {
    let filter = build_event_filter(&input)?;
    let descending = input.pagination.descending_order.unwrap_or(false);
    let raw = jsonrpc
        .rpc(
            "suix_queryEvents",
            json!([
                filter,
                input.pagination.cursor,
                input.pagination.limit.unwrap_or(50),
                descending
            ]),
        )
        .await?;
    let data = raw
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<SuiEventEnvelope>, _>>()?;
    Ok(EventPage {
        data: filter_by_time(data, input.start_time_ms, input.end_time_ms),
        next_cursor: raw
            .get("nextCursor")
            .cloned()
            .filter(|value| !value.is_null()),
        has_next_page: raw
            .get("hasNextPage")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        raw,
    })
}

pub fn build_event_filter(input: &EventQueryInput) -> Result<Value> {
    if input.move_event_type.is_some()
        && (input.event_type.is_some()
            || input.sender.is_some()
            || input.package_id.is_some()
            || input.module.is_some())
    {
        return Err(PaperProofError::EventParse {
            message: "move_event_type cannot be combined with sender/package/module filters for one Sui queryEvents call".to_string(),
        });
    }
    if input.sender.is_some()
        && (input.event_type.is_some() || input.package_id.is_some() || input.module.is_some())
    {
        return Err(PaperProofError::EventParse {
            message: "sender cannot be combined with event type or package/module filters in one Sui queryEvents call; query one filter and post-filter locally".to_string(),
        });
    }
    if input.module.is_some() && input.package_id.is_none() {
        return Err(PaperProofError::EventParse {
            message: "module event queries require package_id".to_string(),
        });
    }
    if let Some(move_event_type) = &input.move_event_type {
        return Ok(json!({ "MoveEventType": move_event_type }));
    }
    if let Some(event_type) = &input.event_type {
        return Ok(json!({ "MoveEventType": event_type }));
    }
    if let Some(sender) = &input.sender {
        validate_address(sender)?;
        return Ok(json!({ "Sender": sender }));
    }
    if let Some(package_id) = &input.package_id {
        validate_object_id(package_id)?;
        if let Some(module) = &input.module {
            return Ok(json!({ "MoveModule": { "package": package_id, "module": module } }));
        }
        return Ok(json!({ "Package": package_id }));
    }
    Ok(Value::Null)
}

pub fn build_graphql_event_filter(input: &EventQueryInput) -> Result<Value> {
    if let Some(move_event_type) = &input.move_event_type {
        return Ok(json!({ "type": move_event_type }));
    }
    if let Some(event_type) = &input.event_type {
        return Ok(json!({ "type": event_type }));
    }
    if let Some(sender) = &input.sender {
        validate_address(sender)?;
        return Ok(json!({ "sender": sender }));
    }
    if let Some(package_id) = &input.package_id {
        validate_object_id(package_id)?;
        if let Some(module) = &input.module {
            return Ok(json!({ "module": format!("{package_id}::{module}") }));
        }
        return Err(PaperProofError::EventParse {
            message: "Sui GraphQL EventFilter does not support package-only event filtering; use package+module or a fully qualified event type".to_string(),
        });
    }
    match (input.start_time_ms, input.end_time_ms) {
        (Some(_), _) | (_, Some(_)) => Err(PaperProofError::EventParse {
            message: "Sui GraphQL EventFilter does not support timestamp event filtering; use checkpoint filters or filter timestamps locally".to_string(),
        }),
        (None, None) => Ok(Value::Null),
    }
}

pub fn dedupe_events(events: Vec<SuiEventEnvelope>) -> Vec<SuiEventEnvelope> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for event in events {
        let key = event_dedupe_key(&event);
        if seen.insert(key) {
            result.push(event);
        }
    }
    result
}

pub fn event_dedupe_key(event: &SuiEventEnvelope) -> String {
    if let Some((digest, seq)) = event_id_parts(event) {
        return format!("{digest}:{seq}");
    }
    format!("{}:{}", event.event_type, event.parsed_json)
}

fn event_id_parts(event: &SuiEventEnvelope) -> Option<(String, String)> {
    let id = event.id.as_ref()?;
    let digest = id
        .get("txDigest")
        .or_else(|| id.get("transactionDigest"))?
        .as_str()?
        .to_string();
    let seq = id
        .get("eventSeq")
        .or_else(|| id.get("sequenceNumber"))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| value.as_u64().map(|v| v.to_string()))
        })
        .unwrap_or_else(|| "-".to_string());
    Some((digest, seq))
}

fn event_from_graphql_node(node: Value) -> Result<SuiEventEnvelope> {
    let contents = node.get("contents").cloned().unwrap_or(Value::Null);
    let event_type = contents
        .pointer("/type/repr")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut parts = event_type.split("::");
    let fallback_package = parts.next().unwrap_or_default().to_string();
    let fallback_module = parts.next().unwrap_or_default().to_string();
    let module = node
        .get("transactionModule")
        .cloned()
        .unwrap_or(Value::Null);
    let transaction = node.get("transaction").cloned().unwrap_or(Value::Null);
    let digest = transaction
        .get("digest")
        .and_then(Value::as_str)
        .map(str::to_string);
    let sequence = node.get("sequenceNumber").cloned().unwrap_or(Value::Null);
    Ok(SuiEventEnvelope {
        id: digest.map(|digest| {
            json!({
                "txDigest": digest,
                "eventSeq": sequence.as_str().map(str::to_string).or_else(|| sequence.as_u64().map(|v| v.to_string())).unwrap_or_else(|| "0".to_string())
            })
        }),
        package_id: module
            .pointer("/package/address")
            .and_then(Value::as_str)
            .unwrap_or(&fallback_package)
            .to_string(),
        transaction_module: module
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(&fallback_module)
            .to_string(),
        sender: node
            .pointer("/sender/address")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        event_type,
        parsed_json: contents.get("json").cloned().unwrap_or(Value::Null),
        bcs: None,
        timestamp_ms: node
            .get("timestamp")
            .or_else(|| node.get("timestampMs"))
            .and_then(|value| value.as_str().map(str::to_string).or_else(|| value.as_u64().map(|v| v.to_string()))),
    })
}

fn filter_by_time(
    events: Vec<SuiEventEnvelope>,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
) -> Vec<SuiEventEnvelope> {
    events
        .into_iter()
        .filter(|event| {
            let Some(timestamp) = event
                .timestamp_ms
                .as_deref()
                .and_then(|value| value.parse::<u64>().ok())
            else {
                return start_time_ms.is_none() && end_time_ms.is_none();
            };
            start_time_ms.is_none_or(|start| timestamp >= start)
                && end_time_ms.is_none_or(|end| timestamp <= end)
        })
        .collect()
}

const EVENTS_QUERY: &str = r#"
query PaperProofEvents($filter: EventFilter, $first: Int, $last: Int, $after: String, $before: String) {
  events(filter: $filter, first: $first, last: $last, after: $after, before: $before) {
    pageInfo { hasNextPage hasPreviousPage startCursor endCursor }
    nodes {
      sender { address }
      transactionModule { name package { address } }
      contents { type { repr } json }
      transaction { digest }
      sequenceNumber
      timestamp
    }
  }
}
"#;
