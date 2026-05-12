// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::{PaperProofError, Result};

#[derive(Clone, Debug)]
pub struct WalrusClient {
    aggregator_url: String,
    publisher_url: Option<String>,
    http: reqwest::Client,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalrusBlob {
    pub blob_id: String,
    pub bytes: Vec<u8>,
    pub sha256_hex: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalrusWriteOptions {
    pub epochs: u64,
    pub share: bool,
    pub deletable: Option<bool>,
}

impl Default for WalrusWriteOptions {
    fn default() -> Self {
        Self {
            epochs: 5,
            share: false,
            deletable: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WalrusExtendOptions {
    pub epochs: u64,
    pub signer_address: Option<String>,
    pub skip_shared_check: bool,
}

impl Default for WalrusExtendOptions {
    fn default() -> Self {
        Self {
            epochs: 5,
            signer_address: None,
            skip_shared_check: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WalrusExtendResult {
    pub blob_object_id: String,
    pub extended: bool,
    pub shared: Option<bool>,
    pub raw: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct WalrusTransferOptions {
    pub signer_address: Option<String>,
    pub skip_owner_check: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WalrusTransferResult {
    pub blob_object_id: String,
    pub recipient: String,
    pub transferred: bool,
    pub raw: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentPublishOptions {
    pub epochs: u64,
    pub content_type: Option<String>,
    pub share: bool,
    pub deletable: Option<bool>,
}

impl Default for ContentPublishOptions {
    fn default() -> Self {
        Self {
            epochs: 5,
            content_type: None,
            share: false,
            deletable: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentPublishResult {
    pub blob_id: String,
    pub blob_object_id: Option<String>,
    pub content_hash: String,
    pub content_type: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentReadResult {
    pub blob_id: String,
    pub bytes: Vec<u8>,
    pub digest: String,
    pub verified: bool,
}

#[derive(Clone, Debug)]
pub struct PaperProofContentService<B = WalrusClient> {
    pub backend: B,
}

#[derive(Clone, Debug)]
pub struct WalrusCliClient {
    pub cli_path: String,
    pub aggregator_url: String,
    pub extra_store_args: Vec<String>,
}

impl Default for WalrusCliClient {
    fn default() -> Self {
        Self::new("walrus")
    }
}

impl WalrusCliClient {
    pub fn new(cli_path: impl Into<String>) -> Self {
        Self {
            cli_path: cli_path.into(),
            aggregator_url: "https://aggregator.walrus-mainnet.walrus.space".to_string(),
            extra_store_args: Vec::new(),
        }
    }

    pub fn with_aggregator_url(mut self, aggregator_url: impl Into<String>) -> Self {
        self.aggregator_url = aggregator_url.into();
        self
    }

    pub fn with_extra_store_args(
        mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.extra_store_args = args.into_iter().map(Into::into).collect();
        self
    }
}

#[async_trait]
pub trait PaperProofContentBackend: Send + Sync {
    async fn publish_content_backend(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value>;

    async fn read_content_backend(&self, blob_id: &str) -> Result<WalrusBlob>;

    async fn read_and_verify_content_backend(
        &self,
        blob_id: &str,
        expected_sha256_hex: &str,
    ) -> Result<WalrusBlob>;

    async fn extend_content_backend(
        &self,
        blob_object_id: &str,
        options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult>;

    async fn transfer_owned_content_backend(
        &self,
        blob_object_id: &str,
        recipient: &str,
        options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult>;
}

impl WalrusClient {
    pub fn new(aggregator_url: impl Into<String>, publisher_url: Option<String>) -> Self {
        Self {
            aggregator_url: aggregator_url.into().trim_end_matches('/').to_string(),
            publisher_url,
            http: reqwest::Client::new(),
        }
    }

    pub async fn read_blob(&self, blob_id: &str) -> Result<WalrusBlob> {
        crate::validation::validate_required_text("blob_id", blob_id, 128)?;
        let url = format!("{}/v1/blobs/{}", self.aggregator_url, blob_id);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?;
        if !response.status().is_success() {
            return Err(PaperProofError::network(
                &url,
                format!("HTTP {}", response.status()),
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?
            .to_vec();
        Ok(WalrusBlob {
            blob_id: blob_id.to_string(),
            sha256_hex: sha256_hex(&bytes),
            bytes,
        })
    }

    pub async fn read_and_verify_sha256(
        &self,
        blob_id: &str,
        expected_sha256_hex: &str,
    ) -> Result<WalrusBlob> {
        let blob = self.read_blob(blob_id).await?;
        if !blob.sha256_hex.eq_ignore_ascii_case(expected_sha256_hex) {
            return Err(PaperProofError::WalrusDigestMismatch {
                expected: expected_sha256_hex.to_string(),
                actual: blob.sha256_hex.clone(),
            });
        }
        Ok(blob)
    }

    pub async fn write_blob(&self, bytes: Vec<u8>) -> Result<serde_json::Value> {
        self.write_blob_with_options(bytes, WalrusWriteOptions::default())
            .await
    }

    pub async fn write_blob_with_options(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value> {
        let Some(publisher_url) = &self.publisher_url else {
            return Err(PaperProofError::invalid_input(
                "publisher_url",
                "publisher URL is required to write Walrus content",
            ));
        };
        let deletable = options.deletable.unwrap_or(!options.share);
        let url = format!(
            "{}/v1/blobs?epochs={}&deletable={}",
            publisher_url.trim_end_matches('/'),
            options.epochs,
            deletable
        );
        let response = self
            .http
            .put(&url)
            .body(bytes)
            .send()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?;
        if !response.status().is_success() {
            return Err(PaperProofError::network(
                &url,
                format!("HTTP {}", response.status()),
            ));
        }
        response.json().await.map_err(Into::into)
    }

    pub async fn get_blob_object(&self, blob_object_id: &str) -> Result<Option<serde_json::Value>> {
        crate::validation::validate_object_id(blob_object_id)?;
        let url = format!("{}/v1/blob-objects/{}", self.aggregator_url, blob_object_id);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(PaperProofError::network(
                &url,
                format!("HTTP {}", response.status()),
            ));
        }
        response.json().await.map(Some).map_err(Into::into)
    }

    pub async fn extend_blob(
        &self,
        blob_object_id: &str,
        options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        crate::validation::validate_object_id(blob_object_id)?;
        let mut shared = None;
        if !options.skip_shared_check {
            let Some(info) = self.get_blob_object(blob_object_id).await? else {
                return Err(PaperProofError::invalid_input(
                    "blob_object_id",
                    format!("Walrus blob object {blob_object_id} was not found"),
                ));
            };
            shared = Some(is_shared_blob_object(&info));
            if let (Some(signer), Some(owner)) =
                (options.signer_address.as_deref(), owner_address(&info))
                && !same_address(signer, owner)
                && shared != Some(true)
            {
                return Err(PaperProofError::invalid_input(
                    "blob_object_id",
                    format!(
                        "signer {signer} is not the owner of {blob_object_id}, and the blob object is not shared"
                    ),
                ));
            }
        }
        let Some(publisher_url) = &self.publisher_url else {
            return Err(PaperProofError::invalid_input(
                "publisher_url",
                "publisher URL is required to extend Walrus content",
            ));
        };
        let url = format!(
            "{}/v1/blob-objects/{}/extend?epochs={}",
            publisher_url.trim_end_matches('/'),
            blob_object_id,
            options.epochs
        );
        let response = self
            .http
            .post(&url)
            .send()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?;
        if !response.status().is_success() {
            return Err(PaperProofError::network(
                &url,
                format!("HTTP {}", response.status()),
            ));
        }
        let raw = response.json().await.map_err(PaperProofError::from)?;
        Ok(WalrusExtendResult {
            blob_object_id: blob_object_id.to_string(),
            extended: true,
            shared,
            raw,
        })
    }

    pub async fn transfer_blob(
        &self,
        blob_object_id: &str,
        recipient: &str,
        options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        crate::validation::validate_object_id(blob_object_id)?;
        crate::validation::validate_address(recipient)?;
        if !options.skip_owner_check {
            let Some(info) = self.get_blob_object(blob_object_id).await? else {
                return Err(PaperProofError::invalid_input(
                    "blob_object_id",
                    format!("Walrus blob object {blob_object_id} was not found"),
                ));
            };
            assert_transferable_owned_blob(
                blob_object_id,
                options.signer_address.as_deref(),
                &info,
            )?;
        }
        let Some(publisher_url) = &self.publisher_url else {
            return Err(PaperProofError::invalid_input(
                "publisher_url",
                "publisher URL is required to transfer Walrus content",
            ));
        };
        let url = format!(
            "{}/v1/blob-objects/{}/transfer",
            publisher_url.trim_end_matches('/'),
            blob_object_id
        );
        let response = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "recipient": recipient }))
            .send()
            .await
            .map_err(|err| PaperProofError::network(&url, err.to_string()))?;
        if !response.status().is_success() {
            return Err(PaperProofError::network(
                &url,
                format!("HTTP {}", response.status()),
            ));
        }
        let raw = response.json().await.map_err(PaperProofError::from)?;
        Ok(WalrusTransferResult {
            blob_object_id: blob_object_id.to_string(),
            recipient: recipient.to_string(),
            transferred: true,
            raw,
        })
    }
}

#[async_trait]
impl PaperProofContentBackend for WalrusCliClient {
    async fn publish_content_backend(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value> {
        let cli = self.clone();
        let handle = tokio::task::spawn_blocking(move || cli.write_blob_sync(bytes, options));
        handle
            .await
            .map_err(|err| PaperProofError::network("walrus cli", err.to_string()))?
    }

    async fn read_content_backend(&self, blob_id: &str) -> Result<WalrusBlob> {
        WalrusClient::new(&self.aggregator_url, None)
            .read_blob(blob_id)
            .await
    }

    async fn read_and_verify_content_backend(
        &self,
        blob_id: &str,
        expected_sha256_hex: &str,
    ) -> Result<WalrusBlob> {
        WalrusClient::new(&self.aggregator_url, None)
            .read_and_verify_sha256(blob_id, expected_sha256_hex)
            .await
    }

    async fn extend_content_backend(
        &self,
        blob_object_id: &str,
        _options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        Err(PaperProofError::invalid_input(
            "walrus_cli_extend",
            format!(
                "Walrus CLI extend is not implemented by this adapter yet for {blob_object_id}"
            ),
        ))
    }

    async fn transfer_owned_content_backend(
        &self,
        blob_object_id: &str,
        _recipient: &str,
        _options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        Err(PaperProofError::invalid_input(
            "walrus_cli_transfer",
            format!(
                "Walrus CLI transfer is not implemented by this adapter yet for {blob_object_id}"
            ),
        ))
    }
}

impl WalrusCliClient {
    fn write_blob_sync(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value> {
        let path = temp_walrus_path()?;
        std::fs::write(&path, bytes)
            .map_err(|err| PaperProofError::network(path.display().to_string(), err.to_string()))?;
        let result = self.run_store_command(&path, options);
        let _ = std::fs::remove_file(&path);
        result
    }

    fn run_store_command(
        &self,
        path: &std::path::Path,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value> {
        let mut command = Command::new(&self.cli_path);
        command
            .arg("store")
            .arg(path)
            .arg("--epochs")
            .arg(options.epochs.to_string())
            .arg("--json");
        if options.share {
            command.arg("--share");
        }
        if options.deletable.unwrap_or(!options.share) {
            command.arg("--deletable");
        }
        for arg in &self.extra_store_args {
            command.arg(arg);
        }
        let output = command.output().map_err(|err| {
            PaperProofError::network(
                &self.cli_path,
                format!("failed to launch walrus CLI: {err}"),
            )
        })?;
        if !output.status.success() {
            return Err(PaperProofError::network(
                &self.cli_path,
                format!(
                    "walrus CLI exited with {}; stderr={}; stdout={}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&output.stdout)
                ),
            ));
        }
        serde_json::from_slice(&output.stdout).map_err(|err| {
            PaperProofError::network(
                &self.cli_path,
                format!(
                    "walrus CLI did not return JSON; error={err}; stdout={}; stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            )
        })
    }
}

fn temp_walrus_path() -> Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| PaperProofError::network("system_time", err.to_string()))?
        .as_nanos();
    Ok(std::env::temp_dir().join(format!("paperproof-walrus-{nanos}.bin")))
}

#[async_trait]
impl PaperProofContentBackend for WalrusClient {
    async fn publish_content_backend(
        &self,
        bytes: Vec<u8>,
        options: WalrusWriteOptions,
    ) -> Result<serde_json::Value> {
        self.write_blob_with_options(bytes, options).await
    }

    async fn read_content_backend(&self, blob_id: &str) -> Result<WalrusBlob> {
        self.read_blob(blob_id).await
    }

    async fn read_and_verify_content_backend(
        &self,
        blob_id: &str,
        expected_sha256_hex: &str,
    ) -> Result<WalrusBlob> {
        self.read_and_verify_sha256(blob_id, expected_sha256_hex)
            .await
    }

    async fn extend_content_backend(
        &self,
        blob_object_id: &str,
        options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        self.extend_blob(blob_object_id, options).await
    }

    async fn transfer_owned_content_backend(
        &self,
        blob_object_id: &str,
        recipient: &str,
        options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        self.transfer_blob(blob_object_id, recipient, options).await
    }
}

impl<B> PaperProofContentService<B>
where
    B: PaperProofContentBackend,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub async fn publish_content(
        &self,
        bytes: Vec<u8>,
        options: ContentPublishOptions,
    ) -> Result<ContentPublishResult> {
        let content_hash = format!("sha256:{}", sha256_hex(&bytes));
        let raw = self
            .backend
            .publish_content_backend(
                bytes,
                WalrusWriteOptions {
                    epochs: options.epochs,
                    share: options.share,
                    deletable: options.deletable,
                },
            )
            .await?;
        let (blob_id, blob_object_id) = parse_walrus_write_response(&raw)?;
        Ok(ContentPublishResult {
            blob_id,
            blob_object_id,
            content_hash,
            content_type: options.content_type,
            raw,
        })
    }

    pub async fn read_content(
        &self,
        blob_id: &str,
        expected_sha256_hex: Option<&str>,
    ) -> Result<ContentReadResult> {
        let blob = match expected_sha256_hex {
            Some(expected) => {
                self.backend
                    .read_and_verify_content_backend(blob_id, expected)
                    .await?
            }
            None => self.backend.read_content_backend(blob_id).await?,
        };
        let digest = blob.sha256_hex.clone();
        Ok(ContentReadResult {
            blob_id: blob.blob_id,
            bytes: blob.bytes,
            digest,
            verified: expected_sha256_hex
                .is_none_or(|expected| blob.sha256_hex.eq_ignore_ascii_case(expected)),
        })
    }

    pub async fn extend_content(
        &self,
        blob_object_id: &str,
        options: WalrusExtendOptions,
    ) -> Result<WalrusExtendResult> {
        self.backend
            .extend_content_backend(blob_object_id, options)
            .await
    }

    pub async fn transfer_owned_content(
        &self,
        blob_object_id: &str,
        recipient: &str,
        options: WalrusTransferOptions,
    ) -> Result<WalrusTransferResult> {
        self.backend
            .transfer_owned_content_backend(blob_object_id, recipient, options)
            .await
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn owner_address(value: &serde_json::Value) -> Option<&str> {
    let owner = value.get("owner").unwrap_or(value);
    if let Some(text) = owner.as_str()
        && text.starts_with("0x")
    {
        return Some(text);
    }
    owner
        .get("AddressOwner")
        .or_else(|| owner.get("ObjectOwner"))
        .or_else(|| owner.get("addressOwner"))
        .or_else(|| owner.get("owner"))
        .and_then(serde_json::Value::as_str)
}

pub fn is_shared_blob_object(value: &serde_json::Value) -> bool {
    if value.get("shared").and_then(serde_json::Value::as_bool) == Some(true) {
        return true;
    }
    let owner = value.get("owner").unwrap_or(value);
    if owner
        .as_str()
        .is_some_and(|text| text.eq_ignore_ascii_case("shared"))
    {
        return true;
    }
    owner.get("Shared").is_some()
        || owner.get("shared").is_some()
        || owner.get("Immutable").is_some()
}

pub fn assert_transferable_owned_blob(
    blob_object_id: &str,
    signer_address: Option<&str>,
    value: &serde_json::Value,
) -> Result<()> {
    if is_shared_blob_object(value) {
        return Err(PaperProofError::invalid_input(
            "blob_object_id",
            format!(
                "Walrus blob object {blob_object_id} is shared and cannot be transferred as an owned blob"
            ),
        ));
    }
    if let (Some(signer), Some(owner)) = (signer_address, owner_address(value))
        && !same_address(signer, owner)
    {
        return Err(PaperProofError::invalid_input(
            "blob_object_id",
            format!("signer {signer} is not the owner of Walrus blob object {blob_object_id}"),
        ));
    }
    Ok(())
}

fn same_address(left: &str, right: &str) -> bool {
    normalize_address(left) == normalize_address(right)
}

fn normalize_address(value: &str) -> String {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    format!("0x{}", raw.trim_start_matches('0').to_ascii_lowercase())
}

pub fn parse_walrus_write_response(raw: &serde_json::Value) -> Result<(String, Option<String>)> {
    let blob_info = raw
        .get("newlyCreated")
        .and_then(|value| value.get("blobObject"))
        .or_else(|| {
            raw.get("alreadyCertified")
                .and_then(|value| value.get("blobObject"))
        })
        .or_else(|| raw.get("blobObject"))
        .ok_or_else(|| {
            PaperProofError::invalid_input(
                "walrus_response",
                "Walrus write response does not contain blob object information",
            )
        })?;
    let blob_id = blob_info
        .get("blobId")
        .or_else(|| blob_info.get("blob_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            PaperProofError::invalid_input(
                "walrus_response",
                "Walrus write response does not contain blobId",
            )
        })?
        .to_string();
    let object_id = blob_info
        .get("id")
        .or_else(|| blob_info.get("objectId"))
        .or_else(|| blob_info.get("object_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    Ok((blob_id, object_id))
}
