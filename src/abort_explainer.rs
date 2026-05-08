// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct MoveAbortInfo {
    pub package_id: Option<String>,
    pub module: Option<String>,
    pub code: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PaperProofErrorExplanation {
    pub matched: bool,
    pub title: String,
    pub detail: String,
    pub suggestion: Option<String>,
    pub move_abort: Option<MoveAbortInfo>,
    pub raw: String,
}

#[derive(Clone, Copy)]
struct AbortExplanation {
    module: &'static str,
    code: u64,
    title: &'static str,
    detail: &'static str,
    suggestion: &'static str,
}

const MODULE_HINTS: &[&str] = &[
    "comments",
    "governance_voting",
    "governance",
    "publishing",
    "validation",
    "artifact_types",
];

const ABORTS: &[AbortExplanation] = &[
    AbortExplanation {
        module: "comments",
        code: 2,
        title: "Comments tree is not open",
        detail: "The target comments tree is locked or archived.",
        suggestion: "Read the tree status and only add comments when it is open.",
    },
    AbortExplanation {
        module: "comments",
        code: 3,
        title: "Parent comment not found",
        detail: "The parent comment id does not exist in this tree.",
        suggestion: "Use root parent id 0 or read the comment node before replying.",
    },
    AbortExplanation {
        module: "comments",
        code: 4,
        title: "Empty on-chain comment",
        detail: "The on-chain comment content is empty.",
        suggestion: "Provide non-empty comment text.",
    },
    AbortExplanation {
        module: "comments",
        code: 5,
        title: "On-chain comment too large",
        detail: "The on-chain comment exceeds the tree limit.",
        suggestion: "Use a blob comment or shorten the content.",
    },
    AbortExplanation {
        module: "comments",
        code: 14,
        title: "Invalid comments fee or vault object",
        detail: "The GovernanceVault/FeeManager does not match the comments tree binding.",
        suggestion: "Use canonical deployment objects from MAINNET_DEPLOYMENT.",
    },
    AbortExplanation {
        module: "comments",
        code: 16,
        title: "Insufficient PPRF proof balance for like",
        detail: "The supplied PPRF coin has less than the required proof balance.",
        suggestion: "Use a PPRF coin with at least 1 PPRF.",
    },
    AbortExplanation {
        module: "comments",
        code: 17,
        title: "Already liked",
        detail: "The liker has already liked this artifact.",
        suggestion: "Call unlike before liking again, or treat the current state as success.",
    },
    AbortExplanation {
        module: "comments",
        code: 18,
        title: "Not liked yet",
        detail: "The liker has no existing like to remove.",
        suggestion: "Check has_liked before calling unlike.",
    },
    AbortExplanation {
        module: "comments",
        code: 20,
        title: "Parent comment is not active",
        detail: "Replies are not allowed under hidden or deleted comments.",
        suggestion: "Reply to an active comment or the root.",
    },
    AbortExplanation {
        module: "comments",
        code: 26,
        title: "Deleted comment is final",
        detail: "A deleted comment cannot be restored or changed.",
        suggestion: "Treat deleted as a terminal state.",
    },
    AbortExplanation {
        module: "governance",
        code: 3,
        title: "Not governance authority",
        detail: "The signer is not the configured governance authority.",
        suggestion: "Use governance voting flow or the current governance authority account.",
    },
    AbortExplanation {
        module: "governance",
        code: 6,
        title: "Invalid registry binding",
        detail: "One or more governance objects are from different registries.",
        suggestion: "Use canonical objects from the same PaperProof deployment.",
    },
    AbortExplanation {
        module: "governance",
        code: 10,
        title: "Fee payment required",
        detail: "The call requires a SUI fee payment coin.",
        suggestion: "Pass a payment coin or use a free fee level path if available.",
    },
    AbortExplanation {
        module: "governance",
        code: 11,
        title: "Insufficient fee payment",
        detail: "The supplied payment coin is below the required fee.",
        suggestion: "Select a larger SUI coin or split enough SUI before the call.",
    },
    AbortExplanation {
        module: "governance",
        code: 12,
        title: "Not upgrade authority",
        detail: "The signer is not the configured upgrade authority.",
        suggestion: "Use the upgrade authority account or migrate authority through governance.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 5,
        title: "Proposal not active",
        detail: "The proposal is not in active voting state.",
        suggestion: "Read proposal status before voting or finalizing.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 6,
        title: "Already voted",
        detail: "This voter has already voted on the proposal.",
        suggestion: "Claim locked tokens after finalization instead of voting again.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 9,
        title: "Proposal not passed",
        detail: "Only passed executable proposals can be executed.",
        suggestion: "Check proposal status and votes before execution.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 10,
        title: "Proposal is not executable",
        detail: "Signal proposals cannot execute protocol actions.",
        suggestion: "Use executable proposal type for protocol changes.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 11,
        title: "Proposal already executed",
        detail: "The proposal action has already been consumed.",
        suggestion: "Do not execute the same proposal twice.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 17,
        title: "Another active proposal exists",
        detail: "Only one active proposal is allowed at a time.",
        suggestion: "Finalize, execute, expire, or wait for the active proposal before creating another.",
    },
    AbortExplanation {
        module: "governance_voting",
        code: 20,
        title: "Voting power below minimum",
        detail: "The supplied PPRF coin is below minimum voting stake.",
        suggestion: "Use a PPRF coin with at least MIN_VOTE_STAKE.",
    },
    AbortExplanation {
        module: "publishing",
        code: 1,
        title: "Publishing is paused",
        detail: "The root paused flag blocks publish and add-version calls.",
        suggestion: "Wait until publishing is unpaused.",
    },
    AbortExplanation {
        module: "publishing",
        code: 6,
        title: "Artifact type disabled",
        detail: "The selected artifact type is disabled for publishing/versioning.",
        suggestion: "Choose an enabled type or wait for governance to enable it.",
    },
    AbortExplanation {
        module: "publishing",
        code: 8,
        title: "Invalid governance vault",
        detail: "The supplied GovernanceVault is not the canonical object bound to root.",
        suggestion: "Use MAINNET_DEPLOYMENT.objects.governance_vault.",
    },
    AbortExplanation {
        module: "publishing",
        code: 9,
        title: "Invalid fee manager",
        detail: "The supplied FeeManager is not the canonical object bound to root.",
        suggestion: "Use MAINNET_DEPLOYMENT.objects.fee_manager.",
    },
    AbortExplanation {
        module: "publishing",
        code: 21,
        title: "Not series owner",
        detail: "Only the series owner can perform this action.",
        suggestion: "Use the current owner account or transfer ownership first.",
    },
    AbortExplanation {
        module: "publishing",
        code: 25,
        title: "Too many versions",
        detail: "The series reached MAX_VERSIONS_PER_SERIES.",
        suggestion: "Start a new series or stop appending versions.",
    },
    AbortExplanation {
        module: "publishing",
        code: 29,
        title: "Duplicate metadata key",
        detail: "metadata_extensions contains duplicate keys.",
        suggestion: "Deduplicate keys before building the transaction.",
    },
];

pub fn parse_move_abort(input: impl AsRef<str>) -> Option<MoveAbortInfo> {
    let raw = input.as_ref();
    if raw.is_empty() {
        return None;
    }
    let package_id = raw
        .split_whitespace()
        .find_map(|part| {
            part.split("::")
                .next()
                .filter(|item| item.starts_with("0x"))
        })
        .map(trim_punctuation);
    let module = MODULE_HINTS
        .iter()
        .find(|hint| raw.contains(&format!("::{hint}::")) || raw.contains(**hint))
        .map(|hint| (*hint).to_string());
    let code = parse_code(raw);
    if package_id.is_none() && module.is_none() && code.is_none() {
        return None;
    }
    Some(MoveAbortInfo {
        package_id,
        module,
        code,
    })
}

pub fn explain_paperproof_error(input: impl AsRef<str>) -> PaperProofErrorExplanation {
    let raw = input.as_ref().to_string();
    let move_abort = parse_move_abort(&raw);
    if let Some(info) = &move_abort
        && let (Some(module), Some(code)) = (&info.module, info.code)
        && let Some(found) = ABORTS
            .iter()
            .find(|item| item.module == module && item.code == code)
    {
        return PaperProofErrorExplanation {
            matched: true,
            title: found.title.to_string(),
            detail: found.detail.to_string(),
            suggestion: Some(found.suggestion.to_string()),
            move_abort,
            raw,
        };
    }
    let lower = raw.to_ascii_lowercase();
    let (matched, title, detail, suggestion) = if lower.contains("insufficient")
        || lower.contains("not enough")
    {
        (
            true,
            "Insufficient coin balance",
            "The signer or selected payment/proof coin does not hold enough balance for this operation.",
            "Top up the required SUI/WAL/PPRF balance, choose a larger coin object, or use coin selection helpers before submitting.",
        )
    } else if lower.contains("gas") || lower.contains("no valid gas coins") {
        (
            true,
            "Gas or coin selection problem",
            "The transaction failed while selecting or charging gas/payment coins.",
            "Check SUI balance for gas, avoid using the same coin for multiple roles, and set a larger gas budget for complex flows.",
        )
    } else if lower.contains("typemismatch") || lower.contains("objecttypemismatch") {
        (
            true,
            "Transaction argument type mismatch",
            "A supplied object or coin has the wrong Move type for the target function.",
            "Check object IDs, PPRF proof coins, and canonical deployment objects.",
        )
    } else {
        (
            false,
            "Unrecognized PaperProof/Sui error",
            raw.as_str(),
            "Inspect the transaction digest and Sui explorer details.",
        )
    };
    PaperProofErrorExplanation {
        matched,
        title: title.to_string(),
        detail: detail.to_string(),
        suggestion: Some(suggestion.to_string()),
        move_abort,
        raw,
    }
}

pub fn format_paperproof_error_explanation(explanation: &PaperProofErrorExplanation) -> String {
    let abort = explanation
        .move_abort
        .as_ref()
        .map(|abort| {
            format!(
                " module={} code={}",
                abort.module.as_deref().unwrap_or("unknown"),
                abort
                    .code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )
        })
        .unwrap_or_default();
    format!(
        "{}.{} {} Suggestion: {}",
        explanation.title,
        abort,
        explanation.detail,
        explanation.suggestion.as_deref().unwrap_or("")
    )
}

fn trim_punctuation(value: &str) -> String {
    value
        .trim_matches(|ch: char| !ch.is_ascii_hexdigit() && ch != 'x')
        .to_string()
}

fn parse_code(raw: &str) -> Option<u64> {
    let lower = raw.to_ascii_lowercase();
    if let Some(index) = lower.rfind("code") {
        let tail = &raw[index + "code".len()..];
        if let Some(number) = first_number(tail) {
            return Some(number);
        }
    }
    all_numbers(raw).into_iter().last()
}

fn all_numbers(value: &str) -> Vec<u64> {
    value
        .split(|ch: char| !ch.is_ascii_hexdigit() && ch != 'x')
        .filter_map(parse_number_token)
        .collect()
}

fn parse_number_token(token: &str) -> Option<u64> {
    if token.is_empty() {
        return None;
    }
    if let Some(hex) = token.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok();
    }
    if token.chars().all(|ch| ch.is_ascii_digit()) {
        return token.parse().ok();
    }
    None
}

fn first_number(value: &str) -> Option<u64> {
    for token in value.split(|ch: char| !ch.is_ascii_hexdigit() && ch != 'x') {
        if let Some(number) = parse_number_token(token) {
            return Some(number);
        }
    }
    None
}
