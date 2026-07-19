// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde_json::{Value, json};

use crate::{
    deployment::Deployment,
    error::Result,
    events::{SuiEventEnvelope, event_struct_name},
    events_trust::{
        EventIssueSeverity, EventTrustLevel, EventVerificationIssue, EventVerificationReport,
        EventVerificationStatus, TrustedSuiEventEnvelope, attach_event_verification,
        verification_report_from_canonical_check,
    },
    read::PaperProofReadClient,
    walrus::{PaperProofContentBackend, sha256_hex},
    walrus::{
        WalrusBlob, WalrusExtendOptions, WalrusExtendResult, WalrusTransferOptions,
        WalrusTransferResult, WalrusWriteOptions,
    },
};

#[derive(Clone, Debug, Default)]
pub struct VerifyEventOptions {
    pub trust: EventTrustLevel,
    pub verify_walrus: bool,
    pub provider: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PaperProofEventVerifier<B = ()> {
    pub read: PaperProofReadClient,
    pub deployment: Deployment,
    pub walrus: Option<B>,
    pub verify_walrus_default: bool,
}

impl PaperProofEventVerifier<()> {
    pub fn new(read: PaperProofReadClient) -> Self {
        let deployment = read.deployment.clone();
        Self {
            read,
            deployment,
            walrus: None,
            verify_walrus_default: false,
        }
    }
}

impl<B> PaperProofEventVerifier<B> {
    pub fn with_walrus(read: PaperProofReadClient, walrus: B) -> Self {
        let deployment = read.deployment.clone();
        Self {
            read,
            deployment,
            walrus: Some(walrus),
            verify_walrus_default: false,
        }
    }

    pub fn verify_walrus_by_default(mut self, enabled: bool) -> Self {
        self.verify_walrus_default = enabled;
        self
    }
}

impl<B> PaperProofEventVerifier<B>
where
    B: PaperProofContentBackend,
{
    pub async fn verify_event(
        &self,
        event: &SuiEventEnvelope,
        options: VerifyEventOptions,
    ) -> Result<EventVerificationReport> {
        let trust = options.trust;
        let mut report =
            verification_report_from_canonical_check(event, &self.deployment, trust.clone());
        report.provider = options.provider;
        if trust != EventTrustLevel::Verified || !report.trusted {
            return Ok(report);
        }

        let mut issues = report.issues.clone();
        let mut bindings = serde_json::Map::new();
        let fields = &event.parsed_json;
        match event_struct_name(&event.event_type) {
            "ArtifactPublishedEvent" => {
                self.verify_published(fields, &mut issues, &mut bindings, options.verify_walrus)
                    .await;
            }
            "ArtifactVersionAddedEvent" => {
                self.verify_version_added(
                    fields,
                    &mut issues,
                    &mut bindings,
                    options.verify_walrus,
                )
                .await;
            }
            "PreprintCodeReservedEvent" => {
                bindings.insert(
                    "preprint_reservation".to_string(),
                    json!({
                        "reservation_id": fields.get("reservation_id").cloned(),
                        "series_id": fields.get("series_id").cloned(),
                        "artifact_code": fields.get("artifact_code").cloned(),
                    }),
                );
            }
            "CommentAddedEvent" => {
                self.verify_comment(fields, &mut issues, &mut bindings)
                    .await;
            }
            "PaperLikedEvent" | "PaperUnlikedEvent" => {
                self.verify_like(fields, &mut issues, &mut bindings).await;
            }
            "ProposalCreatedEvent"
            | "VoteCastEvent"
            | "ProposalFinalizedEvent"
            | "ProposalExecutedEvent"
            | "ProposalExpiredEvent"
            | "VoteClaimedEvent" => {
                self.verify_governance(fields, &mut issues, &mut bindings)
                    .await;
            }
            _ => {
                bindings.insert(
                    "note".to_string(),
                    json!("No deeper binding rule is registered for this event type; canonical checks were applied."),
                );
                issues.push(EventVerificationIssue {
                    code: "VERIFIED_RULE_NOT_REGISTERED".to_string(),
                    severity: EventIssueSeverity::Error,
                    reason: format!(
                        "Verified PaperProof checks are not implemented for {}. Treat this event as canonical, not fully verified.",
                        event_struct_name(&event.event_type)
                    ),
                    details: Some(json!({
                        "event_type": event.event_type,
                        "struct_name": event_struct_name(&event.event_type)
                    })),
                });
            }
        }

        let has_errors = issues
            .iter()
            .any(|issue| issue.severity == EventIssueSeverity::Error);
        report.status = if has_errors {
            EventVerificationStatus::Incomplete
        } else {
            EventVerificationStatus::Verified
        };
        report.trusted = !has_errors;
        report.canonical = true;
        report.verified = !has_errors;
        report.issues = issues;
        report.bindings = bindings;
        Ok(report)
    }

    pub async fn verify_events(
        &self,
        events: &[SuiEventEnvelope],
        options: VerifyEventOptions,
    ) -> Result<Vec<TrustedSuiEventEnvelope>> {
        let mut out = Vec::with_capacity(events.len());
        for event in events {
            out.push(attach_event_verification(
                self.verify_event(event, options.clone()).await?,
            ));
        }
        Ok(out)
    }

    async fn verify_published(
        &self,
        fields: &Value,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
        verify_walrus: bool,
    ) {
        let Some(series_id) = string_field(fields, "series_id") else {
            return;
        };
        let Some(version_id) = string_field(fields, "version_id") else {
            return;
        };
        let Some(tree_id) = string_field(fields, "comments_tree_id") else {
            return;
        };
        let Some(likes_book_id) = string_field(fields, "likes_book_id") else {
            return;
        };
        let series = match self.read.get_series_view(&series_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "SERIES_READ_FAILED", error.to_string());
                return;
            }
        };
        let version = match self.read.get_version_view(&version_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "VERSION_READ_FAILED", error.to_string());
                return;
            }
        };
        let tree = match self.read.get_comments_tree_view(&tree_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "TREE_READ_FAILED", error.to_string());
                return;
            }
        };
        let likes = match self.read.get_likes_book_view(&likes_book_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "LIKES_READ_FAILED", error.to_string());
                return;
            }
        };
        bindings.insert("series".to_string(), json!(series));
        bindings.insert("version".to_string(), json!(version));
        bindings.insert("comments_tree".to_string(), json!(tree));
        bindings.insert("likes_book".to_string(), json!(likes));
        if !series
            .version_ids
            .iter()
            .any(|candidate| same_id(candidate, &version_id))
        {
            issues.push(EventVerificationIssue {
                code: "SERIES_VERSION_LIST_MISSING".to_string(),
                severity: EventIssueSeverity::Error,
                reason: "series.version_ids does not contain event version_id".to_string(),
                details: None,
            });
        }
        expect_same(
            issues,
            "SERIES_TREE_MISMATCH",
            series.comments_tree_id.as_deref(),
            Some(&tree_id),
            "series.comments_tree_id does not match event comments_tree_id",
        );
        expect_same(
            issues,
            "SERIES_LIKES_MISMATCH",
            series.likes_book_id.as_deref(),
            Some(&likes_book_id),
            "series.likes_book_id does not match event likes_book_id",
        );
        expect_same(
            issues,
            "VERSION_SERIES_MISMATCH",
            version.series_id.as_deref(),
            Some(&series_id),
            "version.series_id does not match event series_id",
        );
        expect_same(
            issues,
            "TREE_SERIES_MISMATCH",
            tree.target_series_id.as_deref(),
            Some(&series_id),
            "comments tree is not bound to event series_id",
        );
        expect_same(
            issues,
            "TREE_LIKES_MISMATCH",
            tree.likes_book_id.as_deref(),
            Some(&likes_book_id),
            "comments tree likes_book_id does not match event likes_book_id",
        );
        expect_same(
            issues,
            "LIKES_SERIES_MISMATCH",
            likes.target_series_id.as_deref(),
            Some(&series_id),
            "likes book target_series_id does not match event series_id",
        );
        expect_same(
            issues,
            "LIKES_TREE_MISMATCH",
            likes.comments_tree_id.as_deref(),
            Some(&tree_id),
            "likes book comments_tree_id does not match event tree_id",
        );
        if verify_walrus || self.verify_walrus_default {
            self.verify_walrus_reference(
                &version.raw_fields,
                version.content_hash.as_deref(),
                issues,
                bindings,
            )
            .await;
        }
    }

    async fn verify_version_added(
        &self,
        fields: &Value,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
        verify_walrus: bool,
    ) {
        let Some(series_id) = string_field(fields, "series_id") else {
            return;
        };
        let Some(version_id) =
            string_field(fields, "version_id").or_else(|| string_field(fields, "new_version_id"))
        else {
            return;
        };
        let series = match self.read.get_series_view(&series_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "SERIES_READ_FAILED", error.to_string());
                return;
            }
        };
        let version = match self.read.get_version_view(&version_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "VERSION_READ_FAILED", error.to_string());
                return;
            }
        };
        bindings.insert("series".to_string(), json!(series));
        bindings.insert("version".to_string(), json!(version));
        expect_same(
            issues,
            "VERSION_SERIES_MISMATCH",
            version.series_id.as_deref(),
            Some(&series_id),
            "version.series_id does not match event series_id",
        );
        if !series.version_ids.iter().any(|id| same_id(id, &version_id)) {
            push_issue(
                issues,
                "SERIES_VERSION_LIST_MISSING",
                "series.version_ids does not contain event version_id",
                None,
            );
        }
        if verify_walrus || self.verify_walrus_default {
            self.verify_walrus_reference(
                &version.raw_fields,
                version.content_hash.as_deref(),
                issues,
                bindings,
            )
            .await;
        }
    }

    async fn verify_comment(
        &self,
        fields: &Value,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
    ) {
        let Some(tree_id) = string_field(fields, "tree_id") else {
            return;
        };
        let Some(comment_id) = u64_field(fields, "comment_id") else {
            return;
        };
        match self.read.get_comments_tree_view(&tree_id).await {
            Ok(tree) => {
                bindings.insert("comments_tree".to_string(), json!(tree));
            }
            Err(error) => {
                push_read_failed(issues, "TREE_READ_FAILED", error.to_string());
                return;
            }
        }
        match self.read.get_comment_node_view(&tree_id, comment_id).await {
            Ok(Some(node)) => {
                if node.comment_id != Some(comment_id) {
                    push_issue(
                        issues,
                        "COMMENT_ID_MISMATCH",
                        "comment node id does not match event comment_id",
                        None,
                    );
                }
                if let Some(parent_id) = u64_field(fields, "parent_comment_id")
                    && node.parent_comment_id.is_some()
                    && node.parent_comment_id != Some(parent_id)
                {
                    push_issue(
                        issues,
                        "COMMENT_PARENT_MISMATCH",
                        "comment node parent does not match event parent_comment_id",
                        None,
                    );
                }
                bindings.insert("comment_node".to_string(), json!(node));
            }
            Ok(None) => push_issue(
                issues,
                "COMMENT_NODE_NOT_FOUND",
                "comment event points to a missing comment node",
                None,
            ),
            Err(error) => push_read_failed(issues, "COMMENT_NODE_READ_FAILED", error.to_string()),
        }
    }

    async fn verify_like(
        &self,
        fields: &Value,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
    ) {
        let Some(likes_book_id) = string_field(fields, "likes_book_id") else {
            return;
        };
        let Some(series_id) = string_field(fields, "target_series_id") else {
            return;
        };
        let likes = match self.read.get_likes_book_view(&likes_book_id).await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "LIKES_READ_FAILED", error.to_string());
                return;
            }
        };
        bindings.insert("likes_book".to_string(), json!(likes));
        expect_same(
            issues,
            "LIKES_SERIES_MISMATCH",
            likes.target_series_id.as_deref(),
            Some(&series_id),
            "likes book target_series_id does not match event target_series_id",
        );
        if let Some(tree_id) = string_field(fields, "tree_id") {
            expect_same(
                issues,
                "LIKES_TREE_MISMATCH",
                likes.comments_tree_id.as_deref(),
                Some(&tree_id),
                "likes book comments_tree_id does not match event tree_id",
            );
        }
    }

    async fn verify_governance(
        &self,
        fields: &Value,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
    ) {
        let config = match self.read.get_governance_config_view().await {
            Ok(value) => value,
            Err(error) => {
                push_read_failed(issues, "GOVERNANCE_CONFIG_READ_FAILED", error.to_string());
                return;
            }
        };
        expect_same(
            issues,
            "GOVERNANCE_CONFIG_REGISTRY_MISMATCH",
            config.registry_id.as_deref(),
            Some(&self.deployment.objects.root),
            "governance config is not bound to deployment root",
        );
        bindings.insert("governance_config".to_string(), json!(config));
        let proposal_object_id = match string_field(fields, "proposal_object_id") {
            Some(id) => Some(id),
            None => match u64_field(fields, "proposal_id") {
                Some(id) => match self.read.get_proposal_object_id(id).await {
                    Ok(id) => Some(id),
                    Err(error) => {
                        push_read_failed(issues, "PROPOSAL_ID_LOOKUP_FAILED", error.to_string());
                        None
                    }
                },
                None => None,
            },
        };
        if let Some(proposal_object_id) = proposal_object_id {
            match self.read.get_proposal_view(&proposal_object_id).await {
                Ok(proposal) => {
                    expect_same(
                        issues,
                        "PROPOSAL_REGISTRY_MISMATCH",
                        proposal.registry_id.as_deref(),
                        Some(&self.deployment.objects.root),
                        "proposal object is not bound to deployment root",
                    );
                    if let Some(event_proposal_id) = u64_field(fields, "proposal_id")
                        && proposal.proposal_id.is_some()
                        && proposal.proposal_id != Some(event_proposal_id)
                    {
                        push_issue(
                            issues,
                            "PROPOSAL_ID_MISMATCH",
                            "proposal object proposal_id does not match event proposal_id",
                            None,
                        );
                    }
                    bindings.insert("proposal".to_string(), json!(proposal));
                }
                Err(error) => push_read_failed(issues, "PROPOSAL_READ_FAILED", error.to_string()),
            }
        }
    }

    async fn verify_walrus_reference(
        &self,
        raw_fields: &Value,
        content_hash: Option<&str>,
        issues: &mut Vec<EventVerificationIssue>,
        bindings: &mut serde_json::Map<String, Value>,
    ) {
        let Some(walrus) = &self.walrus else {
            push_issue(
                issues,
                "WALRUS_PROVIDER_MISSING",
                "Walrus verification was requested but no Walrus backend was configured",
                None,
            );
            return;
        };
        let Some(blob_id) = raw_fields
            .pointer("/header/fields/walrus_blob_id")
            .or_else(|| raw_fields.pointer("/header/walrus_blob_id"))
            .and_then(Value::as_str)
        else {
            push_issue(
                issues,
                "WALRUS_REFERENCE_MISSING",
                "version metadata does not contain walrus_blob_id",
                None,
            );
            return;
        };
        match walrus.read_content_backend(blob_id).await {
            Ok(blob) => {
                let expected = content_hash.map(normalize_content_hash);
                let ok = expected
                    .as_deref()
                    .is_none_or(|expected| expected.eq_ignore_ascii_case(&blob.sha256_hex));
                bindings.insert(
                    "walrus".to_string(),
                    json!({
                        "blob_id": blob.blob_id,
                        "byte_length": blob.bytes.len(),
                        "sha256": sha256_hex(&blob.bytes),
                        "expected_hash": content_hash,
                        "ok": ok
                    }),
                );
                if !ok {
                    push_issue(
                        issues,
                        "WALRUS_CONTENT_MISMATCH",
                        "Walrus blob did not match expected content hash",
                        None,
                    );
                }
            }
            Err(error) => push_read_failed(issues, "WALRUS_READ_FAILED", error.to_string()),
        }
    }
}

#[async_trait::async_trait]
impl PaperProofContentBackend for () {
    async fn publish_content_backend(
        &self,
        _bytes: Vec<u8>,
        _options: WalrusWriteOptions,
    ) -> Result<Value> {
        Err(crate::error::PaperProofError::invalid_input(
            "walrus",
            "no Walrus backend is configured",
        ))
    }

    async fn read_content_backend(&self, _blob_id: &str) -> Result<WalrusBlob> {
        Err(crate::error::PaperProofError::invalid_input(
            "walrus",
            "no Walrus backend is configured",
        ))
    }

    async fn read_and_verify_content_backend(
        &self,
        _blob_id: &str,
        _expected_sha256_hex: &str,
    ) -> Result<WalrusBlob> {
        Err(crate::error::PaperProofError::invalid_input(
            "walrus",
            "no Walrus backend is configured",
        ))
    }

    async fn extend_content_backend(
        &self,
        _blob_object_id: &str,
        _options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        Err(crate::error::PaperProofError::invalid_input(
            "walrus",
            "no Walrus backend is configured",
        ))
    }

    async fn transfer_owned_content_backend(
        &self,
        _blob_object_id: &str,
        _recipient: &str,
        _options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        Err(crate::error::PaperProofError::invalid_input(
            "walrus",
            "no Walrus backend is configured",
        ))
    }
}

fn push_read_failed(issues: &mut Vec<EventVerificationIssue>, code: &str, reason: String) {
    push_issue(issues, code, reason, None);
}

fn push_issue(
    issues: &mut Vec<EventVerificationIssue>,
    code: impl Into<String>,
    reason: impl Into<String>,
    details: Option<Value>,
) {
    issues.push(EventVerificationIssue {
        code: code.into(),
        reason: reason.into(),
        severity: EventIssueSeverity::Error,
        details,
    });
}

fn expect_same(
    issues: &mut Vec<EventVerificationIssue>,
    code: &str,
    actual: Option<&str>,
    expected: Option<&str>,
    reason: &str,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if same_id(actual, expected) => {}
        (Some(actual), Some(expected)) => push_issue(
            issues,
            code,
            reason,
            Some(json!({ "actual": actual, "expected": expected })),
        ),
        _ => push_issue(
            issues,
            format!("{code}_INCOMPLETE"),
            format!("{reason}; one side was missing"),
            Some(json!({ "actual": actual, "expected": expected })),
        ),
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field)?.as_str().map(ToString::to_string)
}

fn u64_field(value: &Value, field: &str) -> Option<u64> {
    let value = value.get(field)?;
    value.as_u64().or_else(|| value.as_str()?.parse().ok())
}

fn same_id(left: &str, right: &str) -> bool {
    normalize_id(left) == normalize_id(right)
}

fn normalize_id(value: &str) -> String {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", raw.trim_start_matches('0').to_ascii_lowercase())
}

fn normalize_content_hash(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .strip_prefix("sha256:")
        .unwrap_or(value.trim())
        .to_string()
}
