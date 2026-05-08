// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use async_trait::async_trait;
use paperproof_sdk_rs::walrus::{WalrusBlob, sha256_hex};
use paperproof_sdk_rs::{
    ContentPublishOptions, PaperProofContentBackend, PaperProofContentService, PaperProofError,
    Result, WalrusExtendOptions, WalrusExtendResult, WalrusTransferOptions, WalrusTransferResult,
    WalrusWriteOptions,
};
use serde_json::{Value, json};

const TS_DIR: &str = "D:/Works/VscodeProject/PaperProofLabs/PaperProof-SDK-ts";
const HELPER: &str = "D:/Works/VscodeProject/PaperProofLabs/PaperProof-SDK-ts/examples/mainnet-walrus-addr4-helper.mjs";
const ADDR4: &str = "0x8fdd4a2185cc81bc0fef20e56cabe29803ea4afc63d20550ad88cbcafb85dbb6";

#[derive(Clone, Debug)]
struct MainnetWalrusBridge;

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

#[async_trait]
impl PaperProofContentBackend for MainnetWalrusBridge {
    async fn publish_content_backend(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<Value> {
        let text = String::from_utf8_lossy(&bytes).to_string();
        let out = run_helper(&[
            "--op",
            "write",
            "--epochs",
            &options.epochs.to_string(),
            "--deletable",
            if options.deletable.unwrap_or(!options.share) {
                "true"
            } else {
                "false"
            },
            "--content",
            &text,
        ])
        .map_err(|err| PaperProofError::network("mainnet-walrus-helper write", err.to_string()))?;
        Ok(json!({
            "blobObject": {
                "blobId": out["blobId"],
                "id": out["blobObjectId"],
                "owner": out.get("owner").cloned().unwrap_or(Value::Null),
            },
            "raw": out,
        }))
    }

    async fn read_content_backend(&self, blob_id: &str) -> Result<WalrusBlob> {
        let out = run_helper(&["--op", "read", "--blobId", blob_id]).map_err(|err| {
            PaperProofError::network("mainnet-walrus-helper read", err.to_string())
        })?;
        let text = out.get("text").and_then(Value::as_str).unwrap_or_default();
        let bytes = text.as_bytes().to_vec();
        Ok(WalrusBlob {
            blob_id: blob_id.to_string(),
            sha256_hex: sha256_hex(&bytes),
            bytes,
        })
    }

    async fn read_and_verify_content_backend(
        &self,
        blob_id: &str,
        expected_sha256_hex: &str,
    ) -> Result<WalrusBlob> {
        let blob = self.read_content_backend(blob_id).await?;
        if !blob.sha256_hex.eq_ignore_ascii_case(expected_sha256_hex) {
            return Err(paperproof_sdk_rs::PaperProofError::WalrusDigestMismatch {
                expected: expected_sha256_hex.to_string(),
                actual: blob.sha256_hex.clone(),
            });
        }
        Ok(blob)
    }

    async fn extend_content_backend(
        &self,
        blob_object_id: &str,
        options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        let raw = run_helper(&[
            "--op",
            "extend",
            "--blobObjectId",
            blob_object_id,
            "--epochs",
            &options.epochs.to_string(),
        ])
        .map_err(|err| PaperProofError::network("mainnet-walrus-helper extend", err.to_string()))?;
        Ok(WalrusExtendResult {
            blob_object_id: blob_object_id.to_string(),
            extended: true,
            shared: Some(false),
            raw,
        })
    }

    async fn transfer_owned_content_backend(
        &self,
        blob_object_id: &str,
        recipient: &str,
        _options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        let raw = run_helper(&[
            "--op",
            "transfer",
            "--blobObjectId",
            blob_object_id,
            "--recipient",
            recipient,
        ])
        .map_err(|err| {
            PaperProofError::network("mainnet-walrus-helper transfer", err.to_string())
        })?;
        Ok(WalrusTransferResult {
            blob_object_id: blob_object_id.to_string(),
            recipient: recipient.to_string(),
            transferred: true,
            raw,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("PAPERPROOF_RUN_MAINNET_CONTENT_SERVICE").as_deref() != Ok("1") {
        println!(
            "Set PAPERPROOF_RUN_MAINNET_CONTENT_SERVICE=1 to run the ADDR_4 mainnet ContentService example."
        );
        return Ok(());
    }

    let service = PaperProofContentService::new(MainnetWalrusBridge);
    let published = service
        .publish_content(
            b"PaperProof Rust ContentService mainnet".to_vec(),
            ContentPublishOptions {
                epochs: 1,
                content_type: Some("text/plain".to_string()),
                share: false,
                deletable: Some(true),
            },
        )
        .await?;
    let blob_object_id = published
        .blob_object_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing blob object id"))?;
    let read = service
        .read_content(
            blob_object_id_to_blob_id(&published),
            Some(published.content_hash.trim_start_matches("sha256:")),
        )
        .await?;
    let extend = service
        .extend_content(
            blob_object_id,
            WalrusExtendOptions {
                epochs: 1,
                signer_address: Some(ADDR4.to_string()),
                skip_shared_check: false,
            },
        )
        .await?;
    let transfer = service
        .transfer_owned_content(
            blob_object_id,
            ADDR4,
            WalrusTransferOptions {
                signer_address: Some(ADDR4.to_string()),
                skip_owner_check: false,
            },
        )
        .await?;

    let out = json!({
        "sdk": "rust",
        "address": ADDR4,
        "publish": {
            "blobId": published.blob_id,
            "blobObjectId": blob_object_id,
            "contentHash": published.content_hash,
            "writeRegisterTx": published.raw
                .pointer("/raw/steps")
                .and_then(Value::as_array)
                .and_then(|steps| steps.iter().find(|step| step.get("step").and_then(Value::as_str) == Some("registered")))
                .and_then(|step| step.get("txDigest"))
                .and_then(Value::as_str),
        },
        "read": {
            "verified": read.verified,
            "digest": read.digest,
            "byteLength": read.bytes.len(),
        },
        "extendDigest": digest(&extend.raw),
        "transferDigest": digest(&transfer.raw),
        "explorer": {
            "object": format!("https://suivision.xyz/object/{blob_object_id}"),
            "extendTx": extend.raw.pointer("/explorer/transaction"),
            "transferTx": transfer.raw.pointer("/explorer/transaction"),
        }
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn blob_object_id_to_blob_id(published: &paperproof_sdk_rs::ContentPublishResult) -> &str {
    &published.blob_id
}
