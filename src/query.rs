// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    client::JsonRpcClient,
    deployment::Deployment,
    error::{PaperProofError, Result},
    events::{SuiEventEnvelope, extract_events_by_struct},
    events_trust::validate_event_trust,
    read::PaperProofReadClient,
    types::{ArtifactSeriesView, ArtifactVersionView, CommentsTreeView, LikesBookView},
    validation::{validate_address, validate_object_id},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PaginationInput {
    pub cursor: Option<Value>,
    pub limit: Option<u64>,
    pub descending_order: Option<bool>,
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
}

impl PaperProofQueryClient {
    pub fn new(rpc: JsonRpcClient, deployment: Deployment) -> Self {
        Self {
            read: PaperProofReadClient::new(rpc, deployment.clone()),
            deployment,
        }
    }

    pub fn mainnet() -> Self {
        let deployment = crate::deployment::mainnet_deployment();
        Self::new(JsonRpcClient::new(deployment.rpc_url.clone()), deployment)
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
        let filter = build_event_filter(&input)?;
        let descending = input.pagination.descending_order.unwrap_or(false);
        let raw = self
            .read
            .rpc
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
        let filtered = filter_by_time(data, input.start_time_ms, input.end_time_ms);
        Ok(EventPage {
            data: filtered,
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

    pub async fn query_canonical_events(
        &self,
        input: EventQueryInput,
    ) -> Result<EventPage<SuiEventEnvelope>> {
        let mut page = self.query_events(input).await?;
        page.data
            .retain(|event| validate_event_trust(event, &self.deployment).trusted);
        Ok(page)
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
