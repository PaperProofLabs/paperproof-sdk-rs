// Copyright (c) 2026 PaperProof Labs
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use paperproof_sdk_rs::{
    AddBlobCommentInput, AddOnchainCommentInput, AddVersionInput, CliExecutionOptions,
    CommonContentInput, ExecutionMode, GenericFileInput, MetadataAttribute, PaperProofClient,
    PreprintInput, SetCommentStatusInput, SuiCliExecutor,
    constants::{comment_status, tree_status},
    events::{AddVersionResult, CommentResult, PublishResult},
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const OPT_IN_ENV: &str = "PAPERPROOF_RS_MAINNET_LARGE";
const DEFAULT_TARGET_TX: usize = 88;

#[derive(Clone, Debug)]
struct Account {
    key: String,
    address: String,
}

#[derive(Clone, Debug)]
struct SeriesContext {
    owner: Account,
    series_id: String,
    version_id: String,
    comments_tree_id: String,
    likes_book_id: String,
    first_comment_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
struct TxRecord {
    label: String,
    sender: String,
    digest: Option<String>,
    status_success: bool,
    expected_failure: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Report {
    run_id: String,
    mode: String,
    target_tx: usize,
    accounts: BTreeMap<String, String>,
    transactions: Vec<TxRecord>,
    series: Vec<ReportSeries>,
}

#[derive(Clone, Debug, Serialize)]
struct ReportSeries {
    owner: String,
    series_id: String,
    version_id: String,
    comments_tree_id: String,
    likes_book_id: String,
    first_comment_id: Option<u64>,
}

fn main() -> paperproof_sdk_rs::Result<()> {
    let args = Args::parse();
    if args.help {
        println!("{}", usage());
        return Ok(());
    }

    if args.execute && std::env::var(OPT_IN_ENV).ok().as_deref() != Some("1") {
        println!(
            "Refusing to write mainnet. Set {OPT_IN_ENV}=1 and pass --execute to run real writes."
        );
        println!("{}", usage());
        return Ok(());
    }

    let mode = if args.execute {
        ExecutionMode::Execute
    } else if args.dev_inspect {
        ExecutionMode::DevInspect
    } else {
        ExecutionMode::DryRun
    };
    let mode_label = match mode {
        ExecutionMode::Preview => "preview",
        ExecutionMode::DryRun => "dry-run",
        ExecutionMode::DevInspect => "dev-inspect",
        ExecutionMode::Execute => "execute",
    }
    .to_string();

    let env_path = find_contracts_env()?;
    let mut accounts = load_accounts(&env_path)?;
    if accounts.is_empty() {
        return Err(paperproof_sdk_rs::PaperProofError::invalid_input(
            "accounts",
            format!("no ADDR_1..ADDR_4 values found in {}", env_path.display()),
        ));
    }
    if !args.all_accounts
        && let Some(addr4) = accounts
            .iter()
            .find(|account| account.key == "ADDR_4")
            .cloned()
    {
        accounts = vec![addr4];
    }

    let client = PaperProofClient::mainnet();
    let deployment = client.deployment.clone();
    let executor = SuiCliExecutor::mainnet();
    let run_id = format!(
        "rs-large-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    );

    let mut runner = Runner {
        client,
        executor,
        mode,
        gas_budget: args.gas_budget,
        run_id: run_id.clone(),
        txs: Vec::new(),
    };

    println!(
        "PaperProof Rust SDK mainnet large integration: mode={mode_label} target_tx={} accounts={} run_id={run_id}",
        args.target_tx,
        accounts.len()
    );

    if !matches!(runner.mode, ExecutionMode::Execute) {
        let account = accounts[0].clone();
        let plan = runner
            .client
            .publishing
            .publish_preprint(&sample_preprint_input(&runner.run_id, &account, 0))?;
        runner.run(&account, "dry-run publish preprint", &plan)?;
        let report = Report {
            run_id,
            mode: mode_label,
            target_tx: args.target_tx,
            accounts: accounts
                .iter()
                .map(|account| (account.key.clone(), account.address.clone()))
                .collect(),
            transactions: runner.txs,
            series: Vec::new(),
        };
        let report_path = write_report(&report)?;
        println!(
            "dry-run/dev-inspect validated the first publish PTB; dependent steps need real event ids from --execute. report={}",
            report_path.display()
        );
        return Ok(());
    }

    let mut contexts = Vec::new();
    let publish_count = args.publish_count.max(2);
    for index in 0..publish_count {
        let account = accounts[index % accounts.len()].clone();
        let publish = if index % 2 == 0 {
            runner.publish_preprint(&account, index)?
        } else {
            runner.publish_generic_file(&account, index)?
        };
        println!(
            "published {} series={} tree={}",
            account.key, publish.series_id, publish.comments_tree_id
        );
        contexts.push(SeriesContext {
            owner: account,
            series_id: publish.series_id,
            version_id: publish.version_id,
            comments_tree_id: publish.comments_tree_id,
            likes_book_id: publish.likes_book_id,
            first_comment_id: None,
        });
    }

    for round in 0..args.version_rounds {
        for (index, context) in contexts.iter().enumerate() {
            let result = if index % 2 == 0 {
                runner.add_preprint_version(context, round)?
            } else {
                runner.add_generic_file_version(context, round)?
            };
            println!(
                "version added owner={} series={} version_id={}",
                context.owner.key, result.series_id, result.version_id
            );
        }
    }

    for round in 0..args.comment_rounds {
        for (index, context) in contexts.iter_mut().enumerate() {
            let parent = if round % 3 == 0 {
                0
            } else {
                context.first_comment_id.unwrap_or(0)
            };
            let commenter = &accounts[(index + round + 1) % accounts.len()];
            let comment = runner.add_onchain_comment(context, commenter, parent, round)?;
            if context.first_comment_id.is_none() {
                context.first_comment_id = Some(comment.comment_id);
            }
            println!(
                "comment sender={} tree={} comment_id={}",
                commenter.key, comment.tree_id, comment.comment_id
            );
        }
    }

    for round in 0..args.blob_comment_rounds {
        for (index, context) in contexts.iter_mut().enumerate() {
            let commenter = &accounts[(index + round + 2) % accounts.len()];
            let comment = runner.add_blob_comment(
                context,
                commenter,
                context.first_comment_id.unwrap_or(0),
                round,
            )?;
            println!(
                "blob comment sender={} tree={} comment_id={}",
                commenter.key, comment.tree_id, comment.comment_id
            );
        }
    }

    for round in 0..args.metadata_rounds {
        for context in &contexts {
            runner.update_series_metadata(context, round)?;
        }
    }

    for (index, context) in contexts.iter().enumerate() {
        if let Some(comment_id) = context.first_comment_id {
            runner.set_comment_status(context, comment_id, comment_status::HIDDEN, index)?;
            runner.set_comment_status(context, comment_id, comment_status::ACTIVE, index)?;
        }
    }

    for context in &contexts {
        runner.set_tree_status(context, tree_status::LOCKED)?;
        runner.set_tree_status(context, tree_status::OPEN)?;
    }

    while runner.txs.len() < args.target_tx {
        let index = runner.txs.len();
        let context_index = index % contexts.len();
        let commenter = &accounts[(index + 1) % accounts.len()];
        let context = &mut contexts[context_index];
        let parent = context.first_comment_id.unwrap_or(0);
        let comment = if index.is_multiple_of(4) {
            runner.add_blob_comment(context, commenter, parent, index)?
        } else {
            runner.add_onchain_comment(context, commenter, parent, index)?
        };
        if context.first_comment_id.is_none() {
            context.first_comment_id = Some(comment.comment_id);
        }
    }

    let report = Report {
        run_id,
        mode: mode_label,
        target_tx: args.target_tx,
        accounts: accounts
            .iter()
            .map(|account| (account.key.clone(), account.address.clone()))
            .collect(),
        transactions: runner.txs,
        series: contexts
            .iter()
            .map(|context| ReportSeries {
                owner: context.owner.key.clone(),
                series_id: context.series_id.clone(),
                version_id: context.version_id.clone(),
                comments_tree_id: context.comments_tree_id.clone(),
                likes_book_id: context.likes_book_id.clone(),
                first_comment_id: context.first_comment_id,
            })
            .collect(),
    };
    let report_path = write_report(&report)?;
    println!(
        "completed {} transactions; report={}",
        report.transactions.len(),
        report_path.display()
    );
    for tx in report
        .transactions
        .iter()
        .filter_map(|tx| tx.digest.as_ref())
        .take(12)
    {
        println!("https://suivision.xyz/txblock/{tx}");
    }

    if !args.execute {
        println!("Dry run/dev-inspect completed. Re-run with {OPT_IN_ENV}=1 --execute for writes.");
    }

    let _ = deployment;
    Ok(())
}

struct Runner {
    client: PaperProofClient,
    executor: SuiCliExecutor,
    mode: ExecutionMode,
    gas_budget: u64,
    run_id: String,
    txs: Vec<TxRecord>,
}

impl Runner {
    fn publish_preprint(
        &mut self,
        account: &Account,
        index: usize,
    ) -> paperproof_sdk_rs::Result<PublishResult> {
        let plan = self.client.publishing.publish_preprint(&PreprintInput {
            title: format!(
                "PaperProof Rust SDK mainnet integration preprint {index} {}",
                self.run_id
            ),
            abstract_text: format!(
                "A Rust SDK mainnet integration artifact created by explicit opt-in test run {}.",
                self.run_id
            ),
            authors: vec!["PaperProof Labs".to_string(), account.key.clone()],
            keywords: vec![
                "paperproof".to_string(),
                "rust-sdk".to_string(),
                "mainnet".to_string(),
            ],
            field: "computer science".to_string(),
            license: "CC-BY-4.0".to_string(),
            page_count: 1 + index as u64,
            content: self.content("preprint", index, 0),
            series_metadata: self.metadata("series-kind", "preprint", index, 0),
            version_metadata: self.metadata("version-kind", "initial", index, 0),
            payment_coin_id: None,
        })?;
        let output = self.run(account, "publish preprint", &plan)?;
        output.publish_result(&self.client.deployment)
    }

    fn publish_generic_file(
        &mut self,
        account: &Account,
        index: usize,
    ) -> paperproof_sdk_rs::Result<PublishResult> {
        let plan = self
            .client
            .publishing
            .publish_generic_file(&GenericFileInput {
                title: format!(
                    "PaperProof Rust SDK mainnet integration file {index} {}",
                    self.run_id
                ),
                description:
                    "A small deterministic generic-file record for Rust SDK integration coverage."
                        .to_string(),
                filename: format!("paperproof-rs-mainnet-{index}.txt"),
                file_size: 128 + index as u64,
                license: "Apache-2.0".to_string(),
                content: self.content("generic", index, 0),
                series_metadata: self.metadata("series-kind", "generic-file", index, 0),
                version_metadata: self.metadata("version-kind", "initial", index, 0),
                payment_coin_id: None,
            })?;
        let output = self.run(account, "publish generic file", &plan)?;
        output.publish_result(&self.client.deployment)
    }

    fn add_preprint_version(
        &mut self,
        context: &SeriesContext,
        round: usize,
    ) -> paperproof_sdk_rs::Result<AddVersionResult> {
        let plan = self
            .client
            .publishing
            .add_preprint_version(&AddVersionInput {
                series_id: context.series_id.clone(),
                body: PreprintInput {
                    title: format!(
                        "PaperProof Rust SDK versioned preprint {round} {}",
                        self.run_id
                    ),
                    abstract_text: format!(
                        "Version {round} added during Rust SDK mainnet integration."
                    ),
                    authors: vec!["PaperProof Labs".to_string(), context.owner.key.clone()],
                    keywords: vec!["paperproof".to_string(), "version".to_string()],
                    field: "computer science".to_string(),
                    license: "CC-BY-4.0".to_string(),
                    page_count: 2 + round as u64,
                    content: self.content("preprint-version", self.txs.len(), round),
                    series_metadata: Vec::new(),
                    version_metadata: self.metadata(
                        "version-round",
                        &round.to_string(),
                        self.txs.len(),
                        round,
                    ),
                    payment_coin_id: None,
                },
            })?;
        let output = self.run(&context.owner, "add preprint version", &plan)?;
        output.add_version_result(&self.client.deployment)
    }

    fn add_generic_file_version(
        &mut self,
        context: &SeriesContext,
        round: usize,
    ) -> paperproof_sdk_rs::Result<AddVersionResult> {
        let plan = self
            .client
            .publishing
            .add_generic_file_version(&AddVersionInput {
                series_id: context.series_id.clone(),
                body: GenericFileInput {
                    title: format!(
                        "PaperProof Rust SDK generic file update {round} {}",
                        self.run_id
                    ),
                    description:
                        "A follow-up generic-file record for Rust SDK integration coverage."
                            .to_string(),
                    filename: format!("paperproof-rs-mainnet-update-{round}.txt"),
                    file_size: 256 + round as u64,
                    license: "Apache-2.0".to_string(),
                    content: self.content("generic-version", self.txs.len(), round),
                    series_metadata: Vec::new(),
                    version_metadata: self.metadata(
                        "version-round",
                        &round.to_string(),
                        self.txs.len(),
                        round,
                    ),
                    payment_coin_id: None,
                },
            })?;
        let output = self.run(&context.owner, "add generic file version", &plan)?;
        output.add_version_result(&self.client.deployment)
    }

    fn add_onchain_comment(
        &mut self,
        context: &SeriesContext,
        account: &Account,
        parent_comment_id: u64,
        round: usize,
    ) -> paperproof_sdk_rs::Result<CommentResult> {
        let plan = self
            .client
            .comments
            .add_onchain_comment(&AddOnchainCommentInput {
                tree_id: context.comments_tree_id.clone(),
                parent_comment_id,
                content: format!(
                    "Rust SDK mainnet comment run={} round={} tx={}",
                    self.run_id,
                    round,
                    self.txs.len()
                )
                .into_bytes(),
                payment_coin_id: None,
            })?;
        let output = self.run(account, "add onchain comment", &plan)?;
        output.comment_result(&self.client.deployment)
    }

    fn add_blob_comment(
        &mut self,
        context: &SeriesContext,
        account: &Account,
        parent_comment_id: u64,
        round: usize,
    ) -> paperproof_sdk_rs::Result<CommentResult> {
        let digest = hash_hex(&format!(
            "{}:{}:{}:{}",
            self.run_id, context.series_id, account.address, round
        ));
        let plan = self
            .client
            .comments
            .add_blob_comment(&AddBlobCommentInput {
                tree_id: context.comments_tree_id.clone(),
                parent_comment_id,
                blob_id: format!("rs-mainnet-blob-comment-{}-{round}", self.run_id).into_bytes(),
                blob_object_id: None,
                blob_digest: digest.as_bytes()[0..64].to_vec(),
                preview: format!("Rust SDK blob comment preview round {round}").into_bytes(),
                payment_coin_id: None,
            })?;
        let output = self.run(account, "add blob comment", &plan)?;
        output.comment_result(&self.client.deployment)
    }

    fn update_series_metadata(
        &mut self,
        context: &SeriesContext,
        round: usize,
    ) -> paperproof_sdk_rs::Result<()> {
        let plan = self.client.publishing.update_series_metadata(
            &context.series_id,
            self.metadata("series-update", &round.to_string(), self.txs.len(), round),
        )?;
        self.run(&context.owner, "update series metadata", &plan)?;
        Ok(())
    }

    fn set_comment_status(
        &mut self,
        context: &SeriesContext,
        comment_id: u64,
        status: u8,
        index: usize,
    ) -> paperproof_sdk_rs::Result<()> {
        let plan = self
            .client
            .comments
            .set_comment_status(&SetCommentStatusInput {
                tree_id: context.comments_tree_id.clone(),
                comment_id,
                status,
            })?;
        self.run(
            &context.owner,
            &format!("set comment status {status} #{index}"),
            &plan,
        )?;
        Ok(())
    }

    fn set_tree_status(
        &mut self,
        context: &SeriesContext,
        status: u8,
    ) -> paperproof_sdk_rs::Result<()> {
        let plan = self
            .client
            .comments
            .set_tree_status(&context.comments_tree_id, status)?;
        self.run(&context.owner, &format!("set tree status {status}"), &plan)?;
        Ok(())
    }

    fn run(
        &mut self,
        account: &Account,
        label: &str,
        plan: &paperproof_sdk_rs::TransactionPlan,
    ) -> paperproof_sdk_rs::Result<paperproof_sdk_rs::CliExecutionOutput> {
        let options = CliExecutionOptions {
            sender: Some(account.address.clone()),
            gas_budget: Some(self.gas_budget),
            mode: self.mode.clone(),
            ..Default::default()
        };

        let mut last_error = None;
        for attempt in 1..=3 {
            match self.executor.run(plan, &options) {
                Ok(output) => {
                    let digest = output.digest.clone();
                    let status_success = output.status_success;
                    self.txs.push(TxRecord {
                        label: label.to_string(),
                        sender: account.key.clone(),
                        digest,
                        status_success,
                        expected_failure: false,
                        error: None,
                    });
                    if matches!(self.mode, ExecutionMode::Execute) {
                        thread::sleep(Duration::from_millis(900));
                    }
                    return Ok(output);
                }
                Err(err) if attempt < 3 => {
                    last_error = Some(err.to_string());
                    thread::sleep(Duration::from_millis(1_500 * attempt));
                }
                Err(err) => {
                    self.txs.push(TxRecord {
                        label: label.to_string(),
                        sender: account.key.clone(),
                        digest: None,
                        status_success: false,
                        expected_failure: false,
                        error: Some(err.to_string()),
                    });
                    return Err(err);
                }
            }
        }

        Err(paperproof_sdk_rs::PaperProofError::TransactionExecution {
            message: last_error.unwrap_or_else(|| "transaction failed".to_string()),
        })
    }

    fn content(&self, label: &str, index: usize, round: usize) -> CommonContentInput {
        let digest = hash_hex(&format!("{}:{label}:{index}:{round}", self.run_id));
        CommonContentInput {
            content_hash: format!("sha256:{digest}"),
            walrus_blob_id: format!("{label}-{index}-{round}-{}", &digest[0..24]),
            walrus_blob_object_id: format!("0x{digest}"),
            content_type: "text/plain".to_string(),
        }
    }

    fn metadata(
        &self,
        key: &str,
        value: &str,
        index: usize,
        round: usize,
    ) -> Vec<MetadataAttribute> {
        vec![
            MetadataAttribute {
                key: key.to_string(),
                value: value.to_string(),
            },
            MetadataAttribute {
                key: "run".to_string(),
                value: self.run_id.clone(),
            },
            MetadataAttribute {
                key: "case".to_string(),
                value: format!("{index}-{round}"),
            },
        ]
    }
}

#[derive(Clone, Debug)]
struct Args {
    execute: bool,
    dev_inspect: bool,
    help: bool,
    target_tx: usize,
    publish_count: usize,
    version_rounds: usize,
    comment_rounds: usize,
    blob_comment_rounds: usize,
    metadata_rounds: usize,
    gas_budget: u64,
    all_accounts: bool,
}

impl Args {
    fn parse() -> Self {
        let mut args = Self {
            execute: false,
            dev_inspect: false,
            help: false,
            target_tx: DEFAULT_TARGET_TX,
            publish_count: 8,
            version_rounds: 2,
            comment_rounds: 3,
            blob_comment_rounds: 1,
            metadata_rounds: 1,
            gas_budget: 30_000_000,
            all_accounts: false,
        };
        for arg in std::env::args().skip(1) {
            if arg == "--execute" {
                args.execute = true;
            } else if arg == "--dev-inspect" {
                args.dev_inspect = true;
            } else if arg == "--help" || arg == "-h" {
                args.help = true;
            } else if arg == "--all-accounts" {
                args.all_accounts = true;
            } else if let Some(value) = arg.strip_prefix("--target-tx=") {
                args.target_tx = value.parse().unwrap_or(DEFAULT_TARGET_TX);
            } else if let Some(value) = arg.strip_prefix("--publish-count=") {
                args.publish_count = value.parse().unwrap_or(args.publish_count);
            } else if let Some(value) = arg.strip_prefix("--version-rounds=") {
                args.version_rounds = value.parse().unwrap_or(args.version_rounds);
            } else if let Some(value) = arg.strip_prefix("--comment-rounds=") {
                args.comment_rounds = value.parse().unwrap_or(args.comment_rounds);
            } else if let Some(value) = arg.strip_prefix("--blob-comment-rounds=") {
                args.blob_comment_rounds = value.parse().unwrap_or(args.blob_comment_rounds);
            } else if let Some(value) = arg.strip_prefix("--metadata-rounds=") {
                args.metadata_rounds = value.parse().unwrap_or(args.metadata_rounds);
            } else if let Some(value) = arg.strip_prefix("--gas-budget=") {
                args.gas_budget = value.parse().unwrap_or(args.gas_budget);
            }
        }
        args
    }
}

fn usage() -> String {
    format!(
        r#"PaperProof Rust SDK large mainnet integration example

Usage:
  cargo run --example mainnet_large_integration
  $env:{OPT_IN_ENV}='1'; cargo run --example mainnet_large_integration -- --execute --target-tx=88

Default mode is dry-run. Real writes require both {OPT_IN_ENV}=1 and --execute.
The script reads ADDR_1..ADDR_4 from paperproof-contracts/jstest/.env and
uses ADDR_4 by default because it is the custody account with enough gas. Pass
--all-accounts to rotate ADDR_1..ADDR_4 when those Sui CLI keys all have gas.
It publishes artifacts, adds versions, adds on-chain and blob comments, updates
metadata, and locks/unlocks comment trees. PPRF is not transferred or split by
this example.
"#
    )
}

fn find_contracts_env() -> paperproof_sdk_rs::Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(|err| {
        paperproof_sdk_rs::PaperProofError::invalid_input(
            "cwd",
            format!("failed to read current directory: {err}"),
        )
    })?;
    let candidates: Vec<PathBuf> = vec![
        cwd.join("..")
            .join("paperproof-contracts")
            .join("jstest")
            .join(".env"),
        cwd.join("..")
            .join("..")
            .join("paperproof-contracts")
            .join("jstest")
            .join(".env"),
        PathBuf::from(r"D:\Works\VscodeProject\PaperProofLabs\paperproof-contracts\jstest\.env"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            paperproof_sdk_rs::PaperProofError::invalid_input(
                "env_path",
                "could not find paperproof-contracts/jstest/.env",
            )
        })
}

fn load_accounts(path: &Path) -> paperproof_sdk_rs::Result<Vec<Account>> {
    let text = fs::read_to_string(path).map_err(|err| {
        paperproof_sdk_rs::PaperProofError::invalid_input(
            "env_path",
            format!("failed to read {}: {err}", path.display()),
        )
    })?;
    let env = parse_env(&text);
    let mut accounts = Vec::new();
    for index in 1..=4 {
        let key = format!("ADDR_{index}");
        if let Some(address) = env.get(&key) {
            accounts.push(Account {
                key,
                address: normalize_address(address),
            });
        }
    }
    Ok(accounts)
}

fn parse_env(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((
                key.trim().to_string(),
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            ))
        })
        .collect()
}

fn normalize_address(value: &str) -> String {
    let raw = value.trim().to_ascii_lowercase();
    let no_prefix = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix('x'))
        .unwrap_or(&raw);
    format!("0x{no_prefix:0>64}")
}

fn hash_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn sample_preprint_input(run_id: &str, account: &Account, index: usize) -> PreprintInput {
    let digest = hash_hex(&format!("{run_id}:dry-run-preprint:{index}"));
    PreprintInput {
        title: format!("PaperProof Rust SDK dry-run preprint {index} {run_id}"),
        abstract_text: "A Rust SDK dry-run artifact used to validate transaction construction."
            .to_string(),
        authors: vec!["PaperProof Labs".to_string(), account.key.clone()],
        keywords: vec!["paperproof".to_string(), "rust-sdk".to_string()],
        field: "computer science".to_string(),
        license: "CC-BY-4.0".to_string(),
        page_count: 1,
        content: CommonContentInput {
            content_hash: format!("sha256:{digest}"),
            walrus_blob_id: format!("dry-run-{index}-{}", &digest[0..24]),
            walrus_blob_object_id: format!("0x{digest}"),
            content_type: "text/plain".to_string(),
        },
        series_metadata: vec![MetadataAttribute {
            key: "run".to_string(),
            value: run_id.to_string(),
        }],
        version_metadata: vec![MetadataAttribute {
            key: "kind".to_string(),
            value: "dry-run".to_string(),
        }],
        payment_coin_id: None,
    }
}

fn write_report(report: &Report) -> paperproof_sdk_rs::Result<PathBuf> {
    let dir = PathBuf::from("examples").join("artifacts");
    fs::create_dir_all(&dir).map_err(|err| {
        paperproof_sdk_rs::PaperProofError::invalid_input(
            "report_dir",
            format!("failed to create {}: {err}", dir.display()),
        )
    })?;
    let path = dir.join(format!("mainnet-large-integration-{}.json", report.run_id));
    let text = serde_json::to_string_pretty(report)?;
    fs::write(&path, text).map_err(|err| {
        paperproof_sdk_rs::PaperProofError::invalid_input(
            "report_path",
            format!("failed to write {}: {err}", path.display()),
        )
    })?;
    Ok(path)
}
