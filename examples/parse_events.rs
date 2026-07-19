// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use paperproof_sdk_rs::{
    deployment::mainnet_deployment,
    events::{SuiEventEnvelope, parse_event},
    events_trust::validate_event_trust,
};
use serde_json::json;

fn main() {
    let deployment = mainnet_deployment();
    let event = SuiEventEnvelope {
        id: None,
        package_id: deployment.packages.publishing.clone(),
        transaction_module: "publishing".to_string(),
        sender: "0x1".to_string(),
        event_type: format!(
            "{}::publishing::ArtifactPublishedEvent",
            deployment.packages.publishing
        ),
        parsed_json: json!({ "root_id": deployment.objects.root }),
        bcs: None,
        timestamp_ms: None,
    };
    println!("{:?}", parse_event(&event).kind);
    println!("{:?}", validate_event_trust(&event, &deployment));
}
