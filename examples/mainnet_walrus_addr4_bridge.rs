// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use paperproof_sdk_rs::walrus::{
    WalrusExtendOptions, WalrusTransferOptions, assert_transferable_owned_blob, owner_address,
};
use serde_json::{Value, json};

const TS_DIR: &str = "D:/Works/VscodeProject/PaperProofLabs/PaperProof-SDK-ts";
const HELPER: &str = "D:/Works/VscodeProject/PaperProofLabs/PaperProof-SDK-ts/examples/mainnet-walrus-addr4-helper.mjs";
const ADDR4: &str = "0x8fdd4a2185cc81bc0fef20e56cabe29803ea4afc63d20550ad88cbcafb85dbb6";

fn run_helper(args: &[&str]) -> anyhow::Result<Value> {
    let output = Command::new("node")
        .arg(HELPER)
        .args(args)
        .current_dir(TS_DIR)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "helper failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn digest(value: &Value) -> Option<&str> {
    value.get("digest").and_then(Value::as_str)
}

fn main() -> anyhow::Result<()> {
    if std::env::var("PAPERPROOF_RUN_MAINNET_WALRUS_BRIDGE").as_deref() != Ok("1") {
        println!(
            "Set PAPERPROOF_RUN_MAINNET_WALRUS_BRIDGE=1 to run the ADDR_4 mainnet Walrus bridge example."
        );
        return Ok(());
    }

    let content = format!(
        "PaperProof Rust SDK Walrus mainnet test via SDK interface {:?}",
        std::time::SystemTime::now()
    );
    let write = run_helper(&[
        "--op",
        "write",
        "--epochs",
        "1",
        "--deletable",
        "true",
        "--content",
        &content,
    ])?;
    let blob_object_id = write
        .get("blobObjectId")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing blobObjectId"))?;
    let object_info = run_helper(&["--op", "get-object", "--blobObjectId", blob_object_id])?;
    let normalized_info = json!({
        "id": blob_object_id,
        "owner": object_info.get("owner").cloned().unwrap_or(Value::Null),
        "shared": false,
    });
    assert_transferable_owned_blob(blob_object_id, Some(ADDR4), &normalized_info)?;
    let owner = owner_address(&normalized_info).map(str::to_string);

    let extend_options = WalrusExtendOptions {
        epochs: 1,
        signer_address: Some(ADDR4.to_string()),
        skip_shared_check: false,
    };
    let _ = extend_options;
    let extend = run_helper(&[
        "--op",
        "extend",
        "--blobObjectId",
        blob_object_id,
        "--epochs",
        "1",
    ])?;

    let transfer_options = WalrusTransferOptions {
        signer_address: Some(ADDR4.to_string()),
        skip_owner_check: false,
    };
    let _ = transfer_options;
    assert_transferable_owned_blob(blob_object_id, Some(ADDR4), &normalized_info)?;
    let transfer = run_helper(&[
        "--op",
        "transfer",
        "--blobObjectId",
        blob_object_id,
        "--recipient",
        ADDR4,
    ])?;

    let out = json!({
        "sdk": "rust",
        "address": ADDR4,
        "blobId": write.get("blobId"),
        "blobObjectId": blob_object_id,
        "owner": owner,
        "writeRegisterTx": write
            .get("steps")
            .and_then(Value::as_array)
            .and_then(|steps| steps.iter().find(|step| step.get("step").and_then(Value::as_str) == Some("registered")))
            .and_then(|step| step.get("txDigest"))
            .and_then(Value::as_str),
        "writeObject": format!("https://suivision.xyz/object/{blob_object_id}"),
        "extendDigest": digest(&extend),
        "extendTx": extend.pointer("/explorer/transaction"),
        "transferDigest": digest(&transfer),
        "transferTx": transfer.pointer("/explorer/transaction"),
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
